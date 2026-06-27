/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Version 2.0.
 */
use aiome_core::contracts::HubMessage;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::handlers::verify_bearer;
use crate::models::{CommuneWsQuery, CreateTopicRequest, TopicRecord};
use crate::state::HubState;
use shared::{sql_exec, sql_fetch_all, sql_fetch_optional};

pub async fn list_topics_handler(
    State(state): State<Arc<HubState>>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    let query =
        "SELECT * FROM commune_topics WHERE status = 'Active' ORDER BY updated_at DESC LIMIT 50"
            .to_string();
    let rows: Vec<TopicRecord> = match sql_fetch_all!(&state.pool, TopicRecord, &query) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to fetch topics: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Database error while fetching topics"})),
            ));
        }
    };

    let topics: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "topic_id": row.topic_id,
                "peer_pubkey": row.peer_pubkey,
                "summary": row.summary,
                "turn_count": row.turn_count,
                "created_at": row.created_at,
            })
        })
        .collect();

    Ok((StatusCode::OK, Json(serde_json::json!(topics))))
}

pub async fn create_topic_handler(
    State(state): State<Arc<HubState>>,
    _headers: HeaderMap,
    Json(mut req): Json<CreateTopicRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // 🛡️ [GlassWorm Shield] Sanitize text fields
    req.summary = req
        .summary
        .map(|s| shared::guardrails::strip_invisible_unicode(&s).into_owned());

    // 2. Proof of Karma (PoK) Verification
    // Requirement: Technical Karma weight sum >= 500
    let karma_query = format!(
        "SELECT COALESCE(SUM(weight), 0) FROM approved_karma WHERE node_id = {} AND karma_type = 'Technical'",
        state.pool.ph(0)
    );
    let karma_sum = match sql_fetch_optional!(&state.pool, (i64,), &karma_query, &req.peer_pubkey) {
        Ok(Some((k,))) => k,
        Ok(None) => 0,
        Err(e) => {
            tracing::error!("Failed to fetch karma: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Database error during Karma verification"})),
            );
        }
    };

    info!(
        "🛡️ [Hub] PoK Check for {}: Technical Karma = {}",
        req.peer_pubkey, karma_sum
    );

    if karma_sum < 500 {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Insufficient Technical Karma to create a topic",
                "required": 500,
                "actual": karma_sum
            })),
        );
    }

    // 3. Insert Topic
    let insert_query = format!(
        "INSERT INTO commune_topics (topic_id, peer_pubkey, summary) VALUES ({}, {}, {})",
        state.pool.ph(0),
        state.pool.ph(1),
        state.pool.ph(2)
    );
    let res = sql_exec!(
        &state.pool,
        &insert_query,
        &req.topic_id,
        &req.peer_pubkey,
        &req.summary
    );

    match res {
        Ok(_) => {
            info!(
                "🌟 [Hub] New Commune Topic created: {} by {}",
                req.topic_id, req.peer_pubkey
            );
            (
                StatusCode::CREATED,
                Json(serde_json::json!({"status": "created", "topic_id": req.topic_id})),
            )
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                serde_json::json!({"error": "Failed to create topic due to internal server error"}),
            ),
        ),
    }
}

