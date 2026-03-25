/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use anyhow::{Context, Result};
use secrecy::SecretString;
use std::env;

/// MCP Configuration
#[derive(Clone, Debug)]
pub struct McpConfig {
    /// MCP スキルのメタデータ (Markdown) をプロンプトに注入するかどうか
    pub skill_md_injection: bool,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            skill_md_injection: false,
        }
    }
}

/// Aiome Core Configuration
#[derive(Clone, Debug)]
pub struct AiomeConfig {
    /// データベースファイルのパス
    pub db_path: String,
    /// ログ出力レベル（info, debug, warn等）
    pub log_level: String,
    /// Ollamaサーバーの接続先URL
    pub ollama_host: String,
    /// 使用するOllamaモデル名
    pub ollama_model: String,
    /// Google Gemini APIキー（SecretStringで保護）
    pub gemini_api_key: Option<SecretString>,
    /// OpenAI APIキー（SecretStringで保護）
    pub openai_api_key: Option<SecretString>,
    /// Anthropic APIキー（SecretStringで保護）
    pub anthropic_api_key: Option<SecretString>,
    /// APIサーバーのリッスンポート
    pub api_server_port: u16,
    /// Key Proxy（Abyss Vault）の接続先URL
    pub key_proxy_url: String,
    /// Samsara Hub の接続先URL
    pub samsara_hub_url: String,
    /// CORS許可オリジンのリスト
    pub allowed_origins: Vec<String>,
    /// DRM保護（Abyss Vault）の物理パス
    pub abyss_vault_path: String,
    /// Tremendous APIキー
    pub tremendous_api_key: Option<SecretString>,
    /// 管理者・ギフト受取用メールアドレス
    pub master_email: Option<String>,
    /// XTTSサーバーのエンドポイントURL (Phase 10.1a)
    pub xtts_endpoint: Option<String>,
    /// XTTSで使用するデフォルト話者ID (Phase 10.1a)
    pub xtts_speaker: Option<String>,
    /// MCP 設定
    pub mcp: McpConfig,
}

/// OllamaサーバーのデフォルトURL
pub const DEFAULT_OLLAMA_HOST: &str = "http://127.0.0.1:11434";
/// Key ProxyのデフォルトURL
pub const DEFAULT_KEY_PROXY_URL: &str = "http://127.0.0.1:3017";
/// Samsara HubのデフォルトURL
pub const DEFAULT_SAMSARA_HUB_URL: &str = "http://127.0.0.1:3016";
/// CORS許可オリジンのデフォルト値
pub const DEFAULT_ALLOWED_ORIGINS: &str =
    "http://localhost:1420,http://localhost:5173,http://127.0.0.1:3015";
/// LM StudioのデフォルトURL
pub const DEFAULT_LM_STUDIO_HOST: &str = "http://127.0.0.1:1234";
/// Ruri埋め込みサーバーのデフォルトURL
pub const DEFAULT_RURI_EMBED_URL: &str = "http://127.0.0.1:8100";
/// Abyss Vaultのデフォルトパス
pub const DEFAULT_ABYSS_VAULT_PATH: &str = "~/.aiome/abyss_vault";

impl Default for AiomeConfig {
    fn default() -> Self {
        Self {
            db_path: "sqlite://workspace/aiome.db".to_string(),
            log_level: "info".to_string(),
            ollama_host: DEFAULT_OLLAMA_HOST.to_string(),
            ollama_model: "qwen3.5:9b".to_string(),
            gemini_api_key: None,
            openai_api_key: None,
            anthropic_api_key: None,
            api_server_port: 3015,
            key_proxy_url: DEFAULT_KEY_PROXY_URL.to_string(),
            samsara_hub_url: DEFAULT_SAMSARA_HUB_URL.to_string(),
            allowed_origins: vec!["http://localhost:1420".to_string()],
            abyss_vault_path: DEFAULT_ABYSS_VAULT_PATH.to_string(),
            tremendous_api_key: None,
            master_email: None,
            xtts_endpoint: Some("http://localhost:18020".to_string()),
            xtts_speaker: Some("p225".to_string()),
            mcp: McpConfig::default(),
        }
    }
}

impl AiomeConfig {
    /// 環境変数から設定を読み込む
    pub fn load() -> Result<Self> {
        let db_path =
            env::var("AIOME_DB_PATH").unwrap_or_else(|_| "sqlite://workspace/aiome.db".to_string());

        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        let ollama_host =
            env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_OLLAMA_HOST.to_string());

        let ollama_model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3.5:9b".to_string());

        let api_server_port = env::var("PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3015);

        let key_proxy_url =
            env::var("KEY_PROXY_URL").unwrap_or_else(|_| DEFAULT_KEY_PROXY_URL.to_string());

        let samsara_hub_url = env::var("SAMSARA_HUB_URL")
            .or_else(|_| env::var("SAMSARA_HUB_REST"))
            .unwrap_or_else(|_| DEFAULT_SAMSARA_HUB_URL.to_string());

        let allowed_origins = env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| DEFAULT_ALLOWED_ORIGINS.to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        let abyss_vault_path =
            env::var("ABYSS_VAULT_PATH").unwrap_or_else(|_| DEFAULT_ABYSS_VAULT_PATH.to_string());

        // Load and immediately remove sensitive API keys
        let gemini_api_key = env::var("GEMINI_API_KEY").ok().map(|key| {
            env::remove_var("GEMINI_API_KEY");
            SecretString::from(key)
        });

        let openai_api_key = env::var("OPENAI_API_KEY").ok().map(|key| {
            env::remove_var("OPENAI_API_KEY");
            SecretString::from(key)
        });

        let anthropic_api_key = env::var("ANTHROPIC_API_KEY").ok().map(|key| {
            env::remove_var("ANTHROPIC_API_KEY");
            SecretString::from(key)
        });

        Ok(Self {
            db_path,
            log_level,
            ollama_host,
            ollama_model,
            gemini_api_key,
            openai_api_key,
            anthropic_api_key,
            api_server_port,
            key_proxy_url,
            samsara_hub_url,
            allowed_origins,
            abyss_vault_path,
            tremendous_api_key: env::var("TREMENDOUS_API_KEY").ok().map(|key| {
                env::remove_var("TREMENDOUS_API_KEY");
                SecretString::from(key)
            }),
            master_email: env::var("MASTER_EMAIL").ok(),
            xtts_endpoint: env::var("XTTS_ENDPOINT").ok(),
            xtts_speaker: env::var("XTTS_SPEAKER").ok(),
            mcp: McpConfig {
                skill_md_injection: env::var("MCP_SKILL_MD_INJECTION")
                    .map(|s| s.to_lowercase() == "true")
                    .unwrap_or(false),
            },
        })
    }

    /// ヘルパー: 環境変数からURLを取得し、なければデフォルトを返す
    pub fn get_url(env_name: &str, default: &str) -> String {
        env::var(env_name).unwrap_or_else(|_| default.to_string())
    }
}
