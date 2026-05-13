/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use anyhow::{Context, Result};
use secrecy::SecretString;
use std::env;
use std::path::PathBuf;

/// MCP Configuration
#[derive(Clone, Debug, Default)]
pub struct McpConfig {
    /// MCP スキルのメタデータ (Markdown) をプロンプトに注入するかどうか
    pub skill_md_injection: bool,
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
    /// Abyss Vault Secret Token (for authentication with key-proxy)
    pub vault_secret: Option<SecretString>,
    /// Samsara Hub の接続先URL
    pub samsara_hub_url: String,
    /// CORS許可オリジンのリスト
    pub allowed_origins: Vec<String>,
    /// DRM保護（Abyss Vault）の物理パス
    pub abyss_vault_path: String,
    /// 隔離領域（Protected Area / Vault）の物理パス (Phase 3)
    pub vault_path: PathBuf,
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
    /// パス解決器
    pub resolver: crate::app_data::AppDataResolver,
    /// フロントエンドの静的ファイルパス
    pub frontend_static_path: String,
    /// システムプロンプトに注入するプロジェクトルールの最大文字数
    pub max_project_rules_chars: usize,
    /// WordPress API URL
    pub wp_api_url: Option<String>,
    /// WordPress SDK Enabled
    pub wp_sdk_enabled: bool,
    /// A2A Node URL
    pub a2a_node_url: String,
    /// Geo Optimizer URL
    pub geo_optimizer_url: String,
    /// Geo Citability Threshold
    pub geo_citability_threshold: u32,
    /// RLM API URL
    pub rlm_api_url: String,
    /// ComfyUI URL
    pub comfyui_url: String,
    /// Abyss Vault Server URL
    pub abyss_vault_url: String,
    /// TimesFM Sidecar URL
    pub timesfm_sidecar_url: String,
    /// A2A Node Auth Token (Scrubbed)
    pub a2a_auth_token: Option<SecretString>,
    /// A2A Node Token (Scrubbed)
    pub a2a_node_token: Option<SecretString>,
    /// Shadow Clone gRPC Host
    pub shadow_clone_grpc_host: String,
    /// Shadow Clone gRPC Port
    pub shadow_clone_grpc_port: String,
    /// Discord Bot Token
    pub discord_token: Option<SecretString>,
    /// Telegram Bot Token
    pub telegram_token: Option<SecretString>,
}

/// OllamaサーバーのデフォルトURL
pub const DEFAULT_OLLAMA_HOST: &str = "http://ollama:11434";
/// Key ProxyのデフォルトURL
pub const DEFAULT_KEY_PROXY_URL: &str = "http://key-proxy:3017";
/// Samsara HubのデフォルトURL
pub const DEFAULT_SAMSARA_HUB_URL: &str = "http://samsara-hub:3016";
/// CORS許可オリジンのデフォルト値
pub const DEFAULT_ALLOWED_ORIGINS: &str = "*";
/// LM StudioのデフォルトURL
pub const DEFAULT_LM_STUDIO_HOST: &str = "http://lm-studio:1234";
/// Ruri埋め込みサーバーのデフォルトURL
pub const DEFAULT_RURI_EMBED_URL: &str = "http://ruri-embed:8100";
/// Abyss Vaultのデフォルトパス
pub const DEFAULT_ABYSS_VAULT_PATH: &str = "~/.aiome/abyss_vault";
/// 隔離領域のデフォルトパス
pub const DEFAULT_VAULT_PATH: &str = "~/.aiome/vault";

