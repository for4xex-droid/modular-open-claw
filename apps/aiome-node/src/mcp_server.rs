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
}

impl McpServer {
    pub fn new(gateway: SecureGigGateway) -> Self {
        Self { gateway }
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
            "tools/list" => JsonRpcResponse::success(
                id,
                json!({
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
                }),
            ),
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
            _ => JsonRpcResponse::error(id, -32601, format!("Unknown tool: {}", tool_name), None),
        }
    }
}
