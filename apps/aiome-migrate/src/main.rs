#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use anyhow::{Context, Result};
use clap::Parser;
use infrastructure::db::DatabasePool;
use sqlx::{PgPool, SqlitePool};
use tracing::{info, warn};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, env = "DATABASE_URL", help = "SQLite database file path or URL")]
    sqlite_url: String,

    #[arg(long, env = "POSTGRES_URL", help = "Target PostgreSQL URL")]
    postgres_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    // 1. Initial attempt from CWD (essential for dev environments)
    dotenvy::dotenv().ok();

    let resolver = shared::app_data::AppDataResolver::new();
    
    // 2. Explicit attempt from application root
    let app_env_path = resolver.root().join(".env");
    if app_env_path.exists() {
        dotenvy::from_path(&app_env_path).ok();
    }

    let args = Args::parse();

    info!("🚀 Initializing Aiome db migration tool");
    info!("Source (SQLite): {}", args.sqlite_url);
    info!("Target (Postgres): {}", args.postgres_url);

    let sqlite_pool = SqlitePool::connect(&args.sqlite_url)
        .await
        .context("Failed to connect to SQLite")?;
    let pg_pool = PgPool::connect(&args.postgres_url)
        .await
        .context("Failed to connect to PostgreSQL")?;

    info!("✅ Connected to both databases. Beginning migration...");

    // 1. Ensure target schema is up-to-date
    // Note: We assume `sqlx migrate run` has already been executed on the target PostgreSQL DB.
    migrate_table_timeline_snapshots(&sqlite_pool, &pg_pool).await?;
    migrate_table_souls(&sqlite_pool, &pg_pool).await?;
    migrate_table_karma_ledger(&sqlite_pool, &pg_pool).await?;
    migrate_table_audit_ledger(&sqlite_pool, &pg_pool).await?;
    migrate_table_approved_karma(&sqlite_pool, &pg_pool).await?;
    migrate_table_approved_rules(&sqlite_pool, &pg_pool).await?;
    migrate_table_approved_arena_matches(&sqlite_pool, &pg_pool).await?;

    info!("🎉 Migration complete!");
    Ok(())
}

async fn migrate_table_souls(sqlite: &SqlitePool, pg: &PgPool) -> Result<()> {
    use sqlx::Row;
    info!("📦 Migrating souls...");
    let rows = sqlx::query("SELECT * FROM souls").fetch_all(sqlite).await?;
    let mut count = 0;
    for row in rows {
        let id: i64 = row.get("id");
        let hash: String = row.get("hash");
        let soul_id: String = row.get("soul_id"); // Or uuid::Uuid? In SQLite it's parsed as String if text, UUID if it's binary. Let's try String first and parse it: `uuid::Uuid::parse_str(&soul_id).unwrap()`
        let soul_uuid = uuid::Uuid::parse_str(&soul_id).unwrap_or_default();
        let parent_hash: Option<String> = row.get("parent_hash");
        let sm_str: Option<String> = row.get("somatic_markers");
        let somatic_markers: serde_json::Value = sm_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let attachment_style: String = row.get("attachment_style");
        let narrative_self: String = row.get("narrative_self");
        let prompt_fragment: String = row.get("prompt_fragment");
        let generation: i64 = row.get("generation");

        sqlx::query(
            "INSERT INTO souls (id, hash, soul_id, parent_hash, somatic_markers, created_at, attachment_style, narrative_self, prompt_fragment, generation) 
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) ON CONFLICT (id) DO NOTHING"
        )
        .bind(id)
        .bind(hash)
        .bind(soul_uuid)
        .bind(parent_hash)
        .bind(somatic_markers)
        .bind(created_at)
        .bind(attachment_style)
        .bind(narrative_self)
        .bind(prompt_fragment)
        .bind(generation as i32)
        .execute(pg)
        .await?;
        count += 1;
    }
    info!("✅ souls: migrated {} records", count);
    Ok(())
}

