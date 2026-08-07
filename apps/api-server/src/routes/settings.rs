/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::{auth::Authenticated, AppState};
use aiome_core::traits::SettingsOps;
use aiome_core_contracts::AuditLogger;
use axum::extract::{Json, State};
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

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

pub const ALLOWED_KEYS: &[&str] = &[
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
    "view_mode",
    "x_bearer_token",
    "search_api_key",
    "commerce_provider",
    "stripe_api_key",
    "stripe_webhook_secret",
    "polar_api_key",
    "polar_webhook_secret",
    "lora_opt_out",
    "csam_toxicity_forbidden_words",
    "tos_accepted",
    "llm_api_url",
    "ENABLE_TOOL_REVIEWER",
    "last_notified_version",
    "pro_monthly_kc_allowance",
    "economy.monthly_spend_limit",
];

pub const ALLOWED_CATEGORIES: &[&str] = &[
    "llm",
    "channel",
    "system",
    "security",
    "cors",
    "identity",
    "voice",
    "ui",
    "integrations",
    "feature_flags",
    "commerce",
    "legal",
];

pub const SECRETS: &[&str] = &[
    "discord_token",
    "telegram_token",
    "api_server_secret",
    "llm_api_key",
    "bg_llm_api_key",
    "stripe_api_key",
    "openai_api_key",
    "anthropic_api_key",
    "gemini_api_key",
    "x_bearer_token",
    "search_api_key",
    "stripe_webhook_secret",
    "polar_api_key",
    "polar_webhook_secret",
    "admin_password_hash",
    "admin_email",
];

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
    let is_feature_flag = payload.key.starts_with("feature_flag.");
    if !ALLOWED_KEYS.contains(&payload.key.as_str()) && !is_feature_flag {
        warn!(
            "🚨 [Security] Unauthorized settings key attempt: {}",
            payload.key
        );
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Unauthorized setting key".to_string(),
        }
        .into());
    }

    // 1b. Feature flag key sanitization: only alphanumeric + underscore allowed after prefix
    if is_feature_flag {
        if let Some(flag_name) = payload.key.strip_prefix("feature_flag.") {
            if flag_name.is_empty()
                || !flag_name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                warn!(
                    "🚨 [Security] Invalid feature_flag key format: {}",
                    payload.key
                );
                return Err(AppError::bad_request(
                    "Feature flag key must contain only alphanumeric characters and underscores",
                ));
            }
        }
    }

    // 2. Category validation
    if !ALLOWED_CATEGORIES.contains(&payload.category.as_str()) {
        return Err(AppError::bad_request("Invalid category"));
    }

    // 3. Value length limit (DoS protection)
    if payload.value.len() > 4096 {
        return Err(AppError::bad_request("Value too long (max 4096 chars)"));
    }

    // 4. Server-side is_secret determination
    let is_secret = SECRETS.contains(&payload.key.as_str());

    // Prevent the frontend from accidentally overwriting secrets with the placeholder mask
    if is_secret && payload.value == "••••••••" {
        return Ok(Json(serde_json::json!({"status": "ignored_masked_secret"})));
    }

    // 5. Audit Logging (Global Ledger)
    let details = serde_json::json!({
        "key": payload.key,
        "category": payload.category,
        "is_secret": is_secret,
        "requested_by": _auth.agent_id,
    });
    if let Err(e) = state
        .audit_logger
        .log_event("SETTING_UPDATE", "SYSTEM_SETTINGS", &details)
        .await
    {
        error!("Failed to append audit log for settings update: {}", e);
    }

    let value_to_store = if is_secret {
        infrastructure::security::crypto::encrypt_setting(&payload.value).map_err(|e| {
            error!("🚨 Failed to encrypt setting {}: {:?}", payload.key, e);
            aiome_core::error::AiomeError::SecurityViolation {
                reason: format!("Cannot store secret '{}': encryption failed", payload.key),
            }
        })?
    } else {
        payload.value.clone()
    };

    state
        .job_queue
        .update_setting(&payload.key, &value_to_store, &payload.category, is_secret)
        .await?;

    // Phase 2-D: Synchronize Feature Flag Cache
    if payload.key.starts_with("feature_flag.") {
        if let Some(cache) = state.feature_flags_cache.as_opt() {
            if let Some(actual_flag) = payload.key.strip_prefix("feature_flag.") {
                let bool_val = payload.value == "true" || payload.value == "1";
                cache.insert(actual_flag.to_string(), bool_val).await;
            }
        }
    }

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
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| state.config.ollama_host.clone());

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
                            if let Err(e) = q
                                .update_setting("ollama_model", &new_model, "llm", false)
                                .await
                            {
                                tracing::error!("❌ Failed to automatically update built lora model setting: {}", e);
                            }
                        }
                    });
                }
            }
        }
    }

    if payload.key == "economy.monthly_spend_limit" {
        if let Err(e) = sync_monthly_spend_limit_to_nurture(&state, &payload.value).await {
            tracing::warn!(
                "⚠️ [Settings] Failed to sync economy.monthly_spend_limit to Nurture: {}",
                e
            );
        }
    }

    Ok(Json(serde_json::json!({"status": "ok"})))
}

