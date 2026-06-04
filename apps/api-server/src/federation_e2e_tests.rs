/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
#[cfg(test)]
mod tests {

    use infrastructure::db::DatabasePool;
    use infrastructure::job_queue::federation::FederationOps;
    use infrastructure::job_queue::UniversalJobQueue;

    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::Arc;

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

    #[tokio::test]
    async fn test_federation_sync_respects_lamport_clock_ordering() {
        let mock_server = MockServer::start().await;
        let (jq, db) = setup_test_jq(mock_server.uri()).await;

        let karma1 = aiome_core::contracts::FederatedKarma {
            id: "sync_karma_clock_test".to_string(),
            job_id: None,
            karma_type: "Technical".to_string(),
            related_skill: "ClockSync".to_string(),
            lesson: "Newer version".to_string(),
            weight: 100,
            soul_version_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_applied_at: None,
            score: 1.0,
            lamport_clock: 10,
            node_id: "node_A".to_string(),
            signature: None,
            clone_origin_id: None,
            generation: Some(1),
            somatic_valence: None,
        };

        // First import (clock 10)
        let response_body1 = serde_json::json!({
            "status": "ok",
            "new_karmas": [karma1],
            "new_immune_rules": [],
            "new_arena_matches": [],
            "server_time": chrono::Utc::now().to_rfc3339(),
            "next_cursor": null,
            "has_more": false
        });

        Mock::given(method("POST"))
            .and(path("/api/v1/federation/sync"))
            .and(header("Authorization", "Bearer test_federation_secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body1))
            .expect(1)
            .mount(&mock_server)
            .await;

        jq.do_sync_federated_data().await.unwrap();

        // Check it was saved
        let q = "SELECT lesson, lamport_clock FROM karma_logs WHERE id = 'sync_karma_clock_test'";
        let row1 = sqlx::query_as::<_, (String, i64)>(q)
            .fetch_one(db.get_sqlite_pool_or_err().unwrap())
            .await
            .unwrap();
        assert_eq!(row1.0, "Newer version");
        assert_eq!(row1.1, 10);

        mock_server.reset().await; // Clear previous mock expectations

        // Second import (clock 5 - OLDER data, should be ignored)
        let mut karma2 = karma1.clone();
        karma2.lesson = "Older version".to_string();
        karma2.lamport_clock = 5;

        let response_body2 = serde_json::json!({
            "status": "ok",
            "new_karmas": [karma2],
            "new_immune_rules": [],
            "new_arena_matches": [],
            "server_time": chrono::Utc::now().to_rfc3339(),
            "next_cursor": null,
            "has_more": false
        });

        Mock::given(method("POST"))
            .and(path("/api/v1/federation/sync"))
            .and(header("Authorization", "Bearer test_federation_secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body2))
            .expect(1)
            .mount(&mock_server)
            .await;

        jq.do_sync_federated_data().await.unwrap();

        // Check it was NOT overwritten
        let row2 = sqlx::query_as::<_, (String, i64)>(q)
            .fetch_one(db.get_sqlite_pool_or_err().unwrap())
            .await
            .unwrap();

        assert_eq!(
            row2.0, "Newer version",
            "Older lamport clock data should not overwrite newer data"
        );
        assert_eq!(row2.1, 10);
    }

    #[tokio::test]
    async fn test_federation_sync_handles_server_errors_gracefully() {
        let mock_server = MockServer::start().await;
        let (jq, _) = setup_test_jq(mock_server.uri()).await;

        // Simulate a 500 Internal Server Error from the Samsara Hub
        Mock::given(method("POST"))
            .and(path("/api/v1/federation/sync"))
            .and(header("Authorization", "Bearer test_federation_secret"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = jq.do_sync_federated_data().await;

        // Assert: The sync operation should gracefully return an error without panicking
        assert!(
            result.is_err(),
            "Sync should return Err on 500 response, but got Ok"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("500") || err_msg.contains("Internal Server Error"),
            "Error message should contain HTTP status code or server error description. Got: {}",
            err_msg
        );
    }
}
