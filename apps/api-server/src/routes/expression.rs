/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core::expression::engine::ExpressionEngine;
use aiome_core::traits::*;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ListParams {
    pub limit: Option<i64>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AutoToggle {
    pub enabled: bool,
}

#[utoipa::path(
    get,
    path = "/api/expression/status",
    responses(
        (status = 200, description = "Expression engine status", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn expression_status(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<serde_json::Value>, AppError> {
    let pending_count = state.job_queue.get_pending_job_count().await.unwrap_or(0);
    let auto_enabled = state
        .job_queue
        .get_auto_expression_enabled()
        .await
        .unwrap_or(false);
    let recent_karma = state.job_queue.fetch_all_karma(1).await.unwrap_or_default();

    let status = if pending_count > 0 {
        "processing"
    } else {
        "idle"
    };
    let last_lesson = recent_karma
        .first()
        .and_then(|k| k["lesson"].as_str())
        .unwrap_or("Waiting for new insights...");

    Ok(Json(serde_json::json!({
        "status": status,
        "auto_expression": auto_enabled,
        "pending_expressions": pending_count,
        "last_insight": last_lesson,
        "message_ja": format!("自律表現パイプライン: {} (自動: {})。現在の洞察: {}", status, if auto_enabled { "ON" } else { "OFF" }, last_lesson),
        "message_en": format!("Autonomous expression pipeline {} (Auto: {}). Current insight: {}", status, if auto_enabled { "ON" } else { "OFF" }, last_lesson)
    })))
}

#[utoipa::path(
    post,
    path = "/api/expression/generate",
    responses(
        (status = 200, description = "Generated expression", body = serde_json::Value),
        (status = 400, description = "No karma available")
    ),
    security(("api_key" = []))
)]
pub async fn generate_expression(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
) -> Result<Json<serde_json::Value>, AppError> {
    // 1. Plan-based rate limiting (Step 3: TTS / Expression rate limiting)
    let sub_status = state
        .commerce_engine
        .get_subscription_status(auth.agent_id)
        .await
        .unwrap_or(aiome_core_contracts::commerce::SubscriptionStatus::None);

    let is_pro = matches!(
        sub_status,
        aiome_core_contracts::commerce::SubscriptionStatus::Active
    );

    if !is_pro {
        // Enforce Free plan limits (e.g., max 5 expressions per hour).
        // For accurate limit, we fetch the 5 recent expressions and check their creation time
        let recent_exprs = state.job_queue.fetch_expressions(5).await.map_err(|e| {
            aiome_core::error::AiomeError::Infrastructure {
                reason: format!(
                    "Failed to fetch recent expressions for rate limit check: {}",
                    e
                ),
            }
        })?;
        if recent_exprs.len() >= 5 {
            if let Some(oldest) = recent_exprs.last() {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&oldest.created_at) {
                    if (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_hours() < 1 {
                        return Err(aiome_core::error::AiomeError::BudgetExhausted(
                            aiome_core_contracts::error::BudgetExhaustedError {
                                limit: 5.0,
                                actual: 5.0,
                            },
                        )
                        .into());
                    }
                }
            }
        }
    }

    // 2. Fetch latest Karma
    let karma = state.job_queue.fetch_all_karma(5).await?;

    if karma.is_empty() {
        return Err(aiome_core::error::AiomeError::Infrastructure {
            reason: "No karma available to generate expression".to_string(),
        }
        .into());
    }

    // 2. Fetch Soul Prompt
    let soul_prompt =
        crate::agent_engine::read_app_data_file(&state.config.resolver, "config/SOUL_PROMPT.md")
            .await;

    // 3. VRAM Arbitration (15-C): Wait for GPU resource availability
    let _permit = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        state.llm_semaphore.acquire(),
    )
    .await
    .map_err(|_| aiome_core_contracts::error::AiomeError::ResourceBusy {
        reason: "GPU/VRAM contention (Generation).".to_string(),
    })?
    .map_err(
        |e| aiome_core_contracts::error::AiomeError::Infrastructure {
            reason: format!("Semaphore acquire error: {}", e),
        },
    )?;

    // 4. Generate Expression
    let mut expression =
        ExpressionEngine::generate(&karma, &soul_prompt, state.provider.as_ref()).await?;

    // 4. NG-22 / Phase 10.1a: Trigger TTS if configured
    if let Ok(Some(tts_prov)) = state.job_queue.get_setting_value("tts_provider").await {
        if tts_prov != "none" {
            let voice = state
                .job_queue
                .get_setting_value("tts_voice")
                .await
                .unwrap_or(None)
                .unwrap_or_else(|| "alloy".to_string());

            tracing::info!(
                "🗣️ [TTS] Synthesizing audio stream for Expression {} with voice '{}'",
                expression.id,
                voice
            );

            let tts_provider = state.tts_provider.get_inner();

            match tts_provider
                .synthesize_stream(&expression.content, &voice)
                .await
            {
                Ok(mut stream) => {
                    use tokio_stream::StreamExt;
                    let mut audio_buffer = Vec::new();
                    let mut visemes = Vec::new();

                    while let Some(event_res) = stream.next().await {
                        match event_res {
                            Ok(aiome_core_contracts::traits::TtsStreamEvent::Audio(bytes)) => {
                                audio_buffer.extend_from_slice(&bytes);
                            }
                            Ok(aiome_core_contracts::traits::TtsStreamEvent::Viseme {
                                viseme,
                                timestamp_ms,
                                duration_ms,
                            }) => {
                                visemes.push(serde_json::json!({
                                    "viseme": viseme,
                                    "timestamp_ms": timestamp_ms,
                                    "duration_ms": duration_ms
                                }));
                            }
                            Err(e) => {
                                tracing::error!("❌ [TTS] Stream error: {:?}", e);
                                break;
                            }
                        }
                    }

                    if !audio_buffer.is_empty() {
                        let audio_dir = state.config.resolver.resolve("audio");
                        let _ = std::fs::create_dir_all(&audio_dir);

                        let ext = if tts_prov == "xtts" { "wav" } else { "mp3" };
                        let path = audio_dir.join(format!("{}.{}", expression.id, ext));

                        if let Err(e) = std::fs::write(&path, &audio_buffer) {
                            tracing::error!("Failed to write audio file {}: {}", path.display(), e);
                        } else {
                            expression.audio_path = Some(path.to_string_lossy().to_string());
                            tracing::info!("✅ [TTS] Audio saved to {}", path.display());

                            // Save visemes if available
                            if !visemes.is_empty() {
                                let viseme_path =
                                    audio_dir.join(format!("{}.visemes.json", expression.id));
                                if let Err(e) = std::fs::write(
                                    &viseme_path,
                                    serde_json::to_string(&visemes).unwrap_or_default(),
                                ) {
                                    tracing::error!(
                                        "Failed to write viseme file {}: {}",
                                        viseme_path.display(),
                                        e
                                    );
                                } else {
                                    tracing::info!(
                                        "✅ [TTS] Visemes saved to {}",
                                        viseme_path.display()
                                    );
                                }
                            }
                        }
                    } else {
                        tracing::warn!("❌ [TTS] Stream completed but no audio data was received.");
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "❌ [TTS] Failed to initialize synthesis stream via {}: {:?}",
                        tts_prov,
                        e
                    );
                }
            }
        }
    }

    // 5. Store Expression
    state.job_queue.store_expression(&expression).await?;

    Ok(Json(serde_json::json!(expression)))
}

#[utoipa::path(
    get,
    path = "/api/expression/list",
    params(
        ("limit" = Option<i64>, Query, description = "Limit results")
    ),
    responses(
        (status = 200, description = "Recent expressions", body = [serde_json::Value])
    ),
    security(("api_key" = []))
)]
pub async fn list_expressions(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Query(params): Query<ListParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(20);
    let expressions = state.job_queue.fetch_expressions(limit).await?;

    Ok(Json(serde_json::json!(expressions)))
}

#[utoipa::path(
    post,
    path = "/api/expression/auto",
    request_body = AutoToggle,
    responses(
        (status = 200, description = "Toggled auto-expression", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn toggle_auto_expression(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(payload): Json<AutoToggle>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .job_queue
        .set_auto_expression_enabled(payload.enabled)
        .await?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "auto_expression_enabled": payload.enabled
    })))
}
