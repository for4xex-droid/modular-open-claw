/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![allow(clippy::unwrap_used)]

use aiome_core::commune::CommuneMessage;
use aiome_core_contracts::traits::CommuneRegistry;
use infrastructure::job_queue::UniversalJobQueue;

#[tokio::test]
async fn test_commune_dialogue_limit() {
    let ts = std::sync::Arc::new(
        infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(
            infrastructure::db::DatabasePool::Sqlite(
                sqlx::sqlite::SqlitePoolOptions::new()
                    .connect("sqlite::memory:")
                    .await
                    .unwrap(),
            ),
        ),
    );
    let pool = infrastructure::db::DatabasePool::new_sqlite("sqlite::memory:")
        .await
        .unwrap();
    let queue = UniversalJobQueue::new(pool.clone(), None, ts)
        .await
        .unwrap();
    let topic_id = "test_dialogue_topic";

    // Simulate 10 turns
    for i in 0..10 {
        let count = queue.advance_commune_turn(topic_id, 0).await.unwrap();
        assert_eq!(count, i + 1);

        let msg = CommuneMessage {
            sender_pubkey: "peer_a".to_string(),
            recipient_pubkey: "peer_b".to_string(),
            topic_id: topic_id.to_string(),
            content: format!("Msg {}", i),
            karma_root_cid: "cid".to_string(),
            signature: "sig".to_string(),
            lamport_clock: i as u64,
            timestamp: chrono::Utc::now().to_rfc3339(),
            encryption: "none".to_string(),
            payload_type: None,
        };
        queue.store_commune_message(&msg).await.unwrap();
    }

    let status = queue
        .get_commune_topic_status(topic_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status.0, 10); // 10 turns reached

    // Archive it
    queue.archive_commune_topic(topic_id).await.unwrap();

    // Verify it's archived
    let archived_status: String =
        sqlx::query_scalar::<_, String>("SELECT status FROM commune_topics WHERE topic_id = ?")
            .bind(topic_id)
            .fetch_one(pool.get_sqlite_pool_or_err().unwrap())
            .await
            .unwrap();

    assert_eq!(archived_status, "Archived");
}

#[tokio::test]
async fn test_commune_message_payload_type() {
    let ts = std::sync::Arc::new(
        infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(
            infrastructure::db::DatabasePool::Sqlite(
                sqlx::sqlite::SqlitePoolOptions::new()
                    .connect("sqlite::memory:")
                    .await
                    .unwrap(),
            ),
        ),
    );
    let pool = infrastructure::db::DatabasePool::new_sqlite("sqlite::memory:")
        .await
        .unwrap();
    let queue = UniversalJobQueue::new(pool.clone(), None, ts)
        .await
        .unwrap();
    let topic_id = "payload_type_test";

    queue.advance_commune_turn(topic_id, 0).await.unwrap();

    let msg_with_payload = CommuneMessage {
        sender_pubkey: "peer_a".to_string(),
        recipient_pubkey: "peer_b".to_string(),
        topic_id: topic_id.to_string(),
        content: "{\"blueprint_data\":\"xyz\"}".to_string(),
        karma_root_cid: "cid".to_string(),
        signature: "sig".to_string(),
        lamport_clock: 1,
        timestamp: chrono::Utc::now().to_rfc3339(),
        encryption: "none".to_string(),
        payload_type: Some("genetic_blueprint".to_string()),
    };

    queue
        .store_commune_message(&msg_with_payload)
        .await
        .unwrap();

    let fetched = queue.fetch_commune_messages(topic_id, 10).await.unwrap();
    assert_eq!(fetched.len(), 1);
    let fetched_msg = &fetched[0];
    assert_eq!(
        fetched_msg["payload_type"].as_str(),
        Some("genetic_blueprint")
    );
}

#[tokio::test]
async fn test_commune_shared_genome_exchange() {
    let ts = std::sync::Arc::new(
        infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(
            infrastructure::db::DatabasePool::Sqlite(
                sqlx::sqlite::SqlitePoolOptions::new()
                    .connect("sqlite::memory:")
                    .await
                    .unwrap(),
            ),
        ),
    );
    let pool = infrastructure::db::DatabasePool::new_sqlite("sqlite::memory:")
        .await
        .unwrap();
    let queue = UniversalJobQueue::new(pool.clone(), None, ts)
        .await
        .unwrap();
    let topic_id = "genome_exchange_test";

    sqlx::query("CREATE TABLE IF NOT EXISTS commune_shared_genomes (id INTEGER PRIMARY KEY AUTOINCREMENT, topic_id TEXT NOT NULL, blueprint_json TEXT NOT NULL, created_at TEXT DEFAULT (datetime('now')));")
        .execute(pool.get_sqlite_pool_or_err().unwrap())
        .await
        .unwrap();

    let blueprint_data = "{\"species_name\":\"HelixPredator\",\"generation\":42}";

    let store_res = queue.store_shared_genome(topic_id, blueprint_data).await;
    assert!(
        store_res.is_ok(),
        "Expected store_res to be Ok but got: {:?}",
        store_res
    );

    let fetch_res = queue.fetch_shared_genomes(topic_id, 10).await;
    assert!(
        fetch_res.is_ok(),
        "Expected fetch_res to be Ok but got: {:?}",
        fetch_res
    );
    let list = fetch_res.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["blueprint_json"].as_str().unwrap(), blueprint_data);
}
