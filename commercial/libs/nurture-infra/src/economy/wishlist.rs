/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use chrono::{DateTime, Utc};
use commerce_protocol::error::NurtureError;
use nurture_bridge::db::DatabasePool;
use nurture_bridge::{sql_exec, sql_fetch_all};
use uuid::Uuid;

/// Agent wishlist row (item they tried to buy but could not afford).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WishlistRow {
    pub item_id: Uuid,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct WishlistStore;

impl WishlistStore {
    /// UPSERT when a purchase attempt fails due to insufficient balance.
    pub async fn upsert(
        pool: &DatabasePool,
        agent_id: Uuid,
        item_id: Uuid,
        reason: &str,
    ) -> Result<(), NurtureError> {
        let agent_str = agent_id.to_string();
        let item_str = item_id.to_string();
        sql_exec!(
            pool,
            sqlite: "INSERT INTO nurture_wishlist (agent_id, item_id, reason, created_at) VALUES (?, ?, ?, datetime('now')) ON CONFLICT(agent_id, item_id) DO UPDATE SET reason = excluded.reason, created_at = excluded.created_at",
            pg: "INSERT INTO nurture_wishlist (agent_id, item_id, reason, created_at) VALUES ($1, $2, $3, NOW()) ON CONFLICT (agent_id, item_id) DO UPDATE SET reason = EXCLUDED.reason, created_at = EXCLUDED.created_at",
            agent_str,
            item_str,
            reason
        )
        .map_err(|e| NurtureError::Infrastructure(format!("wishlist upsert failed: {}", e)))?;
        Ok(())
    }

    pub async fn list(
        pool: &DatabasePool,
        agent_id: Uuid,
    ) -> Result<Vec<WishlistRow>, NurtureError> {
        let agent_str = agent_id.to_string();
        // LIMIT 200: transaction-history と同じ上限（無制限応答の防止）
        let rows = sql_fetch_all!(
            pool,
            (String, Option<String>, DateTime<Utc>),
            sqlite: "SELECT item_id, reason, created_at FROM nurture_wishlist WHERE agent_id = ? ORDER BY created_at DESC LIMIT 200",
            pg: "SELECT item_id, reason, created_at FROM nurture_wishlist WHERE agent_id = $1 ORDER BY created_at DESC LIMIT 200",
            agent_str
        )
        .map_err(|e| NurtureError::Infrastructure(format!("wishlist list failed: {}", e)))?;

        rows.into_iter()
            .map(|(item_id, reason, created_at)| {
                let item_id = Uuid::parse_str(&item_id).map_err(|e| {
                    NurtureError::Infrastructure(format!("invalid wishlist item_id: {}", e))
                })?;
                Ok(WishlistRow {
                    item_id,
                    reason,
                    created_at,
                })
            })
            .collect()
    }

    pub async fn remove(
        pool: &DatabasePool,
        agent_id: Uuid,
        item_id: Uuid,
    ) -> Result<(), NurtureError> {
        let agent_str = agent_id.to_string();
        let item_str = item_id.to_string();
        sql_exec!(
            pool,
            sqlite: "DELETE FROM nurture_wishlist WHERE agent_id = ? AND item_id = ?",
            pg: "DELETE FROM nurture_wishlist WHERE agent_id = $1 AND item_id = $2",
            agent_str,
            item_str
        )
        .map_err(|e| NurtureError::Infrastructure(format!("wishlist remove failed: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_pool() -> DatabasePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations/sqlite")
            .run(&pool)
            .await
            .unwrap();
        DatabasePool::Sqlite(pool)
    }

    #[tokio::test]
    async fn test_wishlist_upsert_list_remove() {
        let pool = setup_pool().await;
        let agent_id = Uuid::new_v4();
        let item_id = Uuid::new_v4();

        WishlistStore::upsert(&pool, agent_id, item_id, "insufficient_balance")
            .await
            .unwrap();

        let list = WishlistStore::list(&pool, agent_id).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].item_id, item_id);
        assert_eq!(list[0].reason.as_deref(), Some("insufficient_balance"));

        WishlistStore::remove(&pool, agent_id, item_id)
            .await
            .unwrap();
        assert!(WishlistStore::list(&pool, agent_id)
            .await
            .unwrap()
            .is_empty());
    }
}
