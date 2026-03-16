use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

/// Aiome Core Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiomeConfig {
    pub db_path: String,
    pub log_level: String,
    pub ollama_host: String,
    pub ollama_model: String,
    pub gemini_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub api_server_port: u16,
    pub key_proxy_url: String,
    pub samsara_hub_url: String,
    pub allowed_origins: Vec<String>,
}

impl AiomeConfig {
    pub fn load() -> Result<Self> {
        let db_path = env::var("AIOME_DB_PATH")
            .unwrap_or_else(|_| "sqlite://workspace/aiome.db".to_string());
        
        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        
        let ollama_host = env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
        
        let ollama_model = env::var("OLLAMA_MODEL")
            .unwrap_or_else(|_| "qwen3.5:9b".to_string());

        let api_server_port = env::var("PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3015);

        let key_proxy_url = env::var("KEY_PROXY_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3017".to_string());

        let samsara_hub_url = env::var("SAMSARA_HUB_REST")
            .unwrap_or_else(|_| "http://127.0.0.1:3016".to_string());

        let allowed_origins = env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:1420,http://localhost:5173,http://127.0.0.1:3015".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        Ok(Self {
            db_path,
            log_level,
            ollama_host,
            ollama_model,
            gemini_api_key: env::var("GEMINI_API_KEY").ok(),
            openai_api_key: env::var("OPENAI_API_KEY").ok(),
            anthropic_api_key: env::var("ANTHROPIC_API_KEY").ok(),
            api_server_port,
            key_proxy_url,
            samsara_hub_url,
            allowed_origins,
        })
    }

    /// ヘルパー: 環境変数からURLを取得し、なければデフォルトを返す
    pub fn get_url(env_name: &str, default: &str) -> String {
        env::var(env_name).unwrap_or_else(|_| default.to_string())
    }
}
