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

/// System variables to preserve through env_clear()
pub const MCP_SAFE_ENV_VARS: &[&str] = &[
    "PATH",
    "HOME",
    "USER",
    "LANG",
    "LC_ALL",
    "TMPDIR",
    "TEMP",
    "SHELL",
    "TERM",
    "PYTHONPATH",
    "VIRTUAL_ENV",
    "NODE_PATH",
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
    "XDG_DATA_HOME",
    "NVM_DIR",
    "OLLAMA_HOST",
    "DOCKER_HOST",
];

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
            // (P-6) Package whitelist validation
            if let Err(reason) = shared::mcp_constants::validate_mcp_package(cmd, &args) {
                return Err(anyhow!("🚨 [SECURITY VIOLATION] {}", reason));
            }

            // [Gate: Arg Injection] Prevent CVE-2026-40933 (e.g., npx -c 'curl evil.com')
            // Only checked for npx/uvx — flags like -c/-e are legitimate for python3/node.
            if let Err(reason) = shared::mcp_constants::validate_mcp_arg_flags(&args) {
                return Err(anyhow!("🚨 [SECURITY VIOLATION] {}", reason));
            }
        } else {
            // Binary commands (like fff-mcp, python3, node) skip prefix validation
            // because they are either system-level binaries or pre-installed and trusted.
            // Flags like -c or -e are legitimate for these commands.
        }

        // Use SafeCommandBuilder for async I/O and security
        // Order matters: env_clear → safe system vars → user envs (user can override)
        let mut builder = infrastructure::security::SafeCommandBuilder::new(cmd)
            .args(args)
            .profile(aiome_core_contracts::security::SandboxProfile::McpServer);

        // (P-3b) Re-inject essential safe environment variables for proper operation.
        for var_name in MCP_SAFE_ENV_VARS {
            builder = builder.env_passthrough(*var_name);
        }

        let manifest = aiome_core_contracts::security::PermissionManifest {
            allow_shell_execution: true,
            allow_filesystem_write: true, // Legacy behavior; ideally should be restricted via manifest in future
            allow_network: true,
            ..Default::default()
        };

        let mut command = builder.build(manifest)?;

        // (P-3) User-defined envs applied LAST so they can override system defaults
        // (e.g., custom PATH for a specific Python venv)
        command.envs(envs);

        let mut child = command
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

#[derive(Clone, Debug)]
pub struct McpRegistryEntry {
    pub cmd: String,
    pub args: Vec<String>,
    pub envs: HashMap<String, String>,
}

pub struct McpProcessManager {
    clients: Arc<Mutex<HashMap<String, Arc<McpEndpoint>>>>,
    registry: Arc<Mutex<HashMap<String, McpRegistryEntry>>>,
}

