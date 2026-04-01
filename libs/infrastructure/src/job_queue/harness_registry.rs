/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use aiome_core::error::AiomeError;
use aiome_core_contracts::contracts::{HarnessRecord, HarnessStatus};
use aiome_core_contracts::traits::HarnessRegistryOps;
use async_trait::async_trait;
use sqlx::Row;

use crate::db::DatabasePool;
use crate::job_queue::UniversalJobQueue;

pub struct HarnessOps;

#[async_trait]
impl HarnessRegistryOps for UniversalJobQueue {
    async fn store_harness_record(&self, record: &HarnessRecord) -> Result<(), AiomeError> {
        let q = match &self.pool {
            DatabasePool::Sqlite(_) => {
                format!(
                    "INSERT OR REPLACE INTO harness_registry (id, domain, description, code_payload, status, version, agent_id, fire_count, false_positive_count, severity, created_at, last_fired_at) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, {10}, {11})",
                    self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6), self.pool.ph(7), self.pool.ph(8), self.pool.ph(9), self.pool.ph(10), self.pool.ph(11)
                )
            }
            DatabasePool::Postgres(_) => {
                format!(
                    "INSERT INTO harness_registry (id, domain, description, code_payload, status, version, agent_id, fire_count, false_positive_count, severity, created_at, last_fired_at) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, {10}, {11}) ON CONFLICT (id) DO UPDATE SET domain=EXCLUDED.domain, description=EXCLUDED.description, code_payload=EXCLUDED.code_payload, status=EXCLUDED.status, version=EXCLUDED.version, agent_id=EXCLUDED.agent_id, fire_count=EXCLUDED.fire_count, false_positive_count=EXCLUDED.false_positive_count, severity=EXCLUDED.severity, last_fired_at=EXCLUDED.last_fired_at",
                    self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6), self.pool.ph(7), self.pool.ph(8), self.pool.ph(9), self.pool.ph(10), self.pool.ph(11)
                )
            }
        };

        let agent_id_str = record.agent_id.map(|u| u.to_string());
        let fire_count = record.fire_count as i64;
        let false_positive_count = record.false_positive_count as i64;
        let severity_i32 = record.severity as i32;

        crate::sql_exec!(
            &self.pool,
            &q,
            &record.id,
            &record.domain,
            &record.description,
            &record.code_payload,
            record.status.as_str(),
            &record.version,
            &agent_id_str,
            &fire_count,
            &false_positive_count,
            &severity_i32,
            &record.created_at,
            &record.last_fired_at
        )
        .map(|_| ())
    }

    async fn fetch_harness_records_by_status(
        &self,
        status: &str,
    ) -> Result<Vec<HarnessRecord>, AiomeError> {
        let q = format!(
            "SELECT id, domain, description, code_payload, status, version, agent_id, fire_count, false_positive_count, severity, created_at, last_fired_at FROM harness_registry WHERE status = {}",
            self.pool.ph(0)
        );

        let mut out = Vec::new();
        match &self.pool {
            DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(&q)
                    .bind(status)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for row in rows {
                    let severity_i32: i32 = row.get("severity");
                    let agent_id_str: Option<String> = row.get("agent_id");
                    out.push(HarnessRecord {
                        id: row.get("id"),
                        domain: row.get("domain"),
                        description: row.get("description"),
                        code_payload: row.get("code_payload"),
                        status: HarnessStatus::from_str(row.get::<&str, _>("status")),
                        version: row.get("version"),
                        agent_id: agent_id_str.and_then(|s| uuid::Uuid::parse_str(&s).ok()),
                        fire_count: row.get::<i64, _>("fire_count") as u64,
                        false_positive_count: row.get::<i64, _>("false_positive_count") as u64,
                        severity: severity_i32 as u8,
                        created_at: row.try_get("created_at").unwrap_or_else(|_| String::new()),
                        last_fired_at: row.get("last_fired_at"),
                    });
                }
            }
            DatabasePool::Postgres(p) => {
                let rows = sqlx::query(&q)
                    .bind(status)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for row in rows {
                    let severity_i32: i32 = row.get("severity");
                    let agent_id_str: Option<String> = row.get("agent_id");
                    out.push(HarnessRecord {
                        id: row.get("id"),
                        domain: row.get("domain"),
                        description: row.get("description"),
                        code_payload: row.get("code_payload"),
                        status: HarnessStatus::from_str(row.get::<&str, _>("status")),
                        version: row.get("version"),
                        agent_id: agent_id_str.and_then(|s| uuid::Uuid::parse_str(&s).ok()),
                        fire_count: row.get::<i64, _>("fire_count") as u64,
                        false_positive_count: row.get::<i64, _>("false_positive_count") as u64,
                        severity: severity_i32 as u8,
                        created_at: row.try_get("created_at").unwrap_or_else(|_| String::new()),
                        last_fired_at: row.get("last_fired_at"),
                    });
                }
            }
        };
        Ok(out)
    }

    async fn update_harness_status(&self, id: &str, status: &str) -> Result<(), AiomeError> {
        let q = format!(
            "UPDATE harness_registry SET status = {} WHERE id = {}",
            self.pool.ph(0),
            self.pool.ph(1)
        );
        crate::sql_exec!(&self.pool, &q, status, id).map(|_| ())
    }

    async fn delete_harness_record(&self, id: &str) -> Result<(), AiomeError> {
        let q = format!(
            "DELETE FROM harness_registry WHERE id = {}",
            self.pool.ph(0)
        );
        crate::sql_exec!(&self.pool, &q, id).map(|_| ())
    }

    async fn fetch_harness_record_by_id(
        &self,
        id: &str,
    ) -> Result<Option<HarnessRecord>, AiomeError> {
        let q = format!(
            "SELECT id, domain, description, code_payload, status, version, agent_id, fire_count, false_positive_count, severity, created_at, last_fired_at FROM harness_registry WHERE id = {}",
            self.pool.ph(0)
        );

        match &self.pool {
            DatabasePool::Sqlite(p) => {
                let row = sqlx::query(&q)
                    .bind(id)
                    .fetch_optional(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                if let Some(row) = row {
                    let severity_i32: i32 = row.get("severity");
                    let agent_id_str: Option<String> = row.get("agent_id");
                    Ok(Some(HarnessRecord {
                        id: row.get("id"),
                        domain: row.get("domain"),
                        description: row.get("description"),
                        code_payload: row.get("code_payload"),
                        status: HarnessStatus::from_str(row.get::<&str, _>("status")),
                        version: row.get("version"),
                        agent_id: agent_id_str.and_then(|s| uuid::Uuid::parse_str(&s).ok()),
                        fire_count: row.get::<i64, _>("fire_count") as u64,
                        false_positive_count: row.get::<i64, _>("false_positive_count") as u64,
                        severity: severity_i32 as u8,
                        created_at: row.try_get("created_at").unwrap_or_else(|_| String::new()),
                        last_fired_at: row.get("last_fired_at"),
                    }))
                } else {
                    Ok(None)
                }
            }
            DatabasePool::Postgres(p) => {
                let row = sqlx::query(&q)
                    .bind(id)
                    .fetch_optional(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                if let Some(row) = row {
                    let severity_i32: i32 = row.get("severity");
                    let agent_id_str: Option<String> = row.get("agent_id");
                    Ok(Some(HarnessRecord {
                        id: row.get("id"),
                        domain: row.get("domain"),
                        description: row.get("description"),
                        code_payload: row.get("code_payload"),
                        status: HarnessStatus::from_str(row.get::<&str, _>("status")),
                        version: row.get("version"),
                        agent_id: agent_id_str.and_then(|s| uuid::Uuid::parse_str(&s).ok()),
                        fire_count: row.get::<i64, _>("fire_count") as u64,
                        false_positive_count: row.get::<i64, _>("false_positive_count") as u64,
                        severity: severity_i32 as u8,
                        created_at: row.try_get("created_at").unwrap_or_else(|_| String::new()),
                        last_fired_at: row.get("last_fired_at"),
                    }))
                } else {
                    Ok(None)
                }
            }
        }
    }

    async fn increment_harness_stats(&self, id: &str, fire: bool) -> Result<(), AiomeError> {
        let col = if fire {
            "fire_count"
        } else {
            "false_positive_count"
        };
        let q = format!(
            "UPDATE harness_registry SET {} = {} + 1, last_fired_at = {} WHERE id = {}",
            col,
            col,
            self.pool.ph(0),
            self.pool.ph(1)
        );
        let now = chrono::Utc::now().to_rfc3339();

        crate::sql_exec!(&self.pool, &q, &now, id).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DatabasePool;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_harness_registry_crud() {
        let sq_pool = DatabasePool::Sqlite(
            sqlx::sqlite::SqlitePoolOptions::new()
                .connect("sqlite::memory:")
                .await
                .unwrap(),
        );
        // Run migration manually (v2)
        sqlx::query("CREATE TABLE harness_registry (id TEXT PRIMARY KEY, domain TEXT NOT NULL, description TEXT NOT NULL, code_payload TEXT NOT NULL, status TEXT NOT NULL, version INTEGER NOT NULL DEFAULT 0, agent_id TEXT, fire_count BIGINT DEFAULT 0, false_positive_count BIGINT DEFAULT 0, severity INTEGER NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP, last_fired_at DATETIME)").execute(sq_pool.get_sqlite_pool().unwrap()).await.unwrap();

        let ts = Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(sq_pool.clone()),
        );
        let jq = UniversalJobQueue::from_pool(sq_pool, ts);

        let record = HarnessRecord {
            id: "harness_1".to_string(),
            domain: "security".to_string(),
            description: "Block rm -rf".to_string(),
            code_payload: "print('block')".to_string(),
            status: HarnessStatus::Shadow,
            version: 1,
            agent_id: None,
            fire_count: 0,
            false_positive_count: 0,
            severity: 90,
            created_at: "2026-03-31T00:00:00Z".to_string(),
            last_fired_at: None,
        };

        jq.store_harness_record(&record).await.unwrap();

        let fetched = jq.fetch_harness_records_by_status("Shadow").await.unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].id, "harness_1");

        jq.update_harness_status("harness_1", "Active")
            .await
            .unwrap();

        let active = jq.fetch_harness_records_by_status("Active").await.unwrap();
        assert_eq!(active.len(), 1);

        let shadow = jq.fetch_harness_records_by_status("Shadow").await.unwrap();
        assert_eq!(shadow.len(), 0);

        jq.delete_harness_record("harness_1").await.unwrap();

        let empty = jq.fetch_harness_records_by_status("Active").await.unwrap();
        assert_eq!(empty.len(), 0);
    }
}
