/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use automerge::AutoCommit;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use tracing::{error, warn};

use crate::models::TimelineSyncRequest;
use crate::state::HubState;

pub async fn timeline_sync_handler(
    State(state): State<Arc<HubState>>,
    _headers: HeaderMap,
    Json(payload): Json<TimelineSyncRequest>,
) -> impl IntoResponse {
    // Load or Init Hub Master Doc
    let timeline_fetch_query = format!(
        "SELECT automerge_blob FROM hub_timeline WHERE id = {}",
        state.pool.ph(0)
    );
    let blob_opt = match &state.pool {
        shared::db::DatabasePool::Sqlite(p) => {
            match sqlx::query_scalar::<_, Vec<u8>>(&timeline_fetch_query)
                .bind(&payload.hub_id)
                .fetch_optional(p)
                .await
            {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("Failed to fetch timeline blob: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": "Database error fetching timeline"})),
                    )
                        .into_response();
                }
            }
        }
        shared::db::DatabasePool::Postgres(p) => {
            match sqlx::query_scalar::<_, Vec<u8>>(&timeline_fetch_query)
                .bind(&payload.hub_id)
                .fetch_optional(p)
                .await
            {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("Failed to fetch timeline blob: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": "Database error fetching timeline"})),
                    )
                        .into_response();
                }
            }
        }
    };

    let mut hub_doc = match blob_opt {
        Some(blob) => match AutoCommit::load(&blob) {
            Ok(doc) => doc,
            Err(e) => {
                tracing::error!("Failed to load AutoCommit CRDT blob: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Failed to decode CRDT timeline blob"})),
                )
                    .into_response();
            }
        },
        None => AutoCommit::new(),
    };

    // CSAM Binary Filter: Decline oversized CRDT syncs which implies binary embedding
    if payload.automerge_blob.len() > 1024 * 1024 {
        // 1MB Hard Limit
        warn!(
            "🚨 [CSAM Filter] Blocked oversized CRDT timeline sync ({} bytes) from hub {}",
            payload.automerge_blob.len(),
            payload.hub_id
        );
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            "CRDT Timeline Sync exceeds maximum allowed size (binary embedding suspected)",
        )
            .into_response();
    }

    // Load and Merge Node's Doc
    if let Ok(mut node_doc) = AutoCommit::load(&payload.automerge_blob) {
        let _ = hub_doc.merge(&mut node_doc);
    }

    let finalized_blob = hub_doc.save();

    // Persist Hub Master Doc
    let timeline_persist_query = format!(
        "INSERT INTO hub_timeline (id, automerge_blob) VALUES ({}, {})
         ON CONFLICT(id) DO UPDATE SET automerge_blob = EXCLUDED.automerge_blob, updated_at = {}",
        state.pool.ph(0),
        state.pool.ph(1),
        state.pool.now_fn()
    );
    let res = match &state.pool {
        shared::db::DatabasePool::Sqlite(p) => sqlx::query(&timeline_persist_query)
            .bind(&payload.hub_id)
            .bind(&finalized_blob)
            .execute(p)
            .await
            .map(|_| ()),
        shared::db::DatabasePool::Postgres(p) => sqlx::query(&timeline_persist_query)
            .bind(&payload.hub_id)
            .bind(&finalized_blob)
            .execute(p)
            .await
            .map(|_| ()),
    };
    if let Err(e) = res {
        error!("🛡️ [Timeline] Failed to persist hub timeline: {}", e);
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "synchronized",
            "automerge_blob": finalized_blob
        })),
    )
        .into_response()
}
