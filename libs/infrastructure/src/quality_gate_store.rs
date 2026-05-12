/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

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
    pub entropy_score: Option<f64>,
    pub retry_count: Option<i32>,
    pub details: Option<String>,
}

#[async_trait]
pub trait QualityGateStore: Send + Sync {
    async fn record(
        &self,
        job_id: &str,
        score: i32,
        passed: bool,
        conductor: &str,
        entropy_score: Option<f64>,
        retry_count: Option<i32>,
        details: Option<&str>,
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
        entropy_score: Option<f64>,
        retry_count: Option<i32>,
        details: Option<&str>,
    ) -> Result<(), AiomeError> {
        let q = match &self.pool {
            DatabasePool::Sqlite(_) => "INSERT INTO quality_gate_history (job_id, score, passed, conductor, entropy_score, retry_count, details) VALUES (?, ?, ?, ?, ?, ?, ?)",
            DatabasePool::Postgres(_) => "INSERT INTO quality_gate_history (job_id, score, passed, conductor, entropy_score, retry_count, details) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        };

        crate::sql_exec!(
            &self.pool,
            &q,
            job_id.to_string(),
            i64::from(score),
            passed,
            conductor.to_string(),
            entropy_score,
            retry_count,
            details.map(|s| s.to_string())
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to record quality gate history: {}", e),
        })?;

        Ok(())
    }

    async fn list_recent(&self, limit: u32) -> Result<Vec<QualityGateEntry>, AiomeError> {
        let max_limit = limit.min(100) as i64;

        let q = match &self.pool {
            DatabasePool::Sqlite(_) => "SELECT id, job_id, score, passed, conductor, created_at, entropy_score, retry_count, details FROM quality_gate_history ORDER BY created_at DESC LIMIT ?",
            DatabasePool::Postgres(_) => "SELECT id, job_id, score, passed, conductor, TO_CHAR(created_at, 'YYYY-MM-DD HH24:MI:SS') as created_at, entropy_score, retry_count, details FROM quality_gate_history ORDER BY created_at DESC LIMIT $1",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DatabasePool;

    #[tokio::test]
    async fn test_quality_gate_store_with_entropy() {
        let pool = DatabasePool::new_sqlite("sqlite::memory:").await.unwrap();
        let sqlite_pool = pool.get_sqlite_pool_or_err().unwrap();

        sqlx::query(
            "CREATE TABLE quality_gate_history (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                job_id TEXT NOT NULL,
                score INTEGER NOT NULL,
                passed INTEGER DEFAULT 0,
                conductor TEXT DEFAULT 'GeoAuditConductor',
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                entropy_score REAL,
                retry_count INTEGER,
                details TEXT
            );",
        )
        .execute(sqlite_pool)
        .await
        .unwrap();

        let store = SqliteQualityGateStore::new(pool);
        store
            .record(
                "job-99",
                85,
                true,
                "TestConductor",
                Some(1.23),
                Some(2),
                Some("{\"reason\":\"test\"}"),
            )
            .await
            .unwrap();

        let entries = store.list_recent(10).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].job_id, "job-99");
        assert_eq!(entries[0].score, 85);
        assert!(entries[0].passed);
        assert_eq!(entries[0].conductor, "TestConductor");
        assert_eq!(entries[0].entropy_score, Some(1.23));
        assert_eq!(entries[0].retry_count, Some(2));
        assert_eq!(entries[0].details.as_deref(), Some("{\"reason\":\"test\"}"));
    }

    #[tokio::test]
    async fn test_quality_gate_store_without_entropy() {
        let pool = DatabasePool::new_sqlite("sqlite::memory:").await.unwrap();
        let sqlite_pool = pool.get_sqlite_pool_or_err().unwrap();

        sqlx::query(
            "CREATE TABLE quality_gate_history (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                job_id TEXT NOT NULL,
                score INTEGER NOT NULL,
                passed INTEGER DEFAULT 0,
                conductor TEXT DEFAULT 'GeoAuditConductor',
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                entropy_score REAL,
                retry_count INTEGER,
                details TEXT
            );",
        )
        .execute(sqlite_pool)
        .await
        .unwrap();

        let store = SqliteQualityGateStore::new(pool);
        // None 値での record が正しく動作することを検証
        store
            .record("job-100", 50, false, "BasicConductor", None, None, None)
            .await
            .unwrap();

        let entries = store.list_recent(10).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].job_id, "job-100");
        assert_eq!(entries[0].score, 50);
        assert!(!entries[0].passed);
        assert_eq!(entries[0].entropy_score, None);
        assert_eq!(entries[0].retry_count, None);
        assert_eq!(entries[0].details, None);
    }
}
