/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
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
            } else {
                // Handled via Wasm Skill
                let result_text = crate::skill_handler::execute_wasm_skill(
                    name,
                    &arguments.to_string(),
                    state,
                    None,
                    0,
                )
                .await;

                JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id,
                    result: Some(
                        serde_json::to_value(CallToolResult {
                            content: vec![McpContent::Text { text: result_text }],
                            is_error: false,
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
    // Phase 17 Strict Review: RBAC Whitelist
    match name {
        "fs_reader" | "MarketDataFetcher" | "StringRepeater" | "transcribe" => true,
        "terminal_exec" | "fs_writer" | "forge_publish" => false, // Protected internal tools
        _ => false,
    }
}
