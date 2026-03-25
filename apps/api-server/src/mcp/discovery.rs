/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::mcp::client::McpProcessManager;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{error, info, warn};

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize)]
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
        info!("ℹ️ [MCP Discovery] No server config found at ~/.aiome/mcp_servers.json");
        return Ok(());
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
                if let Err(e) = manager
                    .spawn_stdio_server(id.clone(), &config.command, config.args)
                    .await
                {
                    error!("🚨 [MCP Discovery] Failed to spawn {}: {}", id, e);
                    continue;
                }
            }
            McpTransport::Http => {
                let url = config
                    .url
                    .clone()
                    .ok_or_else(|| anyhow!("Missing URL for HTTP transport: {}", id))?;
                if let Err(e) = manager
                    .connect_http_server(id.clone(), url, config.headers)
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
        let json = r#"{
            "mcp_servers": {
                "stdio_server": {
                    "command": "node",
                    "args": ["server.js"]
                },
                "http_server": {
                    "transport": "http",
                    "url": "http://localhost:8080/mcp",
                    "headers": {
                        "x-api-key": "secret"
                    },
                    "command": "",
                    "args": []
                }
            }
        }"#;

        let discovery: McpDiscoveryFile = serde_json::from_str(json).unwrap();

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

        let registry = infrastructure::registry::RegistryManager::new(pool);

        // This path probably doesn't exist in test env, but we can verify the function returns Ok if file is missing
        let res = discover_and_connect(&manager, &registry).await;
        assert!(res.is_ok());
    }
}
