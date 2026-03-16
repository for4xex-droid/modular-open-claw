/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use crate::error::AppError;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct CommerceBalanceResponse {
    pub agent_id: Uuid,
    pub balance: u64,
}

#[derive(Debug, Deserialize)]
pub struct PurchaseRequest {
    pub item_id: Uuid,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct PurchaseResponse {
    pub transaction_id: String,
    pub status: String,
}

/// [GET] /api/v1/commerce/balance/:agent_id
pub async fn get_balance(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    axum::extract::Path(agent_id): axum::extract::Path<uuid::Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // SEC-2: Authentication is enforced by the Authenticated extractor.
    // TODO: When RBAC is implemented, add agent_id ownership check here.
    tracing::info!("💰 [Commerce] Balance query for agent: {}", agent_id);

    let engine = state.commerce_engine.as_ref().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Commerce Engine not enabled".into(),
        }
    })?;

    let balance = engine.get_balance(agent_id).await?;
    Ok(Json(serde_json::json!({ "balance": balance })))
}

/// [POST] /api/v1/commerce/purchase/:agent_id
pub async fn execute_purchase(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    axum::extract::Path(agent_id): axum::extract::Path<uuid::Uuid>,
    Json(req): Json<PurchaseRequest>,
) -> Result<impl IntoResponse, AppError> {
    // SEC-2: Authentication is enforced by the Authenticated extractor.
    // TODO: When RBAC is implemented, add agent_id ownership check here.
    tracing::info!("🛒 [Commerce] Purchase request for agent: {}, item: {}", agent_id, req.item_id);

    let engine = state.commerce_engine.as_ref().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Commerce Engine not enabled".into(),
        }
    })?;

    let tx_id = engine.execute_autonomous_purchase(
        agent_id,
        req.item_id,
        req.metadata,
    ).await?;
    
    Ok((StatusCode::CREATED, Json(PurchaseResponse {
        transaction_id: tx_id,
        status: "Completed".into(),
    })))
}

