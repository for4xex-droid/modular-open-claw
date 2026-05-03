/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use crate::handlers::verify_bearer;
use crate::state::HubState;
use aiome_core::contracts::HubMessage;
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
use tracing::{debug, error, info, warn};

pub async fn ws_handler(
    headers: HeaderMap,
    ws: WebSocketUpgrade,
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

    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

pub async fn handle_socket(mut socket: WebSocket, state: Arc<HubState>) {
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
        }
    }
    state
        .active_connections
        .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
}
