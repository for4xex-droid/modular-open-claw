/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use avatar_engine::proportions::ProportionsChecker;
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use shared::csam::{ImageHasher, LegalStatus};
use tracing::{error, info, warn};

#[derive(Deserialize, Serialize, utoipa::ToSchema)]
pub struct AvatarAssetRequest {
    pub name: String,
    pub content_base64: String,
    pub head_height: Option<f32>,
    pub total_height: Option<f32>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct AvatarVerificationResult {
    pub identity_verified: bool,
    pub image_status: String,
    pub legal_status: LegalStatus,
    pub overall_safe: bool,
}

/// アバターアセットのアップロードと 3 層検証
#[utoipa::path(
    post,
    path = "/api/avatar/upload",
    request_body = AvatarAssetRequest,
    responses(
        (status = 200, description = "検証結果", body = AvatarVerificationResult),
        (status = 400, description = "不正なデータ")
    )
)]
pub async fn upload_avatar_handler(
    State(state): State<AppState>,
    axum::extract::Extension(user): axum::extract::Extension<crate::auth::AuthenticatedUser>,
    Json(req): Json<AvatarAssetRequest>,
) -> Result<Json<AvatarVerificationResult>, AppError> {
    let safe_len = req.name.len().clamp(0, 1024);
    if req.name.len() != safe_len {
        return Err(AppError::bad_request("Name too long"));
    }

    info!(
        "📤 [Avatar] Processing custom asset: {} for user: {}",
        req.name, user.0.sub
    );

    // 1. eKYC 年齢確認のチェック
    let identity_verified = user.0.ekyc_verified
        || state
            .ekyc_engine
            .check_status(&user.0.sub)
            .await
            .unwrap_or(false);
    if !identity_verified {
        warn!(
            "⚠️ [Avatar] User {} is NOT identity verified. Upload blocked.",
            user.0.sub
        );
        return Err(aiome_core_contracts::error::AiomeError::SecurityViolation {
            reason: "eKYC verification required for custom asset upload".to_string(),
        }
        .into());
    }

    // 2. 画像知覚ハッシュの検証 (CSAM)
    let content_bytes = base64::engine::general_purpose::STANDARD
        .decode(&req.content_base64)
        .map_err(|_| {
            AppError(aiome_core_contracts::error::AiomeError::Infrastructure {
                reason: "Invalid base64 encoding".to_string(),
            })
        })?;

    // P-1: Validate magic bytes to prevent CWE-434/polyglot attacks
    let ext = std::path::Path::new(&req.name)
        .extension()
        .and_then(|ex| ex.to_str())
        .unwrap_or("");
    if let Err(_e) = shared::file_validator::validate_magic_bytes(ext, &content_bytes) {
        return Err(AppError::bad_request(
            "Invalid image file signature. File may be spoofed.",
        ));
    }

    // NOTE: check_blacklist はDBアクセスを含むため await が必要。
    // compute_hash は同期処理でブロックする可能性があるためブロックで囲う
    let hash = {
        let hasher = ImageHasher::new();
        hasher
            .compute_hash(&content_bytes)
            .map_err(|e| AppError::internal(format!("Hash processing error: {}", e)))?
    };
    let is_csam_hit = ImageHasher::check_blacklist(state.db_pool.get_inner(), &hash)
        .await
        .map_err(|e| {
            error!("🚨 [Avatar] CSAM Blacklist DB check failed: {}", e);
            AppError::internal("Compliance verification service unavailable")
        })?;

    let image_status = if is_csam_hit {
        "BLACKLISTED".to_string()
    } else {
        "CLEAN".to_string()
    };

    // 3. 頭身チェック (5.5頭身) - ユーザー申告ではなくバイナリから直接抽出 (G-22)
    let dimensions = ProportionsChecker::extract_from_binary(&content_bytes);
    let legal_status = match dimensions {
        Ok(dim) => match ProportionsChecker::validate(&dim) {
            Ok(_) => LegalStatus::General,
            Err(avatar_engine::proportions::ProportionError::TooYoung(_)) => {
                LegalStatus::Restricted
            }
            Err(_) => LegalStatus::Pending,
        },
        Err(_) => LegalStatus::Pending,
    };

    let overall_safe = identity_verified && !is_csam_hit && legal_status != LegalStatus::Restricted;

    // 非安全なアセットを検疫所に保存
    if !overall_safe {
        let reason = if is_csam_hit {
            infrastructure::compliance::AssetReason::CsamHit
        } else if legal_status == LegalStatus::Restricted {
            infrastructure::compliance::AssetReason::RestrictedProportions
        } else {
            infrastructure::compliance::AssetReason::EkycFailed
        };

        if let Err(e) = state
            .quarantine_store
            .quarantine_asset(&req.name, &hash, reason)
            .await
        {
            error!("🚨 [Avatar] Failed to quarantine asset {}: {}", req.name, e);
        }
    }

    Ok(Json(AvatarVerificationResult {
        identity_verified,
        image_status,
        legal_status,
        overall_safe,
    }))
}

