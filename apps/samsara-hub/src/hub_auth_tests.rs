/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
#[cfg(test)]
mod tests {
    use crate::{build_app, init_hub_db, HubState};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use shared::auth::MockAuthManager;
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn setup_app() -> axum::Router {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create memory db");
        let db_pool = shared::db::DatabasePool::Sqlite(pool);
        init_hub_db(&db_pool).await.expect("Failed to init db");

        let (tx, _) = tokio::sync::broadcast::channel(100);
        let state = Arc::new(HubState {
            pool: db_pool,
            secret: secrecy::SecretString::from("hub-secret".to_string()),
            auth_manager: Arc::new(MockAuthManager::new()),
            tx,
            active_connections: std::sync::atomic::AtomicUsize::new(0),
            agent_registry: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            config: shared::config::AiomeConfig::default(),
        });

        build_app(state)
    }

    #[tokio::test]
    async fn test_health_no_auth() {
        let app = setup_app().await;

        let req = Request::builder()
            .uri("/api/v1/health")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Health endpoint should not require auth"
        );
    }

    #[tokio::test]
    async fn test_sync_requires_auth() {
        let app = setup_app().await;

        let payload = aiome_core::contracts::FederationSyncRequest {
            node_id: "test".to_string(),
            since: None,
            protocol_version: "1.0".to_string(),
        };

        let req = Request::builder()
            .uri("/api/v1/federation/sync")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "Sync endpoint should require auth"
        );
    }
}
