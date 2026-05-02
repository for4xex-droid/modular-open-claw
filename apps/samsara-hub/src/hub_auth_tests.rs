#[cfg(test)]
mod tests {
    use super::*;
    use crate::{auth_middleware, HubState};
    use axum::http::{Request, StatusCode};
    use axum::middleware::Next;
    use axum::response::Response;
    use shared::auth::{AiomeCustomClaims, MockAuthManager, Role};
    use std::sync::Arc;
    use tower::ServiceExt;

    // We need to mock Next and Request to test middleware in isolation or use Router.
    // For TDD, I'll use a simple Router with the middleware.

    #[tokio::test]
    async fn test_hub_auth_middleware_admin_allowed() {
        let state = Arc::new(HubState {
            pool: shared::db::DatabasePool::Sqlite(
                sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap(),
            ),
            secret: secrecy::SecretString::from("hub-secret".to_string()),
            auth_manager: Arc::new(MockAuthManager::new()),
            tx: tokio::sync::broadcast::channel(1).0,
            active_connections: std::sync::atomic::AtomicUsize::new(0),
            agent_registry: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            config: shared::config::AiomeConfig::default(),
        });

        let app = axum::Router::new()
            .route("/", axum::routing::get(|| async { "OK" }))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ))
            .with_state(state);

        // Valid Admin Token (from MockAuthManager)
        let req = Request::builder()
            .uri("/")
            .header("Authorization", "Bearer mock_valid_token_admin")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_hub_auth_middleware_user_rejected() {
        let state = Arc::new(HubState {
            pool: shared::db::DatabasePool::Sqlite(
                sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap(),
            ),
            secret: secrecy::SecretString::from("hub-secret".to_string()),
            auth_manager: Arc::new(MockAuthManager::new()),
            tx: tokio::sync::broadcast::channel(1).0,
            active_connections: std::sync::atomic::AtomicUsize::new(0),
            agent_registry: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            config: shared::config::AiomeConfig::default(),
        });

        let app = axum::Router::new()
            .route("/", axum::routing::get(|| async { "OK" }))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ))
            .with_state(state);

        // Valid User Token (User role should be rejected for Hub ops)
        let req = Request::builder()
            .uri("/")
            .header("Authorization", "Bearer mock_valid_token_user")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
