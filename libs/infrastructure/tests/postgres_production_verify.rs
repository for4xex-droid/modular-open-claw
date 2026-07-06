/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 *
 * OP-012 / R3-1: Production-like PostgreSQL verification.
 * Run via: scripts/verify-production-postgres.sh
 * Env: PRODUCTION_VERIFY_PG_BASE (default: postgres://aiome:...@localhost:5434)
 */

use infrastructure::compliance::ban_store::{BanStore, UniversalBanStore};
use infrastructure::db::DatabasePool;
use infrastructure::job_queue::postgres_init::PostgresInitializer;
use sqlx::postgres::PgPoolOptions;

fn pg_url(db: &str) -> Option<String> {
    let Ok(base) = std::env::var("PRODUCTION_VERIFY_PG_BASE") else {
        eprintln!(
            "Skipping Postgres production verify: PRODUCTION_VERIFY_PG_BASE is not set \
             (run via scripts/verify-production-postgres.sh)"
        );
        return None;
    };
    let base = base.trim_end_matches('/');
    Some(format!("{}/{}", base, db))
}

async fn connect_or_skip(url: &str) -> Option<sqlx::Pool<sqlx::Postgres>> {
    match PgPoolOptions::new().max_connections(2).connect(url).await {
        Ok(pool) => Some(pool),
        Err(e) => {
            eprintln!("Skipping Postgres production verify: connection failed: {e:?}");
            None
        }
    }
}

#[tokio::test]
async fn test_infrastructure_migrations_on_production_profile() {
    let Some(url) = pg_url("aiome") else {
        return;
    };
    let Some(pool) = connect_or_skip(&url).await else {
        return;
    };

    PostgresInitializer::init_db(&pool)
        .await
        .expect("infrastructure postgres migrations must succeed on production profile");

    let version: (String,) = sqlx::query_as("SELECT version()")
        .fetch_one(&pool)
        .await
        .expect("version query");
    assert!(
        version.0.contains("PostgreSQL"),
        "expected PostgreSQL, got {}",
        version.0
    );
}

#[tokio::test]
async fn test_nurture_migrations_on_production_profile() {
    let Some(url) = pg_url("nurture") else {
        return;
    };
    let Some(pool) = connect_or_skip(&url).await else {
        return;
    };

    sqlx::migrate!("../../commercial/migrations/postgres")
        .run(&pool)
        .await
        .expect("nurture postgres migrations must succeed");

    let table: Option<(i64,)> = sqlx::query_as(
        "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'nurture_wallets'",
    )
    .fetch_optional(&pool)
    .await
    .expect("nurture_wallets existence check");
    assert_eq!(table.map(|(c,)| c), Some(1));
}

#[tokio::test]
async fn test_samsara_hub_migrations_on_production_profile() {
    let Some(url) = pg_url("samsara_hub") else {
        return;
    };
    let Some(pool) = connect_or_skip(&url).await else {
        return;
    };

    sqlx::migrate!("../../apps/samsara-hub/migrations/postgres")
        .run(&pool)
        .await
        .expect("samsara-hub postgres migrations must succeed");

    let banned_col: Option<(i64,)> = sqlx::query_as(
        "SELECT COUNT(*) FROM information_schema.columns \
         WHERE table_name = 'node_reputation' AND column_name = 'is_banned'",
    )
    .fetch_optional(&pool)
    .await
    .expect("is_banned column check");
    assert_eq!(banned_col.map(|(c,)| c), Some(1));
}

#[tokio::test]
async fn test_ban_store_postgres_roundtrip() {
    let Some(url) = pg_url("aiome") else {
        return;
    };
    let Some(pool) = connect_or_skip(&url).await else {
        return;
    };

    PostgresInitializer::init_db(&pool)
        .await
        .expect("infrastructure migrations required before BAN test");

    let db_pool = DatabasePool::Postgres(pool);
    let store = UniversalBanStore::new(db_pool);
    store.init().await.expect("BanStore init");

    let actor = uuid::Uuid::new_v4();

    // Positive: ban → is_banned true
    assert!(!store.is_banned(&actor).await.expect("is_banned pre-check"));
    store
        .ban(&actor, "OP-012 verify", "CRITICAL", "verify-script")
        .await
        .expect("ban must succeed");
    assert!(store.is_banned(&actor).await.expect("is_banned after ban"));

    // Revert: unban → is_banned false
    store.unban(&actor).await.expect("unban must succeed");
    assert!(!store
        .is_banned(&actor)
        .await
        .expect("is_banned after unban"));
}

#[tokio::test]
async fn test_negative_invalid_database() {
    let url = std::env::var("PRODUCTION_VERIFY_PG_BASE")
        .unwrap_or_else(|_| "postgres://aiome:aiome_verify_password@localhost:5434".into());
    let url = format!("{}/does_not_exist_db", url.trim_end_matches('/'));

    let result = PgPoolOptions::new().max_connections(1).connect(&url).await;

    assert!(
        result.is_err(),
        "connection to non-existent database should fail (negative test)"
    );
}
