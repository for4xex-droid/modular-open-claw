/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use crate::state::SharedState;
use axum::{
    body::Bytes,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Extension, Router,
};

pub fn polar_routes() -> Router {
    Router::new().route("/webhook", post(handle_polar_webhook))
}

async fn handle_polar_webhook(
    Extension(state): Extension<SharedState>,
    headers: HeaderMap,
    payload: Bytes,
) -> impl IntoResponse {
    let sig = headers
        .get("webhook-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !sig
        .chars()
        .all(|c| c.is_alphanumeric() || c == '=' || c == '-' || c == '_' || c == ',')
    {
        return (
            StatusCode::BAD_REQUEST,
            "Invalid signature format".to_string(),
        );
    }

    let handler = match state.polar_handler.as_ref() {
        Some(h) => h,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Polar handler not configured".to_string(),
            )
        }
    };

    match handler.handle_event(&payload, &headers).await {
        Ok(_) => (StatusCode::OK, "Webhook Processed".to_string()),
        Err(e) => {
            tracing::error!("Polar Webhook Error: {}", e);
            (StatusCode::BAD_REQUEST, "Bad Request".to_string())
        }
    }
}
