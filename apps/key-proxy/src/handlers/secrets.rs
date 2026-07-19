/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::config::AppState;
use crate::telemetry::sanitize_for_log;
use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct SecretsRequest {
    pub keys: Vec<String>,
}

pub async fn handle_get_secrets(
    State(state): State<AppState>,
    Json(payload): Json<SecretsRequest>,
) -> Result<Json<HashMap<String, String>>, (StatusCode, String)> {
    let mut result = HashMap::new();

    // ホワイトリストに存在するキーのみ要求を許可する (§CISO-1)
    for key in &payload.keys {
        if key.is_empty() {
            continue;
        }

        // ホワイトリスト検証
        if !shared::security::ALLOWED_VAULT_SECRETS.contains(&key.as_str()) {
            tracing::warn!(
                "🛡️ [KeyProxy] Security violation: key '{}' is not in the allowed secrets whitelist",
                sanitize_for_log(key)
            );
            return Err((
                StatusCode::BAD_REQUEST,
                "Requested key is not allowed by policy".to_string(),
            ));
        }

        match state.vault_backend.get_secret(key).await {
            Ok(val) => {
                result.insert(key.clone(), (*val).clone());
            }
            Err(_e) => {
                // 部分成功 (Partial Success): 存在しないキーはスキップし、警告ログを出力する
                // Do not log vault error details (may include path/context).
                tracing::warn!(
                    "⚠️ [KeyProxy] Requested secret for key '{}' was not found in vault",
                    sanitize_for_log(key)
                );
            }
        }
    }

    Ok(Json(result))
}