async fn migrate_table_karma_ledger(sqlite: &SqlitePool, pg: &PgPool) -> Result<()> {
    use sqlx::Row;
    info!("📦 Migrating karma_ledger...");
    let rows = sqlx::query("SELECT * FROM karma_ledger")
        .fetch_all(sqlite)
        .await?;
    let mut count = 0;
    for row in rows {
        let id: String = row.get("id");
        let action: String = row.get("action");
        let actor: String = row.get("actor");
        let amount: i64 = row.get("amount");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let details: Option<String> = row.get("details");
        let signature: Option<String> = row.get("signature");

        sqlx::query(
            "INSERT INTO karma_ledger (id, action, actor, amount, created_at, details, signature) 
             VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .bind(action)
        .bind(actor)
        .bind(amount)
        .bind(created_at)
        .bind(details)
        .bind(signature)
        .execute(pg)
        .await?;
        count += 1;
    }
    info!("✅ karma_ledger: migrated {} records", count);
    Ok(())
}

async fn migrate_table_audit_ledger(sqlite: &SqlitePool, pg: &PgPool) -> Result<()> {
    use sqlx::Row;
    info!("📦 Migrating audit_ledger_global...");
    let rows = sqlx::query("SELECT * FROM audit_ledger_global")
        .fetch_all(sqlite)
        .await?;
    let mut count = 0;
    for row in rows {
        let event_id: String = row.get("event_id");
        let table_name: String = row.get("table_name");
        let operation: String = row.get("operation");
        let source_node: String = row.get("source_node");
        let diff_str: Option<String> = row.get("diff_payload");
        let diff_payload: serde_json::Value = diff_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        let recorded_at: chrono::DateTime<chrono::Utc> = row.get("recorded_at");

        sqlx::query(
            "INSERT INTO audit_ledger_global (event_id, table_name, operation, source_node, diff_payload, recorded_at) 
             VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (event_id) DO NOTHING"
        )
        .bind(event_id)
        .bind(table_name)
        .bind(operation)
        .bind(source_node)
        .bind(diff_payload)
        .bind(recorded_at)
        .execute(pg)
        .await?;
        count += 1;
    }
    info!("✅ audit_ledger_global: migrated {} records", count);
    Ok(())
}

async fn migrate_table_approved_karma(sqlite: &SqlitePool, pg: &PgPool) -> Result<()> {
    use sqlx::Row;
    info!("📦 Migrating approved_karma...");
    let rows = sqlx::query("SELECT * FROM approved_karma")
        .fetch_all(sqlite)
        .await?;
    let mut count = 0;
    for row in rows {
        let id: String = row.get("id");
        let node_id: String = row.get("node_id");
        let karma_type: Option<String> = row.get("karma_type");
        let related_skill: Option<String> = row.get("related_skill");
        let lesson: String = row.get("lesson");
        let weight: i64 = row.get("weight");
        let soul_version_hash: Option<String> = row.get("soul_version_hash");
        let lamport_clock: i64 = row.get("lamport_clock");
        let signature: Option<String> = row.get("signature");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let approved_at: chrono::DateTime<chrono::Utc> = row.get("approved_at");
        let tier: String = row.get("tier");
        let clone_origin_id: Option<String> = row.get("clone_origin_id");
        let generation: Option<i64> = row.get("generation");
        let somatic_valence: Option<f64> = row.get("somatic_valence");

        sqlx::query(
            "INSERT INTO approved_karma (id, node_id, karma_type, related_skill, lesson, weight, soul_version_hash, lamport_clock, signature, created_at, approved_at, tier, clone_origin_id, generation, somatic_valence) 
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) ON CONFLICT (id) DO NOTHING"
        )
        .bind(id)
        .bind(node_id)
        .bind(karma_type)
        .bind(related_skill)
        .bind(lesson)
        .bind(weight)
        .bind(soul_version_hash)
        .bind(lamport_clock)
        .bind(signature)
        .bind(created_at)
        .bind(approved_at)
        .bind(tier)
        .bind(clone_origin_id)
        .bind(generation.map(|v| v as i32))
        .bind(somatic_valence)
        .execute(pg)
        .await?;
        count += 1;
    }
    info!("✅ approved_karma: migrated {} records", count);
    Ok(())
}