/// ユーザー自身の eKYC ステータスを取得
///
/// U0-B3: セッション作成は `POST /api/v1/ekyc/session` に分離済み。
/// status 確認のたびに Stripe Identity セッションを新規作成していた副作用
/// （実 API 呼び出しによる遅延・失敗・セッション浪費）を除去した。
#[utoipa::path(
    get,
    path = "/api/avatar/ekyc-status",
    responses(
        (status = 200, description = "eKYC の現在の検証ステータス", body = serde_json::Value)
    )
)]
pub async fn get_ekyc_status_handler(
    State(state): State<AppState>,
    axum::extract::Extension(user): axum::extract::Extension<crate::auth::AuthenticatedUser>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let verified = user.0.ekyc_verified
        || state
            .ekyc_engine
            .check_status(&user.0.sub)
            .await
            .unwrap_or(false);

    let headers = [
        ("Deprecated", "true"),
        ("Sunset", "2026-06-01"),
        ("Link", "</api/v1/ekyc/session>; rel=\"successor-version\""),
    ];

    Ok((headers, Json(serde_json::json!({ "verified": verified }))))
}

/// Inochi2D アセットの安全な配信（パス・トラバーサル防御・CORS対応）
///
/// **Deprecated (Phase E E5)**: Inochi product-frozen; route retained for PathSandbox tests / compat.
#[utoipa::path(
    get,
    path = "/api/v1/avatar/inochi2d/{filename}",
    params(
        ("filename" = String, Path, description = "アセットのファイル名")
    ),
    responses(
        (status = 200, description = "アセットバイナリ", content_type = "application/octet-stream"),
        (status = 400, description = "不正なパスリクエスト"),
        (status = 404, description = "ファイルが見つからない")
    )
)]
// auth-exempt: 静的アセット配信
pub async fn serve_inochi2d_asset(
    axum::extract::Path(filename): axum::extract::Path<String>,
) -> Result<axum::response::Response, AppError> {
    info!("GET Inochi2D asset: {}", filename);

    // 1. セキュリティ制約: .inx 拡張子のみ許可
    if !filename.ends_with(".inx") {
        warn!("🚨 [Avatar] Rejected non-inx file request: {}", filename);
        return Err(AppError::bad_request("Only .inx files are allowed"));
    }

    // 2. パストラバーサル防止のためのサンドボックス（Jail）
    // NOTE: 実際のパスは環境変数や設定から取得すべきだが、ここでは実行ディレクトリ下の .inochi2d_assets とする
    let asset_dir = std::path::Path::new(".inochi2d_assets");
    if !asset_dir.exists() {
        tokio::fs::create_dir_all(asset_dir).await.map_err(|e| {
            error!("🚨 [Avatar] Failed to create asset directory: {}", e);
            AppError::internal("Asset directory initialization failed")
        })?;
    }

    let sandbox = shared::sandbox::PathSandbox::new(asset_dir)
        .map_err(|e| AppError::internal(format!("Sandbox initialization failed: {}", e)))?;

    // validate_pathは トラバーサル(..) や絶対パスを弾く
    let safe_path = match sandbox.validate_path(&filename) {
        Ok(p) => p,
        Err(e) => {
            error!("🚨 [Avatar] Path Sandbox violation: {}", e);
            return Err(AppError::bad_request("Path traversal detected"));
        }
    };

    if !safe_path.exists() || !safe_path.is_file() {
        return Err(AppError::not_found("Asset not found"));
    }

    // 3. ファイルの読み込みとレスポンス構築
    let body = tokio::fs::read(&safe_path)
        .await
        .map_err(|e| AppError::internal(format!("Failed to read asset: {}", e)))?;

    // CORS & MIME Headers
    let response = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "application/octet-stream")
        .header(
            axum::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .header(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*") // Management console needs access
        .body(axum::body::Body::from(body))
        .map_err(|e| AppError::internal(format!("Response build failed: {}", e)))?;

    Ok(response)
}
