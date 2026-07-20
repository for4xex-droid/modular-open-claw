/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 *
 * Usage of this software is subject to the BSL 1.1 terms.
 * Commercial use requires a separate license agreement.
 */

pub mod asset;
pub mod clone;
pub mod internal;
pub mod marketplace;
pub mod polar;
pub mod sandbox;
pub mod stripe;
pub mod wallet;

use crate::state::SharedState;
use axum::{middleware, Extension, Router};

pub fn nurture_routes(state: SharedState) -> Router<()> {
    Router::new()
        .nest("/mcp", crate::mcp::mcp_routes())
        .nest("/marketplace", marketplace::marketplace_routes())
        .nest("/wallet", wallet::wallet_routes())
        .nest("/clone", clone::clone_routes())
        .nest("/stripe", stripe::stripe_routes())
        .nest("/polar", polar::polar_routes())
        .nest("/sandbox", sandbox::sandbox_routes())
        .layer(Extension(state))
}

/// JWT 外に `nest_service("/internal", …)` するための内側ルータ（OP-088 P1）。
///
/// `AiomePlugin::routes()` / `merge_routes` には載せない（G9: JWT と S2S を混ぜない）。
pub fn s2s_internal_service(state: SharedState) -> Router {
    Router::new()
        .merge(internal::internal_routes())
        .layer(middleware::from_fn(crate::auth::internal_auth_middleware))
        .layer(Extension(state))
}

#[cfg(test)]
mod s2s_tests {
    use super::*;
    use axum::{
        body::Body,
        http::{header::AUTHORIZATION, Request, StatusCode},
        Router,
    };
    use nurture_bridge::traits::JobQueue;
    use secrecy::SecretString;
    use tokio_util::sync::CancellationToken;
    use tower::ServiceExt;

    async fn test_state(secret: &str) -> SharedState {
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
    async fn test_s2s_nested_rejects_bad_secret_without_jwt() {
        let state = test_state("s2s-secret").await;
        let app = Router::new().nest_service("/internal", s2s_internal_service(state));
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/internal/balance/00000000-0000-0000-0000-000000000001")
                    .header(AUTHORIZATION, "Bearer wrong")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("Invalid internal credentials"),
            "must be S2S 401, not JWT: {text}"
        );
    }
}
