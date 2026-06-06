/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use crate::state::SharedState;
use axum::{
    http::HeaderMap, http::StatusCode, response::IntoResponse, routing::post, Extension, Router,
};

pub fn stripe_routes() -> Router {
    Router::new().route("/webhook", post(handle_stripe_webhook))
}

async fn handle_stripe_webhook(
    Extension(state): Extension<SharedState>,
    headers: HeaderMap,
    payload: String,
) -> impl IntoResponse {
    let sig = headers
        .get("Stripe-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "Missing Stripe-Signature header".to_string(),
            )
        })?;

    if !sig
        .chars()
        .all(|c| c.is_alphanumeric() || c == '=' || c == ',' || c == '_')
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid signature format".to_string(),
        ));
    }

    let handler = state.stripe_handler.as_ref().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Stripe handler not configured".to_string(),
        )
    })?;

    let event = handler.verify_and_parse(&payload, sig).map_err(|e| {
        (
            StatusCode::UNAUTHORIZED,
            format!("Invalid Signature: {}", e),
        )
    })?;

    handler.handle_event(event).await.map_err(|e| {
        tracing::error!("Stripe Event Process Error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error".to_string(),
        )
    })?;

    Ok::<_, (StatusCode, String)>((StatusCode::OK, "Webhook Processed".to_string()))
}
