/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AppError;
use crate::{auth::Authenticated, AppState};
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, ToSocketAddrs};
use tracing::{error, info, warn};
use url::Url;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateSettingsRequest {
    pub key: String,
    pub value: String,
    pub category: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct TestConnectionRequest {
    pub service: String, // "ollama", "discord", "telegram"
    pub url: String,
    pub token: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/settings",
    responses(
        (status = 200, description = "List all settings", body = [aiome_core::contracts::SystemSetting]),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn get_settings(
    State(state): State<AppState>,
    _auth: Authenticated,
) -> Result<Json<Vec<aiome_core::contracts::SystemSetting>>, AppError> {
    let settings = state.job_queue.fetch_all_settings().await?;

    // Mask secrets
    let mut masked = settings;
    for s in &mut masked {
        if s.is_secret {
            s.value = "••••••••".to_string();
        }
    }
    Ok(Json(masked))
}

#[utoipa::path(
    put,
    path = "/api/v1/settings",
    request_body = UpdateSettingsRequest,
    responses(
        (status = 200, description = "Setting updated successfully"),
        (status = 400, description = "Invalid request or unauthorized key"),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn update_setting(
    State(state): State<AppState>,
    _auth: Authenticated,
    Json(payload): Json<UpdateSettingsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 1. Key whitelist check
    let allowed_keys = [
        "ollama_host",
        "ollama_model",
        "llm_provider",
        "llm_api_key",
        "llm_model",
        "lm_studio_host",
        "bg_llm_provider",
        "bg_llm_model",
        "bg_llm_api_key",
        "discord_chat_channel_id",
        "discord_command_channel_id",
        "discord_log_channel_id",
        "telegram_chat_id",
        "watchtower_enabled",
        "enforce_guardrail",
        "log_level",
        "node_id",
        "samsara_hub_url",
        "allowed_origins",
        "ai_name",
        "ai_motto",
        "ai_vrm_url",
        "lora_adapter_path",
        "lora_base_model",
        "tts_provider",
        "tts_voice",
    ];

    if !allowed_keys.contains(&payload.key.as_str()) {
        warn!(
            "🚨 [Security] Unauthorized settings key attempt: {}",
            payload.key
        );
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Unauthorized setting key".to_string(),
        }
        .into());
    }

    // 2. Category validation
    let allowed_categories = [
        "llm", "channel", "system", "security", "cors", "identity", "voice",
    ];
    if !allowed_categories.contains(&payload.category.as_str()) {
        return Err(
            aiome_core::error::AiomeError::RemoteServiceExecutionFailed {
                reason: "Invalid category".to_string(),
            }
            .into(),
        );
    }

    // 3. Value length limit (DoS protection)
    if payload.value.len() > 1024 {
        return Err(
            aiome_core::error::AiomeError::RemoteServiceExecutionFailed {
                reason: "Value too long (max 1024 chars)".to_string(),
            }
            .into(),
        );
    }

    // 4. Server-side is_secret determination
    let secrets = [
        "ollama_host",
        "discord_token",
        "telegram_token",
        "api_server_secret",
        "llm_api_key",
    ];
    let is_secret = secrets.contains(&payload.key.as_str());

    // 5. Audit Logging
    info!(
        "🔧 [Settings] Audit: Key '{}' updated (category: {}, secret: {})",
        payload.key, payload.category, is_secret
    );

    state
        .job_queue
        .update_setting(&payload.key, &payload.value, &payload.category, is_secret)
        .await?;

    // Phase 6.5: Sync hook for AgentSoul
    if payload.key == "lora_adapter_path" || payload.key == "lora_base_model" {
        if let Ok(Some(mut soul)) = state.soul_store.load_soul("system-soul").await {
            let val = if payload.value.is_empty() {
                None
            } else {
                Some(payload.value.clone())
            };
            if payload.key == "lora_adapter_path" {
                soul.lora_adapter_path = val;
            } else {
                soul.lora_base_model = val;
            }
            if let Err(e) = state.soul_store.save_soul(&soul).await {
                tracing::error!("Failed to sync AgentSoul for key {}: {:?}", payload.key, e);
            } else {
                tracing::info!(
                    "🔄 [Settings] AgentSoul synchronized with new {}",
                    payload.key
                );

                // NG-21: Kick-off Ollama model build dynamically
                if let (Some(base), Some(adapter)) =
                    (soul.lora_base_model.clone(), soul.lora_adapter_path.clone())
                {
                    let host = state
                        .job_queue
                        .get_setting_value("ollama_host")
                        .await
                        .unwrap_or_else(|_| Some("http://localhost:11434".to_string()))
                        .unwrap_or_else(|| "http://localhost:11434".to_string());

                    let new_model = format!("{}-lora", base.replace(':', "-"));
                    let q = state.job_queue.clone();

                    tokio::spawn(async move {
                        if let Err(e) = aiome_core::llm_provider::OllamaProvider::build_lora_model(
                            &host, &base, &adapter, &new_model,
                        )
                        .await
                        {
                            tracing::error!("❌ [Ollama] Async LoRA Build Failed: {}", e);
                        } else {
                            // Automatically update the config to use the new LoRA model
                            let _ = q
                                .update_setting("ollama_model", &new_model, "llm", false)
                                .await;
                        }
                    });
                }
            }
        }
    }

    Ok(Json(serde_json::json!({"status": "ok"})))
}

#[utoipa::path(
    post,
    path = "/api/v1/settings/test",
    request_body = TestConnectionRequest,
    responses(
        (status = 200, description = "Connection test completed", body = TestConnectionResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
#[cfg(debug_assertions)]
pub async fn test_connection(
    state: State<AppState>,
    _auth: Authenticated,
    Json(payload): Json<TestConnectionRequest>,
) -> Result<Json<TestConnectionResponse>, AppError> {
    // SSRF Protection: Use unified SecurityPolicy from State
    if let Err(e) = state.security_policy.validate_url(&payload.url).await {
        return Ok(Json(TestConnectionResponse {
            success: false,
            message: format!("SSRF Blocked: {}", e),
        }));
    }

    let res = match payload.service.as_str() {
        "ollama" => test_ollama(&payload.url, payload.model.as_deref()).await,
        "gemini" | "openai" | "anthropic" => {
            test_cloud_connection(
                &payload.service,
                payload.token.as_deref(),
                payload.model.as_deref(),
            )
            .await
        }
        _ => Json(TestConnectionResponse {
            success: false,
            message: format!("Service '{}' testing not implemented yet", payload.service),
        }),
    };

    Ok(res)
}

async fn test_cloud_connection(
    service: &str,
    token: Option<&str>,
    _model: Option<&str>,
) -> Json<TestConnectionResponse> {
    let Some(token) = token else {
        return Json(TestConnectionResponse {
            success: false,
            message: format!("API Key is required for {}", service),
        });
    };
    if token.is_empty() {
        return Json(TestConnectionResponse {
            success: false,
            message: format!("API Key is required for {}", service),
        });
    }

    let client = aiome_core::http::get_http_client();

    match service {
        "gemini" => {
            let url = "https://generativelanguage.googleapis.com/v1beta/models";
            match client
                .get(url)
                .timeout(std::time::Duration::from_secs(10))
                .header("x-goog-api-key", token)
                .send()
                .await
            {
                Ok(res) if res.status().is_success() => Json(TestConnectionResponse {
                    success: true,
                    message: "Gemini connection verified.".to_string(),
                }),
                Ok(res) => Json(TestConnectionResponse {
                    success: false,
                    message: format!("Gemini error: Status {}", res.status()),
                }),
                Err(e) => Json(TestConnectionResponse {
                    success: false,
                    message: format!("Gemini connection failed: {}", e),
                }),
            }
        }
        "openai" => {
            match client
                .get("https://api.openai.com/v1/models")
                .timeout(std::time::Duration::from_secs(10))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
            {
                Ok(res) if res.status().is_success() => Json(TestConnectionResponse {
                    success: true,
                    message: "OpenAI connection verified.".to_string(),
                }),
                Ok(res) => Json(TestConnectionResponse {
                    success: false,
                    message: format!("OpenAI error: Status {}", res.status()),
                }),
                Err(e) => Json(TestConnectionResponse {
                    success: false,
                    message: format!("OpenAI connection failed: {}", e),
                }),
            }
        }
        "claude" => {
            match client
                .get("https://api.anthropic.com/v1/models")
                .timeout(std::time::Duration::from_secs(10))
                .header("x-api-key", token)
                .header("anthropic-version", "2023-06-01")
                .send()
                .await
            {
                Ok(res) if res.status().is_success() => Json(TestConnectionResponse {
                    success: true,
                    message: "Claude connection verified.".to_string(),
                }),
                Ok(res) => Json(TestConnectionResponse {
                    success: false,
                    message: format!("Claude error: Status {}", res.status()),
                }),
                Err(e) => Json(TestConnectionResponse {
                    success: false,
                    message: format!("Claude connection failed: {}", e),
                }),
            }
        }
        _ => Json(TestConnectionResponse {
            success: false,
            message: format!("Prover '{}' testing not fully implemented", service),
        }),
    }
}

async fn test_ollama(host: &str, model: Option<&str>) -> Json<TestConnectionResponse> {
    let client = aiome_core::http::get_http_client();

    let url = if host.ends_with('/') {
        format!("{}api/tags", host)
    } else {
        format!("{}/api/tags", host)
    };

    match client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(res) if res.status().is_success() => {
            if let Ok(json) = res.json::<serde_json::Value>().await {
                if let Some(model_name) = model {
                    let models = json.get("models").and_then(|m| m.as_array());
                    if let Some(models) = models {
                        let found = models
                            .iter()
                            .any(|m| m.get("name").and_then(|n| n.as_str()) == Some(model_name));
                        if found {
                            Json(TestConnectionResponse {
                                success: true,
                                message: format!(
                                    "Ollama connection OK. Model '{}' found.",
                                    model_name
                                ),
                            })
                        } else {
                            Json(TestConnectionResponse {
                                success: false,
                                message: format!(
                                    "Ollama connection OK, but model '{}' was not found.",
                                    model_name
                                ),
                            })
                        }
                    } else {
                        Json(TestConnectionResponse {
                            success: true,
                            message: "Ollama connection OK (model list empty).".to_string(),
                        })
                    }
                } else {
                    Json(TestConnectionResponse {
                        success: true,
                        message: "Ollama connection OK.".to_string(),
                    })
                }
            } else {
                Json(TestConnectionResponse {
                    success: false,
                    message: "Ollama responded but failed to parse JSON.".to_string(),
                })
            }
        }
        Ok(res) => Json(TestConnectionResponse {
            success: false,
            message: format!("Ollama returned error status: {}", res.status()),
        }),
        Err(e) => Json(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to Ollama: {}", e),
        }),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/ollama/models",
    responses(
        (status = 200, description = "List available Ollama models", body = serde_json::Value),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn get_ollama_models(
    State(state): State<AppState>,
    _auth: Authenticated,
) -> Result<Json<serde_json::Value>, AppError> {
    let host = state
        .job_queue
        .get_setting_value("ollama_host")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            std::env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| shared::config::DEFAULT_OLLAMA_HOST.to_string())
        });
    let url = format!("{}/api/tags", host.trim_end_matches('/'));
    state.security_policy.validate_url(&url).await?;

    let client = aiome_core::http::get_http_client();

    let res = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| aiome_core::error::AiomeError::RemoteServiceError {
            url: url.clone(),
            source: e.into(),
        })?;

    if res.status().is_success() {
        let json = res.json::<serde_json::Value>().await.map_err(|e| {
            aiome_core::error::AiomeError::RemoteServiceExecutionFailed {
                reason: format!("JSON Parse Error: {}", e),
            }
        })?;
        Ok(Json(json))
    } else {
        Err(aiome_core::error::AiomeError::RemoteServiceError {
            url,
            source: anyhow::anyhow!("Ollama returned error: {}", res.status()),
        }
        .into())
    }
}
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct IdentityResponse {
    pub ai_name: String,
    pub ai_motto: String,
    pub ai_vrm_url: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/settings/identity",
    responses(
        (status = 200, description = "Get AI Identity", body = IdentityResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn get_identity(
    State(state): State<AppState>,
    _auth: Authenticated,
) -> Result<Json<IdentityResponse>, AppError> {
    let ai_name = state
        .job_queue
        .get_setting_value("ai_name")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "Aiome Agent".to_string());
    let ai_motto = state
        .job_queue
        .get_setting_value("ai_motto")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "The Autonomous AI Operating System".to_string());
    let ai_vrm_url = state
        .job_queue
        .get_setting_value("ai_vrm_url")
        .await
        .ok()
        .flatten();

    Ok(Json(IdentityResponse {
        ai_name,
        ai_motto,
        ai_vrm_url,
    }))
}
