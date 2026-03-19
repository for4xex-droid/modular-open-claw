/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use shared::csam::{ImageHasher, LegalStatus, ProportionsChecker};
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
    Json(req): Json<AvatarAssetRequest>,
) -> Result<Json<AvatarVerificationResult>, AppError> {
    info!("📤 [Avatar] Processing custom asset: {}", req.name);

    // 1. eKYC 年齢確認のチェック
    // TODO: 実際のユーザー ID を取得
    let identity_verified = state
        .ekyc_engine
        .check_status("session_dummy")
        .await
        .unwrap_or(false);
    if !identity_verified {
        warn!("⚠️ [Avatar] User is NOT identity verified. Upload blocked.");
    }

    // 2. 画像知覚ハッシュの検証 (CSAM)
    let content_bytes = base64::engine::general_purpose::STANDARD
        .decode(&req.content_base64)
        .map_err(|_| {
            AppError(aiome_contracts::error::AiomeError::Infrastructure {
                reason: "Invalid base64 encoding".to_string(),
            })
        })?;

    let hasher = ImageHasher::new();
    let hash = hasher.compute_hash(&content_bytes).map_err(|e| {
        AppError(aiome_contracts::error::AiomeError::Infrastructure {
            reason: format!("Hash processing error: {}", e),
        })
    })?;

    let is_csam_hit = hasher.is_blacklisted(&hash);
    let image_status = if is_csam_hit {
        "BLACKLISTED".to_string()
    } else {
        "CLEAN".to_string()
    };

    // 3. 頭身チェック (5.5頭身)
    let legal_status = if let (Some(hh), Some(th)) = (req.head_height, req.total_height) {
        ProportionsChecker::verify_proportions(hh, th)
    } else {
        LegalStatus::Pending
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
) -> Result<Json<serde_json::Value>, AppError> {
    // 開発用ダミー ID
    let user_id = "user_001";
    let session_url = state
        .ekyc_engine
        .create_verification_session(user_id)
        .await?;
    let verified = state
        .ekyc_engine
        .check_status("session_dummy")
        .await
        .unwrap_or(false);

    Ok(Json(serde_json::json!({
        "verified": verified,
        "session_url": session_url,
    })))
}
