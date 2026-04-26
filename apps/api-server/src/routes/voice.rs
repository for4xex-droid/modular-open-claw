/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::{app_state::AppState, auth::Authenticated, error::AppError};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use infrastructure::registry::{AssetManifest, AssetType};
use infrastructure::security::crypto::encrypt_xchacha20poly1305;
use rand::Rng;
use serde_json::json;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{error, info};
use uuid::Uuid;
use zeroize::Zeroizing;

/// [POST] /api/v1/voice/upload
/// Phase 10.2: Voice asset upload with AES-256-GCM DRM Encryption (§SEC-1, §SEC-4, §4-B)
pub async fn upload_voice_handler(
    auth: Authenticated,
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<impl IntoResponse, AppError> {
    let _safe = 1_u32.clamp(0, 10);
    // F-03: Acquire upload semaphore permit to prevent OOM/DoS from large concurrent uploads
    let _permit = state.upload_semaphore.try_acquire().map_err(|e| {
        crate::error::AppError(aiome_core::error::AiomeError::ResourceBusy {
            reason: format!("System busy: {}", e),
        })
    })?;

    let mut temp_file = tokio::task::spawn_blocking(tempfile::NamedTempFile::new)
        .await
        .map_err(|e| AppError::internal(format!("Tokio spawn blocking failed: {}", e)))?
        .map_err(|e| AppError::internal(format!("Failed to create tempfile: {}", e)))?;

    let temp_path = temp_file.path().to_owned();
    let mut file = fs::File::create(&temp_path)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    let mut size = 0;

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
                size += chunk.len();

                // Real-time quota check during upload
                state
                    .disk_quota
                    .check_quota(auth.agent_id, size as u64)
                    .await
                    .map_err(|e| {
                        crate::error::AppError(e) // Wrap in AppError
                    })?;
            }
        }
    }

    if size == 0 {
        return Err(AppError::bad_request("Voice asset body cannot be empty"));
    }

    let body = fs::read(&temp_path)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // P-1: Validate magic bytes to prevent CWE-434/polyglot attacks
    if let Err(_e) = shared::file_validator::validate_magic_bytes("wav", &body) {
        return Err(AppError::bad_request(
            "Invalid audio file signature. Only WAV is supported.",
        ));
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
    let encrypted =
        encrypt_xchacha20poly1305(&body, &key).map_err(|e| AppError::internal(e.to_string()))?;

    // 4. AppDataResolver 配下の安全な領域に保存 (.abyss_vault/)
    let vault_base = if state.config.abyss_vault_path.is_empty() {
        state.config.resolver.root().to_path_buf()
    } else {
        PathBuf::from(&state.config.abyss_vault_path)
    };
    let vault_dir = vault_base.join(".abyss_vault");

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
            safety_level: aiome_core_contracts::contracts::ToolSafetyLevel::Safe,
            metadata: None,
        })
        .await?;

    // 6. 成功時にディスク使用量を記録
    if let Err(e) = state.disk_quota.record_usage(agent_id, size as u64).await {
        tracing::warn!("⚠️ [DiskQuota] Failed to record usage: {:?}", e);
    }

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

#[derive(serde::Deserialize)]
pub struct SynthesizeQuery {
    pub stream: Option<bool>,
}

/// [POST] /api/v1/voice/synthesize
/// AI Voice Synthesis via TtsProvider
#[utoipa::path(
    post,
    path = "/api/v1/voice/synthesize",
    params(
        ("stream" = Option<bool>, Query, description = "Enable streaming response")
    ),
    responses(
        (status = 200, description = "Synthesized audio bytes or stream")
    ),
    security(("api_key" = []))
)]

pub async fn synthesize_voice_handler(
    _auth: Authenticated,
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<SynthesizeQuery>,
    Json(req): Json<SynthesizeRequest>,
) -> Result<axum::response::Response, AppError> {
    info!(
        "🎙️ [Voice] Synthesizing text: {} chars (stream={:?})",
        req.text.len(),
        query.stream
    );

    // Use requested voice_id, or default from config, or fallback to p225
    let voice_id = req
        .voice_id
        .as_deref()
        .or(state.config.xtts_speaker.as_deref())
        .unwrap_or("p225");

    let is_stream = query.stream.unwrap_or(false);

    if is_stream {
        let audio_stream = state
            .tts_provider
            .synthesize_stream(&req.text, voice_id)
            .await
            .map_err(|e| {
                error!("❌ [Voice] Streaming synthesis failed: {}", e);
                AppError::internal(format!("TTS streaming failed: {}", e))
            })?;

        use axum::response::IntoResponse;
        let body = axum::body::Body::from_stream(audio_stream);
        let mut res = body.into_response();
        res.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("audio/wav"),
        );
        Ok(res)
    } else {
        let audio_bytes = state
            .tts_provider
            .synthesize(&req.text, voice_id)
            .await
            .map_err(|e| {
                error!("❌ [Voice] Synthesis failed: {}", e);
                AppError::internal(format!("TTS synthesis failed: {}", e))
            })?;

        use axum::response::IntoResponse;
        Ok((
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "audio/wav")],
            audio_bytes,
        )
            .into_response())
    }
}
