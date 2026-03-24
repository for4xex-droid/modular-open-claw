/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::UniversalJobQueue;
use aiome_core::traits::JobQueue;
use std::env;

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
    let jq = UniversalJobQueue::new(&url).await?;

    // Assert
    assert!(jq.get_pool().is_postgres(), "Should be a Postgres pool");

    // Simple CRUD to verify 'jobs' and 'karma_logs' tables (already exist in postgres_init.rs)
    let job_id = jq.enqueue("Test", "PG Topic", "PG Style", None, None, None, 0).await?;
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

    let jq = UniversalJobQueue::new(&url).await?;
    let pool = jq.get_pool().get_postgres_pool_or_err()?;

    // このテーブルたちは現時点の postgres_init.rs では未作成のため、ここで失敗（RED）になることが期待される
    let tables_to_check = vec![
        "ai_artifacts",
        "soul_mutation_history",
        "biome_messages",
        "gig_intents",
        "vault_keys",
    ];

    for table in tables_to_check {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = $1)")
            .bind(table)
            .fetch_one(pool)
            .await?;
        
        if !exists {
             anyhow::bail!("Table '{}' is missing in PostgreSQL. Please implement migration in postgres_init.rs.", table);
        }
    }

    Ok(())
}
