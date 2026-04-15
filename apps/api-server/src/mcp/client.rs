/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
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
    pub fn spawn(
        id: String,
        cmd: &str,
        args: Vec<String>,
        envs: HashMap<String, String>,
    ) -> Result<Arc<Self>> {
        info!(
            "🚀 [MCP] Spawning stdio server: {} for session: {}",
            cmd, id
        );

        // (P-6) Strict MCP Command Validation
        let allowed_cmds = shared::mcp_constants::ALLOWED_MCP_COMMANDS;
        if !allowed_cmds.contains(&cmd) {
            return Err(anyhow!(
                "🚨 [SECURITY VIOLATION] Unapproved command '{}'. Only {:?} are allowed for MCP.",
                cmd,
                allowed_cmds
            ));
        }

        if cmd == "npx" || cmd == "uvx" {
            // Must have a package argument starting with a safe prefix
            // Search for first positional argument that isn't a flag
            let pkg = args.iter().find(|a| !a.starts_with('-'));
            if let Some(p) = pkg {
                let allowed_prefixes = shared::mcp_constants::ALLOWED_MCP_PREFIXES;
                if !allowed_prefixes.iter().any(|prefix| p.starts_with(prefix)) {
                    return Err(anyhow!(
                        "🚨 [SECURITY VIOLATION] Unapproved package '{}'. Must start with one of {:?}",
                        p, allowed_prefixes
                    ));
                }
            } else {
                return Err(anyhow!(
                    "🚨 [SECURITY VIOLATION] Missing package name for {}",
                    cmd
                ));
            }
        } else {
            // Binary commands (like fff-mcp, python3, node) skip prefix validation
            // because they are either system-level binaries or pre-installed and trusted.
        }

        // Use tokio::process::Command for async I/O
        let mut child = Command::new(cmd)
            .args(args)
            .envs(envs) // (P-3) Append environment variables
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

/// Unified MCP Endpoint that wraps different transport implementations.
pub enum McpEndpoint {
    Stdio(Arc<McpClient>),
    Http(Arc<crate::mcp::http_client::McpHttpClient>),
}

impl McpEndpoint {
    pub async fn list_tools(&self) -> Result<Vec<crate::mcp::types::McpTool>> {
        match self {
            Self::Stdio(c) => c.list_tools().await,
            Self::Http(c) => c.list_tools().await,
        }
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<crate::mcp::types::CallToolResult> {
        match self {
            Self::Stdio(c) => c.call_tool(name, arguments).await,
            Self::Http(c) => c.call_tool(name, arguments).await,
        }
    }

    pub fn last_activity(&self) -> std::time::Instant {
        match self {
            Self::Stdio(c) => *c.last_activity.read().unwrap_or_else(|e| e.into_inner()),
            Self::Http(_c) => std::time::Instant::now(), // HTTP is stateless mostly, but we could track it
        }
    }
}

const MAX_MCP_PROCESSES: usize = 10;

pub struct McpProcessManager {
    clients: Arc<Mutex<HashMap<String, Arc<McpEndpoint>>>>,
}

impl McpProcessManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_client(&self, id: &str) -> Option<Arc<McpEndpoint>> {
        let clients = self.clients.lock().await;
        clients.get(id).cloned()
    }

    pub async fn spawn_stdio_server(
        &self,
        id: String,
        cmd: &str,
        args: Vec<String>,
        envs: HashMap<String, String>,
    ) -> Result<Arc<McpEndpoint>> {
        let mut clients = self.clients.lock().await;

        // Evict oldest if we are at MAX_MCP_PROCESSES limit
        if clients.len() >= MAX_MCP_PROCESSES && !clients.contains_key(&id) {
            let oldest_id = clients
                .iter()
                .min_by_key(|(_, c)| c.last_activity())
                .map(|(k, _)| k.clone());
            if let Some(oldest_id) = oldest_id {
                info!(
                    "💥 [MCP] Reached max process limit ({}). Evicting least recently used: {}",
                    MAX_MCP_PROCESSES, oldest_id
                );
                clients.remove(&oldest_id);
            }
        }

        let client = McpClient::spawn(id.clone(), cmd, args, envs)?;
        let endpoint = Arc::new(McpEndpoint::Stdio(client));
        clients.insert(id, endpoint.clone());
        Ok(endpoint)
    }

    pub async fn connect_http_server(
        &self,
        id: String,
        url: String,
        headers: HashMap<String, String>,
    ) -> Result<Arc<McpEndpoint>> {
        let mut clients = self.clients.lock().await;

        let client = crate::mcp::http_client::McpHttpClient::new(id.clone(), url, headers);
        let endpoint = Arc::new(McpEndpoint::Http(Arc::new(client)));
        clients.insert(id, endpoint.clone());
        Ok(endpoint)
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

    pub async fn remove_client(&self, id: &str) -> bool {
        let mut clients = self.clients.lock().await;
        let removed = clients.remove(id).is_some();
        if removed {
            info!("🔌 [MCP] Manually removed client: {}", id);
        }
        removed
    }

    pub async fn reap_idle_clients(&self, timeout: std::time::Duration) {
        let mut clients = self.clients.lock().await;
        let now = std::time::Instant::now();
        clients.retain(|id, client| {
            let last_activity = client.last_activity();
            let is_idle = now.duration_since(last_activity) >= timeout;
            if is_idle {
                info!(
                    "💤 [MCP] Reaping idle client: {} (idle time: {:?})",
                    id,
                    now.duration_since(last_activity)
                );
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
        for i in 0..(MAX_MCP_PROCESSES + 1) {
            manager
                .spawn_stdio_server(
                    format!("client{}", i),
                    "python3",
                    vec!["-c".to_string(), "print('{}')".to_string()],
                    HashMap::new(),
                )
                .await
                .unwrap(); // allow-anti-pattern
        }
        let clients = manager.active_client_ids().await;
        assert!(
            clients.len() <= MAX_MCP_PROCESSES,
            "Should not exceed MAX_MCP_PROCESSES"
        );
    }

    #[tokio::test]
    async fn test_mcp_reap_idle() {
        let manager = McpProcessManager::new();
        let endpoint = manager
            .spawn_stdio_server(
                "idle_client".to_string(),
                "python3",
                vec!["-c".to_string(), "print('{}')".to_string()],
                HashMap::new(),
            )
            .await
            .unwrap(); // allow-anti-pattern

        // artificially age the client's last_activity
        if let McpEndpoint::Stdio(client) = &*endpoint {
            if let Ok(mut act) = client.last_activity.write() {
                *act = std::time::Instant::now() - Duration::from_secs(100);
            }
        }

        manager.reap_idle_clients(Duration::from_millis(10)).await;
        let clients = manager.active_client_ids().await;
        assert!(clients.is_empty(), "Idle client should have been reaped");
    }

    #[tokio::test]
    async fn test_mcp_remove_client() {
        let manager = McpProcessManager::new();
        // Since node is allowed, we can spawn it, but echo is NOT allowed anymore!
        // We will spawn a dummy process that passes validation.
        // We use "node" "-e" "console.log('{}')"
        manager
            .spawn_stdio_server(
                "to_remove".to_string(),
                "python3",
                vec!["-c".to_string(), "print('{}')".to_string()],
                HashMap::new(),
            )
            .await
            .unwrap(); // allow-anti-pattern

        assert_eq!(manager.active_client_ids().await.len(), 1);

        let removed = manager.remove_client("to_remove").await;
        assert!(removed, "Client should be removed successfully");
        assert_eq!(manager.active_client_ids().await.len(), 0);
    }

    #[tokio::test]
    async fn test_mcp_client_spawn_validation() {
        // Red test: Try to spawn unapproved command
        let res = McpClient::spawn(
            "test1".to_string(),
            "rm",
            vec!["-rf".to_string(), "/".to_string()],
            HashMap::new(),
        );
        if let Err(e) = res {
            assert!(e.to_string().contains("SECURITY VIOLATION"));
        } else {
            panic!("Expected SECURITY VIOLATION error");
        }

        // Red test: Try to spawn npx with unapproved package
        let res2 = McpClient::spawn(
            "test2".to_string(),
            "npx",
            vec!["-y".to_string(), "evil-package".to_string()],
            HashMap::new(),
        );
        if let Err(e) = res2 {
            assert!(e.to_string().contains("Unapproved package"));
        } else {
            panic!("Expected Unapproved package error");
        }

        // Red test: Try to spawn npx with approved package
        // This should not fail validation but might fail actual OS spawn if package doesn't exist.
        // We just check that error is NOT a SECURITY VIOLATION
        let res3 = McpClient::spawn(
            "test3".to_string(),
            "npx",
            vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-postgres".to_string(),
            ],
            HashMap::new(),
        );
        if let Err(e) = res3 {
            assert!(
                !e.to_string().contains("SECURITY VIOLATION"),
                "Should pass security violation: {}",
                e
            );
        }

        // Green test: Try to spawn fff-mcp binary command
        // Since it's an allowed binary command, it should skip the package prefix check.
        // It might fail OS spawn if fff-mcp isn't installed, but it should NOT be a SECURITY VIOLATION.
        let res4 = McpClient::spawn("test4".to_string(), "fff-mcp", vec![], HashMap::new());
        if let Err(e) = res4 {
            assert!(
                !e.to_string().contains("SECURITY VIOLATION"),
                "Should pass security violation: {}",
                e
            );
        }
    }
}
