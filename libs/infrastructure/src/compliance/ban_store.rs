/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! # BanStore — アカウントBANおよびガバナンス
//!
//! CSAM やその他の重大なポリシー違反によるアカウントのBAN状態を記録・管理する。

use crate::db::DatabasePool;
use crate::sql_exec;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};

/// BANレコード
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BanRecord {
    pub actor_id: uuid::Uuid,
    pub reason: String,
    pub severity: String,
    pub banned_by: String,
    pub banned_at: String,
    pub expires_at: Option<String>,
    pub unbanned_at: Option<String>,
}

/// BAN状態の永続化インターフェース
#[async_trait]
pub trait BanStore: Send + Sync {
    /// データベーステーブルの初期化 (自動スキーマ適用 - Self-Healing Architecture)
    async fn init(&self) -> anyhow::Result<()>;
    /// 指定されたアクターがBANされているかどうかを検証
    async fn is_banned(&self, actor_id: &uuid::Uuid) -> anyhow::Result<bool>;
    /// アクターをBANする
    async fn ban(
        &self,
        actor_id: &uuid::Uuid,
        reason: &str,
        severity: &str,
        banned_by: &str,
    ) -> anyhow::Result<()>;
    /// アクターのBANを解除する
    async fn unban(&self, actor_id: &uuid::Uuid) -> anyhow::Result<()>;
    /// BANレコードの一覧を取得
    async fn list_bans(&self) -> anyhow::Result<Vec<BanRecord>>;
    /// トレイトオブジェクトから具体的な型へのダウンキャストをサポート
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Universal (SQLite/PostgreSQL) implementation for BanStore
pub struct UniversalBanStore {
    pool: DatabasePool,
}

impl UniversalBanStore {
    /// 新規作成
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BanStore for UniversalBanStore {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn init(&self) -> anyhow::Result<()> {
        let q = "CREATE TABLE IF NOT EXISTS nurture_bans (
            actor_id    TEXT PRIMARY KEY,
            reason      TEXT NOT NULL,
            severity    TEXT NOT NULL,
            banned_by   TEXT NOT NULL,
            banned_at   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            expires_at  TIMESTAMP,
            unbanned_at TIMESTAMP
        );";

        sql_exec!(&self.pool, q)
            .map_err(|e| anyhow::anyhow!("Failed to initialize nurture_bans table: {}", e))?;
        info!("🛡️ [BanStore] Self-healing compliance schema (nurture_bans) ensured successfully.");
        Ok(())
    }

    async fn is_banned(&self, actor_id: &uuid::Uuid) -> anyhow::Result<bool> {
        let actor_str = actor_id.to_string();
        // expires_at が NULL（永久BAN）または未来の場合のみ有効。期限切れBANは自動解除扱い。
        let q = format!(
            "SELECT actor_id FROM nurture_bans WHERE actor_id = {} AND unbanned_at IS NULL AND (expires_at IS NULL OR expires_at > CURRENT_TIMESTAMP)",
            self.pool.ph(0)
        );

        let res: Option<String> = match &self.pool {
            DatabasePool::Sqlite(p) => {
                sqlx::query_scalar(&q)
                    .bind(&actor_str)
                    .fetch_optional(p)
                    .await?
            }
            DatabasePool::Postgres(p) => {
                sqlx::query_scalar(&q)
                    .bind(&actor_str)
                    .fetch_optional(p)
                    .await?
            }
        };

        Ok(res.is_some())
    }

    async fn ban(
        &self,
        actor_id: &uuid::Uuid,
        reason: &str,
        severity: &str,
        banned_by: &str,
    ) -> anyhow::Result<()> {
        let actor_str = actor_id.to_string();
        // 冪等 UPSERT: 既にBANされているアクターに再度BANを適用してもエラーにならない。
        // unbanned_at を NULL にリセットすることで、解除後の再BAN にも対応。
        let q = match &self.pool {
            DatabasePool::Sqlite(_) => format!(
                "INSERT INTO nurture_bans (actor_id, reason, severity, banned_by) VALUES ({0}, {1}, {2}, {3}) \
                 ON CONFLICT(actor_id) DO UPDATE SET reason = {1}, severity = {2}, banned_by = {3}, banned_at = CURRENT_TIMESTAMP, unbanned_at = NULL",
                self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3)
            ),
            DatabasePool::Postgres(_) => format!(
                "INSERT INTO nurture_bans (actor_id, reason, severity, banned_by) VALUES ({0}, {1}, {2}, {3}) \
                 ON CONFLICT (actor_id) DO UPDATE SET reason = EXCLUDED.reason, severity = EXCLUDED.severity, banned_by = EXCLUDED.banned_by, banned_at = CURRENT_TIMESTAMP, unbanned_at = NULL",
                self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3)
            ),
        };

        sql_exec!(&self.pool, &q, &actor_str, reason, severity, banned_by)
            .map_err(|e| anyhow::anyhow!(e))?;

