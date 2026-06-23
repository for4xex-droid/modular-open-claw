/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Deserialize, Serialize)]
pub struct StoreSecretRequest {
    pub key: String,
    pub value: String,
}

pub async fn vault_status(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let vault_secret = state
        .config
        .vault_secret
        .as_ref()
        .ok_or_else(|| AppError::internal("VAULT_SECRET not configured"))?;

    let url = format!(
        "{}/api/v1/admin/status",
        state.config.key_proxy_url.trim_end_matches('/')
    );

    let res = state
        .http_client
        .get(&url)
        .header(
            "Authorization",
            format!("Bearer {}", vault_secret.expose_secret()),
        )
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Failed to connect to key-proxy: {}", e)))?;

    if !res.status().is_success() {
        return Err(AppError::internal(format!(
            "key-proxy returned error status: {}",
            res.status()
        )));
    }

    let body = res
        .json::<VaultStatusResponse>()
        .await
        .map_err(|e| AppError::internal(format!("Failed to parse key-proxy response: {}", e)))?;

    Ok((StatusCode::OK, Json(body)))
}

pub async fn vault_upsert(
    State(state): State<AppState>,
    Json(payload): Json<StoreSecretRequest>,
) -> Result<impl IntoResponse, AppError> {
    let vault_secret = state
        .config
        .vault_secret
        .as_ref()
        .ok_or_else(|| AppError::internal("VAULT_SECRET not configured"))?;

    let url = format!(
        "{}/api/v1/admin/secrets",
        state.config.key_proxy_url.trim_end_matches('/')
    );

    let res = state
        .http_client
        .put(&url)
        .header(
            "Authorization",
            format!("Bearer {}", vault_secret.expose_secret()),
        )
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Failed to connect to key-proxy: {}", e)))?;

    if !res.status().is_success() {
        if res.status() == StatusCode::BAD_REQUEST {
            return Err(AppError::bad_request(
                "Requested key is not allowed by policy",
            ));
        }
        return Err(AppError::internal(format!(
            "key-proxy returned error status: {}",
            res.status()
        )));
    }

    Ok(StatusCode::OK)
}

pub async fn vault_delete(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let vault_secret = state
        .config
        .vault_secret
        .as_ref()
        .ok_or_else(|| AppError::internal("VAULT_SECRET not configured"))?;

    let url = format!(
        "{}/api/v1/admin/secrets/{}",
        state.config.key_proxy_url.trim_end_matches('/'),
        key
    );

    let res = state
        .http_client
        .delete(&url)
        .header(
            "Authorization",
            format!("Bearer {}", vault_secret.expose_secret()),
        )
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Failed to connect to key-proxy: {}", e)))?;

    if !res.status().is_success() {
        if res.status() == StatusCode::BAD_REQUEST {
            return Err(AppError::bad_request(
                "Requested key is not allowed by policy",
            ));
        }
        return Err(AppError::internal(format!(
            "key-proxy returned error status: {}",
            res.status()
        )));
    }

    Ok(StatusCode::OK)
}
