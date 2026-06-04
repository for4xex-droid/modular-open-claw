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

/// Bearer トークンをヘッダーから抽出する共通ヘルパー。
/// 不正なヘッダーの場合は None を返す。
fn extract_bearer_token(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h.trim_start_matches("Bearer "))
}

/// nil UUID ガード (Expert Review G-3)。
/// nil agent_id の場合は Err(StatusCode::FORBIDDEN) を返す。
/// PII 保護: sub の先頭 8 文字のみをログに記録する。
fn guard_nil_agent_id(
    claims: &shared::auth::AiomeCustomClaims,
    context: &str,
) -> Result<(), StatusCode> {
    if claims.agent_id == uuid::Uuid::nil() {
        let sub_hash = &claims.sub.chars().take(8).collect::<String>();
        warn!(
            "🛡️ [Auth/{}] Blocked request with nil agent_id for sub: {}...",
            context, sub_hash
        );
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

/// JWT 検証失敗時の共通レスポンス生成。
/// 期限切れの場合のみ X-Token-Expired ヘッダーを付与する。
fn jwt_failure_response(e: &dyn std::fmt::Display) -> Response {
    warn!("⛔ [Auth] JWT validation failed: {}", e);
    let mut resp = (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    let err_str = e.to_string();
    let is_expired = err_str.contains("expired") || err_str.contains("ExpiredSignature");
    if is_expired {
        use axum::http::HeaderValue;
        resp.headers_mut()
            .insert("X-Token-Expired", HeaderValue::from_static("true"));
    }
    resp
}

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
        let token = extract_bearer_token(&parts.headers).ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Missing or malformed Bearer token",
            )
                .into_response()
        })?;

        match state.auth_manager.validate_token(token).await {
            Ok(claims) => {
                guard_nil_agent_id(&claims, "Authenticated")
                    .map_err(|s| (s, "agent_id (UUID) is required").into_response())?;

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
            Err(e) => Err(jwt_failure_response(&e)),
        }
    }
}

/// Pro 限定の操作を強制する認証エクストラクタ。
/// アクティブまたはトライアル中のサブスクリプションが必要です。
pub struct ProAuthenticated {
    pub agent_id: uuid::Uuid,
    pub ekyc_verified: bool,
    pub roles: Vec<shared::auth::Role>,
}

