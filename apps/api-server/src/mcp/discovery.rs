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
use std::path::PathBuf;
use tracing::{error, info, warn};

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
            let _ = std::fs::create_dir_all(parent);
        }
        let default_config = serde_json::json!({
            "mcp_servers": {
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
                }
            }
        });
        let _ = std::fs::write(&config_path, serde_json::to_string_pretty(&default_config)?);
    }

    let content = std::fs::read_to_string(&config_path)?;
    let discovery: McpDiscoveryFile = serde_json::from_str(&content)?;

    for (id, config) in discovery.mcp_servers {
        info!(
            "🔍 [MCP Discovery] Found registered server: {} (transport: {:?})",
            id, config.transport
        );

        match config.transport {
            McpTransport::Stdio => {
                let mut resolved_env = HashMap::new();
                for (k, v) in config.env {
                    let resolved = if let Some(var_name) = v.strip_prefix('$') {
                        std::env::var(var_name).unwrap_or_default()
                    } else {
                        v
                    };
                    // 🛡️ [GlassWorm Shield] Sanitize NUL, newline, and invisible characters
                    let mut safe_val =
                        shared::guardrails::strip_invisible_unicode(&resolved).into_owned();
                    safe_val = safe_val.replace(['\0', '\n'], "");
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
                let url = config
                    .url
                    .clone()
                    .ok_or_else(|| anyhow!("Missing URL for HTTP transport: {}", id))?;
                let safe_url = shared::guardrails::strip_invisible_unicode(&url).into_owned();

                let mut resolved_headers = HashMap::new();
                for (k, v) in config.headers {
                    let resolved = if let Some(var_name) = v.strip_prefix('$') {
                        std::env::var(var_name).unwrap_or_default()
                    } else if v.contains("$") {
                        // rudimentary inline replace for "Bearer $TOKEN"
                        let mut replaced = v.clone();
                        if let Some(idx) = v.find('$') {
                            let end_idx = v[idx..].find(' ').map(|i| idx + i).unwrap_or(v.len());
                            let var_name = &v[idx + 1..end_idx];
                            let var_val = std::env::var(var_name).unwrap_or_default();
                            replaced.replace_range(idx..end_idx, &var_val);
                        }
                        replaced
                    } else {
                        v
                    };
                    // 🛡️ [GlassWorm Shield]
                    let mut safe_val =
                        shared::guardrails::strip_invisible_unicode(&resolved).into_owned();
                    safe_val = safe_val.replace(['\0', '\n'], "");
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

        let discovery: McpDiscoveryFile = serde_json::from_str(&json).unwrap(); // allow-anti-pattern

        let stdio = discovery.mcp_servers.get("stdio_server").unwrap(); // allow-anti-pattern
        assert!(matches!(stdio.transport, McpTransport::Stdio));
        assert_eq!(stdio.command, "node");

        let http = discovery.mcp_servers.get("http_server").unwrap(); // allow-anti-pattern
        assert!(matches!(http.transport, McpTransport::Http));
        assert_eq!(http.url.as_ref().unwrap(), "http://localhost:8080/mcp"); // allow-anti-pattern
        assert_eq!(http.headers.get("x-api-key").unwrap(), "secret"); // allow-anti-pattern
    }

    #[tokio::test]
    async fn test_discover_and_connect_mock() {
        let manager = McpProcessManager::new();
        // Use in-memory sqlite for test registry
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap(); // allow-anti-pattern
        sqlx::query("CREATE TABLE IF NOT EXISTS registry (id TEXT PRIMARY KEY, asset_type TEXT, description TEXT, metadata TEXT, created_at DATETIME, updated_at DATETIME)")
            .execute(&pool).await.unwrap(); // allow-anti-pattern

        let registry = infrastructure::registry::RegistryManager::new(
            infrastructure::db::DatabasePool::Sqlite(pool),
        );

        // This path probably doesn't exist in test env, but we can verify the function returns Ok if file is missing
        let res = discover_and_connect(&manager, &registry).await;
        assert!(res.is_ok());
    }
}
