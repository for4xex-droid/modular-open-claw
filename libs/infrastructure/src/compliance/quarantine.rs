/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! # Quarantine — アセット検疫
//!
//! CSAM やコンプライアンス違反のアセットを永続的に記録・管理する。

use crate::db::DatabasePool;
use crate::sql_exec;
use aiome_core::error::AiomeError;
use aiome_core_contracts::contracts::QuarantinedAsset;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

/// 検疫理由
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetReason {
    /// CSAM 知覚ハッシュヒット
    CsamHit,
    /// 頭身制限違反 (5.5頭身ルール)
    RestrictedProportions,
    /// eKYC 未完了
    EkycFailed,
    /// その他
    Other(String),
}

/// 検疫ステータス
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuarantineStatus {
    /// 検疫中
    Quarantined,
    /// 承認済み（解放）
    Released,
    /// 削除済み
    Deleted,
}

/// アセット検疫の永続化インターフェース
#[async_trait]
pub trait QuarantineStore: Send + Sync {
    /// アセットを検疫所に送る
    async fn quarantine_asset(
        &self,
        asset_name: &str,
        image_hash: &str,
        reason: AssetReason,
    ) -> anyhow::Result<String>;
    /// 検疫済みアセットのチェック
    async fn is_quarantined(&self, image_hash: &str) -> anyhow::Result<bool>;
    /// アセットの解放（検疫解除）
    async fn release_asset(&self, id: &str) -> anyhow::Result<()>;
    /// 検疫アセットの一覧取得
    async fn list_assets(&self) -> anyhow::Result<Vec<QuarantinedAsset>>;
}

/// Universal (SQLite/PostgreSQL) implementation for QuarantineStore
pub struct UniversalQuarantineStore {
    pool: DatabasePool,
}

impl UniversalQuarantineStore {
    /// 新規作成
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl QuarantineStore for UniversalQuarantineStore {
    async fn quarantine_asset(
        &self,
        asset_name: &str,
        image_hash: &str,
        reason: AssetReason,
    ) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let reason_str = serde_json::to_string(&reason)?;
        let status = serde_json::to_string(&QuarantineStatus::Quarantined)?;

        let q = format!(
            "INSERT INTO quarantined_assets (id, asset_name, image_hash, reason, status) VALUES ({0}, {1}, {2}, {3}, {4})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4)
        );

        sql_exec!(&self.pool, &q, &id, asset_name, image_hash, reason_str, status)
            .map_err(|e| anyhow::anyhow!(e))?;

        info!(
            "🛡️ [Quarantine] Asset quarantined: {} (Hash: {})",
            asset_name, image_hash
        );
        Ok(id)
    }

    async fn is_quarantined(&self, image_hash: &str) -> anyhow::Result<bool> {
        let q = format!(
            "SELECT id FROM quarantined_assets WHERE image_hash = {} AND status = {}",
            self.pool.ph(0),
            self.pool.ph(1)
        );
        let status = serde_json::to_string(&QuarantineStatus::Quarantined)?;

        let res: Option<String> = match &self.pool {
            DatabasePool::Sqlite(p) => {
                sqlx::query_scalar(&q)
                    .bind(image_hash)
                    .bind(&status)
                    .fetch_optional(p)
                    .await?
            }
            DatabasePool::Postgres(p) => {
                sqlx::query_scalar(&q)
                    .bind(image_hash)
                    .bind(&status)
                    .fetch_optional(p)
                    .await?
            }
        };

        Ok(res.is_some())
    }

    async fn release_asset(&self, id: &str) -> anyhow::Result<()> {
        let status = serde_json::to_string(&QuarantineStatus::Released)?;
        let q = format!(
            "UPDATE quarantined_assets SET status = {} WHERE id = {}",
            self.pool.ph(0),
            self.pool.ph(1)
        );

        sql_exec!(&self.pool, &q, status, id).map_err(|e| anyhow::anyhow!(e))?;

        info!("🔓 [Quarantine] Asset released: {}", id);
        Ok(())
    }

    async fn list_assets(&self) -> anyhow::Result<Vec<QuarantinedAsset>> {
        use sqlx::Row;
        let q = "SELECT id, asset_name, image_hash, reason, status, uploaded_at FROM quarantined_assets ORDER BY uploaded_at DESC";

        match &self.pool {
            DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(q).fetch_all(p).await?;
                let assets = rows
                    .into_iter()
                    .map(|row| QuarantinedAsset {
                        id: row.get("id"),
                        asset_name: row.get("asset_name"),
                        image_hash: row.get("image_hash"),
                        reason: row.get("reason"),
                        status: row.get("status"),
                        uploaded_at: row.get("uploaded_at"),
                    })
                    .collect();
                Ok(assets)
            }
            DatabasePool::Postgres(p) => {
                let rows = sqlx::query(q).fetch_all(p).await?;
                let assets = rows
                    .into_iter()
                    .map(|row| QuarantinedAsset {
                        id: row.get("id"),
                        asset_name: row.get("asset_name"),
                        image_hash: row.get("image_hash"),
                        reason: row.get("reason"),
                        status: row.get("status"),
                        uploaded_at: row.get("uploaded_at"),
                    })
                    .collect();
                Ok(assets)
            }
        }
    }
}

/// テスト用モック
#[cfg(any(test, debug_assertions))]
pub struct MockQuarantineStore;

#[cfg(any(test, debug_assertions))]
#[async_trait]
impl QuarantineStore for MockQuarantineStore {
    async fn quarantine_asset(
        &self,
        _asset_name: &str,
        _image_hash: &str,
        _reason: AssetReason,
    ) -> anyhow::Result<String> {
        Ok("mock-quarantine-id".to_string())
    }

    async fn is_quarantined(&self, _image_hash: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    async fn release_asset(&self, _id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn list_assets(&self) -> anyhow::Result<Vec<QuarantinedAsset>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql_exec;

    async fn setup_db() -> DatabasePool {
        let pool = DatabasePool::new_sqlite(":memory:").await.unwrap();
        let schema = "CREATE TABLE quarantined_assets (
            id TEXT PRIMARY KEY,
            asset_name TEXT,
            image_hash TEXT,
            reason TEXT,
            status TEXT,
            uploaded_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )";
        sql_exec!(&pool, schema).unwrap();
        pool
    }

    #[tokio::test]
    async fn test_quarantine_flow() {
        let pool = setup_db().await;
        let store = UniversalQuarantineStore::new(pool);

        let hash = "hash_123";
        // 1. Initially not quarantined
        assert!(!store.is_quarantined(hash).await.unwrap());

        // 2. Quarantine asset
        let id = store
            .quarantine_asset("bad_image.png", hash, AssetReason::CsamHit)
            .await
            .unwrap();

        // 3. Now it is quarantined
        assert!(store.is_quarantined(hash).await.unwrap());

        // 4. Release asset
        store.release_asset(&id).await.unwrap();

        // 5. No longer quarantined
        assert!(!store.is_quarantined(hash).await.unwrap());
    }
}
