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
}
