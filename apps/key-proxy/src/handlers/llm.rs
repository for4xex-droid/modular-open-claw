/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::config::{AppState, EmbedResponse, ProxyRequest, ProxyResponse};
use crate::quota::check_and_increment_quota;
use crate::telemetry::{
    emit_cost_metric, emit_embed_metric, emit_stream_start_metric, record_caller_on_span,
    record_endpoint_on_span, redact_display, sanitize_caller_id,
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use secrecy::ExposeSecret;
use tracing::{error, info};

#[tracing::instrument(
    skip(state, payload),
    fields(caller_id = tracing::field::Empty, endpoint = tracing::field::Empty)
)]
pub(crate) async fn handle_llm_complete(
    State(state): State<AppState>,
    Json(payload): Json<ProxyRequest>,
) -> impl IntoResponse {
    let safe_caller_id = sanitize_caller_id(&payload.caller_id);
    record_caller_on_span(&safe_caller_id);
    record_endpoint_on_span(&payload.endpoint);
    info!("📩 [KeyProxy] Request from caller: {}", safe_caller_id);

    if let Err(status) = check_and_increment_quota(&state, &safe_caller_id).await {
        return status.into_response();
    }

    let gemini_model = &state.gemini_model;
    let url = match payload.endpoint.as_str() {
        "gemini" => format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            gemini_model
        ),
        _ => return (StatusCode::BAD_REQUEST, "Invalid endpoint").into_response(),
    };

    let mut gemini_payload = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": payload.prompt
            }]
        }]
    });
    if let Some(s) = payload.system {
        if let Some(obj) = gemini_payload.as_object_mut() {
            obj.insert(
                "system_instruction".to_string(),
                serde_json::json!({ "parts": [{ "text": s }] }),
            );
        }
    }

    let start_time = tokio::time::Instant::now();

    let res = state
        .client
        .post(url)
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", state.gemini_key.expose_secret())
        .json(&gemini_payload)
        .send()
        .await;

    match res {
        Ok(resp) => {
            if resp.status().is_success() {
                let body_res: Result<serde_json::Value, _> = resp.json().await;
                match body_res {
                    Ok(body) => {
                        let text = body
                            .get("candidates")
                            .and_then(|c| c.get(0))
                            .and_then(|c| c.get("content"))
                            .and_then(|c| c.get("parts"))
                            .and_then(|p| p.get(0))
                            .and_then(|p| p.get("text"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string();

                        let total_tokens = body
                            .get("usageMetadata")
                            .and_then(|u| u.get("totalTokenCount"))
                            .and_then(|c| c.as_u64());

                        if let Some(tokens) = total_tokens {
                            let cost_usd = tokens as f64 * 0.00000015;
                            emit_cost_metric(
                                &safe_caller_id,
                                tokens,
                                cost_usd,
                                &state.gemini_model,
                            );
                        }

                        let response_time_ms = start_time.elapsed().as_millis() as u64;

                        Json(ProxyResponse {
                            content: text,
                            stop_reason: "end_turn".to_string(),
                            total_tokens,
                            response_time_ms: Some(response_time_ms),
                        })
                        .into_response()
                    }
                    Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error")
                        .into_response(),
                }
            } else {
                let status = resp.status();
                error!("❌ [KeyProxy] Upstream error: {}", status);
                (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
            }
        }
        Err(e) => {
            error!("❌ [KeyProxy] Request failed: {}", redact_display(&e));
            (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
        }
    }
}

#[tracing::instrument(
    skip(state, payload),
    fields(caller_id = tracing::field::Empty, endpoint = tracing::field::Empty)
)]
pub(crate) async fn handle_llm_embed(
    State(state): State<AppState>,
    Json(payload): Json<ProxyRequest>,
) -> impl IntoResponse {
    let safe_caller_id = sanitize_caller_id(&payload.caller_id);
    record_caller_on_span(&safe_caller_id);
    record_endpoint_on_span(&payload.endpoint);
    info!(
        "🧬 [KeyProxy] Embedding request from caller: {}",
        safe_caller_id
    );

    if let Err(status) = check_and_increment_quota(&state, &safe_caller_id).await {
        return status.into_response();
    }

    let embed_model = &state.gemini_embed_model;
    let url = match payload.endpoint.as_str() {
        "gemini-embed" => format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent",
            embed_model
        ),
        _ => return (StatusCode::BAD_REQUEST, "Invalid endpoint").into_response(),
    };

    let gemini_payload = serde_json::json!({
        "content": {
            "parts": [{ "text": payload.prompt }]
        }
    });

    let start_time = tokio::time::Instant::now();

    let res = state
        .client
        .post(url)
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", state.gemini_key.expose_secret())
        .json(&gemini_payload)
        .send()
        .await;

    match res {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(body) => {
                        let emb = body["embedding"]["values"].as_array();
                        if let Some(values) = emb {
                            let vec: Vec<f32> = values
                                .iter()
                                .filter_map(|v| v.as_f64().map(|f| f as f32))
                                .collect();
                            let response_time_ms = start_time.elapsed().as_millis() as u64;
                            emit_embed_metric(&safe_caller_id, response_time_ms, vec.len());
                            Json(EmbedResponse {
                                embedding: vec,
                                response_time_ms: Some(response_time_ms),
                            })
                            .into_response()
                        } else {
                            error!("❌ [KeyProxy] Embed response missing 'embedding.values' field");
                            (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error")
                                .into_response()
                        }
                    }
                    Err(e) => {
                        error!(
                            "❌ [KeyProxy] Failed to parse embed response: {}",
                            redact_display(&e)
                        );
                        (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error")
                            .into_response()
                    }
                }
            } else {
                error!("❌ [KeyProxy] Upstream error: {}", resp.status());
                (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
            }
        }
        Err(e) => {
            error!("❌ [KeyProxy] Request failed: {}", redact_display(&e));
            (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
        }
    }
}

#[tracing::instrument(
    skip(state, payload),
    fields(caller_id = tracing::field::Empty, endpoint = tracing::field::Empty)
)]
pub(crate) async fn handle_llm_stream(
    State(state): State<AppState>,
    Json(payload): Json<ProxyRequest>,
) -> impl IntoResponse {
    let safe_caller_id = sanitize_caller_id(&payload.caller_id);
    record_caller_on_span(&safe_caller_id);
    record_endpoint_on_span(&payload.endpoint);
    info!(
        "🌊 [KeyProxy] Streaming request from caller: {}",
        safe_caller_id
    );

    if let Err(status) = check_and_increment_quota(&state, &safe_caller_id).await {
        return status.into_response();
    }

    let gemini_model = &state.gemini_model;
    let url = match payload.endpoint.as_str() {
        "gemini" => format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse",
            gemini_model
        ),
        _ => return (StatusCode::BAD_REQUEST, "Invalid endpoint").into_response(),
    };

    let mut gemini_payload = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": payload.prompt
            }]
        }]
    });
    if let Some(s) = payload.system {
        if let Some(obj) = gemini_payload.as_object_mut() {
            obj.insert(
                "system_instruction".to_string(),
                serde_json::json!({ "parts": [{ "text": s }] }),
            );
        }
    }

    let start_time = tokio::time::Instant::now();

    let res = state
        .client
        .post(url)
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", state.gemini_key.expose_secret())
        .json(&gemini_payload)
        .send()
        .await;

    match res {
        Ok(resp) => {
            if resp.status().is_success() {
                let response_time_ms = start_time.elapsed().as_millis() as u64;
                emit_stream_start_metric(&safe_caller_id, response_time_ms);

                use futures::StreamExt;
                let stream = resp.bytes_stream().map(|chunk_res| match chunk_res {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes).to_string();
                        Ok::<axum::response::sse::Event, std::convert::Infallible>(
                            axum::response::sse::Event::default().data(text),
                        )
                    }
                    Err(e) => {
                        let error_json = serde_json::json!({ "error": e.to_string() });
                        Ok::<axum::response::sse::Event, std::convert::Infallible>(
                            axum::response::sse::Event::default().data(
                                serde_json::to_string(&error_json)
                                    .unwrap_or_else(|_| "{}".to_string()),
                            ),
                        )
                    }
                });
                axum::response::sse::Sse::new(stream).into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
            }
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response(),
    }
}
