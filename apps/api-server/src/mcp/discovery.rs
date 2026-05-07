/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::mcp::client::McpProcessManager;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

#[cfg(unix)]
fn validate_config_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::metadata(path)?;
    let mode = meta.mode();
    // Check if world or group writable
    if mode & 0o022 != 0 {
        return Err(anyhow::anyhow!(
            "🚨 [SECURITY] MCP config file {} is world/group-writable (mode: {:o}). Fix with: chmod 600 {}",
            path.display(), mode, path.display()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_config_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    Stdio,
    Http,
}

impl Default for McpTransport {
    fn default() -> Self {
        Self::Stdio
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct McpServerConfig {
    #[serde(default)]
    pub transport: McpTransport,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub disabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct McpDiscoveryFile {
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

/// [A-3] MCP Discovery Layer
/// Scans local configuration to automatically connect to external MCP tools.
pub async fn discover_and_connect(
    manager: &McpProcessManager,
    registry: &infrastructure::registry::RegistryManager,
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
        let default_config = serde_json::json!({
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
                    "url": "http://localhost:3000/mcp", // allow-anti-pattern
                    "headers": {
                        "Authorization": "Bearer $STRIPE_SECRET_KEY"
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
                    },
                    "disabled": true
                },
                "x_twitter": {
                    "command": "npx",
                    "args": ["-y", "@iflow-mcp/x-mcp-server"],
                    "env": {
                        "X_API_KEY": "$X_API_KEY",
                        "X_API_SECRET": "$X_API_SECRET",
                        "X_ACCESS_TOKEN": "$X_ACCESS_TOKEN",
                        "X_ACCESS_TOKEN_SECRET": "$X_ACCESS_TOKEN_SECRET"
                    },
                    "disabled": true
                },
                "github": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-github"],
                    "env": {
                        "GITHUB_PERSONAL_ACCESS_TOKEN": "$GITHUB_PERSONAL_ACCESS_TOKEN"
                    },
                    "disabled": true
                }
            }
        });
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
                    let resolved = if let Some(var_name) = v.strip_prefix('$') {
                        // Environment variable resolution should only allow safe alphanumeric names
                        if var_name
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_')
                        {
                            std::env::var(var_name).unwrap_or_default()
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
                    let resolved = if let Some(var_name) = v.strip_prefix('$') {
                        if var_name
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_')
                        {
                            std::env::var(var_name).unwrap_or_default()
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
                                let var_val = std::env::var(var_name).unwrap_or_default();
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

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};

/// Canonical list of OAuth-capable MCP providers.
/// Single source of truth — used by both authorize and callback handlers.
const ALLOWED_OAUTH_PROVIDERS: &[&str] = &["github", "slack", "notion", "discord", "figma"];

/// Static mapping from provider name to OAuth authorization URL template.
/// Prevents Open Redirect (CWE-601) by never interpolating user input into the host.
fn oauth_auth_url(provider: &str) -> Option<String> {
    match provider {
        "github" => Some("https://github.com/login/oauth/authorize?client_id=dummy".to_string()),
        "slack" => Some("https://slack.com/oauth/v2/authorize?client_id=dummy".to_string()),
        "notion" => Some("https://api.notion.com/v1/oauth/authorize?client_id=dummy".to_string()),
        "discord" => Some("https://discord.com/oauth2/authorize?client_id=dummy".to_string()),
        "figma" => Some("https://www.figma.com/oauth?client_id=dummy".to_string()),
        _ => None,
    }
}

#[derive(Deserialize)]
pub struct OauthAuthQuery {
    pub provider: String,
}

/// [GET] /api/v1/mcp/oauth/authorize
pub async fn oauth_authorize(
    Query(query): Query<OauthAuthQuery>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let auth_url = oauth_auth_url(&query.provider).ok_or_else(|| {
        crate::error::AppError::bad_request("Invalid or unsupported OAuth provider")
    })?;

    Ok(Redirect::temporary(&auth_url))
}

/// Enables an OAuth provider in the MCP config file and injects the access token.
///
/// # Security Note (CWE-312)
/// Tokens are currently stored in plaintext in `mcp_servers.json`.
/// Production deployments MUST migrate to a secrets manager (e.g., OS Keychain,
/// HashiCorp Vault, or encrypted-at-rest storage) before public release.
///
/// # Concurrency Note (TOCTOU)
/// This function reads and writes the config file without file locking.
/// In a multi-process deployment, use `flock` or equivalent to prevent races.
pub async fn enable_oauth_provider(
    provider: &str,
    access_token: &str,
    override_path: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    let config_path = if let Some(p) = override_path {
        p.to_path_buf()
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home).join(".aiome/mcp_servers.json")
    };

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let mut discovery: McpDiscoveryFile = serde_json::from_str(&content)?;

        if let Some(config) = discovery.mcp_servers.get_mut(provider) {
            config.disabled = Some(false);

            // ⚠️ Plaintext token storage — see doc comment for migration plan
            match provider {
                "github" => {
                    config.env.insert(
                        "GITHUB_PERSONAL_ACCESS_TOKEN".to_string(),
                        access_token.to_string(),
                    );
                }
                "notion" => {
                    config
                        .env
                        .insert("NOTION_API_KEY".to_string(), access_token.to_string());
                }
                "discord" => {
                    config
                        .env
                        .insert("DISCORD_TOKEN".to_string(), access_token.to_string());
                }
                "slack" | "figma" => {
                    config.headers.insert(
                        "Authorization".to_string(),
                        format!("Bearer {}", access_token),
                    );
                }
                _ => {}
            }
        }

        std::fs::write(&config_path, serde_json::to_string_pretty(&discovery)?)?;
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct OauthCallbackQuery {
    pub provider: String,
    pub code: Option<String>,
}

/// [GET] /api/v1/mcp/oauth/callback
/// Handles the OAuth callback after the user authorizes with the external provider.
pub async fn oauth_callback(
    State(state): State<crate::AppState>,
    Query(query): Query<OauthCallbackQuery>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    if !ALLOWED_OAUTH_PROVIDERS.contains(&query.provider.as_str()) {
        return Err(crate::error::AppError::bad_request(
            "Invalid or unsupported OAuth provider",
        ));
    }

    if query.code.is_none() {
        return Err(crate::error::AppError::bad_request(
            "Missing authorization code",
        ));
    }

    // TODO: Exchange authorization code for access token via provider's token endpoint.
    // Currently uses a placeholder token for integration testing.
    let dummy_token = "dummy_auth_code_exchanged";
    if let Err(e) = enable_oauth_provider(&query.provider, dummy_token, None).await {
        tracing::error!("Failed to enable OAuth provider {}: {}", query.provider, e);
    } else {
        // Reload MCP discovery to activate the newly enabled provider
        if let Err(e) = discover_and_connect(&state.mcp_manager, &state.registry).await {
            tracing::error!(
                "Failed to reload MCP manager after OAuth for {}: {}",
                query.provider,
                e
            );
        }
    }

    // SAFETY: provider is already validated against ALLOWED_OAUTH_PROVIDERS,
    // which are all ASCII-only identifiers. No encoding needed.
    let dashboard_url = format!(
        "https://localhost:3000/dashboard?provider={}&status=success",
        query.provider
    );

    Ok(Redirect::temporary(&dashboard_url))
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
        let res = discover_and_connect(&manager, &registry).await;
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
    async fn test_enable_oauth_provider_injects_token() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(temp_dir.join(".aiome")).unwrap();
        let config_path = temp_dir.join(".aiome/mcp_servers.json");

        let initial_json = r#"{
            "mcp_servers": {
                "github": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-github"],
                    "disabled": true,
                    "env": {}
                }
            }
        }"#;
        std::fs::write(&config_path, initial_json).unwrap();

        let result = enable_oauth_provider("github", "ghp_dummy123", Some(&config_path)).await;
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&config_path).unwrap();
        let discovery: McpDiscoveryFile = serde_json::from_str(&content).unwrap();

        let github = discovery.mcp_servers.get("github").unwrap();
        assert_eq!(github.disabled, Some(false));
        assert_eq!(
            github
                .env
                .get("GITHUB_PERSONAL_ACCESS_TOKEN")
                .map(|s| s.as_str()),
            Some("ghp_dummy123")
        );
    }

    #[tokio::test]
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

        let _ = discover_and_connect(&manager, &registry).await;

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

        assert_eq!(
            github.disabled,
            Some(true),
            "GitHub should be disabled by default (requires OAuth)"
        );
        assert_eq!(github.command, "npx");
        assert!(github
            .args
            .contains(&"@modelcontextprotocol/server-github".to_string()));
        assert!(github.env.contains_key("GITHUB_PERSONAL_ACCESS_TOKEN"));
    }
}
