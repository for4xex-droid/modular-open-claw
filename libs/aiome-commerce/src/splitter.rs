/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use aiome_core::error::AiomeError;
use shared::db::DatabaseTransaction;
use uuid::Uuid;

/// プラットフォームとクリエイター間の収益分配を担当するモジュール
pub struct RevenueSplitter;

impl RevenueSplitter {
    /// 収益をプラットフォームとクリエイターに分配する
    /// `total_amount` は基本通貨単位（例: セントやコイン）とする
    pub async fn split_revenue(
        tx: &mut DatabaseTransaction<'_>,
        tx_id: &str,
        total_amount: i64,
        creator_id: Uuid,
        platform_fee_pct: f64,
    ) -> Result<(), AiomeError> {
        let platform_amount = (total_amount as f64 * platform_fee_pct).round() as i64;
        let creator_amount = total_amount - platform_amount;

        // クリエイターへの分配
        let q_creator = "INSERT INTO revenue_splits (tx_id, recipient_id, role, amount) VALUES (?, ?, 'creator', ?)";
        // Note: RevenueSplitter doesn't have access to pool for placeholders,
        // but for SQLite/Postgres simple INSERTs, '?' often works if using Any,
        // however our DatabaseTransaction variants use specific drivers.
        // For simplicity, we assume '?' currently, but let's be robust.

        match tx {
            DatabaseTransaction::Sqlite(itx) => {
                sqlx::query(q_creator)
                    .bind(tx_id)
                    .bind(creator_id.to_string())
                    .bind(creator_amount)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;

                sqlx::query("INSERT INTO revenue_splits (tx_id, recipient_id, role, amount) VALUES (?, 'platform', 'platform', ?)")
                    .bind(tx_id)
                    .bind(platform_amount)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
            }
            DatabaseTransaction::Postgres(itx) => {
                sqlx::query("INSERT INTO revenue_splits (tx_id, recipient_id, role, amount) VALUES ($1, $2, 'creator', $3)")
                    .bind(tx_id)
                    .bind(creator_id.to_string())
                    .bind(creator_amount)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

                sqlx::query("INSERT INTO revenue_splits (tx_id, recipient_id, role, amount) VALUES ($1, 'platform', 'platform', $2)")
                    .bind(tx_id)
                    .bind(platform_amount)
                    .execute(&mut **itx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_db() -> shared::db::DatabasePool {
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

        shared::db::DatabasePool::Sqlite(pool)
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
        let inner_pool = pool.get_sqlite_pool_or_err().unwrap();
        let splits: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT recipient_id, role, amount FROM revenue_splits WHERE tx_id = ? ORDER BY role ASC"
        )
        .bind(tx_id)
        .fetch_all(inner_pool)
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