#[async_trait]
impl FromRequestParts<crate::AppState> for ProAuthenticated {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::AppState,
    ) -> Result<Self, Self::Rejection> {
        // 1. 通常の認証（BANチェック等を含む）をパスしているか検証
        let auth = Authenticated::from_request_parts(parts, state).await?;

        // 2. Stripe サブスクリプションのステータスを確認
        let commerce = match state.commerce_engine.as_opt() {
            Some(c) => c,
            None => {
                error!("🚨 [Auth] Commerce engine not initialized in AppState");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Commerce engine not initialized",
                )
                    .into_response());
            }
        };

        match commerce.get_subscription_status(auth.agent_id).await {
            Ok(aiome_core::commerce::SubscriptionStatus::Active)
            | Ok(aiome_core::commerce::SubscriptionStatus::Trialing) => Ok(ProAuthenticated {
                agent_id: auth.agent_id,
                ekyc_verified: auth.ekyc_verified,
                roles: auth.roles,
            }),
            Ok(other) => {
                warn!(
                    "⛔ [Auth] Agent {} attempted to access Pro route with sub status: {:?}",
                    auth.agent_id, other
                );
                let err = crate::error::AppError::payment_required(
                    "Subscription required (Active or Trialing)",
                );
                Err(err.into_response())
            }
            Err(e) => {
                error!(
                    "🚨 [Auth] Failed to retrieve subscription status (Fail-Closed): {}",
                    e
                );
                // Fail-Closed: Commerce infrastructure failure returns 503,
                // consistent with BanStore error handling in Authenticated extractor.
                Err((
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Subscription verification failed temporarily, please retry",
                )
                    .into_response())
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
        let token = extract_bearer_token(&parts.headers).ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Missing or malformed Bearer token",
            )
                .into_response()
        })?;

        match state.auth_manager.validate_token(token).await {
            Ok(claims) => {
                guard_nil_agent_id(&claims, "BanExempt")
                    .map_err(|s| (s, "agent_id (UUID) is required").into_response())?;

                Ok(BanExemptAuthenticated {
                    agent_id: claims.agent_id,
                    ekyc_verified: claims.ekyc_verified,
                    roles: claims.roles,
                })
            }
            Err(e) => Err(jwt_failure_response(&e)),
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
    let token = extract_bearer_token(req.headers()).ok_or(StatusCode::UNAUTHORIZED)?;

    match state.auth_manager.validate_token(token).await {
        Ok(claims) => {
            guard_nil_agent_id(&claims, "Middleware")?;

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
    let token = extract_bearer_token(req.headers()).ok_or_else(|| {
        warn!("⛔ [Auth] Missing or malformed Bearer token");
        StatusCode::UNAUTHORIZED
    })?;

    match state.auth_manager.validate_token(token).await {
        Ok(claims) => {
            guard_nil_agent_id(&claims, "JwtMiddleware")?;

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
    let token = extract_bearer_token(req.headers()).ok_or(StatusCode::UNAUTHORIZED)?;

    match state.auth_manager.validate_token(token).await {
        Ok(claims) => {
            guard_nil_agent_id(&claims, "Admin")?;

            // RBAC: Admin or System role allowed
            if claims
                .roles
                .iter()
                .any(|r| matches!(r, shared::auth::Role::Admin | shared::auth::Role::System))
            {
                Ok(next.run(req).await)
            } else {
                // SEC: PII protection — truncate sub
                let sub_hash = &claims.sub.chars().take(8).collect::<String>();
                warn!(
                    "⛔ [Auth] Access denied: Admin or System role required for sub: {}...",
                    sub_hash
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

#[cfg(test)]
mod tests {

    use crate::api_integration_tests::create_test_server;
    use axum::http::StatusCode;

    /// Helper: POST /api/v1/buzz/generate with a given bearer token.
    async fn post_buzz_generate(
        server: &axum_test::TestServer,
        bearer: &str,
    ) -> axum_test::TestResponse {
        server
            .post("/api/v1/buzz/generate")
            .add_header(
                axum::http::header::AUTHORIZATION,
                axum::http::HeaderValue::from_str(&format!("Bearer {}", bearer)).unwrap(),
            )
            .await
    }

    // ── T-1 fix: Active subscriber passes Pro gate → gets 400 (empty body), NOT 402 ──
    #[tokio::test]
    async fn test_pro_gating_active_subscriber_passes() {
        let (server, _state, _tmp) = create_test_server().await;

        // Active user (default agent_id ...0001 → MockCommerceEngine returns Active)
        let res = post_buzz_generate(&server, "mock_valid_token_ekyctest_user").await;

        // After passing ProAuthenticated, the handler rejects the empty body with 400.
        // The key assertion: it MUST NOT be 402 (which would mean the gate blocked it).
        assert_eq!(
            res.status_code(),
            StatusCode::BAD_REQUEST,
            "Active subscriber should pass Pro gate and hit handler validation (400), got {}",
            res.status_code()
        );
    }

    // ── T-2: Trialing subscriber also passes Pro gate ──
    #[tokio::test]
    async fn test_pro_gating_trialing_subscriber_passes() {
        let (server, _state, _tmp) = create_test_server().await;

        // agent_id ...0003 → MockCommerceEngine returns Trialing
        let res = post_buzz_generate(
            &server,
            "mock_valid_token_ekyc_trial_user:00000000-0000-0000-0000-000000000003",
        )
        .await;

        assert_eq!(
            res.status_code(),
            StatusCode::BAD_REQUEST,
            "Trialing subscriber should pass Pro gate and hit handler validation (400), got {}",
            res.status_code()
        );
    }

    // ── Original Negative Test: None subscriber gets 402 ──
    #[tokio::test]
    async fn test_pro_gating_none_subscriber_blocked() {
        let (server, _state, _tmp) = create_test_server().await;

        // agent_id ...0002 → MockCommerceEngine returns None
        let res = post_buzz_generate(
            &server,
            "mock_valid_token_ekyc_test_user:00000000-0000-0000-0000-000000000002",
        )
        .await;

        assert_eq!(
            res.status_code(),
            StatusCode::PAYMENT_REQUIRED,
            "None subscriber must get 402 Payment Required"
        );
    }

    // ── E-1: PastDue subscriber gets 402 ──
    #[tokio::test]
    async fn test_pro_gating_past_due_subscriber_blocked() {
        let (server, _state, _tmp) = create_test_server().await;

        // agent_id ...0004 → MockCommerceEngine returns PastDue
        let res = post_buzz_generate(
            &server,
            "mock_valid_token_ekyc_pd_user:00000000-0000-0000-0000-000000000004",
        )
        .await;

        assert_eq!(
            res.status_code(),
            StatusCode::PAYMENT_REQUIRED,
            "PastDue subscriber must get 402 Payment Required"
        );
    }

    // ── E-1: Cancelled subscriber gets 402 ──
    #[tokio::test]
    async fn test_pro_gating_cancelled_subscriber_blocked() {
        let (server, _state, _tmp) = create_test_server().await;

        // agent_id ...0005 → MockCommerceEngine returns Cancelled
        let res = post_buzz_generate(
            &server,
            "mock_valid_token_ekyc_cx_user:00000000-0000-0000-0000-000000000005",
        )
        .await;

        assert_eq!(
            res.status_code(),
            StatusCode::PAYMENT_REQUIRED,
            "Cancelled subscriber must get 402 Payment Required"
        );
    }

    // ── E-1: Unpaid subscriber gets 402 ──
    #[tokio::test]
    async fn test_pro_gating_unpaid_subscriber_blocked() {
        let (server, _state, _tmp) = create_test_server().await;

        // agent_id ...0006 → MockCommerceEngine returns Unpaid
        let res = post_buzz_generate(
            &server,
            "mock_valid_token_ekyc_up_user:00000000-0000-0000-0000-000000000006",
        )
        .await;

        assert_eq!(
            res.status_code(),
            StatusCode::PAYMENT_REQUIRED,
            "Unpaid subscriber must get 402 Payment Required"
        );
    }
}
