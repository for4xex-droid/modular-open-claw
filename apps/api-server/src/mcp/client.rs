/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::mcp::types::{JsonRpcRequest, JsonRpcResponse};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{oneshot, Mutex};
use tracing::{info, warn};

/// Phase 17-B: Zombie Defense - Managed child process.
/// It uses a background task to handle JSON-RPC multiplexing.
pub struct McpClient {
    pub id: String,
    stdin: Arc<Mutex<tokio::process::ChildStdin>>,
    pending_requests: Arc<Mutex<HashMap<i64, oneshot::Sender<JsonRpcResponse>>>>,
    request_counter: AtomicI64,
    pub last_activity: std::sync::RwLock<std::time::Instant>,
}

impl McpClient {
    pub fn spawn(id: String, cmd: &str, args: Vec<String>) -> Result<Arc<Self>> {
        info!(
            "🚀 [MCP] Spawning stdio server: {} for session: {}",
            cmd, id
        );

        if !infrastructure::security::GLOBAL_SECURITY_CONFIG
            .allowed_binaries
            .contains(&cmd.to_string())
        {
            return Err(anyhow!(
                "🚨 [SECURITY VIOLATION] MCP Client command '{}' bypasses BastionGuard whitelist.",
                cmd
            ));
        }

        // Use tokio::process::Command for async I/O
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true) // Defense against zombie processes
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to open stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to open stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("Failed to open stderr"))?;

        let pending_requests = Arc::new(Mutex::new(
            HashMap::<i64, oneshot::Sender<JsonRpcResponse>>::new(),
        ));
        let _pending_requests_clone = pending_requests.clone();
        let client_id = id.clone();

        // Stderr logging task
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                warn!("⚠️ [MCP:{}] stderr: {}", client_id, line);
            }
        });

        // Stdout JSON-RPC parser task
        let pending_requests_for_stdout = pending_requests.clone();
        let client_id_for_stdout = id.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&line) {
                    if let Some(id_val) = response.id.as_i64() {
                        let mut reqs = pending_requests_for_stdout.lock().await;
                        if let Some(tx) = reqs.remove(&id_val) {
                            let _ = tx.send(response);
                        }
                    }
                } else {
                    info!("📖 [MCP:{}] raw line: {}", client_id_for_stdout, line);
                }
            }
            info!(
                "🔌 [MCP:{}] stdout task ended (connection closed)",
                client_id_for_stdout
            );
        });

        Ok(Arc::new(Self {
            id,
            stdin: Arc::new(Mutex::new(stdin)),
            pending_requests,
            request_counter: AtomicI64::new(1),
            last_activity: std::sync::RwLock::new(std::time::Instant::now()),
        }))
    }

    pub async fn call(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        if let Ok(mut last) = self.last_activity.write() {
            *last = std::time::Instant::now();
        }

        let id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Some(serde_json::json!(id)),
        };

        let (tx, rx) = oneshot::channel();
        {
            let mut reqs = self.pending_requests.lock().await;
            reqs.insert(id, tx);
        }

        let mut stdin = self.stdin.lock().await;
        let json_req = serde_json::to_string(&request)? + "\n";
        stdin.write_all(json_req.as_bytes()).await?;
        stdin.flush().await?;

        // Wait for response
        let response = rx
            .await
            .map_err(|_| anyhow!("MCP connection closed before response"))?;

        if let Some(error) = response.error {
            return Err(anyhow!("MCP Error ({}): {}", error.code, error.message));
        }

        response
            .result
            .ok_or_else(|| anyhow!("Empty result from MCP"))
    }

    // High level MCP methods
    pub async fn list_tools(&self) -> Result<Vec<crate::mcp::types::McpTool>> {
        let res = self.call("tools/list", None).await?;
        let list: crate::mcp::types::ListToolsResult = serde_json::from_value(res)?;
        Ok(list.tools)
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<crate::mcp::types::CallToolResult> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });
        let res = self.call("tools/call", Some(params)).await?;
        Ok(serde_json::from_value(res)?)
    }
}

pub struct McpProcessManager {
    clients: Arc<Mutex<HashMap<String, Arc<McpClient>>>>,
}

impl McpProcessManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_client(&self, id: &str) -> Option<Arc<McpClient>> {
        let clients = self.clients.lock().await;
        clients.get(id).cloned()
    }

    pub async fn spawn_stdio_server(
        &self,
        id: String,
        cmd: &str,
        args: Vec<String>,
    ) -> Result<Arc<McpClient>> {
        let mut clients = self.clients.lock().await;

        // Evict oldest if we are at MAX_MCP_PROCESSES limit
        const MAX_MCP_PROCESSES: usize = 5;
        if clients.len() >= MAX_MCP_PROCESSES && !clients.contains_key(&id) {
            let oldest_id = clients
                .iter()
                .min_by_key(|(_, c)| *c.last_activity.read().unwrap())
                .map(|(k, _)| k.clone());
            if let Some(oldest_id) = oldest_id {
                info!("💥 [MCP] Reached max process limit ({}). Evicting least recently used: {}", MAX_MCP_PROCESSES, oldest_id);
                clients.remove(&oldest_id);
            }
        }

        // Must drop lock before calling spawn because spawn takes time? 
        // We can just keep it or spawn first then lock. Let's spawn first to minimize lock time,
        // but wait, if we drop lock, we might exceed MAX_MCP_PROCESSES if multiple spawn concurrently.
        // It's safer to keep the lock, but spawn doesn't "take time" (it's sync OS process creation).
        
        let client = McpClient::spawn(id.clone(), cmd, args)?;
        clients.insert(id, client.clone());
        Ok(client)
    }

    pub async fn active_client_ids(&self) -> Vec<String> {
        let clients = self.clients.lock().await;
        clients.keys().cloned().collect()
    }

    pub async fn kill_all(&self) {
        let mut clients = self.clients.lock().await;
        info!("💥 [MCP] Evicting {} managed MCP clients", clients.len());
        clients.clear();
    }

    pub async fn reap_idle_clients(&self, timeout: std::time::Duration) {
        let mut clients = self.clients.lock().await;
        let now = std::time::Instant::now();
        clients.retain(|id, client| {
            let last_activity = *client.last_activity.read().unwrap();
            let is_idle = now.duration_since(last_activity) >= timeout;
            if is_idle {
                info!("💤 [MCP] Reaping idle client: {} (idle time: {:?})", id, now.duration_since(last_activity));
            }
            !is_idle // keep if not idle
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_mcp_max_processes() {
        let manager = McpProcessManager::new();
        for i in 0..6 {
            manager.spawn_stdio_server(format!("client{}", i), "echo", vec![]).await.unwrap();
        }
        let clients = manager.active_client_ids().await;
        assert!(clients.len() <= 5, "Should not exceed MAX_MCP_PROCESSES");
    }

    #[tokio::test]
    async fn test_mcp_reap_idle() {
        let manager = McpProcessManager::new();
        let client = manager.spawn_stdio_server("idle_client".to_string(), "echo", vec![]).await.unwrap();
        
        // artificially age the client's last_activity
        if let Ok(mut act) = client.last_activity.write() {
            *act = std::time::Instant::now() - Duration::from_secs(100);
        }

        manager.reap_idle_clients(Duration::from_millis(10)).await;
        let clients = manager.active_client_ids().await;
        assert!(clients.is_empty(), "Idle client should have been reaped");
    }
}