impl Default for AiomeConfig {
    fn default() -> Self {
        let resolver = crate::app_data::AppDataResolver::new().unwrap();
        Self {
            db_path: resolver.db_url(),
            log_level: "info".to_string(),
            ollama_host: DEFAULT_OLLAMA_HOST.to_string(),
            ollama_model: "gemma4:26b".to_string(),
            gemini_api_key: None,
            openai_api_key: None,
            anthropic_api_key: None,
            api_server_port: 3015,
            key_proxy_url: DEFAULT_KEY_PROXY_URL.to_string(),
            vault_secret: None,
            samsara_hub_url: DEFAULT_SAMSARA_HUB_URL.to_string(),
            allowed_origins: vec!["*".to_string()],
            abyss_vault_path: DEFAULT_ABYSS_VAULT_PATH.to_string(),
            vault_path: resolver.resolve("vault"),
            tremendous_api_key: None,
            master_email: None,
            xtts_endpoint: Some("http://xtts:18020".to_string()),
            xtts_speaker: Some("p225".to_string()),
            mcp: McpConfig::default(),
            resolver,
            frontend_static_path: "apps/api-server/static".to_string(),
            max_project_rules_chars: 3000,
            wp_api_url: None,
            wp_sdk_enabled: false,
            a2a_node_url: "http://a2a-node:50051".to_string(),
            geo_optimizer_url: "http://geo-optimizer:8080".to_string(),
            geo_citability_threshold: 60,
            rlm_api_url: "http://rlm-api:3026".to_string(),
            comfyui_url: "http://comfyui:8188".to_string(),
            abyss_vault_url: "http://samsara-hub:3016".to_string(),
            timesfm_sidecar_url: "http://timesfm-sidecar:8000".to_string(),
            a2a_auth_token: None,
            a2a_node_token: None,
            shadow_clone_grpc_host: "localhost".to_string(),
            shadow_clone_grpc_port: "50051".to_string(),
            discord_token: None,
            telegram_token: None,
        }
    }
}

impl AiomeConfig {
    /// 環境変数から設定を読み込む
    pub fn load() -> Result<Self> {
        let resolver = crate::app_data::AppDataResolver::new()
            .map_err(|e| anyhow::anyhow!(e))
            .context("Failed to initialize AppDataResolver")?;

        let db_path = env::var("AIOME_DB_PATH").unwrap_or_else(|_| resolver.db_url());

        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        let ollama_host =
            env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_OLLAMA_HOST.to_string());

