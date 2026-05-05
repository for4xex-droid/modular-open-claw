/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use crate::db::DatabasePool;
use aiome_core::error::AiomeError;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use super::types::{IncidentRecord, IncidentStatus, WeeklyStats};

#[derive(Clone)]
pub struct IncidentRepository {
    pool: DatabasePool,
}

impl IncidentRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    pub async fn insert_incident(
        &self,
        skill_name: &str,
        wasm_hash: &str,
        input_payload: &str,
        stack_trace: &str,
    ) -> Result<String, AiomeError> {
        let id = Uuid::new_v4().to_string();
        let query = match &self.pool {
            DatabasePool::Sqlite(_) => {
                r#"
                INSERT INTO aegis_incidents (id, skill_name, wasm_hash, input_payload, stack_trace, status)
                VALUES ($1, $2, $3, $4, $5, 'Open')
                "#
            }
            DatabasePool::Postgres(_) => {
                r#"
                INSERT INTO aegis_incidents (id, skill_name, wasm_hash, input_payload, stack_trace, status)
                VALUES ($1, $2, $3, $4, $5, 'Open')
                "#
            }
        };

        match &self.pool {
            DatabasePool::Sqlite(p) => {
                sqlx::query(query)
                    .bind(&id)
                    .bind(skill_name)
                    .bind(wasm_hash)
                    .bind(input_payload)
                    .bind(stack_trace)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to insert incident: {}", e),
                    })?;
            }
            DatabasePool::Postgres(p) => {
                sqlx::query(query)
                    .bind(&id)
                    .bind(skill_name)
                    .bind(wasm_hash)
                    .bind(input_payload)
                    .bind(stack_trace)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to insert incident: {}", e),
                    })?;
            }
        }

        Ok(id)
    }

    pub async fn compute_weekly_stats(&self) -> Result<WeeklyStats, AiomeError> {
        let (total, distinct, unresolved, top_skill) = match &self.pool {
            DatabasePool::Sqlite(p) => {
                let row = sqlx::query(
                    r#"
                    SELECT
                        (SELECT COUNT(*) FROM aegis_incidents WHERE created_at >= datetime('now', '-7 days')) as total,
                        (SELECT COUNT(DISTINCT skill_name) FROM aegis_incidents WHERE created_at >= datetime('now', '-7 days')) as distinct_skills,
                        (SELECT COUNT(*) FROM aegis_incidents WHERE status IN ('Open', 'Analyzing', 'PatchGenerated', 'KaniVerifying')) as unresolved,
                        (SELECT skill_name FROM aegis_incidents WHERE created_at >= datetime('now', '-7 days') GROUP BY skill_name ORDER BY COUNT(*) DESC LIMIT 1) as top_skill
                    "#
                )
                .fetch_one(p)
                .await
                .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

                (
                    row.try_get::<i64, _>("total").unwrap_or(0),
                    row.try_get::<i64, _>("distinct_skills").unwrap_or(0),
                    row.try_get::<i64, _>("unresolved").unwrap_or(0),
                    row.try_get::<Option<String>, _>("top_skill")
                        .unwrap_or(None),
                )
            }
            DatabasePool::Postgres(p) => {
                let row = sqlx::query(
                    r#"
                    SELECT
                        (SELECT COUNT(*) FROM aegis_incidents WHERE created_at >= NOW() - INTERVAL '7 days') as total,
                        (SELECT COUNT(DISTINCT skill_name) FROM aegis_incidents WHERE created_at >= NOW() - INTERVAL '7 days') as distinct_skills,
                        (SELECT COUNT(*) FROM aegis_incidents WHERE status IN ('Open', 'Analyzing', 'PatchGenerated', 'KaniVerifying')) as unresolved,
                        (SELECT skill_name FROM aegis_incidents WHERE created_at >= NOW() - INTERVAL '7 days' GROUP BY skill_name ORDER BY COUNT(*) DESC LIMIT 1) as top_skill
                    "#
                )
                .fetch_one(p)
                .await
                .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

                (
                    row.try_get::<i64, _>("total").unwrap_or(0),
                    row.try_get::<i64, _>("distinct_skills").unwrap_or(0),
                    row.try_get::<i64, _>("unresolved").unwrap_or(0),
                    row.try_get::<Option<String>, _>("top_skill")
                        .unwrap_or(None),
                )
            }
        };

        Ok(WeeklyStats {
            total_incidents_7d: total,
            distinct_skills: distinct,
            unresolved,
            top_failing_skill: top_skill,
        })
    }

    pub async fn fetch_incident(&self, id: &str) -> Result<Option<IncidentRecord>, AiomeError> {
        let query = "SELECT * FROM aegis_incidents WHERE id = $1";
        let row_opt = match &self.pool {
            DatabasePool::Sqlite(p) => sqlx::query(query)
                .bind(id)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?
                .map(|r| self.map_row_sqlite(r)),
            DatabasePool::Postgres(p) => sqlx::query(query)
                .bind(id)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?
                .map(|r| self.map_row_postgres(r)),
        };

        match row_opt {
            Some(res) => Ok(Some(res?)),
            None => Ok(None),
        }
    }

    pub async fn fetch_open_incidents(
        &self,
        limit: i64,
    ) -> Result<Vec<IncidentRecord>, AiomeError> {
        let query =
            "SELECT * FROM aegis_incidents WHERE status = 'Open' ORDER BY created_at ASC LIMIT $1";
        let mut incidents = Vec::new();
        match &self.pool {
            DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(query)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    incidents.push(self.map_row_sqlite(r)?);
                }
            }
            DatabasePool::Postgres(p) => {
                let rows = sqlx::query(query)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    incidents.push(self.map_row_postgres(r)?);
                }
            }
        }
        Ok(incidents)
    }

    pub async fn update_status(&self, id: &str, status: IncidentStatus) -> Result<(), AiomeError> {
        let status_str = status.to_string();
        let query =
            "UPDATE aegis_incidents SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2";
        match &self.pool {
            DatabasePool::Sqlite(p) => {
                sqlx::query(query)
                    .bind(&status_str)
                    .bind(id)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
            DatabasePool::Postgres(p) => {
                sqlx::query(query)
                    .bind(&status_str)
                    .bind(id)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
        }
        Ok(())
    }

    pub async fn increment_retry_count(&self, id: &str) -> Result<(), AiomeError> {
        let query = "UPDATE aegis_incidents SET retry_count = retry_count + 1, updated_at = CURRENT_TIMESTAMP WHERE id = $1";
        match &self.pool {
            DatabasePool::Sqlite(p) => {
                sqlx::query(query).bind(id).execute(p).await.map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
            }
            DatabasePool::Postgres(p) => {
                sqlx::query(query).bind(id).execute(p).await.map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
            }
        }
        Ok(())
    }

    // Helper to map DB row to IncidentRecord
    fn map_row_sqlite(&self, row: sqlx::sqlite::SqliteRow) -> Result<IncidentRecord, AiomeError> {
        let status_str: String = row.try_get("status").unwrap_or_else(|_| "Open".to_string());
        let status = status_str.parse().unwrap_or(IncidentStatus::Open);
        let retry_count = row.try_get::<u32, _>("retry_count").unwrap_or(0);

        Ok(IncidentRecord {
            id: row.try_get("id").unwrap_or_default(),
            skill_name: row.try_get("skill_name").unwrap_or_default(),
            wasm_hash: row.try_get("wasm_hash").unwrap_or_default(),
            input_payload: row.try_get("input_payload").unwrap_or_default(),
            stack_trace: row.try_get("stack_trace").unwrap_or_default(),
            status,
            retry_count,
            created_at: row
                .try_get("created_at")
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: row
                .try_get("updated_at")
                .unwrap_or_else(|_| chrono::Utc::now()),
        })
    }

    fn map_row_postgres(&self, row: sqlx::postgres::PgRow) -> Result<IncidentRecord, AiomeError> {
        let status_str: String = row.try_get("status").unwrap_or_else(|_| "Open".to_string());
        let status = status_str.parse().unwrap_or(IncidentStatus::Open);
        let retry_count = row.try_get::<i32, _>("retry_count").unwrap_or(0) as u32;

        Ok(IncidentRecord {
            id: row.try_get("id").unwrap_or_default(),
            skill_name: row.try_get("skill_name").unwrap_or_default(),
            wasm_hash: row.try_get("wasm_hash").unwrap_or_default(),
            input_payload: row.try_get("input_payload").unwrap_or_default(),
            stack_trace: row.try_get("stack_trace").unwrap_or_default(),
            status,
            retry_count,
            created_at: row
                .try_get("created_at")
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: row
                .try_get("updated_at")
                .unwrap_or_else(|_| chrono::Utc::now()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> DatabasePool {
        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();

        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let _jq = crate::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
            .await
            .expect("Failed to create in-memory job queue");

        pool
    }

    #[tokio::test]
    async fn test_insert_incident() -> Result<(), AiomeError> {
        let pool = setup_db().await;
        let repo = IncidentRepository::new(pool);

        let id = repo
            .insert_incident("test_skill", "hash123", "{}", "panic!")
            .await?;
        assert!(!id.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_compute_weekly_stats() -> Result<(), AiomeError> {
        let pool = setup_db().await;
        let repo = IncidentRepository::new(pool);

        // Insert some
        let _ = repo.insert_incident("skill_A", "h1", "{}", "err1").await?;
        let _ = repo.insert_incident("skill_B", "h2", "{}", "err2").await?;
        let _ = repo.insert_incident("skill_A", "h3", "{}", "err3").await?;

        let stats = repo.compute_weekly_stats().await?;
        assert_eq!(stats.total_incidents_7d, 3);
        assert_eq!(stats.distinct_skills, 2);
        assert_eq!(stats.unresolved, 3);
        assert_eq!(stats.top_failing_skill.as_deref(), Some("skill_A"));

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_and_update_incidents() -> Result<(), AiomeError> {
        let pool = setup_db().await;
        let repo = IncidentRepository::new(pool);

        repo.insert_incident("skill_A", "hash1", "payload", "trace_1")
            .await?;
        repo.insert_incident("skill_B", "hash2", "payload", "trace_2")
            .await?;

        // Test fetching open incidents
        let open_incidents = repo.fetch_open_incidents(10).await?;
        assert_eq!(open_incidents.len(), 2);

        let target_id = &open_incidents[0].id;

        // Test update status and retry count
        repo.update_status(target_id, IncidentStatus::KaniVerifying)
            .await?;
        repo.increment_retry_count(target_id).await?;

        let fetched = repo.fetch_incident(target_id).await?.unwrap();
        assert_eq!(fetched.status, IncidentStatus::KaniVerifying);
        assert_eq!(fetched.retry_count, 1);

        Ok(())
    }
}