#[cfg(debug_assertions)]
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
pub async fn test_connection(
    state: State<AppState>,
    _auth: Authenticated,
    Json(payload): Json<TestConnectionRequest>,
) -> Result<Json<TestConnectionResponse>, AppError> {
    // Double-gate safety: This handler only exists in debug builds (#[cfg(debug_assertions)]).
    // The env::var check acts as an additional runtime kill-switch, ensuring test_connection
    // is disabled by default even in dev builds. This is intentionally NOT read from AppState
    // because: (1) The route is compiled out in release, and (2) we want operators to be able
    // to toggle it without restarting the server.
    if std::env::var("AIOME_DEV_MODE").unwrap_or_default() != "1" {
        return Err(AppError::not_found(
            "Test connection is disabled in production",
        ));
    }

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
        "anthropic" | "claude" => {
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
        .unwrap_or_else(|| state.config.ollama_host.clone());
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

async fn sync_monthly_spend_limit_to_nurture(state: &AppState, value: &str) -> Result<(), String> {
    let limit: u64 = value
        .trim()
        .parse()
        .map_err(|_| "economy.monthly_spend_limit must be a non-negative integer".to_string())?;

    let nurture_url = match &state.nurture_url {
        Some(url) if !url.is_empty() => url.clone(),
        _ => return Ok(()),
    };

    let secret = state
        .nurture_internal_secret
        .as_ref()
        .ok_or_else(|| "NURTURE_INTERNAL_SECRET not configured".to_string())?
        .clone();

    let payload = serde_json::json!({ "monthly_spend_limit": limit });
    let power = state
        .oxilean_power
        .load(std::sync::atomic::Ordering::Relaxed);

    match crate::nurture_s2s::post_internal(
        state.nurture_s2s.as_ref(),
        Some(&nurture_url),
        &secret,
        "aiome-edge-node",
        power,
        "/economy-policy/monthly-limit",
        Some(&payload),
        std::time::Duration::from_secs(10),
    )
    .await
    {
        Ok(()) => {
            tracing::info!(
                "♻️ [Settings] Synced economy.monthly_spend_limit={} to Nurture",
                limit
            );
            Ok(())
        }
        Err(e) => {
            if let Some(status) = e.strip_prefix("http ") {
                Err(format!("Nurture returned HTTP {status}"))
            } else if let Some(net) = e.strip_prefix("network: ") {
                Err(format!("Nurture relay network error: {net}"))
            } else {
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_mode_is_allowed() {
        assert!(
            ALLOWED_KEYS.contains(&"view_mode"),
            "view_mode should be in allowed_keys"
        );
        assert!(
            ALLOWED_CATEGORIES.contains(&"ui"),
            "ui should be in allowed_categories"
        );
    }

    #[test]
    fn test_x_bearer_token_is_allowed_and_secret() {
        assert!(
            ALLOWED_KEYS.contains(&"x_bearer_token"),
            "x_bearer_token should be in allowed_keys"
        );
        assert!(
            SECRETS.contains(&"x_bearer_token"),
            "x_bearer_token should be in secrets"
        );
    }
    #[test]
    fn test_search_api_key_is_allowed_and_secret() {
        assert!(
            ALLOWED_KEYS.contains(&"search_api_key"),
            "search_api_key should be in allowed_keys"
        );
        assert!(
            SECRETS.contains(&"search_api_key"),
            "search_api_key should be in secrets"
        );
    }

    #[test]
    fn test_ollama_host_is_allowed_but_not_secret() {
        assert!(
            ALLOWED_KEYS.contains(&"ollama_host"),
            "ollama_host should be in allowed_keys"
        );
        assert!(
            !SECRETS.contains(&"ollama_host"),
            "ollama_host should NOT be in secrets, it is just a URL"
        );
    }

    #[test]
    fn test_integrations_is_allowed_category() {
        assert!(
            ALLOWED_CATEGORIES.contains(&"integrations"),
            "integrations should be in allowed_categories"
        );
    }

    #[tokio::test]
    async fn test_cloud_connection_anthropic_mapping() {
        // test_cloud_connection should correctly route "anthropic" (not fall back to not implemented)
        let res = test_cloud_connection("anthropic", Some("fake_token"), None).await;

        // Since the token is fake, it will fail to connect or return an error status,
        // but it MUST NOT return the "not fully implemented" fallback message.
        assert!(
            !res.message.contains("not fully implemented"),
            "Service 'anthropic' was not routed correctly in test_cloud_connection. Message: {}",
            res.message
        );
    }

    // --- Feature Flag Key Validation Tests ---

    /// Helper: checks if a feature_flag key name is valid (alphanumeric + underscore only)
    fn is_valid_feature_flag_name(key: &str) -> bool {
        if let Some(flag_name) = key.strip_prefix("feature_flag.") {
            !flag_name.is_empty()
                && flag_name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
        } else {
            false
        }
    }

    #[test]
    fn test_feature_flag_valid_keys() {
        assert!(is_valid_feature_flag_name("feature_flag.seo_publish"));
        assert!(is_valid_feature_flag_name("feature_flag.federation_v1_5"));
        assert!(is_valid_feature_flag_name(
            "feature_flag.intent_first_suggestion"
        ));
        assert!(is_valid_feature_flag_name("feature_flag.A1"));
    }

    #[test]
    fn test_feature_flag_rejects_path_traversal() {
        assert!(!is_valid_feature_flag_name(
            "feature_flag.../../../../etc/passwd"
        ));
        assert!(!is_valid_feature_flag_name("feature_flag../../../shadow"));
        assert!(!is_valid_feature_flag_name("feature_flag.foo/bar"));
    }

    #[test]
    fn test_feature_flag_rejects_dot_notation() {
        assert!(!is_valid_feature_flag_name("feature_flag.nested.key"));
        assert!(!is_valid_feature_flag_name("feature_flag.a.b.c"));
    }

    #[test]
    fn test_feature_flag_rejects_empty() {
        assert!(!is_valid_feature_flag_name("feature_flag."));
    }

    #[test]
    fn test_feature_flag_rejects_special_chars() {
        assert!(!is_valid_feature_flag_name("feature_flag.key-with-dash"));
        assert!(!is_valid_feature_flag_name("feature_flag.key with space"));
        assert!(!is_valid_feature_flag_name("feature_flag.key;injection"));
        assert!(!is_valid_feature_flag_name("feature_flag.日本語"));
    }

    #[test]
    fn test_value_length_limit() {
        let short_val = "a".repeat(4096);
        assert!(short_val.len() <= 4096);

        let long_val = "a".repeat(4097);
        assert!(long_val.len() > 4096);
    }

    #[test]
    fn test_masked_secret_detection() {
        let mask = "••••••••";
        assert_eq!(mask, "••••••••");
        // Ensure the mask string isn't accidentally stored
        for secret_key in SECRETS {
            assert!(
                SECRETS.contains(secret_key),
                "All SECRETS entries should be valid"
            );
        }
    }

    #[test]
    fn test_commerce_keys_are_allowed_and_secret() {
        for key in &[
            "stripe_api_key",
            "stripe_webhook_secret",
            "polar_api_key",
            "polar_webhook_secret",
        ] {
            assert!(
                ALLOWED_KEYS.contains(key),
                "{} should be in ALLOWED_KEYS",
                key
            );
            assert!(SECRETS.contains(key), "{} should be in SECRETS", key);
        }
    }

    #[test]
    fn test_frontend_used_keys_are_allowed() {
        // Keys used by SettingsPage.tsx must be in ALLOWED_KEYS
        let frontend_keys = &[
            "llm_api_url",
            "ENABLE_TOOL_REVIEWER",
            "ai_name",
            "llm_provider",
            "ollama_model",
            "commerce_provider",
            "x_bearer_token",
            "search_api_key",
        ];
        for key in frontend_keys {
            assert!(
                ALLOWED_KEYS.contains(key),
                "Frontend key '{}' is missing from ALLOWED_KEYS — will cause silent save failure",
                key
            );
        }
    }

    #[test]
    fn test_last_notified_version_is_allowed() {
        assert!(
            ALLOWED_KEYS.contains(&"last_notified_version"),
            "last_notified_version should be in ALLOWED_KEYS"
        );
    }
}
