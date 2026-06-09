/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::artifact_store::UniversalArtifactStore;
use crate::job_queue::UniversalJobQueue;
use crate::soul_store::UniversalSoulStore;
use aiome_core_contracts::traits::{ArtifactStore, CreateArtifactRequest, SoulStore, TaskRegistry};
use aiome_core::traits::JobQueue;
use soul::model::AgentSoul;
use std::env;
use std::sync::Arc;

async fn create_pg_queue(url: &str) -> anyhow::Result<UniversalJobQueue> {
    let ts_pool = {
        let pg = sqlx::PgPool::connect(url).await?;
        crate::db::DatabasePool::Postgres(pg)
    };
    let ts = std::sync::Arc::new(
        crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(ts_pool),
    );
    let jq = UniversalJobQueue::new(url, None, ts).await?;
    Ok(jq)
}

#[tokio::test]
async fn test_postgres_job_queue_connection() -> anyhow::Result<()> {
    // DATABASE_URL が無ければテストをスキップする
    let url = match env::var("DATABASE_URL") {
        Ok(val) => val,
        Err(_) => {
            println!("Skipping Postgres test (DATABASE_URL not set)");
            return Ok(());
        }
    };

    println!("🐘 Testing PostgreSQL with URL: {}", url);

    // Act
    let jq = create_pg_queue(&url).await?;

    // Assert
    assert!(&pool.is_postgres(), "Should be a Postgres pool");

    // Simple CRUD to verify 'jobs' and 'karma_logs' tables (already exist in postgres_init.rs)
    let job_id = jq
        .enqueue("Test", "PG Topic", "PG Style", None, None, None, 0)
        .await?;
    let job = jq.fetch_job(&job_id).await?.expect("Job not found in PG");
    assert_eq!(job.topic, "PG Topic");

    Ok(())
}

#[tokio::test]
async fn test_postgres_schema_full_coverage() -> anyhow::Result<()> {
    let url = match env::var("DATABASE_URL") {
        Ok(val) => val,
        Err(_) => return Ok(()),
    };

    let jq = create_pg_queue(&url).await?;
    let pool = &pool.get_postgres_pool_or_err()?;

    // このテーブルたちは現時点の postgres_init.rs では未作成のため、ここで失敗（RED）になることが期待される
    let tables_to_check = vec![
        "ai_artifacts",
        "soul_mutation_history",
        "soul_versions",
        "commune_messages",
        "gig_intents",
        "gig_bids",
        "escrows",
        "gig_deliveries",
        "verification_logs",
        "vault_keys",
        "ekyc_sessions",
        "quarantined_assets",
        "diagnostic_reports",
        // Hub Specific Tables (Phase 30)
        "approved_karma",
        "quarantined_karma",
        "approved_rules",
        "quarantined_rules",
        "node_reputation",
        "commune_relay_queue",
        "hub_timeline",
    ];

    for table in tables_to_check {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = $1)",
        )
        .bind(table)
        .fetch_one(pool)
        .await?;

        if !exists {
            anyhow::bail!("Table '{}' is missing in PostgreSQL. Please implement migration in postgres_init.rs.", table);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_postgres_soul_store_crud() -> anyhow::Result<()> {
    let url = match env::var("DATABASE_URL") {
        Ok(val) => val,
        Err(_) => return Ok(()),
    };

    let jq = create_pg_queue(&url).await?;
    let pool = pool.clone();

    let store = UniversalSoulStore::new(pool);

    let mut soul = AgentSoul::new("pg-soul-1".to_string());
    soul.soul_hash = "hash-123".to_string();

    store.save_soul(&soul).await?;

    let loaded = store.load_soul("pg-soul-1").await?.expect("Soul not found");
    assert_eq!(loaded.soul_hash, "hash-123");

    Ok(())
}

#[tokio::test]
async fn test_postgres_artifact_store_crud() -> anyhow::Result<()> {
    let url = match env::var("DATABASE_URL") {
        Ok(val) => val,
        Err(_) => return Ok(()),
    };

    let jq = create_pg_queue(&url).await?;
    let pool = pool.clone();

    let store = UniversalArtifactStore::new(pool, std::path::PathBuf::from("/tmp/pg_artifacts"));

    let jail = bastion::fs_guard::Jail::new("/tmp/pg_jail").expect("Failed to create jail");
    let req = CreateArtifactRequest {
        title: "PG Artifact Test".to_string(),
        category: aiome_core_contracts::traits::ArtifactCategory::Report,
        files: vec![(
            "test.txt".to_string(),
            vec![1, 2, 3],
            "text/plain".to_string(),
        )],
        tags: vec!["postgres".to_string()],
        created_by: "test-user".to_string(),
        text_content: Some("PostgreSQL artifact test content".to_string()),
        karma_refs: vec![],
        job_ref: None,
        parent_refs: vec![],
        is_protected: false,
    };

    let id = ArtifactStore::save_artifact(&store, req, &jail).await?;
    let loaded = ArtifactStore::fetch_artifact(&store, &id)
        .await?
        .expect("Artifact not found");
    assert_eq!(loaded.title, "PG Artifact Test");

    Ok(())
}

#[tokio::test]
async fn test_postgres_gig_engine_placeholder() -> anyhow::Result<()> {
    let url = match env::var("DATABASE_URL") {
        Ok(val) => val,
        Err(_) => return Ok(()),
    };
    let jq = create_pg_queue(&url).await?;
    let _pool = pool.clone();

    Ok(())
}
