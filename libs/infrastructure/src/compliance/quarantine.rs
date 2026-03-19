//! # Quarantine — アセット検疫
//!
//! CSAM やコンプライアンス違反のアセットを永続的に記録・管理する。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
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
}

/// SQLite を使用した検疫ストア実装
pub struct SqliteQuarantineStore {
    pool: SqlitePool,
}

impl SqliteQuarantineStore {
    /// 新規作成とテーブル初期化
    pub async fn new(pool: SqlitePool) -> anyhow::Result<Self> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS quarantined_assets (
                id TEXT PRIMARY KEY,
                asset_name TEXT NOT NULL,
                image_hash TEXT NOT NULL,
                reason TEXT NOT NULL,
                status TEXT NOT NULL,
                uploaded_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl QuarantineStore for SqliteQuarantineStore {
    async fn quarantine_asset(
        &self,
        asset_name: &str,
        image_hash: &str,
        reason: AssetReason,
    ) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let reason_str = serde_json::to_string(&reason)?;
        let status = serde_json::to_string(&QuarantineStatus::Quarantined)?;

        sqlx::query(
            "INSERT INTO quarantined_assets (id, asset_name, image_hash, reason, status)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(asset_name)
        .bind(image_hash)
        .bind(reason_str)
        .bind(status)
        .execute(&self.pool)
        .await?;

        info!(
            "🛡️ [Quarantine] Asset quarantined: {} (Hash: {})",
            asset_name, image_hash
        );
        Ok(id)
    }

    async fn is_quarantined(&self, image_hash: &str) -> anyhow::Result<bool> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT id FROM quarantined_assets WHERE image_hash = ? AND status = ?")
                .bind(image_hash)
                .bind(serde_json::to_string(&QuarantineStatus::Quarantined)?)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.is_some())
    }

    async fn release_asset(&self, id: &str) -> anyhow::Result<()> {
        let status = serde_json::to_string(&QuarantineStatus::Released)?;
        sqlx::query("UPDATE quarantined_assets SET status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await?;

        info!("🔓 [Quarantine] Asset released: {}", id);
        Ok(())
    }
}

/// テスト用モック
pub struct MockQuarantineStore;

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn test_quarantine_flow() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let store = SqliteQuarantineStore::new(pool).await.unwrap();

        let hash = "fake-hash-123";
        let id = store
            .quarantine_asset("test-asset", hash, AssetReason::CsamHit)
            .await
            .unwrap();

        assert!(store.is_quarantined(hash).await.unwrap());

        store.release_asset(&id).await.unwrap();
        assert!(!store.is_quarantined(hash).await.unwrap());
    }
}