        info!(
            "🚫 [BanStore] Agent banned: {} (Reason: {}, Severity: {})",
            actor_str, reason, severity
        );
        Ok(())
    }

    async fn unban(&self, actor_id: &uuid::Uuid) -> anyhow::Result<()> {
        let actor_str = actor_id.to_string();
        let q = format!(
            "UPDATE nurture_bans SET unbanned_at = CURRENT_TIMESTAMP WHERE actor_id = {} AND unbanned_at IS NULL",
            self.pool.ph(0)
        );

        sql_exec!(&self.pool, &q, &actor_str).map_err(|e| anyhow::anyhow!(e))?;

        info!("🔓 [BanStore] Agent unbanned: {}", actor_str);
        Ok(())
    }

    async fn list_bans(&self) -> anyhow::Result<Vec<BanRecord>> {
        use sqlx::Row;
        let q = "SELECT actor_id, reason, severity, banned_by, banned_at, expires_at, unbanned_at FROM nurture_bans ORDER BY banned_at DESC";

        // NOTE: SQLite/Postgres で行パース処理が同一だが、sqlx の型制約上
        // SqliteRow と PgRow は異なる型のためクロージャ共通化が不可能。
        // sqlx::any::AnyRow への統一は DatabasePool の設計方針と不整合になるため現状維持。
        match &self.pool {
            DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(q).fetch_all(p).await?;
                rows.iter()
                    .map(|r| {
                        let actor_str: String = r.get("actor_id");
                        let actor_id = uuid::Uuid::parse_str(&actor_str)
                            .map_err(|_| anyhow::anyhow!("Invalid UUID format in database"))?;
                        Ok(BanRecord {
                            actor_id,
                            reason: r.get("reason"),
                            severity: r.get("severity"),
                            banned_by: r.get("banned_by"),
                            banned_at: r.get("banned_at"),
                            expires_at: r.try_get("expires_at").ok(),
                            unbanned_at: r.try_get("unbanned_at").ok(),
                        })
                    })
                    .collect()
            }
            DatabasePool::Postgres(p) => {
                let rows = sqlx::query(q).fetch_all(p).await?;
                rows.iter()
                    .map(|r| {
                        let actor_str: String = r.get("actor_id");
                        let actor_id = uuid::Uuid::parse_str(&actor_str)
                            .map_err(|_| anyhow::anyhow!("Invalid UUID format in database"))?;
                        Ok(BanRecord {
                            actor_id,
                            reason: r.get("reason"),
                            severity: r.get("severity"),
                            banned_by: r.get("banned_by"),
                            banned_at: r.get("banned_at"),
                            expires_at: r.try_get("expires_at").ok(),
                            unbanned_at: r.try_get("unbanned_at").ok(),
                        })
                    })
                    .collect()
            }
        }
    }
}

/// テスト用モック
#[derive(Clone)]
pub struct MockBanStore {
    banned_agents: Arc<tokio::sync::RwLock<std::collections::HashSet<uuid::Uuid>>>,
    pub should_fail: Arc<std::sync::atomic::AtomicBool>,
}

impl Default for MockBanStore {
    fn default() -> Self {
        Self {
            banned_agents: Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
            should_fail: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

impl MockBanStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// テスト用に手動でBAN状態にする
    pub async fn set_banned(&self, actor_id: uuid::Uuid, is_banned: bool) {
        let mut lock = self.banned_agents.write().await;
        if is_banned {
            lock.insert(actor_id);
        } else {
            lock.remove(&actor_id);
        }
    }

    /// テスト用にエラー発生状態にする
    pub fn set_should_fail(&self, fail: bool) {
        self.should_fail
            .store(fail, std::sync::atomic::Ordering::SeqCst);
    }
}

#[async_trait]
impl BanStore for MockBanStore {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn init(&self) -> anyhow::Result<()> {
        if self.should_fail.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Mock initialization failure"));
        }
        Ok(())
    }

    async fn is_banned(&self, actor_id: &uuid::Uuid) -> anyhow::Result<bool> {
        if self.should_fail.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Mock DB connection failure"));
        }
        let lock = self.banned_agents.read().await;
        Ok(lock.contains(actor_id))
    }

    async fn ban(
        &self,
        actor_id: &uuid::Uuid,
        _reason: &str,
        _severity: &str,
        _banned_by: &str,
    ) -> anyhow::Result<()> {
        if self.should_fail.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Mock DB connection failure"));
        }
        let mut lock = self.banned_agents.write().await;
        lock.insert(*actor_id);
        Ok(())
    }

    async fn unban(&self, actor_id: &uuid::Uuid) -> anyhow::Result<()> {
        if self.should_fail.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Mock DB connection failure"));
        }
        let mut lock = self.banned_agents.write().await;
        lock.remove(actor_id);
        Ok(())
    }

    async fn list_bans(&self) -> anyhow::Result<Vec<BanRecord>> {
        let lock = self.banned_agents.read().await;
        let mut records = Vec::new();
        for agent in lock.iter() {
            records.push(BanRecord {
                actor_id: *agent,
                reason: "Mock ban".to_string(),
                severity: "CRITICAL".to_string(),
                banned_by: "System".to_string(),
                banned_at: "2026-05-20T00:00:00Z".to_string(),
                expires_at: None,
                unbanned_at: None,
            });
        }
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ban_store_flow() {
        let pool = DatabasePool::new_sqlite(":memory:").await.unwrap();
        let store = UniversalBanStore::new(pool);
        store.init().await.unwrap();

        let actor = uuid::Uuid::new_v4();
        // 1. Not banned initially
        assert!(!store.is_banned(&actor).await.unwrap());

        // 2. Ban actor
        store
            .ban(&actor, "CSAM Policy", "CRITICAL", "admin")
            .await
            .unwrap();

        // 3. Now banned
        assert!(store.is_banned(&actor).await.unwrap());

        // 4. Unban actor
        store.unban(&actor).await.unwrap();

        // 5. No longer banned
        assert!(!store.is_banned(&actor).await.unwrap());
    }
}
