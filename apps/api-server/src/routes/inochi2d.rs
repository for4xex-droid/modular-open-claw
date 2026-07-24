/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 *
 * DEPRECATED (Phase E E5, 2026-07-25): Inochi2D upload is product-frozen.
 * Route kept for compatibility / tests; do not expand. Removal needs explicit approval.
 * Shipping avatars: 2D images + 3D (VRM/GLB). Live2D = Phase F (separate stack).
 */

use crate::auth::AuthenticatedUser;
use crate::error::AppError;
use crate::AppState;
use avatar_engine::loader::Inochi2dLoader;
use axum::{extract::State, Json};
use infrastructure::registry::{AssetManifest, AssetType};
use serde::Serialize;
use shared::sandbox::PathSandbox;
use tokio::io::AsyncWriteExt;
use tracing::{error, info, warn};

#[derive(Serialize, utoipa::ToSchema)]
pub struct Inochi2dUploadResponse {
    pub asset_id: uuid::Uuid,
    pub model_url: String,
}

/// Inochi2D (INX) モデルをアップロード
///
/// **Deprecated (Phase E E5)**: product-frozen; not exposed in Mission Control UI.
#[utoipa::path(
    post,
    path = "/api/v1/mascot/inochi2d",
    request_body(content = String, description = "multipart/form-data with `file` field", content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Inochi2D upload success", body = Inochi2dUploadResponse),
        (status = 403, description = "Forbidden: eKYC verification required"),
        (status = 400, description = "Bad Request: Invalid file")
    ),
    security(
        ("jwt" = [])
    )
)]
pub async fn upload_inochi2d_handler(
    State(state): State<AppState>,
    axum::extract::Extension(user): axum::extract::Extension<AuthenticatedUser>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<Inochi2dUploadResponse>, AppError> {
    warn!(
        "⚠️ [Inochi2D] DEPRECATED upload invoked (Phase E E5 freeze). user={}",
        user.0.sub
    );
    let safe_id = user.0.sub.trim();
    if !safe_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::bad_request("Invalid user ID format"));
    }

    // F-03: Acquire upload semaphore permit to prevent OOM/DoS from large concurrent uploads
    let _permit = state.upload_semaphore.try_acquire().map_err(|e| {
        crate::error::AppError(aiome_core::error::AiomeError::ResourceBusy {
            reason: format!("System busy: {}", e),
        })
    })?;

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
    let temp_file = tokio::task::spawn_blocking(tempfile::NamedTempFile::new)
        .await
        .map_err(|e| AppError::internal(format!("Tokio spawn blocking failed: {}", e)))?
        .map_err(|e| AppError::internal(format!("Failed to create tempfile: {}", e)))?;

    let temp_path = temp_file.path().to_owned();
    let mut file = tokio::fs::File::create(&temp_path)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
    {
        if field.name() == Some("file") {
            while let Some(chunk) = field
                .chunk()
                .await
                .map_err(|e| AppError::internal(e.to_string()))?
            {
                file.write_all(&chunk)
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))?;
            }
        }
    }

    file.flush()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    drop(file);

    let body = tokio::fs::read(&temp_path)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // P-1: Validate magic bytes to prevent CWE-434/polyglot attacks
    if let Err(_e) = shared::file_validator::validate_magic_bytes("inx", &body) {
        return Err(AppError::bad_request(
            "Invalid INX file signature. Must be a valid archive.",
        ));
    }

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
        safety_level: aiome_core_contracts::contracts::ToolSafetyLevel::Safe,
        metadata: None,
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
