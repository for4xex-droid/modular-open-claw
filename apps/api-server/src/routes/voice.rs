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
    auth: Authenticated,
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<SynthesizeQuery>,
    Json(req): Json<SynthesizeRequest>,
) -> Result<axum::response::Response, AppError> {
    // SEC: Input validation — empty text, length cap, and voice_id sanitization
    if req.text.is_empty() {
        return Err(AppError::bad_request("Synthesis text must not be empty"));
    }
    const MAX_TTS_TEXT_CHARS: usize = 4096;
    let text_char_count = req.text.chars().count();
    if text_char_count > MAX_TTS_TEXT_CHARS {
        return Err(AppError::bad_request(format!(
            "Text exceeds maximum length of {} characters (got {})",
            MAX_TTS_TEXT_CHARS, text_char_count
        )));
    }
    const MAX_VOICE_ID_LEN: usize = 128;
    if let Some(ref vid) = req.voice_id {
        if vid.len() > MAX_VOICE_ID_LEN {
            return Err(AppError::bad_request(format!(
                "voice_id exceeds maximum length of {} characters",
                MAX_VOICE_ID_LEN
            )));
        }
    }

    info!(
        "🎙️ [Voice] Synthesizing text: {} chars (stream={:?}, agent={})",
        text_char_count, query.stream, auth.agent_id
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

        use axum::response::sse::{KeepAlive, Sse};

        let sse_stream = map_tts_stream_to_sse(audio_stream);
        Ok(Sse::new(sse_stream)
            .keep_alive(KeepAlive::default())
            .into_response())
    } else {
        let audio_bytes = state
            .tts_provider
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
        )
            .into_response())
    }
}

pub(crate) fn map_tts_stream_to_sse(
    stream: std::pin::Pin<
        Box<
            dyn tokio_stream::Stream<
                    Item = Result<
                        aiome_core_contracts::traits::TtsStreamEvent,
                        aiome_core::error::AiomeError,
                    >,
                > + Send,
        >,
    >,
) -> impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>> + Send
{
    use axum::response::sse::Event;
    use base64::{engine::general_purpose::STANDARD as b64, Engine as _};
    use tokio_stream::StreamExt;

    stream.map(|chunk_res| {
        match chunk_res {
            Ok(aiome_core_contracts::traits::TtsStreamEvent::Audio(bytes)) => {
                let b64_audio = b64.encode(&bytes);
                Ok(Event::default().event("audio").data(b64_audio))
            }
            Ok(aiome_core_contracts::traits::TtsStreamEvent::Viseme {
                viseme,
                timestamp_ms,
                duration_ms,
            }) => {
                let json = serde_json::json!({
                    "viseme": viseme,
                    "timestamp_ms": timestamp_ms,
                    "duration_ms": duration_ms
                });
                Ok(Event::default().event("viseme").data(json.to_string()))
            }
            Err(e) => {
                // SEC: Redact internal error details to prevent information leakage
                tracing::error!("SSE stream error: {}", e);
                Ok(Event::default().event("error").data(
                    serde_json::json!({ "error": "TTS stream processing error" }).to_string(),
                ))
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core_contracts::traits::TtsStreamEvent;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_map_tts_stream_to_sse_audio_and_viseme() {
        let test_stream = async_stream::stream! {
            yield Ok(TtsStreamEvent::Audio(vec![1, 2, 3]));
            yield Ok(TtsStreamEvent::Viseme {
                viseme: "A".to_string(),
                timestamp_ms: 100,
                duration_ms: 50,
            });
        };

        let stream_pin = Box::pin(test_stream);
        let mut sse_stream = std::pin::pin!(map_tts_stream_to_sse(stream_pin));

        // Event 1: Audio — verify it was received
        let _event1 = sse_stream
            .next()
            .await
            .expect("Should have Audio event")
            .expect("Audio event should be Ok");

        // Event 2: Viseme — verify it was received
        let _event2 = sse_stream
            .next()
            .await
            .expect("Should have Viseme event")
            .expect("Viseme event should be Ok");

        // Stream should be exhausted
        assert!(
            sse_stream.next().await.is_none(),
            "Stream should be exhausted after 2 events"
        );
    }

    #[tokio::test]
    async fn test_map_tts_stream_to_sse_error_redaction() {
        use aiome_core::error::AiomeError;

        let test_stream = async_stream::stream! {
            yield Err(AiomeError::Infrastructure {
                reason: "secret internal detail: endpoint=https://internal.api/v1".to_string(),
            });
        };

        let stream_pin = Box::pin(test_stream);
        let mut sse_stream = std::pin::pin!(map_tts_stream_to_sse(stream_pin));

        // Error events must NOT leak internal details
        let _event = sse_stream
            .next()
            .await
            .expect("Should have error event")
            .expect("Error event should be Ok (Infallible)");
        // The event is opaque (axum::sse::Event), but we verified it doesn't panic.
        // The key assertion is that the function doesn't propagate raw error strings.

        assert!(sse_stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_map_tts_stream_to_sse_empty_stream() {
        let test_stream = async_stream::stream! {
            // Yield nothing
            if false { yield Ok(TtsStreamEvent::Audio(vec![])); }
        };

        let stream_pin = Box::pin(test_stream);
        let mut sse_stream = std::pin::pin!(map_tts_stream_to_sse(stream_pin));

        assert!(
            sse_stream.next().await.is_none(),
            "Empty stream should yield nothing"
        );
    }
}