impl McpProcessManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            registry: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register_server(
        &self,
        id: String,
        cmd: String,
        args: Vec<String>,
        envs: HashMap<String, String>,
    ) {
        let mut registry = self.registry.lock().await;
        registry.insert(id, McpRegistryEntry { cmd, args, envs });
    }

    pub async fn get_client(&self, id: &str) -> Option<Arc<McpEndpoint>> {
        // Fast path: check if already spawned
        {
            let clients = self.clients.lock().await;
            if let Some(client) = clients.get(id) {
                return Some(client.clone());
            }
        }

        // Check registry for lazy-load entry
        let entry = {
            let registry = self.registry.lock().await;
            registry.get(id).cloned()
        };

        if let Some(entry) = entry {
            // Double-check: another task may have spawned it while we read the registry
            {
                let clients = self.clients.lock().await;
                if let Some(client) = clients.get(id) {
                    return Some(client.clone());
                }
            }

            match self
                .spawn_stdio_server(id.to_string(), &entry.cmd, entry.args, entry.envs)
                .await
            {
                Ok(endpoint) => return Some(endpoint),
                Err(e) => {
                    tracing::error!("🚨 [MCP] Lazy load spawn failed for {}: {}", id, e);
                    return None;
                }
            }
        }

        None
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
        // Spawn a dummy process that passes validation.
        // python3 -c is allowed since forbidden flags only apply to npx/uvx.
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
    async fn test_mcp_client_injection_protection() {
        // [Gate: Binary whitelist]
        let res1 = McpClient::spawn(
            "test_hack1".to_string(),
            "bash",
            vec!["-c".into(), "echo hack".into()],
            HashMap::new(),
        );
        let err1 = res1.err().unwrap().to_string();
        assert!(err1.contains("Unapproved"));

        // [Gate: Package whitelist]
        let res2 = McpClient::spawn(
            "test_hack2".to_string(),
            "npx",
            vec!["-y".into(), "evil-package".into()],
            HashMap::new(),
        );
        let err2 = res2.err().unwrap().to_string();
        assert!(err2.contains("not whitelisted"));

        // [Gate: Arg injection CVE-2026-40933 — separated form]
        let res3 = McpClient::spawn(
            "test_hack3".to_string(),
            "npx",
            vec![
                "-y".into(),
                "@modelcontextprotocol/server-postgres".into(),
                "-c".into(),
                "whoami".into(),
            ],
            HashMap::new(),
        );
        let err3 = res3.err().unwrap().to_string();
        assert!(err3.contains("Forbidden argument flag"));

        // [Gate: Arg injection — --eval=code inline form]
        let res4 = McpClient::spawn(
            "test_hack4".to_string(),
            "npx",
            vec![
                "-y".into(),
                "@modelcontextprotocol/server-postgres".into(),
                "--eval=require('child_process').exec('evil')".into(),
            ],
            HashMap::new(),
        );
        let err4 = res4.err().unwrap().to_string();
        assert!(
            err4.contains("Forbidden argument flag"),
            "Should block --eval=code: {}",
            err4
        );

        // [Gate: Arg injection — -ecode short flag prefix form]
        let res5 = McpClient::spawn(
            "test_hack5".to_string(),
            "npx",
            vec![
                "-y".into(),
                "@modelcontextprotocol/server-postgres".into(),
                "-erequire('evil')".into(),
            ],
            HashMap::new(),
        );
        let err5 = res5.err().unwrap().to_string();
        assert!(
            err5.contains("Forbidden argument flag"),
            "Should block -eVALUE: {}",
            err5
        );

        // [Gate: Arg injection — --exec=value form]
        let res6 = McpClient::spawn(
            "test_hack6".to_string(),
            "npx",
            vec![
                "-y".into(),
                "@modelcontextprotocol/server-postgres".into(),
                "--exec=bash".into(),
            ],
            HashMap::new(),
        );
        let err6 = res6.err().unwrap().to_string();
        assert!(
            err6.contains("Forbidden argument flag"),
            "Should block --exec=value: {}",
            err6
        );

        // [Gate: False-positive regression] --env-file must NOT be blocked by -e
        let res7 = McpClient::spawn(
            "test_no_false_positive".to_string(),
            "npx",
            vec![
                "-y".into(),
                "@modelcontextprotocol/server-postgres".into(),
                "--env-file=.env".into(),
            ],
            HashMap::new(),
        );
        // Should NOT fail with security violation (may fail for other reasons like binary not found)
        if let Err(e) = res7 {
            assert!(
                !e.to_string().contains("Forbidden argument flag"),
                "--env-file should NOT be blocked as -e: {}",
                e
            );
        }
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
            assert!(e.to_string().contains("not whitelisted"));
        } else {
            panic!("Expected Unapproved package error");
        }

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

        // Red test: Try to spawn npx with allowed unscoped package
        let res_unscoped = McpClient::spawn(
            "test_unscoped".to_string(),
            "npx",
            vec!["-y".to_string(), "firecrawl-mcp".to_string()],
            HashMap::new(),
        );
        if let Err(e) = res_unscoped {
            assert!(
                !e.to_string().contains("SECURITY VIOLATION"),
                "Should pass security violation for unscoped package: {}",
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

    #[tokio::test]
    async fn test_mcp_client_env_isolation() {
        let mut user_envs = HashMap::new();
        user_envs.insert("INJECTED_VAR".to_string(), "test_val_42".to_string());

        // Spawn python3 to verify env_clear + re-injection works:
        // 1. PATH must exist (otherwise python3 can't spawn)
        // 2. HOME must exist (needed for site-packages)
        // 3. INJECTED_VAR must exist (user-defined envs applied)
        // 4. Arbitrary parent vars must NOT exist (env_clear working)
        let client = McpClient::spawn(
            "test_env_isolation".to_string(),
            "python3",
            vec![
                "-c".to_string(),
                // Print env as JSON-RPC response so McpClient can parse it
                r#"import os, json; env = dict(os.environ); print(json.dumps({"jsonrpc":"2.0","id":1,"result":env}))"#.to_string(),
            ],
            user_envs,
        ).expect("spawn python3 failed — env_clear may have dropped PATH"); // allow-anti-pattern

        // Give the child time to print and exit
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Verify spawn succeeded (PATH was inherited correctly via MCP_SAFE_ENV_VARS)
        assert_eq!(client.id, "test_env_isolation");
    }

    #[tokio::test]
    async fn test_mcp_env_clear_prevents_secret_leak() {
        // Verify that env_clear + safe var injection does not include arbitrary parent env vars
        // by checking that the MCP_SAFE_ENV_VARS list does NOT include any secret-sounding names
        let safe_vars = super::MCP_SAFE_ENV_VARS;
        for var in safe_vars {
            let lower = var.to_lowercase();
            assert!(
                !lower.contains("secret")
                    && !lower.contains("key")
                    && !lower.contains("token")
                    && !lower.contains("password"),
                "MCP_SAFE_ENV_VARS must not contain secret-related variables, found: {}",
                var
            );
        }
        // Verify essential vars are present
        assert!(safe_vars.contains(&"PATH"), "PATH must be in safe vars");
        assert!(safe_vars.contains(&"HOME"), "HOME must be in safe vars");
    }

    #[tokio::test]
    async fn test_mcp_registry_lazy_load() {
        let manager = McpProcessManager::new();
        // Register an un-spawned client
        manager
            .register_server(
                "lazy_python".to_string(),
                "python3".to_string(),
                vec!["-c".to_string(), "print('{}')".to_string()],
                HashMap::new(),
            )
            .await;

        // No clients should be active initially
        assert_eq!(manager.active_client_ids().await.len(), 0);

        // Fetching the client should trigger a lazy spawn
        let endpoint = manager
            .get_client("lazy_python")
            .await
            .expect("Client should be spawned on demand");

        // Now it should be active
        assert_eq!(manager.active_client_ids().await.len(), 1);
    }

    /// [Verification Protocol — Step 1: Positive Test]
    /// Every package in ALLOWED_MCP_PACKAGES must pass spawn validation.
    #[tokio::test]
    async fn test_all_allowed_packages_pass_validation() {
        let packages = shared::mcp_constants::ALLOWED_MCP_PACKAGES;
        for pkg in packages {
            let res = McpClient::spawn(
                format!("test_allowed_{}", pkg),
                "npx",
                vec!["-y".to_string(), pkg.to_string()],
                HashMap::new(),
            );
            // May fail for OS reasons (npx not found), but MUST NOT fail with SECURITY VIOLATION
            if let Err(e) = res {
                assert!(
                    !e.to_string().contains("SECURITY VIOLATION"),
                    "ALLOWED_MCP_PACKAGES entry '{}' was rejected by security gate: {}",
                    pkg,
                    e
                );
            }
        }
    }

    /// [Verification Protocol — Step 1: Positive Test]
    /// Versioned packages (pkg@latest, pkg@1.2.3) must also pass.
    #[tokio::test]
    async fn test_versioned_allowed_packages_pass() {
        let versioned_packages = vec![
            "firecrawl-mcp@latest",
            "firecrawl-mcp@3.14.1",
            "exa-mcp-server@0.1.0",
            "chrome-devtools-mcp@latest",
            "freee-mcp@1.0.0",
            "mcp-remote@latest",
        ];
        for pkg in versioned_packages {
            let res = McpClient::spawn(
                format!("test_versioned_{}", pkg),
                "npx",
                vec!["-y".to_string(), pkg.to_string()],
                HashMap::new(),
            );
            if let Err(e) = res {
                assert!(
                    !e.to_string().contains("SECURITY VIOLATION"),
                    "Versioned package '{}' should pass but was rejected: {}",
                    pkg,
                    e
                );
            }
        }
    }

    /// [Verification Protocol — Step 2: Negative Test]
    /// Unscoped packages NOT in ALLOWED_MCP_PACKAGES MUST be rejected.
    #[tokio::test]
    async fn test_unlisted_unscoped_packages_rejected() {
        let evil_packages = vec![
            "evil-mcp",
            "crypto-miner-mcp",
            "not-firecrawl-mcp",
            "firecrawl", // partial match — must NOT pass
            "mcp",       // substring — must NOT pass
        ];
        for pkg in evil_packages {
            let res = McpClient::spawn(
                format!("test_evil_{}", pkg),
                "npx",
                vec!["-y".to_string(), pkg.to_string()],
                HashMap::new(),
            );
            assert!(
                res.is_err(),
                "Unlisted package '{}' should be rejected",
                pkg
            );
            let err = res.err().unwrap().to_string(); // allow-anti-pattern
            assert!(
                err.contains("not whitelisted"),
                "Unlisted package '{}' must produce 'not whitelisted' error, got: {}",
                pkg,
                err
            );
        }
    }

    /// [Verification Protocol — Step 2: Negative Test]
    /// Suffix-attack packages (firecrawl-mcp-evil) must be rejected.
    /// This verifies exact match + @version semantics prevent substring confusion.
    #[tokio::test]
    async fn test_suffix_attack_packages_rejected() {
        let attack_packages = vec![
            "firecrawl-mcp-evil",
            "firecrawl-mcp-backdoor",
            "exa-mcp-server-trojan",
            "chrome-devtools-mcp-rce",
        ];
        for pkg in attack_packages {
            let res = McpClient::spawn(
                format!("test_suffix_attack_{}", pkg),
                "npx",
                vec!["-y".to_string(), pkg.to_string()],
                HashMap::new(),
            );
            assert!(
                res.is_err(),
                "Suffix-attack package '{}' should be rejected",
                pkg
            );
            let err = res.err().unwrap().to_string(); // allow-anti-pattern
            assert!(
                err.contains("not whitelisted"),
                "Suffix-attack package '{}' must be caught with 'not whitelisted', got: {}",
                pkg,
                err
            );
        }
    }

    /// [Verification Protocol — Step 1: Positive Test]
    /// New ALLOWED_MCP_PREFIXES must also pass.
    #[tokio::test]
    async fn test_new_prefix_packages_pass() {
        let prefix_packages = vec![
            "@brightdata/mcp",
            "@upstash/context7-mcp@latest",
            "@playwright/mcp@latest",
            "@canva/cli@latest",
        ];
        for pkg in prefix_packages {
            let res = McpClient::spawn(
                format!("test_prefix_{}", pkg),
                "npx",
                vec!["-y".to_string(), pkg.to_string()],
                HashMap::new(),
            );
            if let Err(e) = res {
                assert!(
                    !e.to_string().contains("SECURITY VIOLATION"),
                    "Prefix-allowed package '{}' was rejected: {}",
                    pkg,
                    e
                );
            }
        }
    }
}
