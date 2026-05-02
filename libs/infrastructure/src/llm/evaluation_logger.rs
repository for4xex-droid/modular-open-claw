/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::db::DatabasePool;
use aiome_core_contracts::error::AiomeError;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait EvalLogRepository: Send + Sync + std::fmt::Debug {
    async fn insert_eval_log(
        &self,
        prompt_hash: &str,
        entry: &EvaluationLogEntry,
    ) -> Result<(), AiomeError>;
    async fn get_provider_stats(
        &self,
        provider: &str,
        model: &str,
        days: i64,
    ) -> Result<ProviderEvalStat, AiomeError>;
    async fn get_all_provider_stats(&self, days: u32) -> Result<Vec<ProviderEvalStat>, AiomeError>;
    async fn garbage_collect(&self, days: u32) -> Result<u64, AiomeError>;
}

#[derive(Debug)]
pub struct SqlEvalLogRepository {
    pool: DatabasePool,
}

impl SqlEvalLogRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[derive(Debug)]
pub struct EvaluationLogger {
    repo: Arc<dyn EvalLogRepository>,
}

pub struct EvaluationLogEntry {
    pub prompt: String,
    pub system: Option<String>,
    pub provider: String,
    pub model: String,
    pub latency_ms: i64,
    pub token_count_in: Option<i64>,
    pub token_count_out: Option<i64>,
    pub cost_usd: Option<f64>,
    pub cache_hit: bool,
}

impl EvaluationLogger {
    pub fn new(repo: Arc<dyn EvalLogRepository>) -> Self {
        Self { repo }
    }

    pub async fn log(&self, entry: EvaluationLogEntry) -> Result<(), AiomeError> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(entry.prompt.as_bytes());
        if let Some(sys) = &entry.system {
            hasher.update(sys.as_bytes());
        }
        let prompt_hash = hex::encode(hasher.finalize());
        self.repo.insert_eval_log(&prompt_hash, &entry).await
    }

    pub async fn get_provider_stats(
        &self,
        provider: &str,
        model: &str,
        days: i64,
    ) -> Result<ProviderEvalStat, AiomeError> {
        self.repo.get_provider_stats(provider, model, days).await
    }

    pub async fn get_all_provider_stats(
        &self,
        days: u32,
    ) -> Result<Vec<ProviderEvalStat>, AiomeError> {
        self.repo.get_all_provider_stats(days).await
    }

    pub async fn garbage_collect(&self, days: u32) -> Result<u64, AiomeError> {
        self.repo.garbage_collect(days).await
    }
}

