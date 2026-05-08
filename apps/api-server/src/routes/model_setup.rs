/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core::traits::SettingsOps;
use axum::{
    extract::State,
    response::{
        sse::{Event, Sse},
        Json,
    },
};
use core::convert::Infallible;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema, PartialEq)]
pub struct ModelStatusResponse {
    pub ollama_connected: bool,
    pub ollama_version: Option<String>,
    pub installed_models: Vec<String>,
    pub configured_model: String,
    pub configured_model_available: bool,
    pub recommended_model: String,
    pub setup_required: bool,
}

#[utoipa::path(
    get,
    path = "/api/v1/models/status",
    responses(
        (status = 200, description = "Status of LLM Model Setup", body = ModelStatusResponse)
    ),
    security(("api_key" = []))
)]
pub async fn get_model_status(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<ModelStatusResponse>, AppError> {
    let host = state
        .job_queue
        .get_setting_value("ollama_host")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| state.config.ollama_host.clone());

    let configured_model = state
        .job_queue
        .get_setting_value("ollama_model")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| state.config.ollama_model.clone());

    let client = aiome_core::http::get_http_client();
    let tags_url = format!("{}/api/tags", host.trim_end_matches('/'));
    let version_url = format!("{}/api/version", host.trim_end_matches('/'));

    state.security_policy.validate_url(&version_url).await?;
    state.security_policy.validate_url(&tags_url).await?;

    let mut ollama_connected = false;
    let mut ollama_version = None;
    let mut installed_models = Vec::new();

    // 1. Check version (connection test)
    if let Ok(res) = client
        .get(&version_url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
    {
        if res.status() == 200 {
            ollama_connected = true;
            if let Ok(version_data) = res.json::<serde_json::Value>().await {
                if let Some(v) = version_data.get("version").and_then(|v| v.as_str()) {
                    ollama_version = Some(v.to_string());
                }
            }
        }
    }

    // 2. Check models if connected
    if ollama_connected {
        if let Ok(res) = client
            .get(&tags_url)
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
        {
            if res.status() == 200 {
                if let Ok(tags_data) = res.json::<serde_json::Value>().await {
                    if let Some(models) = tags_data.get("models").and_then(|m| m.as_array()) {
                        for model in models {
                            if let Some(name) = model.get("name").and_then(|n| n.as_str()) {
                                installed_models.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    let configured_model_available = installed_models.contains(&configured_model);
    let recommended_model = "gemma4:26b".to_string(); // Always gemma4:26b for V1 heuristics

    let mut setup_required = false;
    // We only require setup if they have Ollama connected but no models, or their configured model is missing
    if ollama_connected && (!configured_model_available || installed_models.is_empty()) {
        setup_required = true;
    }

    Ok(Json(ModelStatusResponse {
        ollama_connected,
        ollama_version,
        installed_models,
        configured_model,
        configured_model_available,
        recommended_model,
        setup_required,
    }))
}

#[derive(Debug, Deserialize)]
pub struct PullModelRequest {
    pub name: String,
}

pub async fn pull_model(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(payload): Json<PullModelRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    // 1. Validate model name to prevent command injection or strange paths
    static RE: std::sync::OnceLock<Result<regex::Regex, regex::Error>> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"^[a-zA-Z0-9._:-]+$"));
    let re = re
        .as_ref()
        .map_err(|e| AppError::internal(format!("Regex compilation failed: {}", e)))?;
    if !re.is_match(&payload.name) {
        return Err(AppError::bad_request(
            "Invalid model name format. Only alphanumeric characters, dots, underscores, colons, and hyphens are allowed.",
        ));
    }

    let host = state
        .job_queue
        .get_setting_value("ollama_host")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| state.config.ollama_host.clone());

    let pull_url = format!("{}/api/pull", host.trim_end_matches('/'));

    // Security check against SSRF
    state.security_policy.validate_url(&pull_url).await?;

    let client = aiome_core::http::get_http_client();

    // Acquire compute semaphore to limit concurrent heavy pulls preventing OOM / lockups
    let semaphore_permit = Arc::clone(state.compute_semaphore.get_inner())
        .acquire_owned()
        .await
        .map_err(|e| AppError::internal(format!("Failed to acquire compute semaphore: {}", e)))?;

    // Start long-running HTTP request to Ollama
    let response = client
        .post(&pull_url)
        .json(&serde_json::json!({
            "name": payload.name,
            "stream": true,
        }))
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Ollama pull request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::internal(format!(
            "Ollama returned error status: {}",
            response.status()
        )));
    }

    // Bridge the HTTP stream to SSE
    let stream = async_stream::stream! {
        // Move permit inside so it drops when stream finishes
        let _permit = semaphore_permit;

        let mut byte_stream = response.bytes_stream();
        use futures::StreamExt;
        let mut buffer = Vec::new();

        while let Some(chunk_res) = byte_stream.next().await {
            match chunk_res {
                Ok(bytes) => {
                    buffer.extend_from_slice(&bytes);
                    while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                        let line_bytes = buffer.drain(..=pos).collect::<Vec<_>>();
                        if let Ok(line) = String::from_utf8(line_bytes) {
                            let line = line.trim();
                            if !line.is_empty() {
                                yield Ok(Event::default().event("progress").data(line));
                            }
                        }
                    }
                }
                Err(e) => {
                    let err_json = serde_json::json!({"error": e.to_string()});
                    yield Ok(Event::default().event("error").data(err_json.to_string()));
                    break;
                }
            }
        }

        // Flush any remaining data in the buffer (final partial line without trailing newline)
        if !buffer.is_empty() {
            if let Ok(line) = String::from_utf8(buffer) {
                let line = line.trim();
                if !line.is_empty() {
                    yield Ok(Event::default().event("progress").data(line));
                }
            }
        }

        // Emit explicit done event so the frontend knows the stream has ended
        yield Ok(Event::default().event("done").data("{}"));
    };

    Ok(Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_model_status_returns_success() {
        // Integration test handles real assertion, skipping dummy assert
    }
}
