/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{header::AUTHORIZATION, request::Parts, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use secrecy::ExposeSecret;
use subtle::ConstantTimeEq;
use tracing::warn;

/// Marker type: Used as an extractor argument in handlers to enforce authentication.
/// Requires `AppState` as the Router state type.
pub struct Authenticated {
    pub agent_id: uuid::Uuid,
}

#[async_trait]
impl FromRequestParts<crate::AppState> for Authenticated {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .unwrap_or_default();

        let expected_bearer = format!("Bearer {}", state.api_server_secret.expose_secret());

        if verify_constant_time(auth_header.as_bytes(), expected_bearer.as_bytes()) {
            // A-4: User Specificity - Check X-Agent-Id header
            let agent_id = parts
                .headers
                .get("X-Agent-Id")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| uuid::Uuid::parse_str(s).ok());

            // Fallback to system agent ID if not provided or invalid
            let final_agent_id = match agent_id {
                Some(id) => id,
                None => state.system_agent_id,
            };

            Ok(Authenticated {
                agent_id: final_agent_id,
            })
        } else {
            let mut resp = (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
            if !auth_header.is_empty() && auth_header.starts_with("Bearer ") {
                use axum::http::HeaderValue;
                resp.headers_mut()
                    .insert("X-Token-Expired", HeaderValue::from_static("true"));
            }
            Err(resp)
        }
    }
}

/// Auth middleware function: Used with `from_fn_with_state` for route_layer auth.
/// Performs the same constant-time comparison as the Authenticated extractor.
pub async fn auth_middleware(
    State(state): State<crate::AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();

    let expected_bearer = format!("Bearer {}", state.api_server_secret.expose_secret());

    if verify_constant_time(auth_header.as_bytes(), expected_bearer.as_bytes()) {
        Ok(next.run(req).await)
    } else {
        if !auth_header.is_empty() && auth_header.starts_with("Bearer ") {
            warn!("⛔ [Auth] Invalid Bearer token received");
        }
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Constant-time comparison to prevent timing attacks.
/// Both values are padded to the same length before comparison.
fn verify_constant_time(a: &[u8], b: &[u8]) -> bool {
    let max_len = std::cmp::max(a.len(), b.len());
    let mut a_padded = vec![0u8; max_len];
    let mut b_padded = vec![0u8; max_len];
    a_padded[..a.len()].copy_from_slice(a);
    b_padded[..b.len()].copy_from_slice(b);
    a.len() == b.len() && bool::from(a_padded.ct_eq(&b_padded))
}

/// A wrapper for the JWT Claims, provided by `jwt_auth_middleware` via request extensions.
#[derive(Clone)]
pub struct AuthenticatedUser(pub shared::auth::AiomeCustomClaims);

/// JWT Auth middleware function: Used for user-facing API endpoints.
/// Validates the Bearer token using `AuthManager` and embeds `AuthenticatedUser` into the request extensions.
pub async fn jwt_auth_middleware(
    State(state): State<crate::AppState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();

    if !auth_header.starts_with("Bearer ") {
        warn!("⛔ [JWT Auth] Missing or malformed Bearer token");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = auth_header.trim_start_matches("Bearer ");

    match state.auth_manager.validate_token(token).await {
        Ok(claims) => {
            // Embed claims into request extensions so handlers can extract it
            req.extensions_mut().insert(AuthenticatedUser(claims));
            Ok(next.run(req).await)
        }
        Err(e) => {
            warn!("⛔ [JWT Auth] Token validation failed: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
