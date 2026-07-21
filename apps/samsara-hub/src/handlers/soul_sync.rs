/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! Soul Sync relay + pairing (OP-020-F5 S-1 / S-2).
//!
//! Hub broadcasts opaque [`EncryptedEnvelope`] only — never decrypts, never persists Soul payload.
//! Relay is gated on `paired_devices` (S-2).

use aiome_core::contracts::HubMessage;
use aiome_core::soul_sync::{EncryptedEnvelope, SoulSyncPairRequest};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::handlers::verify_bearer;
use crate::state::HubState;
use shared::{sql_exec, sql_fetch_optional};

async fn authenticate(state: &HubState, headers: &HeaderMap) -> bool {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if auth_header.starts_with("Bearer ") {
        let token = auth_header.trim_start_matches("Bearer ");
        if let Ok(claims) = state.auth_manager.validate_token(token).await {
            if claims.agent_id != uuid::Uuid::nil() {
                return true;
            }
        }
    }

    verify_bearer(auth_header, &state.secret)
}

/// POST `/api/v1/soul-sync/pair` — register a mutual device pair for a session.
pub async fn soul_sync_pair_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Json(req): Json<SoulSyncPairRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    if !authenticate(&state, &headers).await {
        warn!("🔒 Unauthorized Soul Sync pair request");
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized"})),
        );
    }

    if req.session_id.is_empty() || req.device_a_pubkey.is_empty() || req.device_b_pubkey.is_empty()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "session_id, device_a_pubkey, and device_b_pubkey are required"
            })),
        );
    }

    if req.device_a_pubkey == req.device_b_pubkey {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "device pubkeys must differ"})),
        );
    }

    let created_at = chrono::Utc::now().to_rfc3339();
    let insert = format!(
        "INSERT INTO paired_devices (session_id, device_a_pubkey, device_b_pubkey, created_at) VALUES ({}, {}, {}, {})",
        state.pool.ph(0),
        state.pool.ph(1),
        state.pool.ph(2),
        state.pool.ph(3)
    );

    match sql_exec!(
        &state.pool,
        &insert,
        &req.session_id,
        &req.device_a_pubkey,
        &req.device_b_pubkey,
        &created_at
    ) {
        Ok(_) => {
            info!(
                session_id = %req.session_id,
                "🔗 Soul Sync pair registered"
            );
            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "status": "paired",
                    "session_id": req.session_id
                })),
            )
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("UNIQUE") || msg.contains("unique") || msg.contains("duplicate") {
                return (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({"error": "session_id already paired"})),
                );
            }
            warn!("Soul Sync pair insert failed: {}", msg);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to register pair"})),
            )
        }
    }
}

/// DELETE `/api/v1/soul-sync/pair/:session_id` — revoke pairing (sync must fail after).
pub async fn soul_sync_unpair_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    if !authenticate(&state, &headers).await {
        warn!("🔒 Unauthorized Soul Sync unpair request");
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized"})),
        );
    }

    if session_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "session_id is required"})),
        );
    }

    let delete = format!(
        "DELETE FROM paired_devices WHERE session_id = {}",
        state.pool.ph(0)
    );
    match sql_exec!(&state.pool, &delete, &session_id) {
        Ok(n) if n > 0 => {
            info!(session_id = %session_id, "🔗 Soul Sync pair revoked");
            (
                StatusCode::OK,
                Json(serde_json::json!({"status": "unpaired", "session_id": session_id})),
            )
        }
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "pair not found"})),
        ),
        Err(e) => {
            warn!("Soul Sync unpair failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to unpair"})),
            )
        }
    }
}

async fn is_session_paired(state: &HubState, session_id: &str) -> Result<bool, String> {
    let q = format!(
        "SELECT session_id FROM paired_devices WHERE session_id = {} LIMIT 1",
        state.pool.ph(0)
    );
    match sql_fetch_optional!(&state.pool, (String,), &q, &session_id) {
        Ok(Some(_)) => Ok(true),
        Ok(None) => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

/// POST `/api/v1/soul-sync/relay` — broadcast Soul Sync ciphertext to federation WS peers.
///
/// Acceptance (value_10x F-5 #2): payload stays ciphertext; hub must not write Soul plaintext
/// to DB or log ciphertext contents. S-2: unpaired sessions are rejected.
pub async fn soul_sync_relay_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Json(envelope): Json<EncryptedEnvelope>,
) -> (StatusCode, Json<serde_json::Value>) {
    if !authenticate(&state, &headers).await {
        warn!("🔒 Unauthorized Soul Sync relay request");
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized"})),
        );
    }

    if envelope.session_id.is_empty() || envelope.ciphertext.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "session_id and ciphertext are required"})),
        );
    }

    match is_session_paired(&state, &envelope.session_id).await {
        Ok(true) => {}
        Ok(false) => {
            warn!(
                session_id = %envelope.session_id,
                "🔒 Soul Sync relay rejected: session not paired"
            );
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "session not paired"})),
            );
        }
        Err(e) => {
            warn!("Soul Sync pair lookup failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "pair lookup failed"})),
            );
        }
    }

    // Log metadata only — never ciphertext (plaintext leakage Negative).
    let cipher_len = envelope.ciphertext.len();
    let session_id = envelope.session_id.clone();

    match state.tx.send(HubMessage::SoulSyncRelay(envelope)) {
        Ok(n) => {
            info!(
                session_id = %session_id,
                ciphertext_len = cipher_len,
                receivers = n,
                "🔒 Soul Sync envelope broadcast (opaque)"
            );
            (
                StatusCode::ACCEPTED,
                Json(serde_json::json!({"status": "accepted", "receivers": n})),
            )
        }
        Err(_) => {
            info!(
                session_id = %session_id,
                ciphertext_len = cipher_len,
                "🔒 Soul Sync envelope accepted with zero WS receivers"
            );
            (
                StatusCode::ACCEPTED,
                Json(serde_json::json!({"status": "accepted", "receivers": 0})),
            )
        }
    }
}
