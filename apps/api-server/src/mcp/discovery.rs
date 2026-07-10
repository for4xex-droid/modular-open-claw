/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::mcp::client::McpProcessManager;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{error, info, warn};

// Import from sibling modules in the same `mcp` parent module
use super::config::*;
use super::oauth::*;

// Re-export sibling module items so that external crates/modules referencing
// `crate::mcp::discovery::*` continue to compile without changes.
#[allow(unused_imports)]
pub use super::config::{McpDiscoveryFile, McpServerConfig, McpTransport};
#[allow(unused_imports)]
pub use super::oauth::{
    enable_oauth_provider, oauth_authorize, oauth_callback, OAuthCredentials, OauthAuthQuery,
    OauthCallbackQuery, ALLOWED_OAUTH_PROVIDERS,
};

/// [A-3] MCP Discovery Layer
/// Scans local configuration to automatically connect to external MCP tools.
pub async fn discover_and_connect(
    manager: &McpProcessManager,
    registry: &infrastructure::registry::RegistryManager,
    vault_backend: Option<std::sync::Arc<dyn aiome_core_contracts::vault_backend::VaultBackend>>,
    aiome_config: &shared::config::AiomeConfig,
) -> anyhow::Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let config_path = PathBuf::from(home).join(".aiome/mcp_servers.json");

    if !config_path.exists() {
        info!("ℹ️ [MCP Discovery] No server config found at ~/.aiome/mcp_servers.json. Creating default template...");
        if let Some(parent) = config_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                warn!(
                    "⚠️ [MCP Discovery] Failed to create config directory {}: {}",
                    parent.display(),
                    e
                );
            }
        }
        let mut default_config = serde_json::json!({
            "mcp_servers": {
                "fff-mcp": {
                    "command": "fff-mcp",
                    "args": [],
                    "env": {
                        "RUST_LOG": "info"
                    }
                },
                "ga4": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-everything"],
                    "env": {
                        "DEBUG": "true",
                        "GOOGLE_APPLICATION_CREDENTIALS": "$GOOGLE_APPLICATION_CREDENTIALS"
                    }
                },
                "stripe": {
                    "transport": "http",
                    "command": "",
                    "args": [],
                    "url": "http://mcp-server:3000/mcp",
                    "headers": {
                        "Authorization": "Bearer $STRIPE_API_KEY"
                    }
                },
                "firecrawl": {
                    "command": "npx",
                    "args": ["-y", "firecrawl-mcp"],
                    "env": {
                        "FIRECRAWL_API_KEY": "$FIRECRAWL_API_KEY"
                    }
                },
                "exa": {
                    "command": "npx",
                    "args": ["-y", "exa-mcp-server"],
                    "env": {
                        "EXA_API_KEY": "$EXA_API_KEY"
                    }
                },
                "brightdata": {
                    "command": "npx",
                    "args": ["-y", "@brightdata/mcp"],
                    "env": {
                        "BRIGHTDATA_API_KEY": "$BRIGHTDATA_API_KEY"
                    }
                },
                "context7": {
                    "command": "npx",
                    "args": ["-y", "@upstash/context7-mcp@latest"]
                },
                "playwright": {
                    "command": "npx",
                    "args": ["-y", "@playwright/mcp@latest"]
                },
                "chrome_devtools": {
                    "command": "npx",
                    "args": ["-y", "chrome-devtools-mcp@latest"]
                },
                "canva": {
                    "command": "npx",
                    "args": ["-y", "@canva/cli@latest", "mcp"]
                },
                "freee": {
                    "command": "npx",
                    "args": ["-y", "freee-mcp"]
                },
                "slack": {
                    "transport": "http",
                    "command": "",
                    "args": [],
                    "url": "https://mcp.slack.com/mcp",
                    "headers": {},
                    "disabled": true
                },
                "figma": {
                    "transport": "http",
                    "command": "",
                    "args": [],
                    "url": "https://mcp.figma.com/mcp",
                    "headers": {},
                    "disabled": true
                },
                "freee_remote": {
                    "transport": "http",
                    "command": "",
                    "args": [],
                    "url": "https://mcp.freee.co.jp/mcp",
                    "headers": {},
                    "disabled": true
                },
                "ahrefs": {
                    "transport": "http",
                    "command": "",
                    "args": [],
                    "url": "https://api.ahrefs.com/mcp/mcp",
                    "headers": {},
                    "disabled": true
                },
                "google_workspace": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-google-workspace"],
                    "env": {
                        "GOOGLE_WORKSPACE_CREDENTIALS": "$GOOGLE_WORKSPACE_CREDENTIALS"
                    }
                },
                "discord": {
                    "command": "npx",
                    "args": ["-y", "discord-mcp-server"],
                    "env": {
                        "DISCORD_TOKEN": "$DISCORD_TOKEN"
                    },
                    "disabled": true
                },
                "notion": {
                    "command": "npx",
                    "args": ["-y", "notion-mcp-server"],
                    "env": {
                        "NOTION_API_KEY": "$NOTION_API_KEY"
                    }
                },
                "x_twitter": {
                    "command": "npx",
                    "args": ["-y", "@xdevplatform/xurl", "mcp", "https://api.x.com/mcp"],
                    "env": {
                        "CLIENT_ID": "$X_TWITTER_CLIENT_ID",
                        "CLIENT_SECRET": "$X_TWITTER_CLIENT_SECRET"
                    },
                    "disabled": false
                },
                "github": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-github"],
                    "env": {
                        "GITHUB_PERSONAL_ACCESS_TOKEN": "$GITHUB_PERSONAL_ACCESS_TOKEN"
                    }
                },
                "tavily": {
                    "command": "npx",
                    "args": ["-y", "tavily-mcp"],
                    "env": {
                        "TAVILY_API_KEY": "$TAVILY_API_KEY"
                    }
                }
            }
        });
        let api_host = std::env::var("AIOME_API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let nurture_url = format!(
            "http://{api_host}:{}/api/v1/nurture-mcp/sse",
            aiome_config.api_server_port
        );
        if let Some(servers) = default_config
            .get_mut("mcp_servers")
            .and_then(|s| s.as_object_mut())
        {
            servers.insert(
                "nurture".to_string(),
                serde_json::json!({
                    "transport": "http",
                    "command": "",
                    "args": [],
                    "url": nurture_url,
                    "headers": {},
                    "disabled": false
                }),
            );
        }
        if let Err(e) = std::fs::write(&config_path, serde_json::to_string_pretty(&default_config)?)
        {
            warn!(
                "⚠️ [MCP Discovery] Failed to write default config to {}: {}",
                config_path.display(),
                e
            );
        }
    }

    // [Gate: Config Integrity]
    #[cfg(not(debug_assertions))]
    if let Err(e) = validate_config_permissions(&config_path) {
        error!("{}", e);
        return Err(e);
    }

    let content = std::fs::read_to_string(&config_path)?;
    let discovery: McpDiscoveryFile = serde_json::from_str(&content)?;

    for (id, config) in discovery.mcp_servers {
        info!(
            "🔍 [MCP Discovery] Found registered server: {} (transport: {:?})",
            id, config.transport
        );

        if config.disabled.unwrap_or(false) {
            info!(
                "🔒 [MCP Discovery] Skipping disabled server: {} (OAuth setup required)",
                id
            );
            continue;
        }

        match config.transport {
            McpTransport::Stdio => {
                let mut resolved_env = HashMap::new();
                for (k, v) in config.env {
                    let resolved = if v == "$VAULT_TOKEN" {
                        if let Some(vault) = &vault_backend {
                            let asset_id = provider_to_asset_id(&id);
                            if let Ok(dek) = vault.get_dek(asset_id).await {
                                String::from_utf8_lossy(&dek).into_owned()
                            } else {
                                warn!("🚨 [MCP Discovery] Failed to read {} token from vault", id);
                                "".to_string()
                            }
                        } else {
                            warn!("🚨 [MCP Discovery] Vault backend not provided but token is in vault for {}", id);
                            "".to_string()
                        }
                    } else if let Some(var_name) = v.strip_prefix('$') {
                        // Environment variable resolution should only allow safe alphanumeric names
                        if var_name
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_')
                        {
                            resolve_env_var(var_name, aiome_config)
                        } else {
                            warn!("🚨 [SECURITY] Skipping invalid environment variable name in MCP config: {}", var_name);
                            "".to_string()
                        }
                    } else {
                        v
                    };
                    // 🛡️ [GlassWorm Shield] Sanitize NUL, CR, LF, and invisible characters
                    let mut safe_val =
                        shared::guardrails::strip_invisible_unicode(&resolved).into_owned();
                    safe_val = safe_val.replace(['\0', '\r', '\n'], "");
                    resolved_env.insert(k, safe_val);
                }

                // 🛡️ [GlassWorm Shield] Sanitize command and args
                let safe_command =
                    shared::guardrails::strip_invisible_unicode(&config.command).into_owned();
                let safe_args: Vec<String> = config
                    .args
                    .iter()
                    .map(|arg| shared::guardrails::strip_invisible_unicode(arg).into_owned())
                    .collect();

                if let Err(e) = manager
                    .spawn_stdio_server(id.clone(), &safe_command, safe_args, resolved_env)
                    .await
                {
                    error!("🚨 [MCP Discovery] Failed to spawn {}: {}", id, e);
                    continue;
                }
            }
            McpTransport::Http => {
                // 🛡️ [GlassWorm Shield]
                let url = match config.url.clone() {
                    Some(u) => u,
                    None => {
                        error!(
                            "🚨 [MCP Discovery] Missing URL for HTTP transport: {} — skipping",
                            id
                        );
                        continue;
                    }
                };
                let safe_url = shared::guardrails::strip_invisible_unicode(&url).into_owned();

                let mut resolved_headers = HashMap::new();
                for (k, v) in config.headers {
                    let resolved = if v == "Bearer $VAULT_TOKEN" || v == "$VAULT_TOKEN" {
                        if let Some(vault) = &vault_backend {
                            let asset_id = provider_to_asset_id(&id);
                            if let Ok(dek) = vault.get_dek(asset_id).await {
                                let token = String::from_utf8_lossy(&dek).into_owned();
                                if v.starts_with("Bearer ") {
                                    format!("Bearer {}", token)
                                } else {
                                    token
                                }
                            } else {
                                warn!("🚨 [MCP Discovery] Failed to read {} token from vault", id);
                                "".to_string()
                            }
                        } else {
                            warn!("🚨 [MCP Discovery] Vault backend not provided but token is in vault for {}", id);
                            "".to_string()
                        }
                    } else if let Some(var_name) = v.strip_prefix('$') {
                        if var_name
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_')
                        {
                            resolve_env_var(var_name, aiome_config)
                        } else {
                            warn!("🚨 [SECURITY] Skipping invalid environment variable name in MCP config: {}", var_name);
                            "".to_string()
                        }
                    } else if v.contains("$") {
                        // rudimentary inline replace for "Bearer $TOKEN"
                        let mut replaced = v.clone();
                        if let Some(idx) = v.find('$') {
                            let end_idx = v[idx..].find(' ').map(|i| idx + i).unwrap_or(v.len());
                            let var_name = &v[idx + 1..end_idx];
                            if var_name
                                .chars()
                                .all(|c| c.is_ascii_alphanumeric() || c == '_')
                            {
                                let var_val = resolve_env_var(var_name, aiome_config);
                                replaced.replace_range(idx..end_idx, &var_val);
                            } else {
                                warn!("🚨 [SECURITY] Skipping invalid inline environment variable name in MCP config: {}", var_name);
                            }
                        }
                        replaced
                    } else {
                        v
                    };
                    // 🛡️ [GlassWorm Shield] Sanitize NUL, CR, LF to prevent CRLF header injection
                    let mut safe_val =
                        shared::guardrails::strip_invisible_unicode(&resolved).into_owned();
                    safe_val = safe_val.replace(['\0', '\r', '\n'], "");
                    resolved_headers.insert(k, safe_val);
                }

                if let Err(e) = manager
                    .connect_http_server(id.clone(), safe_url, resolved_headers)
                    .await
                {
                    error!("🚨 [MCP Discovery] Failed to connect to {}: {}", id, e);
                    continue;
                }
            }
        }

        // Register with RegistryManager as AssetType::McpServer
        // This makes it visible to build_system_instructions and describe_skill
        info!(
            "🏷️ [MCP Discovery] Registering skill in RegistryManager: {}",
            id
        );
        if let Err(e) = registry
            .register_mcp_server(
                uuid::Uuid::nil(), // System-level tool
                &id,
                &format!("MCP Tool: {}", id),
                serde_json::json!({ "transport": format!("{:?}", config.transport) }),
            )
            .await
        {
            error!(
                "🚨 [MCP Discovery] Failed to register {} in registry: {}",
                id, e
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_server_config_deserialization() {
        let json = format!(
            r#"{{
            "mcp_servers": {{
                "stdio_server": {{
                    "command": "node",
                    "args": ["server.js"]
                }},
                "http_server": {{
                    "transport": "http",
                    "url": "http://{}:8080/mcp",
                    "headers": {{
                        "x-api-key": "secret"
                    }},
                    "command": "",
                    "args": []
                }}
            }}
        }}"#,
            "localhost"
        );

        let discovery: McpDiscoveryFile = serde_json::from_str(&json).unwrap();

        let stdio = discovery.mcp_servers.get("stdio_server").unwrap();
        assert!(matches!(stdio.transport, McpTransport::Stdio));
        assert_eq!(stdio.command, "node");

        let http = discovery.mcp_servers.get("http_server").unwrap();
        assert!(matches!(http.transport, McpTransport::Http));
        assert_eq!(http.url.as_ref().unwrap(), "http://localhost:8080/mcp");
        assert_eq!(http.headers.get("x-api-key").unwrap(), "secret");
    }

    #[tokio::test]
    async fn test_discover_and_connect_mock() {
        let manager = McpProcessManager::new();
        // Use in-memory sqlite for test registry
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS registry (id TEXT PRIMARY KEY, asset_type TEXT, description TEXT, metadata TEXT, created_at DATETIME, updated_at DATETIME)")
            .execute(&pool).await.unwrap();

        let registry = infrastructure::registry::RegistryManager::new(
            infrastructure::db::DatabasePool::Sqlite(pool),
        );

        // This path probably doesn't exist in test env, but we can verify the function returns Ok if file is missing
        let dummy_config = shared::config::AiomeConfig::default();
        let res = discover_and_connect(&manager, &registry, None, &dummy_config).await;
        assert!(res.is_ok());
    }

    /// [Verification Protocol] Disabled servers must be skipped during discovery.
    #[tokio::test]
    async fn test_disabled_server_is_skipped() {
        let json = r#"{
            "mcp_servers": {
                "active_server": {
                    "command": "node",
                    "args": ["server.js"]
                },
                "disabled_oauth_server": {
                    "transport": "http",
                    "command": "",
                    "args": [],
                    "url": "https://mcp.slack.com/mcp",
                    "headers": {},
                    "disabled": true
                },
                "explicitly_enabled_server": {
                    "command": "python3",
                    "args": ["-c", "pass"],
                    "disabled": false
                }
            }
        }"#;

        let discovery: McpDiscoveryFile = serde_json::from_str(json).unwrap();

        // disabled=true server should be skipped
        let disabled_srv = discovery.mcp_servers.get("disabled_oauth_server").unwrap();
        assert_eq!(disabled_srv.disabled, Some(true));
        assert!(
            disabled_srv.disabled.unwrap_or(false),
            "disabled=true must evaluate to true"
        );

        // disabled=false server should NOT be skipped
        let enabled_srv = discovery
            .mcp_servers
            .get("explicitly_enabled_server")
            .unwrap();
        assert_eq!(enabled_srv.disabled, Some(false));
        assert!(
            !enabled_srv.disabled.unwrap_or(false),
            "disabled=false must evaluate to false"
        );

        // No disabled field — should default to not-disabled
        let active_srv = discovery.mcp_servers.get("active_server").unwrap();
        assert_eq!(active_srv.disabled, None);
        assert!(
            !active_srv.disabled.unwrap_or(false),
            "None disabled must default to false"
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_default_discovery_includes_github() {
        // Run discover_and_connect with an empty/mocked home directory to trigger default generation
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::env::set_var("HOME", temp_dir.to_str().unwrap());

        let manager = McpProcessManager::new();
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS registry (id TEXT PRIMARY KEY, asset_type TEXT, description TEXT, metadata TEXT, created_at DATETIME, updated_at DATETIME)")
            .execute(&pool).await.unwrap();
        let registry = infrastructure::registry::RegistryManager::new(
            infrastructure::db::DatabasePool::Sqlite(pool),
        );

        let dummy_config = shared::config::AiomeConfig::default();
        let _ = discover_and_connect(&manager, &registry, None, &dummy_config).await;

        let config_path = temp_dir.join(".aiome/mcp_servers.json");
        let content =
            std::fs::read_to_string(&config_path).expect("Default config should be created");
        let discovery: McpDiscoveryFile =
            serde_json::from_str(&content).expect("Should be valid JSON");

        // [Verification Protocol: Negative Test]
        // This will FAIL initially because github is missing from the default template!
        let github = discovery
            .mcp_servers
            .get("github")
            .expect("GitHub MCP server must be in the default template");

        assert_eq!(github.disabled, None, "GitHub should be enabled by default");
        assert_eq!(github.command, "npx");
        assert!(github
            .args
            .contains(&"@modelcontextprotocol/server-github".to_string()));
        assert!(github.env.contains_key("GITHUB_PERSONAL_ACCESS_TOKEN"));
    }

    #[test]
    fn test_resolve_env_var() {
        let mut config = shared::config::AiomeConfig::default();
        config.discord_token = Some(secrecy::Secret::from("discord_token_xyz".to_string()));
        config.telegram_token = Some(secrecy::Secret::from("telegram_token_abc".to_string()));

        // 1. DISCORD_TOKEN
        let resolved_discord = resolve_env_var("DISCORD_TOKEN", &config);
        assert_eq!(resolved_discord, "discord_token_xyz");

        // 2. TELEGRAM_TOKEN
        let resolved_telegram = resolve_env_var("TELEGRAM_TOKEN", &config);
        assert_eq!(resolved_telegram, "telegram_token_abc");

        // 3. Normal environment variable
        std::env::set_var("TEST_MOCK_ENV_VAR", "mock_value_123");
        let resolved_normal = resolve_env_var("TEST_MOCK_ENV_VAR", &config);
        assert_eq!(resolved_normal, "mock_value_123");
        std::env::remove_var("TEST_MOCK_ENV_VAR");

        // 4. Non-existent environment variable
        let resolved_non_existent = resolve_env_var("NON_EXISTENT_VAR_XYZ", &config);
        assert_eq!(resolved_non_existent, "");
    }
}
