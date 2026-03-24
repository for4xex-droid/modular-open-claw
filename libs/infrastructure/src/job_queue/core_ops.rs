/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::try_get_opt;
use super::UniversalJobQueue;
use aiome_core::error::AiomeError;
use aiome_core::traits::{Job, JobStatus};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Row;
use tracing::warn;
use uuid::Uuid;

#[async_trait]
pub trait CoreOps {
    async fn do_enqueue(
        &self,
        category: &str,
        topic: &str,
        style: &str,
        karma_directives: Option<&str>,
        permission_manifest: Option<aiome_core::security::PermissionManifest>,
        agent_id: Option<uuid::Uuid>,
        priority: i32,
    ) -> Result<String, AiomeError>;
    async fn do_fetch_job(&self, job_id: &str) -> Result<Option<Job>, AiomeError>;
    async fn do_dequeue(&self, capable_categories: &[&str]) -> Result<Option<Job>, AiomeError>;
    async fn do_complete_job(
        &self,
        job_id: &str,
        output_artifacts: Option<&str>,
    ) -> Result<(), AiomeError>;
    async fn do_fail_job(&self, job_id: &str, reason: &str) -> Result<(), AiomeError>;
    async fn do_reclaim_zombie_jobs(&self, timeout_minutes: i64) -> Result<u64, AiomeError>;
    async fn do_set_creative_rating(&self, job_id: &str, rating: i32) -> Result<(), AiomeError>;
    async fn do_heartbeat_pulse(&self, job_id: &str) -> Result<(), AiomeError>;
    async fn do_store_execution_log(&self, job_id: &str, log: &str) -> Result<(), AiomeError>;
    async fn do_purge_old_jobs(&self, days: i64) -> Result<u64, AiomeError>;
    async fn do_fetch_recent_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError>;
    async fn do_get_pending_job_count(&self) -> Result<i64, AiomeError>;
    async fn do_get_job_count_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<i64, AiomeError>;
    async fn do_fetch_job_retry_count(&self, job_id: &str) -> Result<i64, AiomeError>;
    async fn do_increment_job_retry_count(&self, job_id: &str) -> Result<bool, AiomeError>;
    async fn do_reset_job_retry_count(&self, job_id: &str) -> Result<(), AiomeError>;
    async fn do_storage_gc(&self, threshold_gb: f64) -> Result<u64, AiomeError>;
}