#[async_trait]
impl EvalLogRepository for SqlEvalLogRepository {
    async fn insert_eval_log(
        &self,
        prompt_hash: &str,
        entry: &EvaluationLogEntry,
    ) -> Result<(), AiomeError> {
        let cache_hit_int = if entry.cache_hit { 1 } else { 0 };

        let pool = &self.pool;
        let query = format!(
            "INSERT INTO prompt_evaluation_log (prompt_hash, provider, model, latency_ms, token_count_in, token_count_out, cost_usd, cache_hit) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7})",
            pool.ph(0), pool.ph(1), pool.ph(2), pool.ph(3), pool.ph(4), pool.ph(5), pool.ph(6), pool.ph(7)
        );

        match pool {
            crate::db::DatabasePool::Sqlite(p) => {
                sqlx::query(&query)
                    .bind(prompt_hash)
                    .bind(&entry.provider)
                    .bind(&entry.model)
                    .bind(entry.latency_ms)
                    .bind(entry.token_count_in)
                    .bind(entry.token_count_out)
                    .bind(entry.cost_usd)
                    .bind(cache_hit_int)
                    .execute(p)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to append evaluation log (SQLite): {}", e);
                        AiomeError::Infrastructure {
                            reason: "Failed to persist metrics".to_string(),
                        }
                    })?;
            }
            crate::db::DatabasePool::Postgres(p) => {
                sqlx::query(&query)
                    .bind(prompt_hash)
                    .bind(&entry.provider)
                    .bind(&entry.model)
                    .bind(entry.latency_ms)
                    .bind(entry.token_count_in)
                    .bind(entry.token_count_out)
                    .bind(entry.cost_usd)
                    .bind(cache_hit_int)
                    .execute(p)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to append evaluation log (Postgres): {}", e);
                        AiomeError::Infrastructure {
                            reason: "Failed to persist metrics".to_string(),
                        }
                    })?;
            }
        }
        Ok(())
    }

    async fn get_provider_stats(
        &self,
        provider: &str,
        model: &str,
        days: i64,
    ) -> Result<ProviderEvalStat, AiomeError> {
        let pool = &self.pool;
        let sql_modifier = format!("-{} days", days);

        let query = format!(
            "SELECT provider, model, AVG(latency_ms) as average_latency_ms, COUNT(*) as total_calls, 
             COALESCE(SUM(token_count_in), 0) as total_tokens_in, COALESCE(SUM(token_count_out), 0) as total_tokens_out,
             COALESCE(SUM(cost_usd), 0.0) as total_cost_usd, COALESCE(CAST(SUM(cache_hit) AS REAL) * 100.0 / COUNT(*), 0.0) as cache_hit_rate
             FROM prompt_evaluation_log
             WHERE provider = {0} AND model = {1} AND created_at >= datetime('now', {2})
             GROUP BY provider, model",
             pool.ph(0), pool.ph(1), pool.ph(2)
        );

        let stat = match pool {
            crate::db::DatabasePool::Sqlite(p) => {
                sqlx::query_as::<_, ProviderEvalStat>(&query)
                    .bind(provider)
                    .bind(model)
                    .bind(sql_modifier)
                    .fetch_optional(p)
                    .await
            }
            crate::db::DatabasePool::Postgres(p) => {
                let pg_query = "SELECT provider, model, COALESCE(AVG(latency_ms),0) as average_latency_ms, COUNT(*) as total_calls, 
                    COALESCE(SUM(token_count_in), 0) as total_tokens_in, COALESCE(SUM(token_count_out), 0) as total_tokens_out,
                    COALESCE(SUM(cost_usd), 0.0) as total_cost_usd, COALESCE(CAST(SUM(cache_hit) AS REAL) * 100.0 / COUNT(*), 0.0) as cache_hit_rate
                    FROM prompt_evaluation_log
                    WHERE provider = $1 AND model = $2 AND created_at >= NOW() - INTERVAL '1 day' * $3
                    GROUP BY provider, model";
                sqlx::query_as::<_, ProviderEvalStat>(pg_query)
                    .bind(provider)
                    .bind(model)
                    .bind(days as f64)
                    .fetch_optional(p)
                    .await
            }
        };

        match stat {
            Ok(Some(s)) => Ok(s),
            Ok(None) => Ok(ProviderEvalStat {
                provider: provider.to_string(),
                model: model.to_string(),
                average_latency_ms: 0.0,
                total_calls: 0,
                total_tokens_in: 0,
                total_tokens_out: 0,
                total_cost_usd: 0.0,
                cache_hit_rate: 0.0,
            }),
            Err(e) => {
                tracing::error!("Failed to fetch evaluation stats: {}", e);
                Err(AiomeError::Infrastructure {
                    reason: "Failed to fetch evaluation stats".to_string(),
                })
            }
        }
    }

    async fn get_all_provider_stats(&self, days: u32) -> Result<Vec<ProviderEvalStat>, AiomeError> {
        let pool = &self.pool;

        let stats = match pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let sql_modifier = format!("-{} days", days);
                let query = "SELECT provider, model, AVG(latency_ms) as average_latency_ms, COUNT(*) as total_calls, 
                    COALESCE(SUM(token_count_in), 0) as total_tokens_in, COALESCE(SUM(token_count_out), 0) as total_tokens_out,
                    COALESCE(SUM(cost_usd), 0.0) as total_cost_usd, COALESCE(CAST(SUM(cache_hit) AS REAL) * 100.0 / COUNT(*), 0.0) as cache_hit_rate
                    FROM prompt_evaluation_log
                    WHERE created_at >= datetime('now', ?)
                    GROUP BY provider, model";

                sqlx::query_as::<_, ProviderEvalStat>(query)
                    .bind(sql_modifier)
                    .fetch_all(p)
                    .await
            }
            crate::db::DatabasePool::Postgres(p) => {
                let pg_query = "SELECT provider, model, COALESCE(AVG(latency_ms),0) as average_latency_ms, COUNT(*) as total_calls, 
                    COALESCE(SUM(token_count_in), 0) as total_tokens_in, COALESCE(SUM(token_count_out), 0) as total_tokens_out,
                    COALESCE(SUM(cost_usd), 0.0) as total_cost_usd, COALESCE(CAST(SUM(cache_hit) AS REAL) * 100.0 / COUNT(*), 0.0) as cache_hit_rate
                    FROM prompt_evaluation_log
                    WHERE created_at >= NOW() - INTERVAL '1 day' * $1
                    GROUP BY provider, model";
                sqlx::query_as::<_, ProviderEvalStat>(pg_query)
                    .bind(days as f64)
                    .fetch_all(p)
                    .await
            }
        };

        stats.map_err(|e| {
            tracing::error!("Failed to fetch all evaluation stats: {}", e);
            AiomeError::Infrastructure {
                reason: "Failed to fetch all evaluation stats".to_string(),
            }
        })
    }

    async fn garbage_collect(&self, days: u32) -> Result<u64, AiomeError> {
        let pool = &self.pool;
        let sql_modifier = format!("-{} days", days);
        let query = "DELETE FROM prompt_evaluation_log WHERE created_at < datetime('now', ?)";

        let affected_res: Result<u64, sqlx::Error> = match pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query(query)
                .bind(sql_modifier)
                .execute(p)
                .await
                .map(|r| r.rows_affected()),
            crate::db::DatabasePool::Postgres(p) => {
                let pg_query = "DELETE FROM prompt_evaluation_log WHERE created_at < NOW() - INTERVAL '1 day' * $1";
                sqlx::query(pg_query)
                    .bind(days as f64)
                    .execute(p)
                    .await
                    .map(|r| r.rows_affected())
            }
        };

        match affected_res {
            Ok(rows) => Ok(rows),
            Err(e) => {
                tracing::error!("Failed to garbage collect evaluation stats: {}", e);
                Err(AiomeError::Infrastructure {
                    reason: "Failed to GC evaluation stats".to_string(),
                })
            }
        }
    }
}

