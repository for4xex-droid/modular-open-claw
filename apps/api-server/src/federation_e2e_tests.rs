/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
#[cfg(test)]
mod tests {
    use aiome_core::traits::JobQueue;
    use infrastructure::db::DatabasePool;
    use infrastructure::job_queue::federation::FederationOps;
    use infrastructure::job_queue::UniversalJobQueue;
    use shared::config::AiomeConfig;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::Arc;
    use std::time::Duration;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup_test_jq(mock_server_uri: String) -> (UniversalJobQueue, DatabasePool) {
        let db_pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query("CREATE TABLE agent_profiles (id TEXT, name TEXT, level INTEGER, exp INTEGER, resonance INTEGER, creativity INTEGER, fatigue INTEGER);")
            .execute(&db_pool)
            .await
            .unwrap();

        sqlx::query("CREATE TABLE system_settings (key TEXT PRIMARY KEY, value TEXT, category TEXT, is_secret INTEGER, updated_at DATETIME);")
            .execute(&db_pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO system_settings (key, value, category, is_secret, updated_at) VALUES ('samsara_hub_url', ?, 'system', 0, datetime('now'))")
            .bind(mock_server_uri)
            .execute(&db_pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO system_settings (key, value, category, is_secret, updated_at) VALUES ('federation_secret', 'test_federation_secret', 'system', 1, datetime('now'))")
            .execute(&db_pool)
            .await
            .unwrap();

        let pool = DatabasePool::Sqlite(db_pool);

        let traj_store = Arc::new(
            infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = UniversalJobQueue::new(pool.clone(), None, traj_store)
            .await
            .unwrap();
        (jq, pool)
    }

    #[tokio::test]
    async fn test_push_federated_metrics_makes_http_request() {
        // Arrange
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v1/federation/push"))
            .and(header("Authorization", "Bearer test_federation_secret"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})),
            )
            .expect(1) // We expect exactly 1 request!
            .mount(&mock_server)
            .await;

        let (jq, _) = setup_test_jq(mock_server.uri()).await;

        // Act
        let result = jq.do_push_federated_metrics().await;

        // Assert
        assert!(
            result.is_ok(),
            "do_push_federated_metrics should return Ok. Err: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_import_federated_data_saves_to_db() {
        let mock_server = MockServer::start().await;
        let (jq, db) = setup_test_jq(mock_server.uri()).await;

        let karma = aiome_core::contracts::KarmaEntry {
            id: "fed_karma_1".to_string(),
            job_id: None,
            karma_type: "Technical".to_string(),
            related_skill: "Federation".to_string(),
            lesson: "Nodes must communicate".to_string(),
            weight: 50,
            soul_version_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_applied_at: None,
            score: 0.5,
            lamport_clock: 10,
            node_id: "remote_node".to_string(),
            signature: None,
            clone_origin_id: None,
            generation: Some(1),
            somatic_valence: None,
        };

        let result = jq
            .do_import_federated_data(vec![karma], vec![], vec![])
            .await;
        assert!(
            result.is_ok(),
            "do_import_federated_data should return Ok, got: {:?}",
            result.err()
        );

        // Verify it was saved to DB
        let q = "SELECT id FROM karma_logs WHERE id = 'fed_karma_1'";
        let row = sqlx::query(q)
            .fetch_optional(db.get_sqlite_pool_or_err().unwrap())
            .await
            .unwrap();

        assert!(row.is_some(), "Federated karma should be saved to database");
    }

    #[tokio::test]
    async fn test_sync_federated_data_makes_http_request_and_imports() {
        let mock_server = MockServer::start().await;

        let karma = aiome_core::contracts::FederatedKarma {
            id: "sync_karma_1".to_string(),
            job_id: None,
            karma_type: "Technical".to_string(),
            related_skill: "FederationSync".to_string(),
            lesson: "Nodes must sync".to_string(),
            weight: 100,
            soul_version_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_applied_at: None,
            score: 1.0,
            lamport_clock: 20,
            node_id: "remote_node".to_string(),
            signature: None,
            clone_origin_id: None,
            generation: Some(1),
            somatic_valence: None,
        };

        let response_body = serde_json::json!({
            "status": "ok",
            "new_karmas": [karma],
            "new_immune_rules": [],
            "new_arena_matches": [],
            "server_time": chrono::Utc::now().to_rfc3339(),
            "next_cursor": null,
            "has_more": false
        });

        Mock::given(method("POST"))
            .and(path("/api/v1/federation/sync"))
            .and(header("Authorization", "Bearer test_federation_secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let (jq, db) = setup_test_jq(mock_server.uri()).await;

        let result = jq.do_sync_federated_data().await;
        assert!(
            result.is_ok(),
            "do_sync_federated_data should return Ok, got: {:?}",
            result.err()
        );

        // Verify the synced karma was saved to DB
        let q = "SELECT id FROM karma_logs WHERE id = 'sync_karma_1'";
        let row = sqlx::query(q)
            .fetch_optional(db.get_sqlite_pool_or_err().unwrap())
            .await
            .unwrap();

        assert!(
            row.is_some(),
            "Synced federated karma should be saved to database"
        );
    }

    #[tokio::test]
    async fn test_federation_push_fails_without_secret() {
        let mock_server = MockServer::start().await;
        let (jq, db) = setup_test_jq(mock_server.uri()).await;

        // Intentionally delete the federation_secret from system_settings
        sqlx::query("DELETE FROM system_settings WHERE key = 'federation_secret'")
            .execute(db.get_sqlite_pool_or_err().unwrap())
            .await
            .unwrap();

        let result = jq.do_push_federated_metrics().await;

        // Assert: Push should fail because of missing authentication secret
        assert!(
            result.is_err(),
            "do_push_federated_metrics should return Err when federation_secret is missing"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("federation_secret is not configured"));
    }
}
