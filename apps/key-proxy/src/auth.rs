/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::config::AppState;
use crate::telemetry::{emit_unauthorized_access, sanitize_for_log};
use axum::{extract::State, http::StatusCode, response::Response};
use secrecy::ExposeSecret;

#[tracing::instrument(
    skip(state, req, next),
    fields(
        method = tracing::field::Empty,
        path = tracing::field::Empty,
        auth_result = tracing::field::Empty
    )
)]
pub(crate) async fn auth_middleware(
    State(state): State<AppState>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    let method = sanitize_for_log(req.method().as_str());
    let path = sanitize_for_log(req.uri().path());
    tracing::Span::current().record("method", tracing::field::display(&method));
    tracing::Span::current().record("path", tracing::field::display(&path));

    if path == "/api/v1/health" {
        tracing::Span::current().record("auth_result", "health_bypass");
        return Ok(next.run(req).await);
    }

    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Strategy 1: JWT validation via AuthManager
    let mut authenticated = false;
    let mut auth_via = "none";
    if auth_header.starts_with("Bearer ") {
        let token = auth_header.trim_start_matches("Bearer ");
        if let Ok(claims) = state.auth_manager.validate_token(token).await {
            if claims.agent_id != uuid::Uuid::nil() {
                authenticated = true;
                auth_via = "jwt";
            }
        }
    }

    // Strategy 2: Legacy Vault Secret fallback
    if !authenticated {
        let expected = format!("Bearer {}", state.vault_secret.expose_secret());
        if auth_header.len() == expected.len() {
            if bool::from(subtle::ConstantTimeEq::ct_eq(
                auth_header.as_bytes(),
                expected.as_bytes(),
            )) {
                authenticated = true;
                auth_via = "vault_secret";
            }
        }
    }

    // Strategy 3 removed: query-parameter auth leaks secrets into access logs / Referer.

    // Strategy 4: Custom header fallback (for Google GenAI SDK which uses x-goog-api-key)
    if !authenticated {
        if let Some(goog_key) = req
            .headers()
            .get("x-goog-api-key")
            .and_then(|h| h.to_str().ok())
        {
            if bool::from(subtle::ConstantTimeEq::ct_eq(
                goog_key.as_bytes(),
                state.vault_secret.expose_secret().as_bytes(),
            )) {
                authenticated = true;
                auth_via = "x_goog_api_key";
            }
        }
    }

    if authenticated {
        tracing::Span::current().record("auth_result", auth_via);
        Ok(next.run(req).await)
    } else {
        tracing::Span::current().record("auth_result", "unauthorized");
        emit_unauthorized_access(&method, &path, !auth_header.is_empty());
        Err(StatusCode::UNAUTHORIZED)
    }
}