        let ollama_model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| "gemma4:26b".to_string());

        let api_server_port = env::var("PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3015);

        let key_proxy_url =
            env::var("KEY_PROXY_URL").unwrap_or_else(|_| DEFAULT_KEY_PROXY_URL.to_string());

        let vault_secret = env::var("VAULT_SECRET").ok().map(|secret| {
            crate::security::scrub_env("VAULT_SECRET");
            SecretString::from(secret)
        });

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

        let vault_path = env::var("VAULT_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| resolver.resolve("vault"));

        // Load and immediately remove sensitive API keys
        let gemini_api_key = env::var("GEMINI_API_KEY").ok().map(|key| {
            crate::security::scrub_env("GEMINI_API_KEY");
            SecretString::from(key)
        });

        let openai_api_key = env::var("OPENAI_API_KEY").ok().map(|key| {
            crate::security::scrub_env("OPENAI_API_KEY");
            SecretString::from(key)
        });

        let anthropic_api_key = env::var("ANTHROPIC_API_KEY").ok().map(|key| {
            crate::security::scrub_env("ANTHROPIC_API_KEY");
            SecretString::from(key)
        });

        let a2a_auth_token = env::var("A2A_AUTH_TOKEN").ok().map(|key| {
            crate::security::scrub_env("A2A_AUTH_TOKEN");
            SecretString::from(key)
        });

        let a2a_node_token = env::var("A2A_NODE_TOKEN").ok().map(|key| {
            crate::security::scrub_env("A2A_NODE_TOKEN");
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
            vault_secret,
            samsara_hub_url,
            allowed_origins,
            abyss_vault_path,
            vault_path,
            tremendous_api_key: env::var("TREMENDOUS_API_KEY").ok().map(|key| {
                crate::security::scrub_env("TREMENDOUS_API_KEY");
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
            resolver,
            frontend_static_path: env::var("FRONTEND_STATIC_PATH")
                .unwrap_or_else(|_| "apps/api-server/static".to_string()),
            max_project_rules_chars: env::var("MAX_PROJECT_RULES_CHARS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3000),
            wp_api_url: env::var("WP_API_URL").ok(),
            wp_sdk_enabled: env::var("WP_SDK_ENABLED").is_ok(),
            a2a_node_url: env::var("A2A_NODE_URL")
                .unwrap_or_else(|_| format!("http://a2a-node:{}", 50051)),
            geo_optimizer_url: env::var("GEO_OPTIMIZER_URL")
                .unwrap_or_else(|_| format!("http://geo-optimizer:{}", 8080)),
            geo_citability_threshold: env::var("GEO_CITABILITY_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            rlm_api_url: env::var("RLM_API_URL")
                .unwrap_or_else(|_| format!("http://rlm-api:{}", 3026)),
            comfyui_url: env::var("COMFYUI_URL")
                .unwrap_or_else(|_| format!("http://comfyui:{}", 8188)),
            abyss_vault_url: env::var("ABYSS_VAULT_URL")
                .unwrap_or_else(|_| format!("http://samsara-hub:{}", 3016)),
            timesfm_sidecar_url: env::var("TIMESFM_SIDECAR_URL")
                .unwrap_or_else(|_| "http://timesfm-sidecar:8000".to_string()),
            a2a_auth_token,
            a2a_node_token,
            shadow_clone_grpc_host: env::var("SHADOW_CLONE_GRPC_HOST")
                .unwrap_or_else(|_| "localhost".to_string()),
            shadow_clone_grpc_port: env::var("SHADOW_CLONE_GRPC_PORT")
                .unwrap_or_else(|_| "50051".to_string()),
            discord_token: env::var("DISCORD_TOKEN").ok().map(|token| {
                crate::security::scrub_env("DISCORD_TOKEN");
                SecretString::from(token)
            }),
            telegram_token: env::var("TELEGRAM_TOKEN").ok().map(|token| {
                crate::security::scrub_env("TELEGRAM_TOKEN");
                SecretString::from(token)
            }),
        })
    }

    /// ヘルパー: 環境変数からURLを取得し、なければデフォルトを返す
    pub fn get_url(env_name: &str, default: &str) -> String {
        env::var(env_name).unwrap_or_else(|_| default.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_config_has_vault_path_expansion() {
        // Mock HOME for consistency
        std::env::set_var("HOME", "/Users/aiome_test");
        std::env::remove_var("AIOME_DEV_MODE");

        let config = AiomeConfig::default();

        #[cfg(target_os = "macos")]
        assert!(config
            .vault_path
            .to_string_lossy()
            .contains("Application Support/com.aiome.nexus/test-cell/vault"));

        #[cfg(not(target_os = "macos"))]
        assert!(
            config
                .vault_path
                .to_string_lossy()
                .contains(".aiome/test-cell/vault")
                || config.vault_path.to_string_lossy().contains("aiome")
        );
    }

    #[test]
    #[serial]
    fn test_a2a_auth_token_scrubbing() {
        std::env::set_var("A2A_AUTH_TOKEN", "test-secret-token");

        let config = AiomeConfig::load().expect("Failed to load config");

        // Assert token was loaded correctly
        use secrecy::ExposeSecret;
        assert_eq!(
            config.a2a_auth_token.unwrap().expose_secret(),
            "test-secret-token"
        );

        // Assert token was scrubbed from environment
        assert!(
            std::env::var("A2A_AUTH_TOKEN").is_err(),
            "A2A_AUTH_TOKEN was not scrubbed!"
        );
    }

    #[test]
    #[serial]
    fn test_watchtower_tokens_scrubbing() {
        std::env::set_var("DISCORD_TOKEN", "mock-discord-123");
        std::env::set_var("TELEGRAM_TOKEN", "mock-telegram-456");

        let config = AiomeConfig::load().expect("Failed to load config");

        use secrecy::ExposeSecret;
        assert_eq!(
            config.discord_token.as_ref().unwrap().expose_secret(),
            "mock-discord-123"
        );
        assert_eq!(
            config.telegram_token.as_ref().unwrap().expose_secret(),
            "mock-telegram-456"
        );

        assert!(
            std::env::var("DISCORD_TOKEN").is_err(),
            "DISCORD_TOKEN was not scrubbed!"
        );
        assert!(
            std::env::var("TELEGRAM_TOKEN").is_err(),
            "TELEGRAM_TOKEN was not scrubbed!"
        );
    }
}
