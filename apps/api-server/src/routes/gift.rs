/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core_contracts::commerce::GiftRequest;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct GiftResponse {
    pub order_id: String,
    pub status: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct GiftPolicyResponse {
    pub max_amount_usd: f64,
    pub daily_limit_reached: bool,
}

/// [POST] /api/v1/gift/send/:agent_id
/// AI による自律的ギフト（恩返し / A2C）の実行
#[utoipa::path(
    post,
    path = "/api/v1/gift/send/{agent_id}",
    request_body = GiftRequest,
    responses(
        (status = 201, description = "Gift sent successfully", body = GiftResponse),
        (status = 400, description = "Policy violation or invalid request"),
        (status = 403, description = "Unauthorized access")
    ),
    params(
        ("agent_id" = String, Path, description = "The unique ID of the agent")
    ),
    security(("api_key" = []))
)]
pub async fn send_gift(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    axum::extract::Path(agent_id): axum::extract::Path<Uuid>,
    Json(mut req): Json<GiftRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 🛡️ [GlassWorm Shield] Sanitize text fields
    req.reason = shared::guardrails::strip_invisible_unicode(&req.reason).into_owned();

    // SEC: Authentication & OS Authority
    if agent_id != auth.agent_id {
        return Err(AppError::forbidden(
            "Unauthorized gift request for this agent",
        ));
    }

    if !auth.ekyc_verified {
        tracing::warn!(
            "🛡️ [Gift] Blocked unverified gift request from agent: {}",
            agent_id
        );
        return Err(AppError::forbidden(
            "eKYC verification is required to send gifts",
        ));
    }

    tracing::info!(
        "🎁 [Gift] Processing A2C gift for agent: {} -> recipient: {}, amount: ${}",
        agent_id,
        req.recipient_email,
        req.amount_usd
    );

    // 1. Policy validation (Amount, Limit, Safety)
    state
        .gift_engine
        .validate_gift_policy(agent_id, req.amount_usd)
        .await?;

    // 2. Execute Gift Sending
    let order_id = state
        .gift_engine
        .send_gift_code(&req.recipient_email, req.amount_usd, &req.reason)
        .await?;

    // Phase 15.3: Audit Trail with PII Masking (HMAC)
    use hmac::{Hmac, Mac};
    use secrecy::ExposeSecret;
    use sha2::Sha256;

    // Use API_SERVER_SECRET as a consistent salt for HMAC to prevent rainbow table attacks
    type HmacSha256 = Hmac<Sha256>;
    let secret = state.api_server_secret.expose_secret();
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| AppError::internal("Failed to init HMAC for audit log"))?;

    mac.update(req.recipient_email.as_bytes());
    let hashed_email = hex::encode(mac.finalize().into_bytes());

    let audit_data = serde_json::json!({
        "agent_id": agent_id.to_string(),
        "recipient_email_hmac": hashed_email,
        "amount_usd": req.amount_usd,
        "order_id": &order_id,
        "reason": &req.reason,
    });

    let _ = sqlx::query(
        "INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash)
         VALUES ('gift_transactions', 'SEND', ?, ?, COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16)))"
    )
    .bind(&order_id)
    .bind(audit_data.to_string())
    .execute(state.job_queue.get_pool().get_sqlite_pool_or_err()?)
    .await;

    Ok((
        StatusCode::CREATED,
        Json(GiftResponse {
            order_id,
            status: "Sent".to_string(),
        }),
    ))
}

/// [GET] /api/v1/gift/policy/:agent_id
/// ギフト送信ポリシーの取得（LLM プロンプト構築用に現在の残高や上限を返す）
#[utoipa::path(
    get,
    path = "/api/v1/gift/policy/{agent_id}",
    responses(
        (status = 200, description = "Gift policy context", body = GiftPolicyResponse),
        (status = 403, description = "Unauthorized access")
    ),
    params(
        ("agent_id" = String, Path, description = "The unique ID of the agent")
    ),
    security(("api_key" = []))
)]
pub async fn get_gift_policy(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    axum::extract::Path(agent_id): axum::extract::Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    if agent_id != auth.agent_id {
        return Err(AppError::forbidden("Unauthorized policy query"));
    }

    // Phase 15.3: 動的ポリシースナップショット (Fetches from Engine)
    let policy = state.gift_engine.get_policy_context(agent_id).await?;

    Ok(Json(GiftPolicyResponse {
        max_amount_usd: policy.max_amount_usd,
        daily_limit_reached: policy.daily_limit_reached,
    }))
}
