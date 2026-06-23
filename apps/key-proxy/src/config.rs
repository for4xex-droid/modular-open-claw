/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use chrono::{Datelike, Utc};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ProxyRequest {
    pub(crate) caller_id: String,
    pub(crate) prompt: String,
    pub(crate) system: Option<String>,
    pub(crate) endpoint: String,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct ProxyResponse {
    pub(crate) content: String,
    pub(crate) stop_reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) total_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) response_time_ms: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct EmbedResponse {
    pub(crate) embedding: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) response_time_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct WpProxyRequest {
    pub(crate) caller_id: String,
    pub(crate) title: String,
    pub(crate) content: String,
    pub(crate) status: String,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct WpProxyResponse {
    pub(crate) link: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct QuotaState {
    pub(crate) total_calls: u64,
    pub(crate) last_reset_day: u32,
    #[serde(default)]
    pub(crate) per_caller_calls: HashMap<String, u64>,
}

impl Default for QuotaState {
    fn default() -> Self {
        Self {
            total_calls: 0,
            last_reset_day: Utc::now().ordinal(),
            per_caller_calls: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) gemini_key: Arc<SecretString>,
    pub(crate) vault_secret: Arc<SecretString>,
    pub(crate) client: reqwest::Client,
    pub(crate) state: Arc<RwLock<QuotaState>>,
    pub(crate) auth_manager: Arc<dyn infrastructure::auth::AuthManager>,
    pub(crate) persistence_path: PathBuf,
    pub(crate) caller_quotas: Arc<HashMap<String, u64>>,
    pub(crate) wp_api_url: Option<String>,
    pub(crate) wp_api_token: Option<Arc<SecretString>>,
    pub(crate) gemini_model: String,
    pub(crate) gemini_embed_model: String,
    pub(crate) vault_backend:
        Arc<infrastructure::security::sqlite_vault_backend::UniversalVaultBackend>,
}
