/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![allow(clippy::unwrap_used)]

use infrastructure::job_queue::UniversalJobQueue;
use uuid::Uuid;

#[tokio::test]
async fn test_biome_db_migrations_and_operations() {
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

    // これによって、SQLite のマイグレーションが自動的に実行されます
    let _queue = UniversalJobQueue::new(pool.clone(), None, ts)
        .await
        .unwrap();

    let sqlite_pool = pool.get_sqlite_pool_or_err().unwrap();

    // 1. biome_runs への挿入と取得の検証
    let run_id = Uuid::new_v4().to_string();
    let agent_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO biome_runs (id, agent_id, generation, score, max_generation, cell_count, is_dendou) 
         VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&run_id)
    .bind(&agent_id)
    .bind(10)
    .bind(45.5)
    .bind(100)
    .bind(128)
    .bind(1)
    .execute(sqlite_pool)
    .await
    .unwrap();

    let (fetched_id, fetched_score, fetched_is_dendou): (String, f64, i64) =
        sqlx::query_as("SELECT id, score, is_dendou FROM biome_runs WHERE id = ?")
            .bind(&run_id)
            .fetch_one(sqlite_pool)
            .await
            .unwrap();

    assert_eq!(fetched_id, run_id);
    assert_eq!(fetched_score, 45.5);
    assert_eq!(fetched_is_dendou, 1);

    // 2. biome_specimens への挿入と取得の検証
    let specimen_id = Uuid::new_v4().to_string();
    let genome_data = "{\"sequence\": [1,2,3]}";
    sqlx::query(
        "INSERT INTO biome_specimens (id, run_id, specimen_name, genome_data, rarity, element_balance, morphology_distribution, discovered_reactions, active_cell_count) 
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&specimen_id)
    .bind(&run_id)
    .bind("HelixPredator")
    .bind(genome_data)
    .bind("legendary")
    .bind("{\"C\":100}")
    .bind("{\"Predator\":1}")
    .bind("[\"C+N->P\"]")
    .bind(42)
    .execute(sqlite_pool)
    .await
    .unwrap();

    let (fetched_specimen_name, fetched_rarity, fetched_eb, fetched_md, fetched_dr, fetched_acc): (String, String, String, String, String, i64) =
        sqlx::query_as("SELECT specimen_name, rarity, element_balance, morphology_distribution, discovered_reactions, active_cell_count FROM biome_specimens WHERE id = ?")
            .bind(&specimen_id)
            .fetch_one(sqlite_pool)
            .await
            .unwrap();

    assert_eq!(fetched_specimen_name, "HelixPredator");
    assert_eq!(fetched_rarity, "legendary");
    assert_eq!(fetched_eb, "{\"C\":100}");
    assert_eq!(fetched_md, "{\"Predator\":1}");
    assert_eq!(fetched_dr, "[\"C+N->P\"]");
    assert_eq!(fetched_acc, 42);

    // 3. biome_analytics への挿入と取得の検証
    let analytics_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO biome_analytics (id, run_id, active_cells, frozen_cells, element_imbalance) 
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&analytics_id)
    .bind(&run_id)
    .bind(80)
    .bind(48)
    .bind(0.12)
    .execute(sqlite_pool)
    .await
    .unwrap();

    let (fetched_active, fetched_frozen, fetched_imbalance): (i64, i64, f64) = sqlx::query_as(
        "SELECT active_cells, frozen_cells, element_imbalance FROM biome_analytics WHERE id = ?",
    )
    .bind(&analytics_id)
    .fetch_one(sqlite_pool)
    .await
    .unwrap();

    assert_eq!(fetched_active, 80);
    assert_eq!(fetched_frozen, 48);
    assert_eq!(fetched_imbalance, 0.12);
}
