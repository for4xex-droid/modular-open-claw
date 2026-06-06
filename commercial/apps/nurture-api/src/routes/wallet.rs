/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use crate::auth::McpAuth;
use crate::mcp_tools::{handle_get_balance, handle_get_history, handle_get_points};
use crate::state::SharedState;
use axum::{
    extract::Query,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use nurture_bridge::commerce::CommerceEngine;
use serde::Deserialize;
use uuid::Uuid;

/// AiomeCustomClaims から ActorId を安全に抽出する共通ヘルパー
fn extract_actor_id(
    claims: &nurture_bridge::auth::AiomeCustomClaims,
) -> Result<ActorId, NurtureError> {
    Uuid::parse_str(&claims.sub)
        .map(ActorId)
        .map_err(|_| NurtureError::Unauthorized("Invalid ActorId".to_string()))
}

pub fn wallet_routes() -> Router {
    Router::new()
        .route("/balance", get(balance_handler))
        .route("/points", get(points_handler))
        .route("/points/withdraw", post(withdraw_points_handler))
        .route("/history", get(history_handler))
        .route("/transfer", post(transfer_handler))
}

async fn balance_handler(
    McpAuth(claims): McpAuth,
    Extension(state): Extension<SharedState>,
) -> Result<impl IntoResponse, NurtureError> {
    let actor_id = extract_actor_id(&claims)?;

    let res = handle_get_balance(state, actor_id).await?;
    Ok((StatusCode::OK, Json(res)))
}

async fn points_handler(
    McpAuth(claims): McpAuth,
    Extension(state): Extension<SharedState>,
) -> Result<impl IntoResponse, NurtureError> {
    let actor_id = extract_actor_id(&claims)?;

    let res = handle_get_points(state, actor_id).await?;
    Ok((StatusCode::OK, Json(res)))
}

#[derive(Deserialize)]
struct HistoryQuery {
    limit: Option<u32>,
}

async fn history_handler(
    McpAuth(claims): McpAuth,
    Extension(state): Extension<SharedState>,
    Query(query): Query<HistoryQuery>,
) -> Result<impl IntoResponse, NurtureError> {
    let actor_id = extract_actor_id(&claims)?;

    let limit = query.limit.unwrap_or(20).min(100); // DOS 防止: 上限 100

    let res = handle_get_history(state, actor_id, limit).await?;
    Ok((StatusCode::OK, Json(res)))
}

#[derive(Deserialize)]
struct WithdrawRequest {
    amount: u64,
}

async fn withdraw_points_handler(
    McpAuth(claims): McpAuth,
    Extension(state): Extension<SharedState>,
    Json(req): Json<WithdrawRequest>,
) -> Result<impl IntoResponse, NurtureError> {
    let actor_id = extract_actor_id(&claims)?;

    state
        .commerce_engine
        .withdraw_points(actor_id.0, req.amount)
        .await
        .map_err(|e| NurtureError::Infrastructure(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "success",
            "withdrawn": req.amount
        })),
    ))
}

#[derive(Deserialize)]
struct TransferRequest {
    to_actor_id: Uuid,
    amount: u64,
}

async fn transfer_handler(
    McpAuth(claims): McpAuth,
    Extension(state): Extension<SharedState>,
    Json(req): Json<TransferRequest>,
) -> Result<impl IntoResponse, NurtureError> {
    let actor_id = extract_actor_id(&claims)?;

    // 🔴 AML/KYC 必須要件: 送金機能は本人確認済みユーザーのみ利用可能
    match state.ekyc_store.is_verified(&actor_id).await {
        Ok(true) => {}
        Ok(false) => {
            return Err(NurtureError::PolicyViolation(
                "Transfer requires eKYC verification (AML requirement)".to_string(),
            ));
        }
        Err(e) => {
            return Err(NurtureError::Infrastructure(format!(
                "Failed to verify eKYC status: {}",
                e
            )));
        }
    }

    let tx_id = state
        .commerce_engine
        .transfer(actor_id.0, req.to_actor_id, req.amount)
        .await
        .map_err(|e| NurtureError::Infrastructure(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "success",
            "transaction_id": tx_id
        })),
    ))
}
