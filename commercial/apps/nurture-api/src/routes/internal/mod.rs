/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use crate::state::SharedState;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Extension, Router,
};
use chrono::Utc;
use nurture_bridge::oxilean::OxiLeanProofCertificate;
use secrecy::ExposeSecret;

pub mod balance;
pub mod escrow;
pub mod gdpr;
pub mod idempotency_gate;
pub mod misc;
pub mod types;

// 外部互換の re-export (main.rs やテストから型を参照可能に)
pub use types::*;

pub fn internal_routes() -> Router {
    Router::new()
        .route("/balance/:actor_id", get(balance::get_balance))
        .route("/daily-stats/:actor_id", get(balance::get_daily_stats))
        .route("/coin-charge", post(balance::charge_coins))
        .route("/escrow-create", post(escrow::create_escrow))
        .route("/escrow-release", post(escrow::release_escrow))
        .route("/escrow-refund", post(escrow::refund_escrow))
        .route("/escrow-list/:actor_id", get(escrow::list_escrows))
        .route("/deduct", post(balance::deduct_cost))
        .route(
            "/economy-policy/monthly-limit",
            post(balance::update_monthly_spend_limit),
        )
        .route("/upload", post(misc::upload_handler))
        .route("/forget/:actor_id", post(gdpr::forget_actor))
        .route("/oxilean/status", get(misc::get_oxilean_status))
        .route("/purchase", post(misc::internal_purchase))
        .route("/transfer", post(misc::transfer_coins))
        .route("/points/:actor_id", get(misc::get_points))
        .route("/withdraw-points", post(misc::withdraw_points))
        .route(
            "/transaction-history/:actor_id",
            get(misc::get_transaction_history),
        )
        .route("/wishlist/:actor_id", get(misc::get_wishlist))
        .route("/instant-refund", post(misc::instant_refund))
        .route("/lora-train", post(misc::internal_lora_train))
        .route("/validate-activity", post(misc::internal_validate_activity))
        .nest("/asset", crate::routes::asset::asset_routes())
        .layer(middleware::from_fn(require_oxp_certificate))
}

async fn require_oxp_certificate(
    Extension(state): Extension<SharedState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let headers = req.headers();

    // 1. Extract Header
    let cert_b64 = headers
        .get("x-oxilean-proof-certificate")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::FORBIDDEN)?;

    // 2. Decode Base64
    let cert_json = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, cert_b64)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // 3. Deserialize JSON
    let cert: OxiLeanProofCertificate =
        serde_json::from_slice(&cert_json).map_err(|_| StatusCode::BAD_REQUEST)?;

    // 4. Verify Signature with Nurture Secret
    if !cert.verify(state.internal_secret.expose_secret()) {
        tracing::warn!(
            "OxiLean Certificate verification failed for {}",
            cert.subject_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // 5. Check OXP Score Threshold
    if cert.oxp_score < 900 {
        tracing::warn!(
            "OxiLean Certificate OXP score too low: {} < 900",
            cert.oxp_score
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // 6. Check Timestamp Freshness (prevent replay attacks)
    let cert_time = cert
        .timestamp
        .parse::<chrono::DateTime<Utc>>()
        .map_err(|_| {
            tracing::warn!("Invalid timestamp format in OxiLean Certificate");
            StatusCode::BAD_REQUEST
        })?;

    let now = Utc::now();
    let age = now.signed_duration_since(cert_time).num_seconds();

    // 証明書は5分(300秒)以内に発行されたものでなければならない
    if !(-60..=300).contains(&age) {
        tracing::warn!(
            "OxiLean Certificate is stale or from the future (age: {}s)",
            age
        );
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(req).await)
}
