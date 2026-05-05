/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use aiome_core_contracts::error::AiomeError;
use infrastructure::gig_gateway::{ExternalTaskRequest, SecureGigGateway};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, Write};
use tracing::{debug, error, info};
use uuid::Uuid;

/// JSON-RPC 2.0 Request
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, code: i32, message: String, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data,
            }),
        }
    }
}

/// MCP Server (via stdio)
pub struct McpServer {
    gateway: SecureGigGateway,
    wasm_manager: Option<std::sync::Arc<infrastructure::skills::WasmSkillManager>>,
    skill_arena: Option<std::sync::Arc<infrastructure::skills::skill_arena::SkillArena>>,
}

impl McpServer {
    pub fn new(gateway: SecureGigGateway) -> Self {
        Self {
            gateway,
            wasm_manager: None,
            skill_arena: None,
        }
    }

    pub fn with_wasm_manager(
        mut self,
        wasm_manager: std::sync::Arc<infrastructure::skills::WasmSkillManager>,
    ) -> Self {
        self.wasm_manager = Some(wasm_manager);
        self
    }

    pub fn with_skill_arena(
        mut self,
        skill_arena: std::sync::Arc<infrastructure::skills::skill_arena::SkillArena>,
    ) -> Self {
        self.skill_arena = Some(skill_arena);
        self
    }

    /// Runt the stdio loop
    pub async fn run(&self) {
        info!("🚀 [McpServer] Starting JSON-RPC over stdio loop...");
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());

