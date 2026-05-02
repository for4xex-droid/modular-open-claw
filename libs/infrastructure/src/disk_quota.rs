/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::db::DatabasePool;
use aiome_core::error::AiomeError;
use uuid::Uuid;

/// Phase 1D-2: Agent Disk Quota Management
/// Prevents OOM/Storage-Exhaustion attacks by tracking upload sizes per agent.
#[derive(Clone)]
pub struct DiskQuotaManager {
    pool: DatabasePool,
    default_quota_bytes: u64,
}

impl DiskQuotaManager {
    /// Create a new DiskQuotaManager instance
    pub fn new(pool: DatabasePool, default_quota_bytes: u64) -> Self {
        Self {
            pool,
            default_quota_bytes,
        }
    }

    /// Initialize the disk_quota schema if it does not exist
    pub async fn init(&self) -> Result<(), AiomeError> {
        let q = r#"
            CREATE TABLE IF NOT EXISTS disk_quota (
                agent_id TEXT PRIMARY KEY,
                used_bytes INTEGER NOT NULL DEFAULT 0
            )
        "#;
        crate::sql_exec!(&self.pool, q)?;
        Ok(())
    }

    /// Retrieve the current storage usage for an agent in bytes
    pub async fn get_usage(&self, agent_id: Uuid) -> Result<u64, AiomeError> {
        let q = format!(
            "SELECT used_bytes FROM disk_quota WHERE agent_id = {}",
            self.pool.ph(0)
        );
        let row_opt: Option<(i64,)> = match &self.pool {
            DatabasePool::Sqlite(p) => sqlx::query_as(&q)
                .bind(agent_id.to_string())
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
            DatabasePool::Postgres(p) => sqlx::query_as(&q)
                .bind(agent_id.to_string())
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
        };
        Ok(row_opt.map(|r| r.0 as u64).unwrap_or(0))
    }

    /// Check if the agent can upload an additional amount of bytes
    pub async fn check_quota(
        &self,
        agent_id: Uuid,
        additional_bytes: u64,
    ) -> Result<(), AiomeError> {
        let current_usage = self.get_usage(agent_id).await?;
        if current_usage + additional_bytes > self.default_quota_bytes {
            tracing::warn!(
                "🛡️ [DiskQuota] Blocked upload for agent {}: size {} + {} > limit {}",
                agent_id,
                current_usage,
                additional_bytes,
                self.default_quota_bytes
            );
            return Err(AiomeError::ResourceBusy {
                reason: format!(
                    "Disk quota exceeded: limit {} bytes",
                    self.default_quota_bytes
                ),
            });
        }
        Ok(())
    }

    /// Record newly utilized storage capacity usage for an agent
    pub async fn record_usage(
        &self,
        agent_id: Uuid,
        additional_bytes: u64,
    ) -> Result<(), AiomeError> {
        // Platform agnostic UPSERT abstraction
        // Note: For SQLite, UPSERT requires 3.24+, which sqlx typically targets.
        let q = format!(
            "INSERT INTO disk_quota (agent_id, used_bytes) VALUES ({0}, {1}) ON CONFLICT(agent_id) DO UPDATE SET used_bytes = used_bytes + {1}",
            self.pool.ph(0), self.pool.ph(1)
        );
        match &self.pool {
            DatabasePool::Sqlite(p) => {
                sqlx::query(&q)
                    .bind(agent_id.to_string())
                    .bind(additional_bytes as i64)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
            DatabasePool::Postgres(p) => {
                sqlx::query(&q)
                    .bind(agent_id.to_string())
                    .bind(additional_bytes as i64)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
        }
        tracing::debug!(
            "📊 [DiskQuota] Recorded {} bytes for agent {}",
            additional_bytes,
            agent_id
        );
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_db() -> DatabasePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap(); // allow-anti-pattern
        DatabasePool::Sqlite(pool)
    }

    #[tokio::test]
    async fn test_disk_quota_enforcement() {
        let pool = setup_db().await;
        // 1MB Quota
        let manager = DiskQuotaManager::new(pool, 1_000_000);
        manager.init().await.unwrap(); // allow-anti-pattern

        let agent_id = Uuid::new_v4();

        // Initial usage should be 0
        assert_eq!(manager.get_usage(agent_id).await.unwrap(), 0); // allow-anti-pattern

        // Check quota 500k -> OK
        assert!(manager.check_quota(agent_id, 500_000).await.is_ok());

        // Record 500k
        manager.record_usage(agent_id, 500_000).await.unwrap(); // allow-anti-pattern
        assert_eq!(manager.get_usage(agent_id).await.unwrap(), 500_000); // allow-anti-pattern

        // Check quota 600k -> Error!
        let res = manager.check_quota(agent_id, 600_000).await;
        assert!(res.is_err());
        if let Err(AiomeError::ResourceBusy { reason }) = res {
            assert!(reason.contains("exceeded"));
        } else {
            panic!("Expected ResourceBusy error");
        }
    }
}
