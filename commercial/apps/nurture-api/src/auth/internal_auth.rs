/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 */

//! Server-to-Server Bearer auth for `/internal/*` (NURTURE_INTERNAL_SECRET).

use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use secrecy::ExposeSecret;
use subtle::ConstantTimeEq;

/// Zero-Trust S2S gate: `Authorization: Bearer <NURTURE_INTERNAL_SECRET>`.
pub async fn internal_auth_middleware(
    axum::Extension(state): axum::Extension<crate::state::SharedState>,
    req: Request,
    next: Next,
) -> Response {
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();

    let expected_secret = state.internal_secret.expose_secret();
    let expected_bearer = format!("Bearer {}", expected_secret);

    let is_valid = if auth_header.len() == expected_bearer.len() {
        bool::from(auth_header.as_bytes().ct_eq(expected_bearer.as_bytes()))
    } else {
        false
    };

    if is_valid {
        next.run(req).await
    } else {
        tracing::warn!(
            "🚨 [Nurture-Auth] Unauthorized access attempt. Header present: {}",
            !auth_header.is_empty()
        );
        (
            StatusCode::UNAUTHORIZED,
            "Unauthorized: Invalid internal credentials",
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Extension, Router,
    };
    use nurture_bridge::traits::JobQueue;
    use secrecy::SecretString;
    use tokio_util::sync::CancellationToken;
    use tower::ServiceExt;

    async fn test_state(secret: &str) -> crate::state::SharedState {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let job_queue = std::sync::Arc::new(
            nurture_infra::mock_job_queue::RealJobQueue::new("sqlite::memory:")
                .await
                .unwrap(),
        );
        crate::state::AppState::init(
            nurture_bridge::db::DatabasePool::Sqlite(pool),
            job_queue as std::sync::Arc<dyn JobQueue>,
            crate::state::EconomyPolicy::default(),
            commerce_protocol::identity::ActorId(uuid::Uuid::new_v4()),
            CancellationToken::new(),
            SecretString::from(secret.to_string()),
            None,
            None,
            std::sync::Arc::new(nurture_bridge::auth::MockAuthManager::new()),
            SecretString::from("drm-key".to_string()),
            std::sync::Arc::new(nurture_infra::storage::MockAssetStorage::new()),
            None,
            "localhost".to_string(),
            "50051".to_string(),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_internal_auth_rejects_wrong_bearer() {
        let state = test_state("correct-secret").await;
        let app = Router::new()
            .route("/ping", get(|| async { "ok" }))
            .layer(middleware::from_fn(internal_auth_middleware))
            .layer(Extension(state));

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/ping")
                    .header(AUTHORIZATION, "Bearer wrong-secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_internal_auth_accepts_correct_bearer() {
        let state = test_state("correct-secret").await;
        let app = Router::new()
            .route("/ping", get(|| async { "ok" }))
            .layer(middleware::from_fn(internal_auth_middleware))
            .layer(Extension(state));

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/ping")
                    .header(AUTHORIZATION, "Bearer correct-secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
