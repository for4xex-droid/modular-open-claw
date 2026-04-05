/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core_contracts::gig::{GigBid, GigDeliverable, GigIntent, VerificationResult, AcceptanceCriteria};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

/// [POST] /api/v1/gig/publish
#[utoipa::path(
    post,
    path = "/api/v1/gig/publish",
    request_body = GigIntent,
    responses(
        (status = 201, description = "Gig Intent published", body = serde_json::Value),
        (status = 403, description = "Unauthorized access")
    ),
    security(("api_key" = []))
)]
pub async fn publish_intent(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Json(mut intent): Json<GigIntent>,
) -> Result<impl IntoResponse, AppError> {
    // 🛡️ [GlassWorm Shield] Sanitize text fields deeply including JSON values
    intent.description = shared::guardrails::strip_invisible_unicode(&intent.description).into_owned();
    
    // Sanitize Criteria
    for cri in &mut intent.criteria {
        match cri {
            AcceptanceCriteria::OracleJudge { rubric_prompt, .. } => {
                *rubric_prompt = shared::guardrails::strip_invisible_unicode(rubric_prompt).into_owned();
            }
            AcceptanceCriteria::FileType { mime, .. } => {
                *mime = shared::guardrails::strip_invisible_unicode(mime).into_owned();
            }
            AcceptanceCriteria::WasmValidator { wasm_module_cid } => {
                *wasm_module_cid = shared::guardrails::strip_invisible_unicode(wasm_module_cid).into_owned();
            }
            AcceptanceCriteria::JsonSchema { schema } => {
                if let Ok(schema_str) = serde_json::to_string(schema) {
                    let clean = shared::guardrails::strip_invisible_unicode(&schema_str).into_owned();
                    if let Ok(json) = serde_json::from_str(&clean) {
                        *schema = json;
                    }
                }
            }
        }
    }

    // Sanitize Metadata Map
    if let Some(meta) = &intent.metadata {
        if let Ok(meta_str) = serde_json::to_string(meta) {
            let clean = shared::guardrails::strip_invisible_unicode(&meta_str).into_owned();
            if let Ok(json) = serde_json::from_str(&clean) {
                intent.metadata = Some(json);
            }
        }
    }

    // SEC-2: Authentication check
    // Ensure the requester_id is the authenticated agent
    intent.requester_id = auth.agent_id;

    let engine =
        state
            .gig_engine
            .as_opt()
            .ok_or_else(|| aiome_core::error::AiomeError::Infrastructure {
                reason: "Gig Engine not enabled".into(),
            })?;

    let id = engine.publish_intent(intent).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

/// [POST] /api/v1/gig/bid
#[utoipa::path(
    post,
    path = "/api/v1/gig/bid",
    request_body = GigBid,
    responses(
        (status = 200, description = "Bid submitted"),
        (status = 403, description = "Unauthorized access")
    ),
    security(("api_key" = []))
)]
pub async fn submit_bid(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Json(mut bid): Json<GigBid>,
) -> Result<impl IntoResponse, AppError> {
    // SEC-2: Ensure bidder_id is the authenticated agent
    bid.bidder_id = auth.agent_id;

    let engine =
        state
            .gig_engine
            .as_opt()
            .ok_or_else(|| aiome_core::error::AiomeError::Infrastructure {
                reason: "Gig Engine not enabled".into(),
            })?;

    engine.submit_bid(bid).await?;
    Ok(StatusCode::OK)
}

/// [POST] /api/v1/gig/accept/:intent_id/:bid_id
#[utoipa::path(
    post,
    path = "/api/v1/gig/accept/{intent_id}/{bid_id}",
    responses(
        (status = 200, description = "Bid accepted and escrow locked"),
        (status = 403, description = "Unauthorized access"),
        (status = 404, description = "Intent or Bid not found")
    ),
    params(
        ("intent_id" = String, Path, description = "The unique ID of the intent"),
        ("bid_id" = String, Path, description = "The unique ID of the bid to accept")
    ),
    security(("api_key" = []))
)]
pub async fn accept_bid(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Path((intent_id, bid_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    let engine =
        state
            .gig_engine
            .as_opt()
            .ok_or_else(|| aiome_core::error::AiomeError::Infrastructure {
                reason: "Gig Engine not enabled".into(),
            })?;

    engine.accept_bid(intent_id, bid_id).await?;
    Ok(StatusCode::OK)
}

/// [POST] /api/v1/gig/deliver
#[utoipa::path(
    post,
    path = "/api/v1/gig/deliver",
    request_body = GigDeliverable,
    responses(
        (status = 200, description = "Deliverable record created"),
        (status = 403, description = "Unauthorized access")
    ),
    security(("api_key" = []))
)]
pub async fn deliver(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Json(mut deliverable): Json<GigDeliverable>,
) -> Result<impl IntoResponse, AppError> {
    // 🛡️ [GlassWorm Shield] Sanitize text fields deeply
    deliverable.artifact_path = shared::guardrails::strip_invisible_unicode(&deliverable.artifact_path).into_owned();
    
    // Sanitize JSON Metadata
    if let Ok(meta_str) = serde_json::to_string(&deliverable.metadata) {
        let clean = shared::guardrails::strip_invisible_unicode(&meta_str).into_owned();
        if let Ok(json) = serde_json::from_str(&clean) {
            deliverable.metadata = json;
        }
    }

    // SEC-2: Ensure deliverer_id is the authenticated agent
    deliverable.deliverer_id = auth.agent_id;

    let engine =
        state
            .gig_engine
            .as_opt()
            .ok_or_else(|| aiome_core::error::AiomeError::Infrastructure {
                reason: "Gig Engine not enabled".into(),
            })?;

    engine.deliver(deliverable).await?;
    Ok(StatusCode::OK)
}

/// [POST] /api/v1/gig/verify/:order_id
#[utoipa::path(
    post,
    path = "/api/v1/gig/verify/{order_id}",
    responses(
        (status = 200, description = "Verification performed and settlement executed", body = VerificationResult),
        (status = 403, description = "Unauthorized access"),
        (status = 404, description = "Order not found")
    ),
    params(
        ("order_id" = String, Path, description = "The unique ID of the order to verify")
    ),
    security(("api_key" = []))
)]
pub async fn verify(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Path(order_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let engine =
        state
            .gig_engine
            .as_opt()
            .ok_or_else(|| aiome_core::error::AiomeError::Infrastructure {
                reason: "Gig Engine not enabled".into(),
            })?;

    let result = engine.verify_and_settle(order_id).await?;
    Ok(Json(result))
}
