/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core::traits::*;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CommerceBalanceResponse {
    #[schema(value_type = String)]
    pub agent_id: Uuid,
    pub balance: u64,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PurchaseRequest {
    #[schema(value_type = String)]
    pub item_id: Uuid,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PurchaseResponse {
    pub transaction_id: String,
    pub status: String,
}

/// [GET] /api/v1/commerce/balance/:agent_id
#[utoipa::path(
    get,
    path = "/api/v1/commerce/balance/{agent_id}",
    responses(
        (status = 200, description = "Balance as simple JSON", body = serde_json::Value),
        (status = 403, description = "Unauthorized access")
    ),
    params(
        ("agent_id" = String, Path, description = "The unique ID of the agent")
    ),
    security(("api_key" = []))
)]
pub async fn get_balance(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    axum::extract::Path(agent_id): axum::extract::Path<uuid::Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // SEC-2: Authentication is enforced by the Authenticated extractor.
    // RBAC: Check agent_id ownership
    if agent_id != auth.agent_id {
        return Err(AppError::forbidden(
            "Unauthorized access to this agent's balance",
        ));
    }

    tracing::info!("💰 [Commerce] Balance query for agent: {}", agent_id);

    let engine = state.commerce_engine.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Commerce Engine not enabled".into(),
        }
    })?;

    let balance = engine.get_balance(agent_id).await?;
    Ok(Json(serde_json::json!({ "balance": balance })))
}

/// [POST] /api/v1/commerce/purchase/:agent_id
#[utoipa::path(
    post,
    path = "/api/v1/commerce/purchase/{agent_id}",
    request_body = PurchaseRequest,
    responses(
        (status = 201, description = "Purchase completed", body = PurchaseResponse),
        (status = 403, description = "Unauthorized access")
    ),
    params(
        ("agent_id" = String, Path, description = "The unique ID of the agent")
    ),
    security(("api_key" = []))
)]
pub async fn execute_purchase(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    axum::extract::Path(agent_id): axum::extract::Path<uuid::Uuid>,
    Json(req): Json<PurchaseRequest>,
) -> Result<impl IntoResponse, AppError> {
    // SEC-2: Authentication is enforced by the Authenticated extractor.
    // RBAC: Check agent_id ownership
    if agent_id != auth.agent_id {
        return Err(AppError::forbidden(
            "Unauthorized purchase request for this agent",
        ));
    }

    if !auth.ekyc_verified {
        tracing::warn!(
            "🛡️ [Commerce] Blocked unverified purchase request from agent: {}",
            agent_id
        );
        return Err(AppError::forbidden(
            "eKYC verification is required to make purchases",
        ));
    }

    tracing::info!(
        "🛒 [Commerce] Purchase request for agent: {}, item: {}",
        agent_id,
        req.item_id
    );

    let engine = state.commerce_engine.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Commerce Engine not enabled".into(),
        }
    })?;

    let tx_id = engine
        .execute_autonomous_purchase(agent_id, req.item_id, req.metadata)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(PurchaseResponse {
            transaction_id: tx_id,
            status: "Completed".into(),
        }),
    ))
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateSubscriptionRequest {
    #[schema(value_type = String)]
    pub agent_id: Uuid,
    pub plan_id: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SubscriptionResponse {
    pub subscription_id: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CancelSubscriptionRequest {
    #[schema(value_type = String)]
    pub agent_id: Uuid,
    pub subscription_id: String,
}

/// [POST] /api/v1/commerce/subscription/create
pub async fn create_subscription(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Json(req): Json<CreateSubscriptionRequest>,
) -> Result<impl IntoResponse, AppError> {
    if req.agent_id != auth.agent_id {
        return Err(AppError::forbidden("Unauthorized access to this agent"));
    }

    if !auth.ekyc_verified {
        tracing::warn!(
            "🛡️ [Commerce] Blocked unverified subscription request from agent: {}",
            req.agent_id
        );
        return Err(AppError::forbidden(
            "eKYC verification is required to create subscriptions",
        ));
    }

    let engine = state.commerce_engine.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Commerce Engine not enabled".into(),
        }
    })?;

    let sub_id = engine
        .create_subscription(req.agent_id, &req.plan_id)
        .await?;
    Ok((
        StatusCode::OK,
        Json(SubscriptionResponse {
            subscription_id: sub_id,
        }),
    ))
}