        for line in reader.lines() {
            match line {
                Ok(content) => {
                    if content.trim().is_empty() {
                        continue;
                    }
                    debug!("📥 [McpServer] Received: {}", content);

                    let req: Result<JsonRpcRequest, _> = serde_json::from_str(&content);
                    let response = match req {
                        Ok(mut request) => self.handle_request(&mut request).await,
                        Err(e) => JsonRpcResponse::error(
                            None,
                            -32700,
                            "Parse error".to_string(),
                            Some(json!({ "details": e.to_string() })),
                        ),
                    };

                    // JSON-RPC の通知 (idなし) には応答しない
                    if response.id.is_some() || response.error.is_some() {
                        if let Ok(resp_str) = serde_json::to_string(&response) {
                            debug!("📤 [McpServer] Sending: {}", resp_str);
                            println!("{}", resp_str);
                            let _ = io::stdout().flush();
                        }
                    }
                }
                Err(e) => {
                    error!("🚨 [McpServer] Stdin read error: {}", e);
                    break;
                }
            }
        }
        info!("🛑 [McpServer] Stdio loop terminated.");
    }

    async fn handle_request(&self, req: &mut JsonRpcRequest) -> JsonRpcResponse {
        let id = req.id.clone();

        match req.method.as_str() {
            "initialize" => {
                JsonRpcResponse::success(
                    id,
                    json!({
                        "protocolVersion": "2024-11-05", // MCP Protocol version
                        "capabilities": {
                            "tools": { "listChanged": true }
                        },
                        "serverInfo": {
                            "name": "Aiome Node",
                            "version": "1.0.0"
                        }
                    }),
                )
            }
            "tools/list" => {
                let mut response_content = json!({
                    "tools": [
                        {
                            "name": "gig/capabilities",
                            "description": "Get the list of skills this node is capable of performing.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "profile/info",
                            "description": "Get the Agent Card and SLA info for this node.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "gig/publish",
                            "description": "Publish a new Gig (Task Request) to this node.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "agent_id": { "type": "string", "description": "Your Agent ID (UUID)" },
                                    "description": { "type": "string", "description": "The task instructions" },
                                    "max_budget_coins": { "type": "integer", "description": "Budget Escrow (must be > 0)" }
                                },
                                "required": ["agent_id", "description", "max_budget_coins"]
                            }
                        }
                    ]
                });

                if let Some(manager) = &self.wasm_manager {
                    response_content = Self::append_wasm_tools(response_content, manager).await;
                }

                JsonRpcResponse::success(id, response_content)
            }
            "tools/call" => {
                if let Some(params) = req.params.take() {
                    self.handle_tool_call(id, params).await
                } else {
                    JsonRpcResponse::error(id, -32602, "Invalid params".to_string(), None)
                }
            }
            _ => JsonRpcResponse::error(id, -32601, "Method not found".to_string(), None),
        }
    }

    async fn handle_tool_call(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let default_args = json!({});
        let arguments = params.get("arguments").unwrap_or(&default_args);

        match tool_name {
            "gig/capabilities" => {
                use infrastructure::auto_profile::AutoProfileEngine;
                let workspace = std::env::var("WORKSPACE_DIR").unwrap_or_else(|_| ".".to_string());
                let detected = AutoProfileEngine::scan_workspace(std::path::Path::new(&workspace));

                let skills: Vec<String> = detected
                    .into_iter()
                    .map(|s| format!("{}:{}", s.domain, s.skill))
                    .collect();

                JsonRpcResponse::success(
                    id,
                    json!({
                        "content": [{ "type": "text", "text": format!("Skills: {:?}", skills) }]
                    }),
                )
            }
            "profile/info" => JsonRpcResponse::success(
                id,
                json!({
                    "content": [{ "type": "text", "text": "Aiome Node v1.0, Base Rate: 0.001 USDC" }]
                }),
            ),
            "gig/publish" => {
                // Parse arguments
                let agent_id_str = arguments
                    .get("agent_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let description = arguments
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let budget = arguments
                    .get("max_budget_coins")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                let agent_id = match Uuid::parse_str(agent_id_str) {
                    Ok(u) => u,
                    Err(_) => {
                        return JsonRpcResponse::error(
                            id,
                            -32602,
                            "Invalid agent_id format (must be UUID)".to_string(),
                            None,
                        );
                    }
                };

                // Authentication Check: In production, verify an actual token passed in headers/env
                // For MCP via stdio, we assume the host orchestrator authenticates the connection,
                // but we enforce the gateway check.

                let task = ExternalTaskRequest {
                    agent_id,
                    description: description.to_string(),
                    max_budget_coins: budget,
                };

                match self.gateway.accept_external_task(task).await {
                    Ok(intent_id) => JsonRpcResponse::success(
                        id,
                        json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Task accepted by SecureGigGateway. Intent ID: {}", intent_id)
                            }]
                        }),
                    ),
                    Err(AiomeError::SecurityViolation { reason }) => JsonRpcResponse::success(
                        id,
                        json!({
                            "content": [{ "type": "text", "text": format!("Task Rejected by Constitutional Firewall: {}", reason) }],
                            "isError": true
                        }),
                    ),
                    Err(e) => JsonRpcResponse::error(
                        id,
                        -32000,
                        "Internal Server Error".to_string(),
                        Some(json!({"details": e.to_string()})),
                    ),
                }
            }
            _ if tool_name.starts_with("wasm/") => {
                let skill_name = &tool_name[5..];
                if skill_name.contains('/')
                    || skill_name.contains('\\')
                    || skill_name.contains("..")
                {
                    tracing::warn!("Path traversal blocked in WASM execution: {}", skill_name);
                    return JsonRpcResponse::error(
                        id,
                        -32602,
                        "Invalid skill name: potential path traversal detected".to_string(),
                        None,
                    );
                }

                if let Some(manager) = &self.wasm_manager {
                    // SEC-1: Enforce TypeState Pattern
                    // Do NOT bypass dry-run. Always verify before executing.
                    let args_str = serde_json::to_string(&arguments).unwrap_or_default();
                    let unverified = infrastructure::skills::UnverifiedSkill {
                        name: skill_name.to_string(),
                        input_test_payload: args_str.clone(),
                    };

                    let verified = match unverified.verify(manager).await {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!("Skill {} failed verification: {}", skill_name, e);
                            if let Some(arena) = &self.skill_arena {
                                arena.record_outcome(skill_name, false, 0, -10.0).await;
                            }
                            return JsonRpcResponse::error(
                                id,
                                -32603,
                                format!("Skill verification failed: {}", e),
                                None,
                            );
                        }
                    };

                    let start_time = std::time::Instant::now();

                    // Add tokio::time::timeout(10s) as a safety net beyond Extism's timeout
                    let res = tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        manager.call_skill(&verified, "call", &args_str, None),
                    )
                    .await;

                    let latency = start_time.elapsed().as_millis() as u64;
                    let (is_success, karma_delta, response) = match res {
                        Ok(Ok(output)) => (
                            true,
                            1.0,
                            JsonRpcResponse::success(
                                id.clone(),
                                json!({ "content": [{ "type": "text", "text": output }] }),
                            ),
                        ),
                        Ok(Err(e)) => (
                            false,
                            -1.0,
                            JsonRpcResponse::error(
                                id.clone(),
                                -32000,
                                "Skill Execution Error".to_string(),
                                Some(json!({ "details": e.to_string() })),
                            ),
                        ),
                        Err(_) => {
                            // Timeout
                            (
                                false,
                                -2.0,
                                JsonRpcResponse::error(
                                    id.clone(),
                                    -32000,
                                    "Skill Execution Timeout (10s)".to_string(),
                                    None,
                                ),
                            )
                        }
                    };

                    if let Some(arena) = &self.skill_arena {
                        arena
                            .record_outcome(skill_name, is_success, latency, karma_delta)
                            .await;
                    }

                    response
                } else {
                    JsonRpcResponse::error(
                        id,
                        -32601,
                        "WASM Manager not initialized".to_string(),
                        None,
                    )
                }
            }
            _ => JsonRpcResponse::error(id, -32601, format!("Unknown tool: {}", tool_name), None),
        }
    }

    /// Appends WASM skills to the MCP tools/list response
    pub async fn append_wasm_tools(
        mut response: Value,
        wasm_manager: &std::sync::Arc<infrastructure::skills::WasmSkillManager>,
    ) -> Value {
        let skills = wasm_manager.list_skills_with_metadata();

        if let Some(tools_arr) = response.get_mut("tools").and_then(|v| v.as_array_mut()) {
            for skill in skills {
                // Determine JSON schema properties from inputs. Very basic inference.
                let mut properties = serde_json::Map::new();
                for input in skill.inputs {
                    properties.insert(
                        input,
                        json!({ "type": "string" }), // Simplify for now
                    );
                }

                tools_arr.push(json!({
                    "name": format!("wasm/{}", skill.name),
                    "description": skill.description,
                    "inputSchema": {
                        "type": "object",
                        "properties": properties
                    }
                }));
            }
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use infrastructure::skills::WasmSkillManager;
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_mcp_wasm_adapter_tools_list() -> anyhow::Result<()> {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&temp_dir)?;

        let wasm_manager = Arc::new(
            WasmSkillManager::new(&temp_dir, &temp_dir)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?,
        );

        let response: serde_json::Value = McpServer::append_wasm_tools(
            json!({
                "tools": [
                    { "name": "gig/capabilities" }
                ]
            }),
            &wasm_manager,
        )
        .await;

        let tools = response
            .get("tools")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing tools array"))?;
        assert_eq!(tools.len(), 1);

        std::fs::remove_dir_all(temp_dir).ok();
        Ok(())
    }

    #[tokio::test]
    async fn test_mcp_wasm_adapter_execution_timeout_and_feedback() -> anyhow::Result<()> {
        struct DummyGigEngine;
        #[async_trait::async_trait]
        impl aiome_core_contracts::gig::GigEngine for DummyGigEngine {
            async fn publish_intent(
                &self,
                _intent: aiome_core_contracts::gig::GigIntent,
            ) -> Result<uuid::Uuid, AiomeError> {
                Ok(uuid::Uuid::new_v4())
            }
            async fn submit_bid(
                &self,
                _bid: aiome_core_contracts::gig::GigBid,
            ) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn accept_bid(
                &self,
                _intent_id: uuid::Uuid,
                _bid_id: uuid::Uuid,
            ) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn deliver(
                &self,
                _deliverable: aiome_core_contracts::gig::GigDeliverable,
            ) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn verify_and_settle(
                &self,
                _order_id: uuid::Uuid,
            ) -> Result<aiome_core_contracts::gig::VerificationResult, AiomeError> {
                Err(AiomeError::Infrastructure {
                    reason: "".to_string(),
                })
            }
        }

        struct DummyValidator;
        #[async_trait::async_trait]
        impl aiome_core_contracts::traits::ConstitutionalValidator for DummyValidator {
            async fn verify_constitutional(
                &self,
                _output: &str,
                _soul_md: &str,
            ) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&temp_dir)?;

        let wasm_manager = Arc::new(
            WasmSkillManager::new(&temp_dir, &temp_dir)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?,
        );
        let skill_arena = Arc::new(infrastructure::skills::skill_arena::SkillArena::new());

        let gateway = SecureGigGateway::new(
            Arc::new(DummyGigEngine),
            Arc::new(DummyValidator),
            infrastructure::rate_limiter::AgentRateLimiter::new(10).expect("Constant 10 is valid"),
        );

        let server = McpServer::new(gateway)
            .with_wasm_manager(wasm_manager.clone())
            .with_skill_arena(skill_arena.clone());

        // Test missing tool error (WASM tool but missing file)
        let req_params = json!({ "name": "wasm/non_existent_skill", "arguments": {} });
        let resp = server.handle_tool_call(None, req_params).await;

        // It should return an error
        assert!(resp.error.is_some());

        // Check if the feedback was recorded in SkillArena
        let stats = skill_arena.get_stats("non_existent_skill").await;
        assert!(stats.is_some(), "Feedback should be recorded even on error");
        assert_eq!(stats.unwrap().failure_count, 1);

        std::fs::remove_dir_all(temp_dir).ok();
        Ok(())
    }

    #[tokio::test]
    async fn test_mcp_wasm_adapter_tools_list_integration() -> anyhow::Result<()> {
        use infrastructure::skills::SkillMetadata;
        use std::fs::File;
        use std::io::Write;

        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&temp_dir)?;

        // Mock a metadata file
        let meta = SkillMetadata {
            name: "test-skill".to_string(),
            description: "A test skill".to_string(),
            capabilities: vec![],
            inputs: vec!["param1".to_string()],
            outputs: vec![],
            permissions: aiome_core_contracts::PermissionManifest::default(),
            allowed_hosts: vec![],
        };
        let meta_path = temp_dir.join("test-skill.meta.json");
        let mut file = File::create(&meta_path)?;
        file.write_all(serde_json::to_string(&meta)?.as_bytes())?;

        let wasm_manager = Arc::new(
            WasmSkillManager::new(&temp_dir, &temp_dir)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?,
        );

        let response: serde_json::Value = McpServer::append_wasm_tools(
            json!({
                "tools": [
                    { "name": "gig/capabilities" }
                ]
            }),
            &wasm_manager,
        )
        .await;

        let tools = response
            .get("tools")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing tools array"))?;
        assert_eq!(tools.len(), 2, "WASM skill should be appended");

        let wasm_tool = tools
            .iter()
            .find(|t| t.get("name").and_then(|n| n.as_str()) == Some("wasm/test-skill"))
            .ok_or_else(|| anyhow::anyhow!("WASM tool not found"))?;

        let desc = wasm_tool
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("");
        assert_eq!(desc, "A test skill");

        std::fs::remove_dir_all(temp_dir).ok();
        Ok(())
    }
}
