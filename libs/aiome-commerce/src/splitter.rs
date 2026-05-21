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
        // 入力バリデーション: 金額が0以上であること
        if total_amount < 0 {
            return Err(AiomeError::Infrastructure {
                reason: format!("total_amount must be non-negative, got {}", total_amount),
            });
        }
        // 入力バリデーション: 0.0〜1.0 の範囲 + NaN/Infinity 防御
        if !(0.0..=1.0).contains(&platform_fee_pct) {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "platform_fee_pct must be between 0.0 and 1.0, got {}",
                    platform_fee_pct
                ),
            });
        }
        // f64 パーセンテージを bps 整数に変換 (0.15 → 1500 bps)
        // 浮動小数点金額計算を完全に排除; validated range guarantees 0..=10000
        let fee_bps = (platform_fee_pct * 10000.0).round() as i64;
        let platform_amount = total_amount.saturating_mul(fee_bps) / 10000;
        let creator_amount = total_amount - platform_amount;

        // 2つの定数でクエリを定義 (Dual-Const 戦略)
        const Q_CREATOR_SQLITE: &str = "INSERT INTO revenue_splits (tx_id, recipient_id, role, amount) VALUES (?, ?, 'creator', ?)";
        const Q_CREATOR_PG: &str = "INSERT INTO revenue_splits (tx_id, recipient_id, role, amount) VALUES ($1, $2, 'creator', $3)";

        const Q_PLATFORM_SQLITE: &str = "INSERT INTO revenue_splits (tx_id, recipient_id, role, amount) VALUES (?, 'platform', 'platform', ?)";
        const Q_PLATFORM_PG: &str = "INSERT INTO revenue_splits (tx_id, recipient_id, role, amount) VALUES ($1, 'platform', 'platform', $2)";

        // クリエイターへの分配
        shared::sql_tx_exec!(
            tx,
            sqlite: Q_CREATOR_SQLITE,
            pg: Q_CREATOR_PG,
            tx_id,
            creator_id.to_string(),
            creator_amount
        )?;

        // プラットフォーム手数料の分配
        shared::sql_tx_exec!(
            tx,
            sqlite: Q_PLATFORM_SQLITE,
            pg: Q_PLATFORM_PG,
            tx_id,
            platform_amount
        )?;

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

    #[tokio::test]
    async fn test_split_revenue_rejects_negative_amount() {
        let pool = setup_db().await;
        let mut tx = pool.begin().await.unwrap();
        let result =
            RevenueSplitter::split_revenue(&mut tx, "neg_tx", -100, Uuid::new_v4(), 0.15).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("non-negative"),
            "Expected non-negative error, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_split_revenue_rejects_nan_fee() {
        let pool = setup_db().await;
        let mut tx = pool.begin().await.unwrap();
        let result =
            RevenueSplitter::split_revenue(&mut tx, "nan_tx", 1000, Uuid::new_v4(), f64::NAN).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("between 0.0 and 1.0"),
            "Expected range error for NaN, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_split_revenue_rejects_excessive_fee() {
        let pool = setup_db().await;
        let mut tx = pool.begin().await.unwrap();
        let result =
            RevenueSplitter::split_revenue(&mut tx, "big_tx", 1000, Uuid::new_v4(), 1.5).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("between 0.0 and 1.0"),
            "Expected range error for 1.5, got: {}",
            err_msg
        );
    }
}