/// [POST] /api/v1/commerce/subscription/cancel
pub async fn cancel_subscription(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Json(req): Json<CancelSubscriptionRequest>,
) -> Result<impl IntoResponse, AppError> {
    // SEC-2: RBAC ownership check — prevent IDOR attacks (Reflexion C-1 fix)
    if req.agent_id != auth.agent_id {
        return Err(AppError::forbidden(
            "Unauthorized cancellation request for this agent",
        ));
    }

    if !auth.ekyc_verified {
        tracing::warn!(
            "🛡️ [Commerce] Blocked unverified subscription cancellation from agent: {}",
            req.agent_id
        );
        return Err(AppError::forbidden(
            "eKYC verification is required to cancel subscriptions",
        ));
    }

    let engine = state.commerce_engine.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Commerce Engine not enabled".into(),
        }
    })?;

    engine
        .cancel_subscription(req.agent_id, &req.subscription_id)
        .await?;
    Ok(StatusCode::OK)
}

/// [GET] /api/v1/commerce/subscription/:agent_id
pub async fn get_subscription_status(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Path(agent_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    if agent_id != auth.agent_id {
        return Err(AppError::forbidden("Unauthorized access to this agent"));
    }

    let engine = state.commerce_engine.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Commerce Engine not enabled".into(),
        }
    })?;

    let status = engine.get_subscription_status(agent_id).await?;
    Ok(Json(status))
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ReleaseEscrowRequest {
    #[schema(value_type = String)]
    pub recipient_id: Uuid,
}

/// [GET] /api/v1/commerce/escrow/history/:agent_id
#[utoipa::path(
    get,
    path = "/api/v1/commerce/escrow/history/{agent_id}",
    responses(
        (status = 200, description = "List of escrows", body = Vec<aiome_core_contracts::commerce::EscrowRecord>),
        (status = 403, description = "Unauthorized access")
    ),
    params(
        ("agent_id" = String, Path, description = "The unique ID of the agent")
    ),
    security(("api_key" = []))
)]
pub async fn list_escrows(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Path(agent_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    if agent_id != auth.agent_id {
        return Err(AppError::forbidden("Unauthorized access to this agent"));
    }

    let engine = state.commerce_engine.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Commerce Engine not enabled".into(),
        }
    })?;

    let escrows = engine.list_escrows(agent_id).await?;
    Ok(Json(escrows))
}

/// [POST] /api/v1/commerce/escrow/:escrow_id/release
#[utoipa::path(
    post,
    path = "/api/v1/commerce/escrow/{escrow_id}/release",
    request_body = ReleaseEscrowRequest,
    responses(
        (status = 200, description = "Escrow released successfully"),
        (status = 403, description = "Unauthorized access")
    ),
    params(
        ("escrow_id" = String, Path, description = "The ID of the escrow to release")
    ),
    security(("api_key" = []))
)]
pub async fn release_escrow(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Path(escrow_id): Path<String>,
    Json(req): Json<ReleaseEscrowRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Escrow release typically might be done autonomously or via user approval.
    // Ensure eKYC for payment dispersal.
    if !auth.ekyc_verified {
        return Err(AppError::forbidden(
            "eKYC verification required to release escrow",
        ));
    }

    let engine = state.commerce_engine.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Commerce Engine not enabled".into(),
        }
    })?;

    engine.escrow_release(&escrow_id, req.recipient_id).await?;
    Ok(StatusCode::OK)
}
