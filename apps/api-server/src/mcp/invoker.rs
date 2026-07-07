/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::mcp::client::McpProcessManager;
use aiome_core::error::AiomeError;
use aiome_core_contracts::traits::McpToolInvoker;
use async_trait::async_trait;
use std::sync::Arc;

/// `McpProcessManager` をラップし、ワークフロー実行エンジンから MCP ツールを呼び出す
pub struct McpProcessManagerInvoker {
    manager: Arc<McpProcessManager>,
}

impl McpProcessManagerInvoker {
    pub fn new(manager: Arc<McpProcessManager>) -> Self {
        Self { manager }
    }

    fn format_call_result(res: crate::mcp::types::CallToolResult) -> String {
        let mut out = String::new();
        for c in res.content {
            match c {
                crate::mcp::types::McpContent::Text { text } => out.push_str(&text),
                crate::mcp::types::McpContent::Image { .. } => out.push_str("[Image Data]"),
            }
        }
        if res.is_error {
            format!("Error: {}", out)
        } else {
            out
        }
    }
}

#[async_trait]
impl McpToolInvoker for McpProcessManagerInvoker {
    async fn invoke_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<String, AiomeError> {
        // 1. server_name をクライアント ID として直接解決
        if let Some(client) = self.manager.get_client(server_name).await {
            let res = tokio::time::timeout(
                tokio::time::Duration::from_secs(30),
                client.call_tool(tool_name, arguments),
            )
            .await
            .map_err(|_| AiomeError::RemoteServiceTimeout { timeout_secs: 30 })?
            .map_err(|e| AiomeError::RemoteServiceExecutionFailed {
                reason: format!("MCP tool call on server '{}': {}", server_name, e),
            })?;
            return Ok(Self::format_call_result(res));
        }

        // 2. フォールバック: アクティブクライアントを走査（server_name がツール名の場合）
        let active_clients = self.manager.active_client_ids().await;
        for cid in active_clients {
            if let Some(client) = self.manager.get_client(&cid).await {
                let tools =
                    tokio::time::timeout(tokio::time::Duration::from_secs(2), client.list_tools())
                        .await
                        .map_err(|_| AiomeError::RemoteServiceTimeout { timeout_secs: 2 })?
                        .map_err(|e| AiomeError::RemoteServiceExecutionFailed {
                            reason: format!("MCP list_tools on '{}': {}", cid, e),
                        })?;

                if tools.iter().any(|t| t.name == tool_name) {
                    let res = tokio::time::timeout(
                        tokio::time::Duration::from_secs(30),
                        client.call_tool(tool_name, arguments),
                    )
                    .await
                    .map_err(|_| AiomeError::RemoteServiceTimeout { timeout_secs: 30 })?
                    .map_err(|e| AiomeError::RemoteServiceExecutionFailed {
                        reason: format!("MCP tool '{}' on server '{}': {}", tool_name, cid, e),
                    })?;
                    return Ok(Self::format_call_result(res));
                }
            }
        }

        Err(AiomeError::Infrastructure {
            reason: format!(
                "MCP server '{}' not found and tool '{}' not available on any active server",
                server_name, tool_name
            ),
        })
    }
}
