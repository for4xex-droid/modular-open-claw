/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[cfg(unix)]
pub(crate) fn validate_config_permissions(path: &Path) -> Result<()> {
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
pub(crate) fn validate_config_permissions(_path: &Path) -> Result<()> {
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

pub(crate) fn resolve_env_var(
    var_name: &str,
    aiome_config: &shared::config::AiomeConfig,
) -> String {
    if var_name == "DISCORD_TOKEN" {
        aiome_config
            .discord_token
            .as_ref()
            .map(|t| {
                use secrecy::ExposeSecret;
                t.expose_secret().to_string()
            })
            .unwrap_or_default()
    } else if var_name == "TELEGRAM_TOKEN" {
        aiome_config
            .telegram_token
            .as_ref()
            .map(|t| {
                use secrecy::ExposeSecret;
                t.expose_secret().to_string()
            })
            .unwrap_or_default()
    } else {
        std::env::var(var_name).unwrap_or_default()
    }
}
