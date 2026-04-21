use crate::db::DatabasePool;
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, sqlx::FromRow)]
pub struct QualityGateEntry {
    pub id: String,
    pub job_id: String,
    pub score: i64,
    pub passed: bool,
    pub conductor: String,
    pub created_at: String,
}

#[async_trait]
pub trait QualityGateStore: Send + Sync {
    async fn record(
        &self,
        job_id: &str,
        score: i32,
        passed: bool,
        conductor: &str,
    ) -> Result<(), AiomeError>;
    async fn list_recent(&self, limit: u32) -> Result<Vec<QualityGateEntry>, AiomeError>;
}

pub struct SqliteQualityGateStore {
    pool: DatabasePool,
}

impl SqliteQualityGateStore {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl QualityGateStore for SqliteQualityGateStore {
    async fn record(
        &self,
        job_id: &str,
        score: i32,
        passed: bool,
        conductor: &str,
    ) -> Result<(), AiomeError> {
        let q = match &self.pool {
            DatabasePool::Sqlite(_) => "INSERT INTO quality_gate_history (job_id, score, passed, conductor) VALUES (?, ?, ?, ?)",
            DatabasePool::Postgres(_) => "INSERT INTO quality_gate_history (job_id, score, passed, conductor) VALUES ($1, $2, $3, $4)",
        };

        crate::sql_exec!(
            &self.pool,
            &q,
            job_id.to_string(),
            i64::from(score),
            passed,
            conductor.to_string()
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to record quality gate history: {}", e),
        })?;

        Ok(())
    }

    async fn list_recent(&self, limit: u32) -> Result<Vec<QualityGateEntry>, AiomeError> {
        let max_limit = limit.min(100) as i64;

        let q = match &self.pool {
            DatabasePool::Sqlite(_) => "SELECT id, job_id, score, passed, conductor, created_at as \"created_at!\" FROM quality_gate_history ORDER BY created_at DESC LIMIT ?",
            DatabasePool::Postgres(_) => "SELECT id, job_id, score, passed, conductor, TO_CHAR(created_at, 'YYYY-MM-DD HH24:MI:SS') as \"created_at!\" FROM quality_gate_history ORDER BY created_at DESC LIMIT $1",
        };

        let rows =
            crate::sql_fetch_all!(&self.pool, QualityGateEntry, &q, max_limit).map_err(|e| {
                AiomeError::Infrastructure {
                    reason: format!("Failed to list quality gate history: {}", e),
                }
            })?;

        Ok(rows)
    }
}
