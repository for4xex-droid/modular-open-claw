/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 *
 * Usage of this software is subject to the BSL 1.1 terms.
 * Commercial use requires a separate license agreement.
 */

use crate::auth::McpAuth;
use crate::mcp_tools::{handle_buy, handle_marketplace_search, handle_upload, UploadRequest};
use crate::state::SharedState;
use axum::{
    extract::Query,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use commerce_protocol::error::NurtureError;
use commerce_protocol::mcp_commerce::{BuyRequest, MarketSearchRequest};
use nurture_bridge::commerce::CommerceEngine;

pub fn marketplace_routes() -> Router {
    Router::new()
        .route("/search", get(search_handler))
        .route("/buy", post(buy_handler))
        .route("/upload", post(upload_handler))
        .route("/buy/:tx_id/refund", post(refund_handler))
}

fn extract_actor_id(
    claims: &nurture_bridge::auth::AiomeCustomClaims,
) -> Result<commerce_protocol::identity::ActorId, NurtureError> {
    uuid::Uuid::parse_str(&claims.sub)
        .map(commerce_protocol::identity::ActorId)
        .map_err(|_| NurtureError::Unauthorized("Invalid ActorId".to_string()))
}

async fn search_handler(
    _: McpAuth,
    Extension(state): Extension<SharedState>,
    Query(req): Query<MarketSearchRequest>,
) -> Result<impl IntoResponse, NurtureError> {
    let res = handle_marketplace_search(state, req).await?;
    Ok((StatusCode::OK, Json(res)))
}

async fn buy_handler(
    McpAuth(claims): McpAuth,
    Extension(state): Extension<SharedState>,
    Json(req): Json<BuyRequest>,
) -> Result<impl IntoResponse, NurtureError> {
    // 🔒 冪等性キー必須チェック (二重決済防止)
    // AIエージェントのリトライによる二重課金を物理的に防止するため、
    // Idempotency-Key なしの決済リクエストは一律拒否する。
    if let Some(ref key) = req.idempotency_key {
        if !key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(NurtureError::PolicyViolation(
                "idempotency_key に不正な文字が含まれています".to_string(),
            ));
        }
    } else {
        return Err(NurtureError::PolicyViolation(
            "idempotency_key は決済リクエストに必須です。二重決済防止のため必ず指定してください。"
                .to_string(),
        ));
    }

    // 他人の ActorId を使用した不正購入を防止 (🔴 B1 解決)
    if req.buyer.0.to_string() != claims.sub {
        return Err(NurtureError::Unauthorized(
            "購入者 ID が認証情報と一致しません".to_string(),
        ));
    }

    let res = handle_buy(state, req).await?;
    Ok((StatusCode::CREATED, Json(res)))
}

async fn refund_handler(
    McpAuth(claims): McpAuth,
    axum::extract::Path(tx_id): axum::extract::Path<uuid::Uuid>,
    Extension(state): Extension<SharedState>,
) -> Result<impl IntoResponse, NurtureError> {
    let actor_id = extract_actor_id(&claims)?;

    state
        .commerce_engine
        .instant_refund(&tx_id.to_string(), actor_id.0)
        .await
        .map_err(|e| NurtureError::Refund(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "refunded",
            "receipt_id": ()
        })),
    ))
}

async fn upload_handler(
    McpAuth(claims): McpAuth,
    Extension(state): Extension<SharedState>,
    Json(mut req): Json<UploadRequest>,
) -> Result<impl IntoResponse, NurtureError> {
    let actor_id = extract_actor_id(&claims)?;

    // Prevent impersonation: Ensure the authenticated user is the creator
    req.creator_id = actor_id.0;

    let res = handle_upload(state, req).await?;
    Ok((StatusCode::CREATED, Json(res)))
}
