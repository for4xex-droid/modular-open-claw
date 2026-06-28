/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::handlers::verify_bearer;
use crate::state::HubState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tracing::{info, warn};

use axum::extract::Query;

#[derive(serde::Deserialize)]
pub struct WsQueryParams {
    pub channel_local_id: Option<String>,
}

pub async fn ws_handler(
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    Query(params): Query<WsQueryParams>,
    State(state): State<Arc<HubState>>,
) -> impl IntoResponse {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h: &axum::http::HeaderValue| h.to_str().ok())
        .unwrap_or("");

    let mut authenticated = false;
    if auth_header.starts_with("Bearer ") {
        let token = auth_header.trim_start_matches("Bearer ");
        if let Ok(claims) = state.auth_manager.validate_token(token).await {
            if claims.agent_id != uuid::Uuid::nil() {
                authenticated = true;
            }
        }
    }

    if !authenticated && verify_bearer(auth_header, &state.secret) {
        authenticated = true;
    }

    if !authenticated {
        warn!("🔒 Unauthorized WS upgrade attempt ");
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    ws.on_upgrade(move |socket| handle_socket(socket, state, params.channel_local_id))
}

pub async fn handle_socket(
    mut socket: WebSocket,
    state: Arc<HubState>,
    channel_local_id: Option<String>,
) {
    use aiome_core::contracts::HubMessage;

    // TCP Exhaustion Defense (Max Connections)
    let current_conn = state
        .active_connections
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    if current_conn >= 1000 {
        warn!("🛡️ [BFT] Hub reached max WebSocket connections (1000). Rejecting new node.");
        state
            .active_connections
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        let _ = socket.send(Message::Close(None)).await;
        return;
    }

    info!(
        "🔌 Authorized node connected via WebSocket (Total: {})",
        current_conn + 1
    );

    let mut rx = state.tx.subscribe();
    let mut keepalive_timer = tokio::time::interval(Duration::from_secs(30));

    let mut metadata_free_rx = None;
    if let Some(ref cid) = channel_local_id {
        let (tx_mpsc, rx_mpsc) = tokio::sync::mpsc::unbounded_channel();
        state
            .metadata_free_channels
            .write()
            .await
            .insert(cid.clone(), tx_mpsc);
        metadata_free_rx = Some(rx_mpsc);
        info!("🔒 Registered metadata-free channel: {}", cid);
    }

    loop {
        tokio::select! {
            _ = keepalive_timer.tick() => {
                // Ping-Pong keepalive (Flaw 9)
                if socket.send(Message::Ping(Vec::new())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        info!("🔌 Node disconnected ");
                        break;
                    }
                    Some(Ok(Message::Text(text))) => {
                        // Handle Ping from client (Flaw 9)
                        if let Ok(HubMessage::Ping { client_time: _ }) = serde_json::from_str::<HubMessage>(&text) {
                            let pong = HubMessage::Pong { server_time: chrono::Utc::now().to_rfc3339() };
                            if let Ok(pong_text) = serde_json::to_string(&pong) {
                                let _ = socket.send(Message::Text(pong_text)).await;
                            }
                        }
                    }
                    _ => {}
                }
            }
            res = rx.recv() => {
                match res {
                    Ok(hub_msg) => {
                        if let Ok(text) = serde_json::to_string(&hub_msg) {
                            if socket.send(Message::Text(text)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        warn!("⚠️ WS Client lagged by {} messages. Triggering Catch-up Sync.", n);
                        let hub_msg = HubMessage::LaggedForceSync {
                            server_time: chrono::Utc::now().to_rfc3339()
                        };
                        if let Ok(text) = serde_json::to_string(&hub_msg) {
                            let _ = socket.send(Message::Text(text)).await;
                        }
                        // Continue loop, client will sync via REST
                    }
                    Err(RecvError::Closed) => break,
                }
            }
            envelope_opt = async {
                if let Some(ref mut rx) = metadata_free_rx {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                match envelope_opt {
                    Some(envelope) => {
                        let hub_msg = HubMessage::ZeroMetadataCommuneRelay(envelope);
                        if let Ok(text) = serde_json::to_string(&hub_msg) {
                            if socket.send(Message::Text(text)).await.is_err() {
                                break;
                            }
                        }
                    }
                    None => break,
                }
            }
        }
    }
    if let Some(ref cid) = channel_local_id {
        state.metadata_free_channels.write().await.remove(cid);
        info!("🔒 Unregistered metadata-free channel: {}", cid);
    }
    state
        .active_connections
        .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
}