#[derive(sqlx::FromRow, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ProviderEvalStat {
    pub provider: String,
    pub model: String,
    pub average_latency_ms: f64,
    pub total_calls: i64,
    pub total_tokens_in: i64,
    pub total_tokens_out: i64,
    pub total_cost_usd: f64,
    pub cache_hit_rate: f64,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::db::DatabasePool;
    use crate::job_queue::UniversalJobQueue;
    use sqlx::Row;

    #[tokio::test]
    async fn test_evaluation_logger_inserts_log() {
        let pool = crate::db::DatabasePool::Sqlite(
            sqlx::sqlite::SqlitePoolOptions::new()
                .connect("sqlite::memory:")
                .await
                .unwrap(), // allow-anti-pattern
        );
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = Arc::new(
            UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(), // allow-anti-pattern
        );

        crate::job_queue::migrations::DbInitializer::init_db(&*jq)
            .await
            .unwrap();

        let logger = EvaluationLogger::new(Arc::new(SqlEvalLogRepository::new(pool.clone())));
        let entry = EvaluationLogEntry {
            prompt: "Test prompt".into(),
            system: Some("Test system".into()),
            provider: "mock_provider".into(),
            model: "mock_model".into(),
            latency_ms: 120,
            token_count_in: None,
            token_count_out: None,
            cost_usd: None,
            cache_hit: false,
        };

        // Act
        let res = logger.log(entry).await;

        // Assert: should succeed
        assert!(
            res.is_ok(),
            "Logger should successfully insert. Err: {:?}",
            res.err()
        );

        // Verify insertion
        let pool = &pool;
        if let DatabasePool::Sqlite(p) = pool {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM prompt_evaluation_log WHERE provider = 'mock_provider'",
            )
            .fetch_one(p)
            .await
            .unwrap();
            assert_eq!(count, 1, "Should have inserted exactly 1 record");
        }
    }

    #[tokio::test]
    async fn test_evaluation_logger_stats() {
        let pool = crate::db::DatabasePool::Sqlite(
            sqlx::sqlite::SqlitePoolOptions::new()
                .connect("sqlite::memory:")
                .await
                .unwrap(), // allow-anti-pattern
        );
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = Arc::new(
            UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(), // allow-anti-pattern
        );
        crate::job_queue::migrations::DbInitializer::init_db(&*jq)
            .await
            .unwrap();

        let logger = EvaluationLogger::new(Arc::new(SqlEvalLogRepository::new(pool.clone())));
        logger
            .log(EvaluationLogEntry {
                prompt: "P1".into(),
                system: None,
                provider: "mock_provider".into(),
                model: "mock_model".into(),
                latency_ms: 100,
                token_count_in: Some(10),
                token_count_out: Some(20),
                cost_usd: Some(0.001),
                cache_hit: false,
            })
            .await
            .unwrap();

        let stats = logger
            .get_provider_stats("mock_provider", "mock_model", 7)
            .await
            .unwrap();
        assert_eq!(stats.total_calls, 1);
        assert_eq!(stats.average_latency_ms, 100.0);
    }

    #[tokio::test]
    async fn test_get_all_provider_stats_returns_aggregated_data() {
        let pool = crate::db::DatabasePool::Sqlite(
            sqlx::sqlite::SqlitePoolOptions::new()
                .connect("sqlite::memory:")
                .await
                .unwrap(), // allow-anti-pattern
        );
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = Arc::new(
            UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(), // allow-anti-pattern
        );
        crate::job_queue::migrations::DbInitializer::init_db(&*jq)
            .await
            .unwrap();

        let logger = EvaluationLogger::new(Arc::new(SqlEvalLogRepository::new(pool.clone())));

        // Insert two entries for different providers
        logger
            .log(EvaluationLogEntry {
                prompt: "P1".into(),
                system: None,
                provider: "gemini".into(),
                model: "gemini-2.5-flash".into(),
                latency_ms: 200,
                token_count_in: Some(50),
                token_count_out: Some(100),
                cost_usd: Some(0.005),
                cache_hit: true,
            })
            .await
            .unwrap();

        logger
            .log(EvaluationLogEntry {
                prompt: "P2".into(),
                system: None,
                provider: "ollama".into(),
                model: "llama3".into(),
                latency_ms: 500,
                token_count_in: None,
                token_count_out: None,
                cost_usd: None,
                cache_hit: false,
            })
            .await
            .unwrap();

        // Act
        let stats = logger.get_all_provider_stats(7).await.unwrap();

        // Assert: should return 2 provider groups
        assert_eq!(stats.len(), 2, "Should aggregate by provider+model");

        let gemini = stats.iter().find(|s| s.provider == "gemini").unwrap();
        assert_eq!(gemini.total_calls, 1);
        assert_eq!(gemini.total_tokens_in, 50);
        assert!(
            gemini.cache_hit_rate > 0.0,
            "Cache hit rate should be > 0 for cache_hit=true"
        );
    }

    #[tokio::test]
    async fn test_get_all_provider_stats_returns_empty_for_no_data() {
        let pool = crate::db::DatabasePool::Sqlite(
            sqlx::sqlite::SqlitePoolOptions::new()
                .connect("sqlite::memory:")
                .await
                .unwrap(), // allow-anti-pattern
        );
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = Arc::new(
            UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(), // allow-anti-pattern
        );
        crate::job_queue::migrations::DbInitializer::init_db(&*jq)
            .await
            .unwrap();

        let logger = EvaluationLogger::new(Arc::new(SqlEvalLogRepository::new(pool.clone())));

        // Act: no data inserted
        let stats = logger.get_all_provider_stats(7).await.unwrap();

        // Assert: empty vec, not an error
        assert!(
            stats.is_empty(),
            "Should return empty vec when no data exists"
        );
    }
}
