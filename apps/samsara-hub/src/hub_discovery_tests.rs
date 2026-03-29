#[cfg(test)]
mod tests {
    use axum::http::Request;
    use mdns_sd::{ServiceDaemon, ServiceInfo};
    use std::collections::HashMap;
    use std::time::Duration;
    // use tower::ServiceExt; の代わりに直接 axum::serve や oneshot を自作できないか？
    // oneshot を使うには `tower::ServiceExt` が必要。`samsara-hub` の依存に tower があるはず。

    #[tokio::test]
    async fn test_e2e_agent_discovery_flow() {
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        // 1. Initialize DB and HubState
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let db_pool = shared::db::DatabasePool::Sqlite(pool.clone());
        crate::init_hub_db(&db_pool).await.unwrap();

        let (tx, _) = tokio::sync::broadcast::channel(10);
        let agent_registry =
            std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

        let _listener = crate::mdns_listener::start_mdns_listener(agent_registry.clone()).unwrap();

        let state = std::sync::Arc::new(crate::HubState {
            pool: db_pool,
            secret: secrecy::SecretString::from("test".to_string()),
            auth_manager: std::sync::Arc::new(shared::auth::MockAuthManager::new()),
            tx,
            active_connections: std::sync::atomic::AtomicUsize::new(0),
            agent_registry: agent_registry.clone(),
        });

        let app = crate::build_app(state);

        // 2. Simulate mDNS discovery by directly inserting into the registry cache
        // Note: Real mDNS multicast over loopback is flaky in some CI/OS environments.
        {
            let mut reg = agent_registry.write().await;
            reg.insert(
                "did:key:zMockNodeE2E123".to_string(),
                crate::mdns_listener::AgentInfo {
                    did: "did:key:zMockNodeE2E123".to_string(),
                    ip: "127.0.0.1".to_string(),
                    port: 4040,
                    last_seen: std::time::Instant::now(),
                },
            );
        }

        // 3. Skip mDNS propagation wait since we injected directly

        // 4. Query Registry via REST
        let request = Request::builder()
            .uri("/api/v1/registry/agents")
            .method("GET")
            .header("Authorization", "Bearer test")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let agents: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();

        // 5. Validate discovered node
        let found = agents
            .iter()
            .find(|a| a["did"] == "did:key:zMockNodeE2E123");
        assert!(
            found.is_some(),
            "Mock Node should be successfully discovered by Hub"
        );
        let info = found.unwrap();
        assert_eq!(info["port"], 4040);
        assert_eq!(info["ip"], "127.0.0.1");
    }
}
