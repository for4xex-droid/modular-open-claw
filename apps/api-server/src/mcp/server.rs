/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::types::*;
use crate::AppState;
use avatar_engine::lip_sync::LipSyncFrame;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::sse::{Event, Sse},
    Json,
};
use futures_util::stream::Stream;
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

pub async fn sse_handler(
    _auth: crate::auth::Authenticated,
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let _safe = 1_u32.clamp(0, 10);
    let session_id = Uuid::new_v4().to_string();
    let (tx, mut rx) = mpsc::channel::<String>(1024); // Bounded channel to prevent OOM DoS

    {
        let mut sessions = state.mcp_sessions.write().await;
        sessions.insert(session_id.clone(), tx);
    }

    info!("🔌 [MCP] New SSE session established: {}", session_id);

    // MCP Spec: The server MUST include a `uri` query parameter in the `endpoint` event's data.
    // Since we are nesting under /api/v1/mcp, the full path is /api/v1/mcp/messages
    let endpoint_url = format!("/api/v1/mcp/messages?sessionId={}", session_id);
    let initial_event = Event::default().event("endpoint").data(endpoint_url);

    let stream = async_stream::stream! {
        yield Ok(initial_event);
        while let Some(msg) = rx.recv().await {
            yield Ok(Event::default().event("message").data(msg));
        }
        info!("🔌 [MCP] SSE session closed: {}", session_id);
    };

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

#[derive(serde::Deserialize)]
pub struct MessageQuery {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

pub async fn message_handler(
    _auth: crate::auth::Authenticated,
    State(state): State<AppState>,
    Query(query): Query<MessageQuery>,
    Json(request): Json<JsonRpcRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let session_id = query.session_id;

    let tx = {
        let sessions = state.mcp_sessions.read().await;
        sessions.get(&session_id).cloned()
    };

    let tx = tx.ok_or((StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    // Process the request background and send response via SSE
    tokio::spawn(async move {
        let response = handle_mcp_request(request, &state).await;
        if let Ok(json_resp) = serde_json::to_string(&response) {
            if let Err(e) = tx.send(json_resp).await {
                warn!(
                    "⚠️ [MCP] Failed to send response back to client (session {}): {}",
                    session_id, e
                );
            }
        }
    });

    Ok(StatusCode::ACCEPTED)
}

async fn handle_mcp_request(req: JsonRpcRequest, state: &AppState) -> JsonRpcResponse {
    let id = req.id.unwrap_or(serde_json::Value::Null);

    match req.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "Aiome MCP Server",
                    "version": "0.1.0"
                }
            })),
            error: None,
        },
        "notifications/initialized" => {
            info!("✅ [MCP] Client initialization confirmed");
            JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id,
                result: Some(serde_json::json!({})),
                error: None,
            }
        }
        "tools/list" => {
            let skill_metas = state.wasm_skill_manager.list_skills_with_metadata();
            let mut tools = Vec::new();

            // 1. Register Wasm Skills
            for meta in skill_metas {
                if is_skill_whitelisted(&meta.name) {
                    tools.push(McpTool {
                        name: meta.name.clone(),
                        description: Some(meta.description),
                        input_schema: serde_json::json!({
                            "type": "object"
                        }),
                    });
                }
            }

            // 2. Register Native STT Tool (Phase 38b)
            tools.push(McpTool {
                name: "transcribe".to_string(),
                description: Some("Transcribe audio file to text and generate LipSync frames using insanely-fast-whisper.".to_string()),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "audio_path": {
                            "type": "string",
                            "description": "Path to the audio file (e.g. .wav, .mp3)"
                        }
                    },
                    "required": ["audio_path"]
                }),
            });

            // 3. Register Cortex Query Tool (Phase C)
            tools.push(McpTool {
                name: "cortex_search".to_string(),
                description: Some(
                    "Search the global memory (Cortex Wiki) for project knowledge.".to_string(),
                ),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "question": {
                            "type": "string",
                            "description": "The question or search query"
                        }
                    },
                    "required": ["question"]
                }),
            });

            JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id,
                result: Some(serde_json::to_value(ListToolsResult { tools }).unwrap_or_default()),
                error: None,
            }
        }
        "tools/call" => {
            let params = req.params.unwrap_or(serde_json::Value::Null);
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            if !is_skill_whitelisted(name) {
                return JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32601,
                        message: format!(
                            "Method not found or access denied (RBAC Whitelist): {}",
                            name
                        ),
                        data: None,
                    }),
                };
            }

            info!("🛠️ [MCP] Tool invocation: {}", name);

            if name == "transcribe" {
                // Handled via Native TranscriptionEngine
                let audio_path_str = arguments
                    .get("audio_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if audio_path_str.is_empty() {
                    return JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Missing audio_path".into(),
                            data: None,
                        }),
                    };
                }

                match state
                    .transcription_engine
                    .transcribe(Path::new(audio_path_str))
                    .await
                {
                    Ok(result) => {
                        let mut lip_sync_frames = Vec::new();
                        for segment in &result.segments {
                            let frame = LipSyncFrame::from_segment(segment);
                            lip_sync_frames.push(frame);
                        }
                        JsonRpcResponse {
                            jsonrpc: "2.0".into(),
                            id,
                            result: Some(serde_json::json!({
                                "content": [{"type": "text", "text": result.text}],
                                "segments": result.segments,
                                "lipSync": lip_sync_frames
                            })),
                            error: None,
                        }
                    }
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32603,
                            message: format!("Transcription failed: {:?}", e),
                            data: None,
                        }),
                    },
                }
            } else if name == "cortex_search" {
                let question = arguments
                    .get("question")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if question.is_empty() {
                    return JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Missing question".into(),
                            data: None,
                        }),
                    };
                }
                match state.cortex_query.get_inner().query(question).await {
                    Ok(ans) => JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        id,
                        result: Some(
                            serde_json::to_value(CallToolResult {
                                content: vec![McpContent::Text {
                                    text: ans.answer_md,
                                }],
                                is_error: false,
                            })
                            .unwrap_or_default(),
                        ),
                        error: None,
                    },
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32603,
                            message: format!("Cortex search failed: {:?}", e),
                            data: None,
                        }),
                    },
                }
            } else {
                use crate::tool_call_router::{
                    DefaultToolCallRouter, ToolCallRouter, ToolExecutionEvent,
                };
                let router = DefaultToolCallRouter;
                let input_str = arguments.to_string();

                if let Err(security_error) = router.evaluate_security(&input_str, state).await {
                    warn!(
                        "MCP Security Evaluation blocked tool `{}`: {}",
                        name, security_error
                    );
                    return JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32600,
                            message: security_error, // Remove redundant [Security Block] prefix
                            data: None,
                        }),
                    };
                }

                // 2. Execute via Router (HookChain is enforced inside)
                let mut rx = router.execute_skill(name, &input_str, state).await;
                let mut final_output = String::new();
                let mut is_error = false;

                while let Some(evt) = rx.recv().await {
                    match evt {
                        ToolExecutionEvent::Result(res) => {
                            final_output.push_str(&res);
                        }
                        ToolExecutionEvent::Error(err) => {
                            final_output.push_str(&err);
                            is_error = true;
                        }
                        _ => {} // Ignore Start and Heartbeat for MCP
                    }
                }

                if final_output.starts_with("Error:")
                    || final_output.starts_with("[Hook")
                    || final_output.contains(" Error:")
                {
                    is_error = true;
                }

                let result_text = crate::system_instructions::safe_truncate(&final_output, 50000);

                JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id,
                    result: Some(
                        serde_json::to_value(CallToolResult {
                            content: vec![McpContent::Text { text: result_text }],
                            is_error,
                        })
                        .unwrap_or_default(),
                    ),
                    error: None,
                }
            }
        }
        _ => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", req.method),
                data: None,
            }),
        },
    }
}

