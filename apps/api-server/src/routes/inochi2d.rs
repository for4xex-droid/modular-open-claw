/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::auth::AuthenticatedUser;
use crate::error::AppError;
use crate::AppState;
use avatar_engine::loader::Inochi2dLoader;
use axum::{body::Bytes, extract::State, Json};
use infrastructure::registry::{AssetManifest, AssetType};
use serde::Serialize;
use shared::sandbox::PathSandbox;
use tracing::{error, info, warn};

#[derive(Serialize)]
pub struct Inochi2dUploadResponse {
    pub asset_id: uuid::Uuid,
    pub model_url: String,
}

/// Inochi2D (INX) モデルをアップロード
pub async fn upload_inochi2d_handler(
    State(state): State<AppState>,
    axum::extract::Extension(user): axum::extract::Extension<AuthenticatedUser>,
    body: Bytes,
) -> Result<Json<Inochi2dUploadResponse>, AppError> {
    info!(
        "🎭 [Inochi2D] Uploading mascot model for user: {}",
        user.0.sub
    );

    // 1. eKYC Gate: 年齢確認済みかチェック
    let is_verified = user.0.ekyc_verified
        || state
            .ekyc_engine
            .check_status(&user.0.sub)
            .await
            .unwrap_or(false);

    if !is_verified {
        warn!(
            "⛔ [Inochi2D] Unverified user {} attempted upload. Blocked.",
            user.0.sub
        );
        return Err(AppError::forbidden(
            "eKYC verification required for mascot upload",
        ));
    }

    // 2. Format Validation
    let _metadata = Inochi2dLoader::load_metadata(&body)
        .map_err(|e| AppError::bad_request(format!("Invalid INX file: {}", e)))?;

    // 3. Save to File System (.inochi2d_assets/)
    let asset_id = uuid::Uuid::new_v4();
    let filename = format!("{}.inx", asset_id);

    let base_dir = std::path::PathBuf::from(".inochi2d_assets");
    if !base_dir.exists() {
        std::fs::create_dir_all(&base_dir)
            .map_err(|e| AppError::internal(format!("Failed to create directory: {}", e)))?;
    }

    // Initialize Sandbox (Expert Review v3)
    let sandbox = PathSandbox::new(&base_dir)
        .map_err(|e| AppError::internal(format!("Failed to initialize sandbox: {}", e)))?;

    let save_path = sandbox
        .validate_path(&filename)
        .map_err(|e| AppError::bad_request(format!("Path traversal blocked: {}", e)))?;

    tokio::fs::write(&save_path, &body).await.map_err(|e| {
        error!("🚨 [Inochi2D] Failed to save INX file: {}", e);
        AppError::internal("Failed to save model file")
    })?;

    // 4. Registry 登録
    let manifest = AssetManifest {
        id: asset_id,
        creator_id: user.0.agent_id,
        asset_type: AssetType::Inochi2D,
        name: filename.clone(),
        description: "Uploaded Inochi2D Model".to_string(),
        price_coins: 0,
    };

    state
        .registry
        .register_asset(manifest)
        .await
        .map_err(|e| AppError::internal(format!("Registry error: {}", e)))?;

    Ok(Json(Inochi2dUploadResponse {
        asset_id,
        model_url: format!("/api/artifacts/inochi2d/{}", filename),
    }))
}
