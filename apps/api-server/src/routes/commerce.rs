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

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct WithdrawRequest {
    #[schema(value_type = String)]
    pub agent_id: Uuid,
    pub amount: u64,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct TransferRequest {
    #[schema(value_type = String)]
    pub from_id: Uuid,
    #[schema(value_type = String)]
    pub to_id: Uuid,
    pub amount: u64,
}

/// [GET] /api/v1/commerce/points/:agent_id
#[utoipa::path(
    get,
    path = "/api/v1/commerce/points/{agent_id}",
    responses(
        (status = 200, description = "Points Balance", body = aiome_core_contracts::commerce::PointsBalance),
        (status = 403, description = "Unauthorized access")
    ),
    params(
        ("agent_id" = String, Path, description = "The unique ID of the agent")
    ),
    security(("api_key" = []))
)]
pub async fn get_points(
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
    let points = engine.get_points(agent_id).await?;
    Ok(Json(points))
}

/// [GET] /api/v1/commerce/history/:agent_id
#[utoipa::path(
    get,
    path = "/api/v1/commerce/history/{agent_id}",
    responses(
        (status = 200, description = "Transaction History", body = Vec<aiome_core_contracts::commerce::TransactionRecord>),
        (status = 403, description = "Unauthorized access")
    ),
    params(
        ("agent_id" = String, Path, description = "The unique ID of the agent")
    ),
    security(("api_key" = []))
)]
pub async fn get_transaction_history(
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
    let history = engine.get_transaction_history(agent_id, 100).await?;
    Ok(Json(history))
}

/// [POST] /api/v1/commerce/withdraw
#[utoipa::path(
    post,
    path = "/api/v1/commerce/withdraw",
    request_body = WithdrawRequest,
    responses(
        (status = 200, description = "Withdrawal initiated successfully"),
        (status = 403, description = "Unauthorized access or missing eKYC")
    ),
    security(("api_key" = []))
)]
pub async fn withdraw_points(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Json(req): Json<WithdrawRequest>,
) -> Result<impl IntoResponse, AppError> {
    if req.agent_id != auth.agent_id {
        return Err(AppError::forbidden("Unauthorized access to this agent"));
    }
    if !auth.ekyc_verified {
        return Err(AppError::forbidden(
            "eKYC verification required to withdraw",
        ));
    }
    let engine = state.commerce_engine.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Commerce Engine not enabled".into(),
        }
    })?;
    engine.withdraw_points(req.agent_id, req.amount).await?;
    Ok(StatusCode::OK)
}

/// [POST] /api/v1/commerce/transfer
#[utoipa::path(
    post,
    path = "/api/v1/commerce/transfer",
    request_body = TransferRequest,
    responses(
        (status = 200, description = "Transfer completed", body = serde_json::Value),
        (status = 403, description = "Unauthorized access or missing eKYC")
    ),
    security(("api_key" = []))
)]
pub async fn transfer(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Json(req): Json<TransferRequest>,
) -> Result<impl IntoResponse, AppError> {
    if req.from_id != auth.agent_id {
        return Err(AppError::forbidden("Unauthorized access to this agent"));
    }
    if !auth.ekyc_verified {
        return Err(AppError::forbidden(
            "eKYC verification required to transfer funds",
        ));
    }
    let engine = state.commerce_engine.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Commerce Engine not enabled".into(),
        }
    })?;
    let tx_id = engine.transfer(req.from_id, req.to_id, req.amount).await?;
    Ok(Json(serde_json::json!({ "transaction_id": tx_id })))
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
#[tracing::instrument(skip_all, fields(path = "/api/v1/commerce/purchase"))]
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

    // Ownership check (IDOR mitigation)
    let escrows = engine.list_escrows(auth.agent_id).await?;
    let owns_escrow = escrows.iter().any(|e| e.id == escrow_id);
    if !owns_escrow {
        return Err(AppError::forbidden(
            "You do not have permission to release this escrow",
        ));
    }

    engine.escrow_release(&escrow_id, req.recipient_id).await?;
    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateCheckoutSessionRequest {
    #[schema(value_type = String)]
    pub agent_id: Uuid,
    pub price_id: String,
    pub success_url: String,
    pub cancel_url: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreateCheckoutSessionResponse {
    pub url: String,
}

/// [POST] /api/v1/commerce/checkout-session/create
#[utoipa::path(
    post,
    path = "/api/v1/commerce/checkout-session/create",
    request_body = CreateCheckoutSessionRequest,
    responses(
        (status = 200, description = "Checkout Session created", body = CreateCheckoutSessionResponse),
        (status = 400, description = "Invalid URL scheme"),
        (status = 403, description = "Unauthorized access")
    ),
    security(("api_key" = []))
)]
pub async fn create_checkout_session(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Json(req): Json<CreateCheckoutSessionRequest>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(
        "🛒 [Commerce] Checkout session create for agent: {}",
        req.agent_id
    );

    if req.agent_id != auth.agent_id {
        return Err(AppError::forbidden("Unauthorized access to this agent"));
    }

    // NOTE: AIOME_DEV_MODE is read per-request rather than from AppState.
    // This is acceptable since std::env::var is cheap and this endpoint is low-frequency.
    // Future: consider migrating to a boot-time flag in AppState.
    let is_dev = std::env::var("AIOME_DEV_MODE").unwrap_or_default() == "1";

    let is_valid_scheme = |raw_url: &str| -> bool {
        // Use URL parser to extract scheme and host, preventing bypass via query params
        let parsed = match url::Url::parse(raw_url) {
            Ok(u) => u,
            Err(_) => return false,
        };
        if parsed.scheme() == "https" {
            return true;
        }
        if is_dev && parsed.scheme() == "http" {
            if let Some(host) = parsed.host_str() {
                return host == "localhost" || host == "127.0.0.1";
            }
        }
        false
    };

    if !is_valid_scheme(&req.success_url) || !is_valid_scheme(&req.cancel_url) {
        return Err(AppError::bad_request(
            "success_url and cancel_url must use https:// scheme (http://localhost allowed in dev mode)",
        ));
    }

    let engine = state.commerce_engine.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Commerce Engine not enabled".into(),
        }
    })?;

    let url = engine
        .create_checkout_session(
            req.agent_id,
            &req.price_id,
            &req.success_url,
            &req.cancel_url,
        )
        .await?;

    Ok(Json(CreateCheckoutSessionResponse { url }))
}
