/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use super::types::{
    EscrowCreateRequest, EscrowCreateResponse, EscrowRefundRequest, EscrowReleaseRequest,
};
use crate::state::SharedState;
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Extension, Json};
use nurture_bridge::commerce::CommerceEngine;
use tracing::error;
use uuid::Uuid;

pub async fn create_escrow(
    Extension(state): axum::Extension<SharedState>,
    Json(payload): Json<EscrowCreateRequest>,
) -> impl IntoResponse {
    // S-3: ゼロ金額エスクローの拒否
    if payload.amount == 0 {
        error!(
            "❌ [Internal/Escrow] Rejected zero-amount escrow for {}",
            payload.actor_id
        );
        return (
            StatusCode::BAD_REQUEST,
            "Escrow amount must be greater than zero",
        )
            .into_response();
    }

    // F-1/B-2: KYC AML Policy Check before allowing escrow
    let actor = commerce_protocol::identity::ActorId(payload.actor_id);
    match state.ekyc_store.is_verified(&actor).await {
        Ok(true) => { /* AML Passed */ }
        Ok(false) => {
            error!(
                "🚨 [Internal/Escrow] Escrow rejected: User {} has not completed KYC verification (AML Policy)",
                payload.actor_id
            );
            return (
                StatusCode::FORBIDDEN,
                "KYC verification required for escrow operations",
            )
                .into_response();
        }
        Err(e) => {
            error!("❌ [Internal/Escrow] Failed to verify KYC status: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Identity verification service unavailable",
            )
                .into_response();
        }
    }

    match state
        .commerce_engine
        .escrow_create(payload.actor_id, payload.amount)
        .await
    {
        Ok(escrow_id) => (StatusCode::OK, Json(EscrowCreateResponse { escrow_id })).into_response(),
        Err(e) => {
            error!("❌ [Internal/Escrow] Create failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Escrow creation failed").into_response()
        }
    }
}

pub async fn release_escrow(
    Extension(state): axum::Extension<SharedState>,
    Json(payload): Json<EscrowReleaseRequest>,
) -> impl IntoResponse {
    if payload.escrow_id.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "escrow_id must not be empty").into_response();
    }

    match state
        .commerce_engine
        .escrow_release(&payload.escrow_id, payload.recipient_id)
        .await
    {
        Ok(_) => (StatusCode::OK, "Success").into_response(),
        Err(e) => {
            let msg = e.to_string();
            error!("❌ [Internal/Escrow] Release failed: {}", msg);
            if msg.contains("not found") || msg.contains("already") || msg.contains("invalid") {
                (StatusCode::BAD_REQUEST, "Escrow release rejected").into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, "Escrow release failed").into_response()
            }
        }
    }
}

pub async fn list_escrows(
    Path(actor_id): Path<Uuid>,
    Extension(state): axum::Extension<SharedState>,
) -> impl IntoResponse {
    match state.commerce_engine.list_escrows(actor_id).await {
        Ok(escrows) => (StatusCode::OK, Json(escrows)).into_response(),
        Err(e) => {
            error!(
                "❌ [Internal/EscrowList] Failed to list escrows for {}: {:?}",
                actor_id, e
            );
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch escrows").into_response()
        }
    }
}

pub async fn refund_escrow(
    Extension(state): axum::Extension<SharedState>,
    Json(payload): Json<EscrowRefundRequest>,
) -> impl IntoResponse {
    match state
        .commerce_engine
        .escrow_refund(&payload.escrow_id)
        .await
    {
        Ok(_) => {
            tracing::info!(
                "✅ [Internal/EscrowRefund] Successfully refunded escrow {}",
                payload.escrow_id
            );
            (StatusCode::OK, "Escrow successfully refunded").into_response()
        }
        Err(e) => {
            error!(
                "❌ [Internal/EscrowRefund] Failed to refund escrow {}: {:?}",
                payload.escrow_id, e
            );
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to refund escrow").into_response()
        }
    }
}