pub async fn commune_relay_handler(
    State(state): State<Arc<HubState>>,
    _headers: HeaderMap,
    Json(mut msg): Json<aiome_core::commune::CommuneMessage>,
) -> (StatusCode, Json<serde_json::Value>) {
    // 🛡️ [GlassWorm Shield] Sanitize text fields (Only if plaintext)
    if msg.encryption == "none" {
        msg.content = shared::guardrails::strip_invisible_unicode(&msg.content).into_owned();
    }

    // 1.5 Topic Existence / Status Check
    let topic_check_query = format!(
        "SELECT COUNT(*) FROM commune_topics WHERE topic_id = {} AND status = 'Active'",
        state.pool.ph(0)
    );
    let topic_exists =
        match sql_fetch_optional!(&state.pool, (i64,), &topic_check_query, &msg.topic_id) {
            Ok(Some((count,))) => count > 0,
            Ok(None) => false,
            Err(e) => {
                tracing::error!("Failed to check topic existence: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Database error checking topic existence"})),
                );
            }
        };

    if !topic_exists {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Topic not found or inactive"})),
        );
    }

    // 2. Verification (Signature) - E2E Signature payload (ADR-043)
    let payload = format!(
        "{}:{}:{}:{}",
        msg.sender_pubkey, msg.topic_id, msg.content, msg.lamport_clock
    );
    let valid = crate::auth::verify_ed25519_signature(&msg.sender_pubkey, &msg.signature, &payload);

    if !valid {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"status": "error", "message": "Invalid Signature"})),
        );
    }

    // 3. CSAM Binary Filter (Plan D: Protocol-Level Enforcement) - skip if encrypted
    if msg.encryption == "none"
        && (msg.content.contains("data:image/")
            || msg.content.contains("data:video/")
            || msg.content.contains(";base64,"))
    {
        warn!(
            "🚨 [CSAM Filter] Blocked Commune relay containing binary/base64 data from {}",
            msg.sender_pubkey
        );
        return (
            StatusCode::FORBIDDEN,
            Json(
                serde_json::json!({"status": "blocked", "message": "Binary data and inline assets are strictly prohibited by protocol"}),
            ),
        );
    }

    // 4. Relay Logic
    info!(
        "📫 [Hub] Relaying Commune Message from {} to topic {}",
        msg.sender_pubkey, msg.topic_id
    );

    // Buffer in DB
    let payload_json = match serde_json::to_string(&msg) {
        Ok(j) => j,
        Err(e) => {
            error!(
                "🛡️ [Relay] Failed to serialize commune message for {}: {}",
                msg.recipient_pubkey, e
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Message serialization failed"})),
            );
        }
    };
    let relay_insert_query = format!(
        "INSERT INTO commune_relay_queue (recipient_pubkey, payload) VALUES ({}, {})",
        state.pool.ph(0),
        state.pool.ph(1)
    );
    if let Err(e) = sql_exec!(
        &state.pool,
        &relay_insert_query,
        &msg.recipient_pubkey,
        &payload_json
    ) {
        error!(
            "🛡️ [Relay] Failed to queue commune message for {}: {}",
            msg.recipient_pubkey, e
        );
    }

    // Update Turn Count in Topic (State Channel)
    let turn_update_query = format!(
        "UPDATE commune_topics SET turn_count = turn_count + 1, updated_at = {} WHERE topic_id = {}",
        state.pool.now_fn(),
        state.pool.ph(0)
    );
    if let Err(e) = sql_exec!(&state.pool, &turn_update_query, &msg.topic_id) {
        warn!(
            "🛡️ [Relay] Failed to increment turn_count for {}: {}",
            msg.topic_id, e
        );
    }

    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"status": "accepted"})),
    )
}

pub async fn commune_ws_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Query(query): Query<CommuneWsQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    // Auth Check
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
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
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    ws.on_upgrade(move |socket| handle_commune_ws(socket, state, query.node_id))
}

pub async fn handle_commune_ws(mut socket: WebSocket, state: Arc<HubState>, node_id: String) {
    let mut rx = state.tx.subscribe();

    info!(
        "📪 [CommuneWS] Node {} connected for real-time relay.",
        node_id
    );

    loop {
        tokio::select! {
            Ok(msg) = rx.recv() => {
                if let HubMessage::CommuneRelay(commune_msg) = msg {
                    // SEC: Only send if it's for this recipient
                    if commune_msg.recipient_pubkey != node_id {
                        continue;
                    }
                    let text = match serde_json::to_string(&HubMessage::CommuneRelay(commune_msg)) {
                        Ok(t) => t,
                        Err(e) => {
                            tracing::error!("Failed to serialize CommuneRelay message: {}", e);
                            continue;
                        }
                    };
                    if socket.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                if socket.send(Message::Ping(vec![])).await.is_err() {
                    break;
                }
            }
        }
    }
}

pub async fn commune_relay_metadata_free_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Json(envelope): Json<aiome_core::commune::ZeroMetadataCommuneEnvelope>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Auth Check
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
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
        warn!("🔒 Unauthorized metadata-free relay request");
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized"})),
        );
    }

    let channel_local_id = envelope.channel_local_id.clone();
    let mut channel_closed = false;
    let mut channel_exists = false;

    {
        let channels = state.metadata_free_channels.read().await;
        if let Some(tx) = channels.get(&channel_local_id) {
            channel_exists = true;
            if tx.is_closed() {
                channel_closed = true;
            } else if tx.send(envelope).is_ok() {
                info!(
                    "🔒 Relayed metadata-free message to channel: {}",
                    channel_local_id
                );
                return (
                    StatusCode::ACCEPTED,
                    Json(serde_json::json!({"status": "accepted"})),
                );
            } else {
                channel_closed = true;
            }
        }
    }

    if channel_closed {
        let mut channels = state.metadata_free_channels.write().await;
        channels.remove(&channel_local_id);
        warn!(
            "🔒 Metadata-free channel was closed. Purged: {}",
            channel_local_id
        );
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Channel was closed and purged"})),
        );
    }

    if !channel_exists {
        warn!("🔒 Metadata-free channel not found: {}", channel_local_id);
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Channel not found"})),
        );
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": "Failed to relay message"})),
    )
}
