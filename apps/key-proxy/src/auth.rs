/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::config::AppState;
use axum::{extract::State, http::StatusCode, response::Response};
use secrecy::ExposeSecret;
use tracing::warn;

#[tracing::instrument(skip(state))]
pub(crate) async fn auth_middleware(
    State(state): State<AppState>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Strategy 1: JWT validation via AuthManager
    let mut authenticated = false;
    if auth_header.starts_with("Bearer ") {
        let token = auth_header.trim_start_matches("Bearer ");
        if let Ok(claims) = state.auth_manager.validate_token(token).await {
            if claims.agent_id != uuid::Uuid::nil() {
                authenticated = true;
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
            }
        }
    }

    // Strategy 3: Query parameter fallback (for SDKs that use key=... instead of headers)
    if !authenticated {
        if let Some(query) = req.uri().query() {
            for param in query.split('&') {
                if let Some(provided_key) = param.strip_prefix("key=") {
                    if bool::from(subtle::ConstantTimeEq::ct_eq(
                        provided_key.as_bytes(),
                        state.vault_secret.expose_secret().as_bytes(),
                    )) {
                        authenticated = true;
                        break;
                    }
                }
            }
        }
    }

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
            }
        }
    }

    if authenticated {
        Ok(next.run(req).await)
    } else {
        warn!("⛔ [KeyProxy] Unauthorized access attempt.");
        Err(StatusCode::UNAUTHORIZED)
    }
}
