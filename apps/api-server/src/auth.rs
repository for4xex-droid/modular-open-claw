/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
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
use tracing::{error, warn};

/// Marker type: Used as an extractor argument in handlers to enforce authentication.
/// Requires `AppState` as the Router state type.
pub struct Authenticated {
    pub agent_id: uuid::Uuid,
    pub ekyc_verified: bool,
    pub roles: Vec<shared::auth::Role>,
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

        if !auth_header.starts_with("Bearer ") {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Missing or malformed Bearer token",
            )
                .into_response());
        }

        let token = auth_header.trim_start_matches("Bearer ");

        match state.auth_manager.validate_token(token).await {
            Ok(claims) => {
                // nil UUID Guard (Expert Review G-3)
                if claims.agent_id == uuid::Uuid::nil() {
                    // SEC: PII protection - don't log raw 'sub'
                    let sub_hash = &claims.sub.chars().take(8).collect::<String>();
                    warn!(
                        "🛡️ [Auth] Blocked request with nil agent_id for sub: {}...",
                        sub_hash
                    );
                    return Err(
                        (StatusCode::FORBIDDEN, "agent_id (UUID) is required").into_response()
                    );
                }

                // 🚫 BAN Guard (Phase 1.3A - Fail-Closed Enforcement)
                match state.ban_store.is_banned(&claims.agent_id).await {
                    Ok(true) => {
                        warn!(
                            "🚫 [Auth] Blocked banned agent request: {}",
                            claims.agent_id
                        );
                        return Err((
                            StatusCode::FORBIDDEN,
                            "Agent account has been suspended for compliance violations",
                        )
                            .into_response());
                    }
                    Err(e) => {
                        error!("🚨 [Auth] BanStore error (Fail-Closed triggered): {}", e);
                        return Err((
                            StatusCode::SERVICE_UNAVAILABLE,
                            "Compliance check failed temporarily, please retry",
                        )
                            .into_response());
                    }
                    Ok(false) => {} // Proceed
                }

                Ok(Authenticated {
                    agent_id: claims.agent_id,
                    ekyc_verified: claims.ekyc_verified,
                    roles: claims.roles,
                })
            }
            Err(e) => {
                warn!("⛔ [Auth] JWT validation failed: {}", e);
                let mut resp = (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
                use axum::http::HeaderValue;
                resp.headers_mut()
                    .insert("X-Token-Expired", HeaderValue::from_static("true"));
                Err(resp)
            }
        }
    }
}

/// BANチェックを免除される認証エクストラクタ。
/// BANされたユーザーでも、解約などの一部の操作のみを許可するために使用する。
pub struct BanExemptAuthenticated {
    pub agent_id: uuid::Uuid,
    pub ekyc_verified: bool,
    pub roles: Vec<shared::auth::Role>,
}

#[async_trait]
impl FromRequestParts<crate::AppState> for BanExemptAuthenticated {
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

        if !auth_header.starts_with("Bearer ") {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Missing or malformed Bearer token",
            )
                .into_response());
        }

        let token = auth_header.trim_start_matches("Bearer ");

        match state.auth_manager.validate_token(token).await {
            Ok(claims) => {
                if claims.agent_id == uuid::Uuid::nil() {
                    // SEC: PII protection - don't log raw 'sub' (consistent with Authenticated)
                    let sub_hash = &claims.sub.chars().take(8).collect::<String>();
                    warn!(
                        "🛡️ [Auth/BanExempt] Blocked request with nil agent_id for sub: {}...",
                        sub_hash
                    );
                    return Err(
                        (StatusCode::FORBIDDEN, "agent_id (UUID) is required").into_response()
                    );
                }

                Ok(BanExemptAuthenticated {
                    agent_id: claims.agent_id,
                    ekyc_verified: claims.ekyc_verified,
                    roles: claims.roles,
                })
            }
            Err(e) => {
                warn!("⛔ [Auth] JWT validation failed: {}", e);
                let mut resp = (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
                use axum::http::HeaderValue;
                resp.headers_mut()
                    .insert("X-Token-Expired", HeaderValue::from_static("true"));
                Err(resp)
            }
        }
    }
}

