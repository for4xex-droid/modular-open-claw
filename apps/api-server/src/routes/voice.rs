/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::{app_state::AppState, auth::Authenticated, error::AppError};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use infrastructure::registry::{AssetManifest, AssetType};
use infrastructure::security::crypto::encrypt_aes256gcm;
use rand::Rng;
use serde_json::json;
use std::path::PathBuf;
use tokio::fs;
use tracing::{info, error};
use uuid::Uuid;
use zeroize::Zeroizing;

/// [POST] /api/v1/voice/upload
/// Phase 10.2: Voice asset upload with AES-256-GCM DRM Encryption (§SEC-1, §SEC-4, §4-B)
pub async fn upload_voice_handler(
    auth: Authenticated,
    State(state): State<AppState>,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, AppError> {
    let size = body.len();
    if size == 0 {
        return Err(AppError::bad_request("Voice asset body cannot be empty"));
    }

    let asset_id = Uuid::new_v4();
    let agent_id = auth.agent_id; // §SEC-4: Creator Auth

    info!(
        "🎤 [Voice] Processing voice asset upload: {} bytes (Agent: {}, Asset: {})",
        size, agent_id, asset_id
    );

    // 0. CSAM 検疫チェック (Phase 11: タイムアウト付き spawn_blocking による防御)
    let hasher = infrastructure::compliance::AudioHasher::default();
    let audio_hash = hasher
        .compute_hash(body.to_vec())
        .await
        .map_err(|e| AppError::internal(format!("Audio hashing failed: {}", e)))?;

    info!("🔍 [Voice] Computed Audio CSAM Hash: {}", audio_hash);

    if state
        .quarantine_store
        .is_quarantined(&audio_hash)
        .await
        .unwrap_or(false)
    {
        tracing::warn!(
            "🚨 [Voice] Asset blocked due to Quarantine Rule Match (Hash: {})",
            audio_hash
        );
        return Err(AppError::forbidden("Asset rejected by compliance policy"));
    }

    // 1. AES-256-GCM 鍵生成 (32 bytes)
    let key = Zeroizing::new(rand::thread_rng().gen::<[u8; 32]>().to_vec());

    // 2 & 3. 暗号化: [nonce(12B) || ciphertext || tag(16B)] (§SEC-1, §4-B)
    let encrypted = encrypt_aes256gcm(&body, &key)?;

    // 4. workspace外の安全な領域に保存 (ここでは ~/.aiome/abyss_vault/ をシミュレートするか、適当な一時ディレクトリ)
    let workspace_root = state.config.abyss_vault_path.clone();
    let workspace_root = if workspace_root.is_empty() {
        "workspace".to_string()
    } else {
        workspace_root
    };
    let vault_dir = PathBuf::from(workspace_root).join(".abyss_vault");

    fs::create_dir_all(&vault_dir)
        .await
        .map_err(|e| AppError::internal(format!("Failed to create vault directory: {}", e)))?;

    let file_path = vault_dir.join(format!("{}.aivoice", asset_id));
    fs::write(&file_path, &encrypted)
        .await
        .map_err(|e| AppError::internal(format!("Failed to write encrypted voice asset: {}", e)))?;

    // 5. 鍵を AbyssVoiceVault に登録 (メモリ + 将来的に永続化)
    state.voice_drm.register_asset_key(asset_id, key).await?;

    state
        .registry
        .register_asset(AssetManifest {
            id: asset_id,
            creator_id: agent_id,
            asset_type: AssetType::VoiceModel,
            name: format!("{}.aivoice", asset_id),
            description: "Encrypted Voice Asset".to_string(),
            price_coins: 0,
            metadata: None,
        })
        .await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "asset_id": asset_id,
            "creator_id": agent_id,
            "original_size": size,
            "encrypted_size": encrypted.len(),
            "status": "encrypted_and_stored"
        })),
    ))
}

#[derive(serde::Deserialize)]
pub struct ListVoiceAssetsQuery {
    pub scope: String, // "public" or "owned"
}

/// [GET] /api/v1/voice/list
/// List Voice Models
#[utoipa::path(
    get,
    path = "/api/v1/voice/list",
    params(
        ("scope" = String, Query, description = "Scope of assets (public/owned)")
    ),
    responses(
        (status = 200, description = "List of voice assets")
    ),
    security(("api_key" = []))
)]
pub async fn list_voice_assets_handler(
    auth: Authenticated,
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<ListVoiceAssetsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let assets = state
        .registry
        .list_assets_by_type(
            infrastructure::registry::AssetType::VoiceModel,
            Some(auth.agent_id),
            &query.scope,
        )
        .await?;
    Ok((StatusCode::OK, Json(assets)))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct SynthesizeRequest {
    pub text: String,
    pub voice_id: Option<String>,
}

/// [POST] /api/v1/voice/synthesize
/// AI Voice Synthesis via TtsProvider
#[utoipa::path(
    post,
    path = "/api/v1/voice/synthesize",
    responses(
        (status = 200, description = "Synthesized audio bytes")
    ),
    security(("api_key" = []))
)]
pub async fn synthesize_voice_handler(
    _auth: Authenticated,
    State(state): State<AppState>,
    Json(req): Json<SynthesizeRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!("🎙️ [Voice] Synthesizing text: {} chars", req.text.len());
    
    // Use requested voice_id, or default from config, or fallback to p225
    let voice_id = req.voice_id.as_deref()
        .or(state.config.xtts_speaker.as_deref())
        .unwrap_or("p225");
    
    let audio_bytes = state.tts_provider
        .synthesize(&req.text, voice_id)
        .await
        .map_err(|e| {
            error!("❌ [Voice] Synthesis failed: {}", e);
            AppError::internal(format!("TTS synthesis failed: {}", e))
        })?;

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "audio/wav")],
        audio_bytes,
    ))
}