fn is_skill_whitelisted(name: &str) -> bool {
    match name {
        // Existing core tools
        "fs_reader" | "MarketDataFetcher" | "StringRepeater" | "transcribe" | "cortex_search" => {
            true
        }
        // Phase 1: External MCP tools
        "firecrawl_scrape" | "firecrawl_crawl" | "firecrawl_map"
        | "exa_search" | "exa_contents"
        | "browser_navigate" | "browser_screenshot" | "browser_click"
        | "resolve_library_id" | "get_library_docs" // Context7
        | "freee_api_get" | "freee_api_post" | "freee_authenticate" => true,

        "terminal_exec" | "fs_writer" | "forge_publish" => false, // Protected internal tools
        _ => false,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::app_state::Component;
    use infrastructure::skills::hooks::HookChain;
    use std::sync::Arc;

    /// Public test helper: exposes is_skill_whitelisted for cross-module testing.
    pub fn check_whitelist(name: &str) -> bool {
        is_skill_whitelisted(name)
    }

    async fn setup_mock_state() -> AppState {
        let (_, state, _) = crate::api_integration_tests::create_test_server().await;
        state
    }

    #[tokio::test]
    async fn test_mcp_evaluate_security_and_hookchain() {
        let mut state = setup_mock_state().await;
        let chain = HookChain::new();
        state.hook_chain = Component::new(Arc::new(chain));

        // Let's create an MCP tool call request with an intent that should trigger Sentinel Block
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: None,
            method: "tools/call".into(),
            params: Some(serde_json::json!({
                "name": "fs_reader",
                "arguments": "rm -rf /"  // Known bad pattern
            })),
        };

        let response = handle_mcp_request(req, &state).await;
        assert!(
            response.error.is_some(),
            "Expected an error due to security block"
        );
        let err = response.error.unwrap(); // allow-anti-pattern
        assert!(
            err.message.contains("GUARDRAIL")
                || err.message.contains("SENTINEL")
                || err.message.contains("Hook Block"),
            "Expected Guardrail, Sentinel, or Hook block, got: {}",
            err.message
        );
    }

    /// [Verification Protocol — RBAC] New tools must be allowed, protected tools must be blocked.
    #[tokio::test]
    async fn test_rbac_new_tools_whitelisted() {
        // Positive: new ecosystem tools
        assert!(check_whitelist("firecrawl_scrape"));
        assert!(check_whitelist("firecrawl_crawl"));
        assert!(check_whitelist("firecrawl_map"));
        assert!(check_whitelist("exa_search"));
        assert!(check_whitelist("exa_contents"));
        assert!(check_whitelist("browser_navigate"));
        assert!(check_whitelist("browser_screenshot"));
        assert!(check_whitelist("browser_click"));
        assert!(check_whitelist("resolve_library_id"));
        assert!(check_whitelist("get_library_docs"));
        assert!(check_whitelist("freee_api_get"));
        assert!(check_whitelist("freee_api_post"));
        assert!(check_whitelist("freee_authenticate"));

        // Negative: protected internal tools MUST remain blocked
        assert!(!check_whitelist("terminal_exec"));
        assert!(!check_whitelist("fs_writer"));
        assert!(!check_whitelist("forge_publish"));

        // Negative: arbitrary unknown tools
        assert!(!check_whitelist("evil_tool"));
        assert!(!check_whitelist(""));
        assert!(!check_whitelist("firecrawl_scrape_backdoor"));
    }
}