#[async_trait]
impl CoreOps for UniversalJobQueue {
    async fn do_enqueue(
        &self,
        category: &str,
        topic: &str,
        style: &str,
        karma_directives: Option<&str>,
        permission_manifest: Option<aiome_core::security::PermissionManifest>,
        agent_id: Option<uuid::Uuid>,
        priority: i32,
    ) -> Result<String, AiomeError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let directives = karma_directives.unwrap_or("{}");
        let manifest_json = permission_manifest
            .map(|m| serde_json::to_string(&m).unwrap_or_else(|_| "{}".to_string()))
            .unwrap_or_else(|| "{}".to_string());
        let agent_id_str = agent_id.map(|uid| uid.to_string());        let q = match &self.pool {
            crate::db::DatabasePool::Sqlite(_) => format!(
                "INSERT INTO jobs (id, category, topic, style_name, karma_directives, permission_manifest, agent_id, status, priority, created_at, updated_at) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, {10})",
                self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4),
                self.pool.ph(5), self.pool.ph(6), self.pool.ph(7), self.pool.ph(8), self.pool.ph(9), self.pool.ph(10)
            ),
            crate::db::DatabasePool::Postgres(_) => format!(
                "INSERT INTO jobs (id, category, topic, style_name, karma_directives, permission_manifest, agent_id, status, priority, created_at, updated_at) VALUES ({0}, {1}, {2}, {3}, {4}::jsonb, {5}::jsonb, {6}, {7}, {8}, {9}::timestamptz, {10}::timestamptz)",
                self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4),
                self.pool.ph(5), self.pool.ph(6), self.pool.ph(7), self.pool.ph(8), self.pool.ph(9), self.pool.ph(10)
            ),
        };
        sql_exec!(
            &self.pool,
            &q,
            &id,
            category,
            topic,
            style,
            directives,
            manifest_json,
            agent_id_str,
            JobStatus::Pending.to_string(),
            priority,
            &now,
            &now
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to enqueue job: {}", e),
        })?;

        Ok(id)
    }

    async fn do_fetch_job(&self, job_id: &str) -> Result<Option<Job>, AiomeError> {
        let q = format!(
            "SELECT id, category, topic, style_name, karma_directives, permission_manifest, agent_id, status, started_at, last_heartbeat, tech_karma_extracted, creative_rating, execution_log, error_message, sns_platform, sns_content_id, published_at, output_artifacts, priority FROM jobs WHERE id = {}",
            self.pool.ph(0)
        );
        macro_rules! map_job_row {
            ($r:expr) => {{
                let agent_id_str: Option<String> = try_get_opt($r, "agent_id");
                let status_str: String = $r.get("status");
                Job {
                    id: $r.get("id"),
                    category: $r.get("category"),
                    topic: $r.get("topic"),
                    style: $r.get("style_name"),
                    karma_directives: try_get_opt($r, "karma_directives"),
                    status: JobStatus::from_string(&status_str),
                    started_at: try_get_opt($r, "started_at"),
                    last_heartbeat: try_get_opt($r, "last_heartbeat"),
                    tech_karma_extracted: $r.try_get::<i32, _>("tech_karma_extracted").unwrap_or(0)
                        != 0,
                    creative_rating: $r.try_get("creative_rating").ok(),
                    execution_log: try_get_opt($r, "execution_log"),
                    error_message: try_get_opt($r, "error_message"),
                    sns_platform: try_get_opt($r, "sns_platform"),
                    sns_content_id: try_get_opt($r, "sns_content_id"),
                    published_at: try_get_opt($r, "published_at"),
                    output_artifacts: try_get_opt($r, "output_artifacts"),
                    permission_manifest: $r
                        .try_get::<String, _>("permission_manifest")
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    agent_id: agent_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                    priority: $r.try_get("priority").unwrap_or(0),
                }
            }};
        }

        let job = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let r = sqlx::query(&q)
                    .bind(job_id)
                    .fetch_optional(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                r.map(|r| map_job_row!(&r))
            }
            crate::db::DatabasePool::Postgres(p) => {
                let r = sqlx::query(&q)
                    .bind(job_id)
                    .fetch_optional(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                r.map(|r| map_job_row!(&r))
            }
        };

        Ok(job)
    }

    async fn do_dequeue(&self, capable_categories: &[&str]) -> Result<Option<Job>, AiomeError> {
        let placeholders = capable_categories
            .iter()
            .enumerate()
            .map(|(i, _)| self.pool.ph(i + 1)) // i+1 because status is $1
            .collect::<Vec<_>>()
            .join(", ");

        let query_str = format!(
            "SELECT id, category, topic, style_name, karma_directives, permission_manifest, agent_id, status, started_at, last_heartbeat, tech_karma_extracted, creative_rating, execution_log, error_message, sns_platform, sns_content_id, published_at, output_artifacts, priority FROM jobs WHERE status = {} AND category IN ({}) ORDER BY priority DESC, created_at ASC LIMIT 1",
            self.pool.ph(0), placeholders
        );

        let now = Utc::now().to_rfc3339();
        let update_str = format!(
            "UPDATE jobs SET status = {0}, started_at = {1}, last_heartbeat = {2}, updated_at = {3} WHERE id = {4} AND status = {5}",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5)
        );

        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let mut tx = p.begin().await.map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;
                let mut q = sqlx::query(&query_str).bind(JobStatus::Pending.to_string());
                for cat in capable_categories {
                    q = q.bind(*cat);
                }

                let row =
                    q.fetch_optional(&mut *tx)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                if let Some(r) = row {
                    let id: String = r.get("id");
                    let res = sqlx::query(&update_str)
                        .bind(JobStatus::Processing.to_string())
                        .bind(&now)
                        .bind(&now)
                        .bind(&now)
                        .bind(&id)
                        .bind(JobStatus::Pending.to_string())
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                    if res.rows_affected() == 0 {
                        // Conflict! Another worker snatched it.
                        tx.rollback()
                            .await
                            .map_err(|e| AiomeError::Infrastructure {
                                reason: e.to_string(),
                            })?;
                        return Ok(None);
                    }
                    tx.commit().await.map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;

                    let topic: String = r.get("topic");
                    let style: String = r.get("style_name");
                    let karma_directives: Option<String> = try_get_opt(&r, "karma_directives");
                    let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
                    let creative_rating: Option<i32> = r.try_get("creative_rating").ok();
                    let execution_log: Option<String> = try_get_opt(&r, "execution_log");
                    let error_message: Option<String> = try_get_opt(&r, "error_message");
                    let sns_platform: Option<String> = try_get_opt(&r, "sns_platform");
                    let sns_content_id: Option<String> = try_get_opt(&r, "sns_content_id");
                    let published_at: Option<String> = try_get_opt(&r, "published_at");
                    let output_artifacts: Option<String> = try_get_opt(&r, "output_artifacts");
                    let agent_id_str: Option<String> = try_get_opt(&r, "agent_id");
                    let agent_id = agent_id_str.and_then(|s| Uuid::parse_str(&s).ok());
                    let permission_manifest = r
                        .try_get::<String, _>("permission_manifest")
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok());

                    Ok(Some(Job {
                        id,
                        category: r.get("category"),
                        topic,
                        style,
                        karma_directives,
                        status: JobStatus::Processing,
                        started_at: Some(now.clone()),
                        last_heartbeat: Some(now),
                        tech_karma_extracted: tech_karma_extracted != 0,
                        creative_rating,
                        execution_log,
                        error_message,
                        sns_platform,
                        sns_content_id,
                        published_at,
                        output_artifacts,
                        permission_manifest,
                        agent_id,
                        priority: r.get("priority"),
                    }))
                } else {
                    Ok(None)
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let mut tx = p.begin().await.map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;
                let mut q = sqlx::query(&query_str).bind(JobStatus::Pending.to_string());
                for cat in capable_categories {
                    q = q.bind(*cat);
                }

                let row =
                    q.fetch_optional(&mut *tx)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                if let Some(r) = row {
                    let id: String = r.get("id");
                    let res = sqlx::query(&update_str)
                        .bind(JobStatus::Processing.to_string())
                        .bind(&now)
                        .bind(&now)
                        .bind(&now)
                        .bind(&id)
                        .bind(JobStatus::Pending.to_string())
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                    if res.rows_affected() == 0 {
                        tx.rollback()
                            .await
                            .map_err(|e| AiomeError::Infrastructure {
                                reason: e.to_string(),
                            })?;
                        return Ok(None);
                    }
                    tx.commit().await.map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;

                    let topic: String = r.get("topic");
                    let style: String = r.get("style_name");
                    let karma_directives: Option<String> = try_get_opt(&r, "karma_directives");
                    let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
                    let creative_rating: Option<i32> = r.try_get("creative_rating").ok();
                    let execution_log: Option<String> = try_get_opt(&r, "execution_log");
                    let error_message: Option<String> = try_get_opt(&r, "error_message");
                    let sns_platform: Option<String> = try_get_opt(&r, "sns_platform");
                    let sns_content_id: Option<String> = try_get_opt(&r, "sns_content_id");
                    let published_at: Option<String> = try_get_opt(&r, "published_at");
                    let output_artifacts: Option<String> = try_get_opt(&r, "output_artifacts");
                    let agent_id_str: Option<String> = try_get_opt(&r, "agent_id");
                    let agent_id = agent_id_str.and_then(|s| Uuid::parse_str(&s).ok());

                    // In Postgres, permission_manifest might be JSONB, so we might need a different deserialization.
                    // For now, assuming it's stored as JSON string or compatible.
                    let permission_manifest = r
                        .try_get::<serde_json::Value, _>("permission_manifest")
                        .ok()
                        .and_then(|v| serde_json::from_value(v).ok());

                    Ok(Some(Job {
                        id,
                        category: r.get("category"),
                        topic,
                        style,
                        karma_directives,
                        status: JobStatus::Processing,
                        started_at: Some(now.clone()),
                        last_heartbeat: Some(now),
                        tech_karma_extracted: tech_karma_extracted != 0,
                        creative_rating,
                        execution_log,
                        error_message,
                        sns_platform,
                        sns_content_id,
                        published_at,
                        output_artifacts,
                        permission_manifest,
                        agent_id,
                        priority: r.get("priority"),
                    }))
                } else {
                    Ok(None)
                }
            }
        }
    }

    async fn do_complete_job(
        &self,
        job_id: &str,
        output_artifacts: Option<&str>,
    ) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        let q =
            format!(
            "UPDATE jobs SET status = {0}, output_artifacts = {1}, updated_at = {2} WHERE id = {3}",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3)
        );
        sql_exec!(
            &self.pool,
            &q,
            JobStatus::Completed.to_string(),
            output_artifacts,
            &now,
            job_id
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to complete job {}: {}", job_id, e),
        })?;
        Ok(())
    }

    async fn do_fail_job(&self, job_id: &str, reason: &str) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        let q =
            format!(
            "UPDATE jobs SET status = {0}, error_message = {1}, updated_at = {2} WHERE id = {3}",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3)
        );
        sql_exec!(
            &self.pool,
            &q,
            JobStatus::Failed.to_string(),
            reason,
            &now,
            job_id
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to fail job {}: {}", job_id, e),
        })?;
        Ok(())
    }

    async fn do_reclaim_zombie_jobs(&self, timeout_minutes: i64) -> Result<u64, AiomeError> {
        let now = Utc::now().to_rfc3339();
        let update_str = format!(
            "UPDATE jobs SET status = 'Failed', error_message = 'Zombie reclaimed: heartbeat timeout exceeded', updated_at = {0} 
             WHERE status = 'Processing' 
             AND last_heartbeat IS NOT NULL 
             AND {1}",
            self.pool.ph(0),
            self.pool.interval_minutes_check("last_heartbeat", timeout_minutes)
        );

        let result =
            sql_exec!(&self.pool, &update_str, &now).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to reclaim zombie jobs: {}", e),
            })?;

        let count = result;
        if count > 0 {
            tracing::warn!("🧟 Zombie Hunter: Reclaimed {} ghost job(s)", count);
        }
        Ok(count)
    }

    async fn do_set_creative_rating(&self, job_id: &str, rating: i32) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        let q = format!(
            "UPDATE jobs SET creative_rating = {0}, updated_at = {1} WHERE id = {2} AND status IN ('Completed', 'Processing')",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2)
        );
        let result = sql_exec!(&self.pool, &q, rating, &now, job_id).map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("Failed to set creative rating for job {}: {}", job_id, e),
            }
        })?;

        if result == 0 {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Atomic Guard: Job '{}' is not in Completed/Processing state, rating rejected",
                    job_id
                ),
            });
        }
        Ok(())
    }

    async fn do_heartbeat_pulse(&self, job_id: &str) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        let q = format!(
            "UPDATE jobs SET last_heartbeat = {0}, updated_at = {1} WHERE id = {2}",
            self.pool.ph(0),
            self.pool.ph(1),
            self.pool.ph(2)
        );
        sql_exec!(&self.pool, &q, &now, &now, job_id).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to pulse heartbeat for job {}: {}", job_id, e),
        })?;
        Ok(())
    }

    async fn do_store_execution_log(&self, job_id: &str, log: &str) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        let q = format!(
            "UPDATE jobs SET execution_log = {0}, updated_at = {1} WHERE id = {2}",
            self.pool.ph(0),
            self.pool.ph(1),
            self.pool.ph(2)
        );
        sql_exec!(&self.pool, &q, log, &now, job_id).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to store execution log for job {}: {}", job_id, e),
        })?;
        Ok(())
    }

    async fn do_purge_old_jobs(&self, days: i64) -> Result<u64, AiomeError> {
        let q = match &self.pool {
            crate::db::DatabasePool::Sqlite(_) => format!("DELETE FROM jobs WHERE status IN ('Completed', 'Failed') AND created_at < datetime('now', '-{} days')", days),
            crate::db::DatabasePool::Postgres(_) => format!("DELETE FROM jobs WHERE status IN ('Completed', 'Failed') AND created_at < NOW() - INTERVAL '{} days'", days),
        };

        let result = sql_exec!(&self.pool, &q).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to purge old jobs: {}", e),
        })?;

        let purged = result;
        if let Some(p) = self.pool.get_sqlite_pool() {
            let _ = sqlx::query("PRAGMA optimize;").execute(p).await;
        }
        Ok(purged)
    }

    async fn do_fetch_recent_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        let q = format!("SELECT id, category, topic, style_name, karma_directives, permission_manifest, agent_id, status, started_at, last_heartbeat, 
                     tech_karma_extracted, creative_rating, execution_log, error_message,
                      sns_platform, sns_content_id, published_at, output_artifacts, priority 
              FROM jobs 
              ORDER BY created_at DESC LIMIT {}", self.pool.ph(0));

        macro_rules! map_job_row {
            ($r:expr) => {{
                let agent_id_str: Option<String> = try_get_opt($r, "agent_id");
                let status_str: String = $r.get("status");
                Job {
                    id: $r.get("id"),
                    category: $r.get("category"),
                    topic: $r.get("topic"),
                    style: $r.get("style_name"),
                    karma_directives: try_get_opt($r, "karma_directives"),
                    status: JobStatus::from_string(&status_str),
                    started_at: try_get_opt($r, "started_at"),
                    last_heartbeat: try_get_opt($r, "last_heartbeat"),
                    tech_karma_extracted: $r.try_get::<i32, _>("tech_karma_extracted").unwrap_or(0)
                        != 0,
                    creative_rating: $r.try_get("creative_rating").ok(),
                    execution_log: try_get_opt($r, "execution_log"),
                    error_message: try_get_opt($r, "error_message"),
                    sns_platform: try_get_opt($r, "sns_platform"),
                    sns_content_id: try_get_opt($r, "sns_content_id"),
                    published_at: try_get_opt($r, "published_at"),
                    output_artifacts: try_get_opt($r, "output_artifacts"),
                    permission_manifest: $r
                        .try_get::<String, _>("permission_manifest")
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    agent_id: agent_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                    priority: $r.try_get("priority").unwrap_or(0),
                }
            }};
        }

        let mut jobs = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(&q)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    jobs.push(map_job_row!(&r));
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(&q)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    jobs.push(map_job_row!(&r));
                }
            }
        }
        Ok(jobs)
    }

    async fn do_get_pending_job_count(&self) -> Result<i64, AiomeError> {
        let q = format!(
            "SELECT COUNT(*) as count FROM jobs WHERE status = {}",
            self.pool.ph(0)
        );
        let count: i64 = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                sqlx::query_scalar(&q)
                    .bind(JobStatus::Pending.to_string())
                    .fetch_one(p)
                    .await
            }
            crate::db::DatabasePool::Postgres(p) => {
                sqlx::query_scalar(&q)
                    .bind(JobStatus::Pending.to_string())
                    .fetch_one(p)
                    .await
            }
        }
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to count pending jobs: {}", e),
        })?;
        Ok(count)
    }

    async fn do_get_job_count_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<i64, AiomeError> {
        let since_str = since.to_rfc3339();
        let q = format!(
            "SELECT COUNT(*) as count FROM jobs WHERE created_at >= {}",
            self.pool.ph(0)
        );
        let count: i64 = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                sqlx::query_scalar(&q).bind(since_str).fetch_one(p).await
            }
            crate::db::DatabasePool::Postgres(p) => {
                sqlx::query_scalar(&q).bind(since_str).fetch_one(p).await
            }
        }
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to count jobs since: {}", e),
        })?;
        Ok(count)
    }

    async fn do_fetch_job_retry_count(&self, job_id: &str) -> Result<i64, AiomeError> {
        let q = format!(
            "SELECT retry_count FROM jobs WHERE id = {}",
            self.pool.ph(0)
        );
        let count: i64 = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar(&q)
                .bind(job_id)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to fetch retry count: {}", e),
                })?
                .unwrap_or(0),
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar(&q)
                .bind(job_id)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to fetch retry count: {}", e),
                })?
                .unwrap_or(0),
        };

        Ok(count)
    }

    async fn do_reset_job_retry_count(&self, job_id: &str) -> Result<(), AiomeError> {
        let q = format!(
            "UPDATE jobs SET retry_count = 0 WHERE id = {}",
            self.pool.ph(0)
        );
        sql_exec!(&self.pool, &q, job_id).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to reset retry count: {}", e),
        })?;
        Ok(())
    }

    async fn do_increment_job_retry_count(&self, job_id: &str) -> Result<bool, AiomeError> {
        let q = format!(
            "UPDATE jobs SET retry_count = retry_count + 1 WHERE id = {} RETURNING retry_count",
            self.pool.ph(0)
        );
        let count: i64 = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                sqlx::query_scalar(&q).bind(job_id).fetch_one(p).await
            }
            crate::db::DatabasePool::Postgres(p) => {
                sqlx::query_scalar(&q).bind(job_id).fetch_one(p).await
            }
        }
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to increment job retry count: {}", e),
        })?;

        if count >= 3 {
            let poison_pill = format!("UPDATE jobs SET status = 'Failed', error_message = 'Poison Pill Activated: API continually fails.' WHERE id = {}", self.pool.ph(0));
            if let Err(e) = sql_exec!(&self.pool, &poison_pill, job_id) {
                warn!(
                    "⚠️ [CoreOps] Failed to execute poison pill for job {}: {}",
                    job_id, e
                );
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn do_storage_gc(&self, threshold_gb: f64) -> Result<u64, AiomeError> {
        let threshold_bytes = (threshold_gb * 1024.0 * 1024.0 * 1024.0) as u64;

        // Fetch all jobs with artifacts
        let q = "SELECT id, output_artifacts FROM jobs WHERE output_artifacts IS NOT NULL AND status IN ('Completed', 'Failed') ORDER BY created_at ASC";
        let job_artifacts_raw: Vec<(String, String)> = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_as(q).fetch_all(p).await,
            crate::db::DatabasePool::Postgres(p) => sqlx::query_as(q).fetch_all(p).await,
        }
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("GC: failed to fetch artifacts: {}", e),
        })?;

        let mut current_total_size: u64 = 0;
        let mut job_artifacts = Vec::new();

        for (id, artifacts_json) in job_artifacts_raw {
            if let Ok(paths) = serde_json::from_str::<Vec<String>>(&artifacts_json) {
                let mut job_size = 0;
                for p in &paths {
                    if let Ok(meta) = std::fs::metadata(p) {
                        job_size += meta.len();
                    }
                }
                current_total_size += job_size;
                job_artifacts.push((id, paths, job_size));
            }
        }

        if current_total_size <= threshold_bytes {
            return Ok(0);
        }

        tracing::info!("♻️ [StorageGC] Current storage usage ({} bytes) exceeds threshold ({} bytes). Starting cleanup.", current_total_size, threshold_bytes);

        let mut deleted_count = 0;
        let mut target_reduction = current_total_size - threshold_bytes;
        let mut reduced_so_far = 0;

        let clear_q = format!(
            "UPDATE jobs SET output_artifacts = NULL WHERE id = {}",
            self.pool.ph(0)
        );

        for (id, paths, size) in job_artifacts {
            if reduced_so_far >= target_reduction {
                break;
            }

            for p in paths {
                if std::fs::remove_file(&p).is_ok() {
                    deleted_count += 1;
                }
            }

            // Clear artifact list in DB to prevent re-scanning
            let _ = sql_exec!(&self.pool, &clear_q, &id);

            reduced_so_far += size;
        }

        tracing::info!(
            "♻️ [StorageGC] Cleanup complete. Removed {} artifact files.",
            deleted_count
        );
        Ok(deleted_count)
    }
}