/// legacy auth middleware function (Shared Secret) - Keep for grace period
pub async fn legacy_auth_middleware(
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
            warn!("⛔ [Auth] Invalid Bearer token (legacy) received");
        }
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// JWT Auth middleware function (Default for Phase 8.2+)
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

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = auth_header.trim_start_matches("Bearer ");

    match state.auth_manager.validate_token(token).await {
        Ok(claims) => {
            // nil UUID Guard (Expert Review G-3)
            if claims.agent_id == uuid::Uuid::nil() {
                let sub_hash = &claims.sub.chars().take(8).collect::<String>();
                tracing::warn!(
                    "🛡️ [Auth] Blocked request with nil agent_id for sub: {}...",
                    sub_hash
                );
                return Err(StatusCode::FORBIDDEN);
            }

            // G-2: Per-Agent Rate Limiting
            if let Err(e) = state.rate_limiter.check(claims.agent_id) {
                warn!(
                    "⚠️ [RateLimit] Blocked request for agent {}: {}",
                    claims.agent_id, e
                );
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
            Ok(next.run(req).await)
        }
        Err(e) => {
            warn!("⛔ [Auth] JWT validation failed: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Constant-time comparison to prevent timing attacks.
/// Both values are padded to the same length before comparison.
pub fn verify_constant_time(a: &[u8], b: &[u8]) -> bool {
    let max_len = std::cmp::max(a.len(), b.len());
    let mut a_padded = vec![0u8; max_len];
    let mut b_padded = vec![0u8; max_len];
    a_padded[..a.len()].copy_from_slice(a);
    b_padded[..b.len()].copy_from_slice(b);

    let length_match = a.len() == b.len();
    let content_match = bool::from(a_padded.ct_eq(&b_padded));

    // Non-short-circuiting bitwise AND
    length_match & content_match
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
        warn!("⛔ [Auth] Missing or malformed Bearer token");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = auth_header.trim_start_matches("Bearer ");

    match state.auth_manager.validate_token(token).await {
        Ok(claims) => {
            // nil UUID Guard (Expert Review G-3)
            if claims.agent_id == uuid::Uuid::nil() {
                let sub_hash = &claims.sub.chars().take(8).collect::<String>();
                warn!(
                    "🛡️ [Auth] Blocked request with nil agent_id for sub: {}...",
                    sub_hash
                );
                return Err(StatusCode::FORBIDDEN);
            }

            // G-2: Per-Agent Rate Limiting
            if let Err(e) = state.rate_limiter.check(claims.agent_id) {
                warn!(
                    "⚠️ [RateLimit] Blocked request for agent {}: {}",
                    claims.agent_id, e
                );
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }

            // Embed claims into request extensions so handlers can extract it
            req.extensions_mut().insert(AuthenticatedUser(claims));
            Ok(next.run(req).await)
        }
        Err(e) => {
            warn!("⛔ [Auth] JWT validation failed: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Admin Only middleware function: Used for Cleanroom Audit APIs.
pub async fn admin_only_middleware(
    State(state): State<crate::AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = auth_header.trim_start_matches("Bearer ");

    match state.auth_manager.validate_token(token).await {
        Ok(claims) => {
            // nil UUID Guard (Expert Review G-3) — consistent with auth_middleware
            if claims.agent_id == uuid::Uuid::nil() {
                let sub_hash = &claims.sub.chars().take(8).collect::<String>();
                warn!(
                    "🛡️ [Auth] Blocked admin request with nil agent_id for sub: {}...",
                    sub_hash
                );
                return Err(StatusCode::FORBIDDEN);
            }

            // RBAC: Admin or System role allowed
            if claims
                .roles
                .iter()
                .any(|r| matches!(r, shared::auth::Role::Admin | shared::auth::Role::System))
            {
                Ok(next.run(req).await)
            } else {
                warn!(
                    "⛔ [Auth] Access denied: Admin or System role required for sub: {}",
                    claims.sub
                );
                Err(StatusCode::FORBIDDEN)
            }
        }
        Err(e) => {
            warn!("⛔ [Auth] JWT validation failed: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
