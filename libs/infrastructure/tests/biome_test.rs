/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::biome::BiomeMessage;
use aiome_core_contracts::traits::BiomeRegistry;
use infrastructure::job_queue::UniversalJobQueue;

#[tokio::test]
async fn test_biome_dialogue_limit() {
    let ts = std::sync::Arc::new(
        infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(
            infrastructure::db::DatabasePool::Sqlite(
                sqlx::sqlite::SqlitePoolOptions::new()
                    .connect("sqlite::memory:")
                    .await
                    .unwrap(), // allow-anti-pattern
            ),
        ),
    );
    let pool = infrastructure::db::DatabasePool::new_sqlite("sqlite::memory:")
        .await
        .unwrap();
    let queue = UniversalJobQueue::new(pool.clone(), None, ts)
        .await
        .unwrap(); // allow-anti-pattern
    let topic_id = "test_dialogue_topic";

    // Simulate 10 turns
    for i in 0..10 {
        let count = queue.advance_biome_turn(topic_id, 0).await.unwrap(); // allow-anti-pattern
        assert_eq!(count, i + 1);

        let msg = BiomeMessage {
            sender_pubkey: "peer_a".to_string(),
            recipient_pubkey: "peer_b".to_string(),
            topic_id: topic_id.to_string(),
            content: format!("Msg {}", i),
            karma_root_cid: "cid".to_string(),
            signature: "sig".to_string(),
            lamport_clock: i as u64,
            timestamp: chrono::Utc::now().to_rfc3339(),
            encryption: "none".to_string(),
        };
        queue.store_biome_message(&msg).await.unwrap(); // allow-anti-pattern
    }

    let status = queue
        .get_biome_topic_status(topic_id)
        .await
        .unwrap() // allow-anti-pattern
        .unwrap(); // allow-anti-pattern
    assert_eq!(status.0, 10); // 10 turns reached

    // Archive it
    queue.archive_biome_topic(topic_id).await.unwrap(); // allow-anti-pattern

    // Verify it's archived
    let archived_status: String =
        sqlx::query_scalar::<_, String>("SELECT status FROM biome_topics WHERE topic_id = ?")
            .bind(topic_id)
            .fetch_one(pool.get_sqlite_pool_or_err().unwrap()) // allow-anti-pattern
            .await
            .unwrap(); // allow-anti-pattern

    assert_eq!(archived_status, "Archived");
}
