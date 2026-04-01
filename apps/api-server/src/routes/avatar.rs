/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use avatar_engine::proportions::{AvatarDimensions, ProportionsChecker};
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
        return Err(aiome_contracts::error::AiomeError::SecurityViolation {
            reason: "eKYC verification required for custom asset upload".to_string(),
        }
        .into());
    }

    // 2. 画像知覚ハッシュの検証 (CSAM)
    let content_bytes = base64::engine::general_purpose::STANDARD
        .decode(&req.content_base64)
        .map_err(|_| {
            AppError(aiome_contracts::error::AiomeError::Infrastructure {
                reason: "Invalid base64 encoding".to_string(),
            })
        })?;

    // NOTE: ImageHasher は非 Send なため、await ポイント前にドロップさせるためブロックで囲う
    let (hash, is_csam_hit) = {
        let hasher = ImageHasher::new();
        let h = hasher.compute_hash(&content_bytes).map_err(|e| {
            AppError(aiome_contracts::error::AiomeError::Infrastructure {
                reason: format!("Hash processing error: {}", e),
            })
        })?;
        let hit = hasher.is_blacklisted(&h);
        (h, hit)
    };

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

/// ユーザー自身の eKYC ステータスを取得・セッション作成
#[utoipa::path(
    get,
    path = "/api/avatar/ekyc-status",
    responses(
        (status = 200, description = "eKYC の現在のステータスとセッション URL", body = serde_json::Value)
    )
)]
pub async fn get_ekyc_status_handler(
    State(state): State<AppState>,
    axum::extract::Extension(user): axum::extract::Extension<crate::auth::AuthenticatedUser>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let session = state
        .ekyc_engine
        .create_verification_session(&user.0.sub)
        .await?;
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

    Ok((
        headers,
        Json(serde_json::json!({
            "verified": verified,
            "session_url": session.url,
            "session_id": session.session_id,
        })),
    ))
}