async fn migrate_table_approved_rules(sqlite: &SqlitePool, pg: &PgPool) -> Result<()> {
    use sqlx::Row;
    info!("📦 Migrating approved_rules...");
    let rows = sqlx::query("SELECT * FROM approved_rules")
        .fetch_all(sqlite)
        .await?;
    let mut count = 0;
    for row in rows {
        let id: String = row.get("id");
        let pattern: String = row.get("pattern");
        let severity: i64 = row.get("severity");
        let action: String = row.get("action");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let approved_at: chrono::DateTime<chrono::Utc> = row.get("approved_at");
        let node_id: String = row.get("node_id");
        let lamport_clock: i64 = row.get("lamport_clock");
        let signature: Option<String> = row.get("signature");

        sqlx::query(
            "INSERT INTO approved_rules (id, pattern, severity, action, created_at, approved_at, node_id, lamport_clock, signature) 
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) ON CONFLICT (id) DO NOTHING"
        )
        .bind(id)
        .bind(pattern)
        .bind(severity as i32)
        .bind(action)
        .bind(created_at)
        .bind(approved_at)
        .bind(node_id)
        .bind(lamport_clock)
        .bind(signature)
        .execute(pg)
        .await?;
        count += 1;
    }
    info!("✅ approved_rules: migrated {} records", count);
    Ok(())
}

async fn migrate_table_approved_arena_matches(sqlite: &SqlitePool, pg: &PgPool) -> Result<()> {
    use sqlx::Row;
    info!("📦 Migrating approved_arena_matches...");
    let rows = sqlx::query("SELECT * FROM approved_arena_matches")
        .fetch_all(sqlite)
        .await?;
    let mut count = 0;
    for row in rows {
        let id: String = row.get("id");
        let node_id: String = row.get("node_id");
        let skill_a: String = row.get("skill_a");
        let skill_b: String = row.get("skill_b");
        let topic: String = row.get("topic");
        let winner: String = row.get("winner");
        let reasoning: String = row.get("reasoning");
        let prompt_payload: Option<String> = row.get("prompt_payload");
        let analysis_payload: Option<String> = row.get("analysis_payload");
        let model_id: Option<String> = row.get("model_id");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let approved_at: chrono::DateTime<chrono::Utc> = row.get("approved_at");

        sqlx::query(
            "INSERT INTO approved_arena_matches (id, node_id, skill_a, skill_b, topic, winner, reasoning, prompt_payload, analysis_payload, model_id, created_at, approved_at) 
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) ON CONFLICT (id) DO NOTHING"
        )
        .bind(id)
        .bind(node_id)
        .bind(skill_a)
        .bind(skill_b)
        .bind(topic)
        .bind(winner)
        .bind(reasoning)
        .bind(prompt_payload)
        .bind(analysis_payload)
        .bind(model_id)
        .bind(created_at)
        .bind(approved_at)
        .execute(pg)
        .await?;
        count += 1;
    }
    info!("✅ approved_arena_matches: migrated {} records", count);
    Ok(())
}

async fn migrate_table_timeline_snapshots(sqlite: &SqlitePool, pg: &PgPool) -> Result<()> {
    use sqlx::Row;
    info!("📦 Migrating timeline_snapshots...");
    let rows = sqlx::query("SELECT node_id, snapshot_blob FROM timeline_snapshots")
        .fetch_all(sqlite)
        .await?;

    let mut count = 0;
    for row in rows {
        let node_id: String = row.get("node_id");
        let blob: Vec<u8> = row.get("snapshot_blob");

        sqlx::query("INSERT INTO timeline_snapshots (node_id, snapshot_blob) VALUES ($1, $2) ON CONFLICT (node_id) DO NOTHING")
            .bind(node_id)
            .bind(blob)
            .execute(pg)
            .await?;
        count += 1;
    }
    info!("✅ timeline_snapshots: migrated {} records", count);
    Ok(())
}
