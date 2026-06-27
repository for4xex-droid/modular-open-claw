/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_clock_u32_boundary_failure() {
        let overflow_clock: i64 = (u32::MAX as i64) + 100;
        // 期待: JSON 変換後に i64 としての値が維持されていること。
        // 現状の実装（もし u32 にキャストして返すハンドラがあれば）では、ここで値が 99 などに化ける。
        let serialized = serde_json::to_value(overflow_clock).unwrap();
        assert_eq!(
            serialized.as_i64().unwrap(),
            overflow_clock,
            "Clock value must be preserved as i64 in JSON"
        );
    }

    #[tokio::test]
    async fn test_hub_db_clock_integrity() {
        // HubState を生成（SQLite memory）
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let db_pool = shared::db::DatabasePool::Sqlite(pool.clone());
        let _: () = crate::init_hub_db(&db_pool).await.unwrap();

        let overflow_clock: i64 = (u32::MAX as i64) + 123456;

        // 直列化と保存のシミュレーション（本来は handler 内で行われるが、ここでは直接 SQL を叩いて確認）
        // backend-agnostic な DatabasePool の挙動を確認
        sqlx::query("INSERT INTO quarantined_karma (id, node_id, karma_type, related_skill, lesson, weight, lamport_clock, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
            .bind("test_id")
            .bind("node1")
            .bind("type")
            .bind("skill")
            .bind("lesson")
            .bind(10i32)
            .bind(overflow_clock)
            .bind(chrono::Utc::now())
            .execute(&pool)
            .await
            .unwrap();

        let row = sqlx::query("SELECT lamport_clock FROM quarantined_karma WHERE id = 'test_id'")
            .fetch_one(&pool)
            .await
            .unwrap();

        // ここで i64 として取得。
        use sqlx::Row;
        let recovered: i64 = row.get("lamport_clock");
        assert_eq!(
            recovered, overflow_clock,
            "DB must store i64 clock accurately"
        );
    }

    #[tokio::test]
    async fn test_hub_rest_push_sync_integrity() {
        use aiome_core::contracts::{FederatedKarma, FederationPushRequest, FederationSyncRequest};
        use axum::extract::State;
        use axum::response::IntoResponse;
        use axum::Json;
        use std::sync::Arc;

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let db_pool = shared::db::DatabasePool::Sqlite(pool.clone());
        let _: () = crate::init_hub_db(&db_pool).await.unwrap();

        let (tx, _) = tokio::sync::broadcast::channel(10);
        let auth = Arc::new(shared::auth::MockAuthManager::new());
        let state = Arc::new(crate::HubState {
            pool: db_pool,
            secret: secrecy::SecretString::new("test".into()),
            auth_manager: auth.clone(),
            tx,
            active_connections: std::sync::atomic::AtomicUsize::new(0),
            agent_registry: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            config: shared::config::AiomeConfig::default(),
            metadata_free_channels: Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
        });

        let overflow_clock: i64 = (u32::MAX as i64) + 123456;
        let test_blob = vec![0u8, 255, 128, 64];

        // 1. PUSH
        let karma = FederatedKarma {
            id: "k1".into(),
            node_id: "n1".into(),
            karma_type: "type".into(),
            related_skill: "skill".into(),
            lesson: "lesson".into(),
            weight: 10,
            lamport_clock: overflow_clock as u64,
            created_at: chrono::Utc::now().to_rfc3339(),
            ..Default::default()
        };

        let push_req = FederationPushRequest {
            node_id: "n1".into(),
            karmas: vec![karma],
            rules: vec![],
            arena_matches: vec![],
            automerge_snapshot: Some(test_blob.clone()),
            metrics: None,
        };

        // push_handler(state, headers, payload)
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("Authorization", "Bearer test".parse().unwrap());
        let _ = crate::push_handler(State(state.clone()), headers, Json(push_req)).await;

        // 2. SYNC
        let sync_req = FederationSyncRequest {
            node_id: "n1".into(),
            since: None,
            protocol_version: "1".into(),
        };
        // sync_handler(state, headers, payload)
        let mut headers_sync = axum::http::HeaderMap::new();
        headers_sync.insert("Authorization", "Bearer test".parse().unwrap());
        let sync_res = crate::sync_handler(State(state.clone()), headers_sync, Json(sync_req))
            .await
            .into_response();

        // Response Body を解析
        let body_bytes = axum::body::to_bytes(sync_res.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let sync_body: aiome_core::contracts::FederationSyncResponse =
            serde_json::from_slice(&body_bytes).unwrap();

        // 期待: クロックとバイナリが維持されていること
        assert_eq!(
            sync_body.new_karmas[0].lamport_clock, overflow_clock as u64,
            "Clock must survive PUSH/SYNC roundtrip"
        );
        assert_eq!(
            sync_body.automerge_snapshot,
            Some(test_blob),
            "Binary timeline must survive PUSH/SYNC roundtrip"
        );
    }

    #[tokio::test]
    async fn test_hub_purge_logic() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let db_pool = shared::db::DatabasePool::Sqlite(pool.clone());
        let _: () = crate::init_hub_db(&db_pool).await.unwrap();

        // 1. Insert 5 records into quarantined_karma
        for i in 1..=5 {
            sqlx::query("INSERT INTO quarantined_karma (id, node_id, karma_type, related_skill, lesson, weight, lamport_clock, created_at, received_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
                .bind(format!("test_id_{}", i))
                .bind("node1")
                .bind("type")
                .bind("skill")
                .bind(format!("lesson {}", i))
                .bind(10i32)
                .bind(100i64)
                .bind(chrono::Utc::now())
                .bind(chrono::Utc::now())
                .execute(&pool)
                .await
                .unwrap();
        }

        // Verify 5 records exist
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM quarantined_karma")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 5);

        // 2. Execute purge logic with LIMIT 2 (simulating 100,000 limit)
        // Note: The actual production limit is 100,000, but we test the SQL logic with 2 here
        let q_karma_prune = "DELETE FROM quarantined_karma WHERE id NOT IN (SELECT id FROM quarantined_karma ORDER BY received_at DESC LIMIT 2)";
        let _ = shared::sql_exec!(&db_pool, q_karma_prune).unwrap();

        // 3. Verify only 2 records remain (the most recent ones based on received_at)
        let count_after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM quarantined_karma")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count_after.0, 2);
    }
}
