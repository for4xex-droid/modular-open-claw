/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

// migrate_licenses.rs
use sqlx::{sqlite::SqlitePoolOptions, Row};
use std::env;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 環境変数のセットアップ
    let _ = dotenvy::dotenv(); // Load .env
    let resolver = shared::app_data::AppDataResolver::new();
    let db_url = env::var("AIOME_DB_PATH")
        .or_else(|_| env::var("DATABASE_URL"))
        .unwrap_or_else(|_| resolver.db_url());
    println!("🚀 Starting migration script...");
    println!("📂 Using database: {}", db_url);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url.replace("sqlite://", "sqlite:"))
        .await?;

    // licenses テーブルがもし無ければ作る（マイグレーション未実行の場合）
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS licenses (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            asset_id TEXT NOT NULL,
            original_event_id TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            granted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        );",
    )
    .execute(&pool)
    .await?;

    // 2. 過去の Webhook 完了イベントを取得する前に、テーブルの存在確認を行う（開発環境用）
    let table_exists: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='stripe_webhook_events';",
    )
    .fetch_one(&pool)
    .await?;

    if table_exists.0 == 0 {
        println!("⚠️ Table 'stripe_webhook_events' does not exist. No data to migrate (likely a fresh dev DB).");
        return Ok(());
    }

    let events = sqlx::query(
        r#"
        SELECT event_id, metadata 
        FROM stripe_webhook_events 
        WHERE event_type = 'checkout.session.completed'
        "#,
    )
    .fetch_all(&pool)
    .await?;

    println!(
        "🔍 Found {} Completed Checkout Sessions locally.",
        events.len()
    );

    let mut migrated_count = 0;
    let mut skipped_count = 0;
    let mut parsing_failed_count = 0;

    // 3. ループ処理
    for row in events {
        let event_id: String = row.get("event_id");
        let metadata_str: String = row.get("metadata");

        if let Ok(metadata_json) = serde_json::from_str::<serde_json::Value>(&metadata_str) {
            // パターン A: 非常に素直に {"agent_id": "...", "asset_id": "..."} が保存されている場合 (テスト環境等)
            let agent_id = metadata_json.get("agent_id").and_then(|v| v.as_str());
            let asset_id = metadata_json.get("asset_id").and_then(|v| v.as_str());

            // パターン B: {"metadata": {"agent_id": "..."}} (CheckoutSession 直接保存)
            let (agent_id, asset_id) = if agent_id.is_none() {
                let m = metadata_json.get("metadata");
                let ag = m.and_then(|v| v.get("agent_id")).and_then(|v| v.as_str());
                let as_ = m.and_then(|v| v.get("asset_id")).and_then(|v| v.as_str());
                (ag, as_)
            } else {
                (agent_id, asset_id)
            };

            // パターン C: {"data": {"object": {"metadata": {"agent_id": "..."}}}} (Event フル保存)
            let (agent_id, asset_id) = if agent_id.is_none() {
                let m = metadata_json
                    .get("data")
                    .and_then(|v| v.get("object"))
                    .and_then(|v| v.get("metadata"));
                let ag = m.and_then(|v| v.get("agent_id")).and_then(|v| v.as_str());
                let as_ = m.and_then(|v| v.get("asset_id")).and_then(|v| v.as_str());
                (ag, as_)
            } else {
                (agent_id, asset_id)
            };

            if let (Some(ag_str), Some(as_str)) = (agent_id, asset_id) {
                if let (Ok(ag_uuid), Ok(as_uuid)) =
                    (Uuid::parse_str(ag_str), Uuid::parse_str(as_str))
                {
                    // DB に既にこの original_event_id + agent + asset の組み合わせが移行されていないか確認
                    let count: (i64,) =
                        sqlx::query_as("SELECT COUNT(*) FROM licenses WHERE original_event_id = ?")
                            .bind(&event_id)
                            .fetch_one(&pool)
                            .await?;

                    if count.0 > 0 {
                        skipped_count += 1;
                        continue;
                    }

                    // ライセンス挿入
                    let license_id = Uuid::new_v4();
                    sqlx::query(
                        r#"
                        INSERT INTO licenses (id, agent_id, asset_id, original_event_id, status)
                        VALUES (?, ?, ?, ?, 'active')
                        "#,
                    )
                    .bind(license_id.to_string())
                    .bind(ag_uuid.to_string())
                    .bind(as_uuid.to_string())
                    .bind(&event_id)
                    .execute(&pool)
                    .await?;

                    println!(
                        "✅ Migrated Event {}: Agent {} -> Asset {}",
                        event_id, ag_uuid, as_uuid
                    );
                    migrated_count += 1;
                } else {
                    println!("⚠️ Event {}: Invalid UUID format", event_id);
                    parsing_failed_count += 1;
                }
            } else {
                println!(
                    "⚠️ Event {}: Could not find agent_id/asset_id anywhere",
                    event_id
                );
                parsing_failed_count += 1;
            }
        } else {
            println!("⚠️ Event {}: Invalid JSON string", event_id);
            parsing_failed_count += 1;
        }
    }

    println!("\n🎉 Migration Summary:");
    println!("  - Migrated: {}", migrated_count);
    println!("  - Skipped (already migrated): {}", skipped_count);
    println!("  - Failed (parse validation): {}", parsing_failed_count);

    Ok(())
}
