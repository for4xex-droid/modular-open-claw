/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::error::AiomeError;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

/// プラットフォームとクリエイター間の収益分配を担当するモジュール
pub struct RevenueSplitter;

impl RevenueSplitter {
    /// 収益をプラットフォームとクリエイターに分配する
    /// `total_amount` は基本通貨単位（例: セントやコイン）とする
    pub async fn split_revenue(
        tx: &mut Transaction<'_, Sqlite>,
        tx_id: &str,
        total_amount: i64,
        creator_id: Uuid,
        platform_fee_pct: f64,
    ) -> Result<(), AiomeError> {
        let platform_amount = (total_amount as f64 * platform_fee_pct).round() as i64;
        let creator_amount = total_amount - platform_amount;

        // クリエイターへの分配
        sqlx::query(
            "INSERT INTO revenue_splits (tx_id, recipient_id, role, amount) VALUES (?, ?, 'creator', ?)"
        )
        .bind(tx_id)
        .bind(creator_id.to_string())
        .bind(creator_amount)
        .execute(&mut **tx)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // プラットフォームへの分配
        sqlx::query(
            "INSERT INTO revenue_splits (tx_id, recipient_id, role, amount) VALUES (?, 'platform', 'platform', ?)"
        )
        .bind(tx_id)
        .bind(platform_amount)
        .execute(&mut **tx)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_db() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE revenue_splits (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tx_id TEXT NOT NULL,
                recipient_id TEXT NOT NULL,
                role TEXT NOT NULL, -- 'creator', 'platform'
                amount INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_split_revenue_calculates_correctly() {
        let pool = setup_db().await;
        let creator_id = Uuid::new_v4();
        let tx_id = "test_tx_123";
        let total_amount = 1000; // 10.00
        let platform_fee_pct = 0.15; // 15%

        // TDD Act: Execute split inside a transaction
        let mut tx = pool.begin().await.unwrap();
        RevenueSplitter::split_revenue(&mut tx, tx_id, total_amount, creator_id, platform_fee_pct)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        // TDD Assert: Verify the records in the database
        let splits: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT recipient_id, role, amount FROM revenue_splits WHERE tx_id = ? ORDER BY role ASC"
        )
        .bind(tx_id)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(splits.len(), 2, "There should be exactly two split records");

        // We ordered by role ASC. 'creator' comes before 'platform'
        assert_eq!(splits[0].0, creator_id.to_string());
        assert_eq!(splits[0].1, "creator");
        assert_eq!(splits[0].2, 850); // 1000 * 0.85

        assert_eq!(splits[1].0, "platform".to_string());
        assert_eq!(splits[1].1, "platform");
        assert_eq!(splits[1].2, 150); // 1000 * 0.15
    }
}
