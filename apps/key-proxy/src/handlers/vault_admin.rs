/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::config::AppState;
use crate::telemetry::{redact_display, sanitize_for_log};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretStatusItem {
    pub key: String,
    pub category: String,
    pub is_set: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultStatusResponse {
    pub secrets: Vec<SecretStatusItem>,
    pub total: usize,
    pub configured: usize,
}

#[derive(Debug, Deserialize)]
pub struct StoreSecretRequest {
    pub key: String,
    pub value: String,
}

fn get_category(key: &str) -> &'static str {
    match key {
        "GEMINI_API_KEY" | "OPENAI_API_KEY" | "ANTHROPIC_API_KEY" | "TTS_OPENAI_API_KEY"
        | "FAL_KEY" | "SEARCH_API_KEY" => "ai",
        "STRIPE_API_KEY"
        | "STRIPE_WEBHOOK_SECRET"
        | "POLAR_API_KEY"
        | "POLAR_WEBHOOK_SECRET"
        | "TREMENDOUS_API_KEY" => "commerce",
        "X_BEARER_TOKEN" | "DISCORD_TOKEN" | "TELEGRAM_TOKEN" => "bridge",
        "API_SERVER_SECRET"
        | "FEDERATION_SECRET"
        | "TIMESFM_AUTH_TOKEN"
        | "JWT_PRIVATE_KEY_B64" => "infrastructure",
        _ => "unknown",
    }
}

pub async fn handle_vault_status(
    State(state): State<AppState>,
) -> Result<Json<VaultStatusResponse>, (StatusCode, String)> {
    let existing_keys = state.vault_backend.list_secret_keys().await.map_err(|e| {
        // Do not echo internal error details to clients (path/backend context).
        tracing::error!(
            "❌ [KeyProxy] Failed to list vault keys: {}",
            redact_display(&e)
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to retrieve vault status".to_string(),
        )
    })?;

    let mut secrets = Vec::new();
    let mut configured = 0;

    for &key in shared::security::ALLOWED_VAULT_SECRETS {
        let is_set = existing_keys.contains(&key.to_string());
        if is_set {
            configured += 1;
        }
        secrets.push(SecretStatusItem {
            key: key.to_string(),
            category: get_category(key).to_string(),
            is_set,
        });
    }

    let total = secrets.len();
    Ok(Json(VaultStatusResponse {
        secrets,
        total,
        configured,
    }))
}

pub async fn handle_vault_store(
    State(state): State<AppState>,
    Json(payload): Json<StoreSecretRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if !shared::security::ALLOWED_VAULT_SECRETS.contains(&payload.key.as_str()) {
        tracing::warn!(
            "🛡️ [KeyProxy] Store secret violation: key '{}' is not in the allowed secrets whitelist",
            sanitize_for_log(&payload.key)
        );
        return Err((
            StatusCode::BAD_REQUEST,
            "Requested key is not allowed by policy".to_string(),
        ));
    }

    state
        .vault_backend
        .store_secret(&payload.key, &payload.value)
        .await
        .map_err(|e| {
            tracing::error!(
                "❌ [KeyProxy] Failed to store secret key='{}': {}",
                sanitize_for_log(&payload.key),
                redact_display(&e)
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to store secret".to_string(),
            )
        })?;

    Ok(StatusCode::OK)
}

pub async fn handle_vault_delete(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    if !shared::security::ALLOWED_VAULT_SECRETS.contains(&key.as_str()) {
        tracing::warn!(
            "🛡️ [KeyProxy] Delete secret violation: key '{}' is not in the allowed secrets whitelist",
            sanitize_for_log(&key)
        );
        return Err((
            StatusCode::BAD_REQUEST,
            "Requested key is not allowed by policy".to_string(),
        ));
    }

    state.vault_backend.delete_secret(&key).await.map_err(|e| {
        tracing::error!(
            "❌ [KeyProxy] Failed to delete secret key='{}': {}",
            sanitize_for_log(&key),
            redact_display(&e)
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to delete secret".to_string(),
        )
    })?;

    Ok(StatusCode::OK)
}
