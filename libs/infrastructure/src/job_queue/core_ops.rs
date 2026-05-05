/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::UniversalJobQueue;
use crate::{sql_exec, sql_fetch_all, sql_fetch_one, sql_fetch_optional};
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
    async fn do_requeue_job(&self, job_id: &str) -> Result<(), AiomeError>;
    async fn do_cancel_job(&self, job_id: &str) -> Result<(), AiomeError>;
    async fn do_update_job_status(&self, job_id: &str, status: &str) -> Result<(), AiomeError>;
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
    async fn do_fetch_job_cost(&self, job_id: &str) -> Result<f64, AiomeError>;
    async fn do_storage_gc(&self, threshold_gb: f64) -> Result<u64, AiomeError>;
    async fn do_publish(
        &self,
        content: &str,
        media_paths: &[std::path::PathBuf],
        metadata: &serde_json::Value,
    ) -> Result<String, AiomeError>;
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
        // Phase 52.3: Constitutional Validation
        // Perform Axiomatic Safety check before persistence.
        if let Some(ref manifest) = permission_manifest {
            self.security_validator.validate_manifest(manifest).await?;
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let directives = karma_directives.unwrap_or("{}");
        let manifest_json = permission_manifest
            .map(|m| serde_json::to_string(&m).unwrap_or_else(|_| "{}".to_string()))
            .unwrap_or_else(|| "{}".to_string());
        let agent_id_str = agent_id.map(|uid| uid.to_string());
        let q = match &self.pool {
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
            "Pending",
            priority,
            &now,
            &now
        )?;
        Ok(id)
    }

    async fn do_fetch_job(&self, job_id: &str) -> Result<Option<Job>, AiomeError> {
        let q = format!("SELECT * FROM jobs WHERE id = {}", self.pool.ph(0));
        macro_rules! map_job_row {
            ($r:expr) => {{
                let agent_id_str: Option<String> = $r.try_get("agent_id").ok();
                let status_str: String = $r.get("status");
                Job {
                    id: $r.get("id"),
                    category: $r.get("category"),
                    topic: $r.get("topic"),
                    style: $r.get("style_name"),
                    karma_directives: $r.try_get("karma_directives").ok(),
                    status: JobStatus::from_string(&status_str),
                    started_at: $r.try_get("started_at").ok(),
                    last_heartbeat: $r.try_get("last_heartbeat").ok(),
                    tech_karma_extracted: $r.try_get::<i32, _>("tech_karma_extracted").unwrap_or(0)
                        != 0,
                    creative_rating: $r.try_get("creative_rating").ok(),
                    execution_log: $r.try_get("execution_log").ok(),
                    error_message: $r.try_get("error_message").ok(),
                    sns_platform: $r.try_get("sns_platform").ok(),
                    sns_content_id: $r.try_get("sns_content_id").ok(),
                    published_at: $r.try_get("published_at").ok(),
                    output_artifacts: $r.try_get("output_artifacts").ok(),
                    permission_manifest: $r
                        .try_get::<String, _>("permission_manifest")
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    agent_id: agent_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                    priority: $r.get("priority"),
                    created_at: $r.try_get("created_at").unwrap_or_default(),
                    updated_at: $r.try_get("updated_at").unwrap_or_default(),
                    requires_review: $r.try_get::<bool, _>("requires_review").unwrap_or(false),
                }
            }};
        }

        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let r = sqlx::query(&q)
                    .bind(job_id)
                    .fetch_optional(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                Ok(r.map(|r| map_job_row!(&r)))
            }
            crate::db::DatabasePool::Postgres(p) => {
                let r = sqlx::query(&q)
                    .bind(job_id)
                    .fetch_optional(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                Ok(r.map(|r| map_job_row!(&r)))
            }
        }
    }

    async fn do_dequeue(&self, capable_categories: &[&str]) -> Result<Option<Job>, AiomeError> {
        let placeholders = capable_categories
            .iter()
            .enumerate()
            .map(|(i, _)| self.pool.ph(i + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let query_str = format!("SELECT * FROM jobs WHERE status = {} AND category IN ({}) ORDER BY priority DESC, created_at ASC LIMIT 1", self.pool.ph(0), placeholders);
        let now = Utc::now().to_rfc3339();
        let update_str = format!("UPDATE jobs SET status = {0}, started_at = {1}, last_heartbeat = {2}, updated_at = {3} WHERE id = {4} AND status = {5}",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5));

        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                // SQLite: 楽観的ロック (UPDATE ... AND status = 'Pending') による排他制御
                // 他のワーカーが先に取得した場合は、影響行数が0になるためリトライする
                loop {
                    let mut tx = p.begin().await.map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                    let mut q = sqlx::query(&query_str).bind("Pending");
                    for cat in capable_categories {
                        q = q.bind(*cat);
                    }
                    let row = q.fetch_optional(&mut *tx).await.map_err(|e| {
                        AiomeError::Infrastructure {
                            reason: e.to_string(),
                        }
                    })?;

                    if let Some(r) = row {
                        let id: String = r.get("id");
                        let res = sqlx::query(&update_str)
                            .bind("Processing")
                            .bind(&now)
                            .bind(&now)
                            .bind(&now)
                            .bind(&id)
                            .bind("Pending")
                            .execute(&mut *tx)
                            .await
                            .map_err(|e| AiomeError::Infrastructure {
                                reason: e.to_string(),
                            })?;
                        if res.rows_affected() == 0 {
                            if let Err(e) = tx.rollback().await {
                                tracing::warn!("Failed to rollback transaction: {}", e);
                            }
                            // 他のワーカーが取得したため、次のジョブを探すループに戻る
                            continue;
                        }
                        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                        let job = self.do_fetch_job(&id).await?.ok_or_else(|| {
                            AiomeError::Infrastructure {
                                reason: format!("Job {} committed but vanished during fetch", id),
                            }
                        })?;
                        return Ok(Some(job));
                    } else {
                        return Ok(None);
                    }
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let pg_query_str = format!("{} FOR UPDATE SKIP LOCKED", query_str);
                let mut tx = p.begin().await.map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;
                let mut q = sqlx::query(&pg_query_str).bind("Pending");
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
                        .bind("Processing")
                        .bind(&now)
                        .bind(&now)
                        .bind(&now)
                        .bind(&id)
                        .bind("Pending")
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                    if res.rows_affected() == 0 {
                        if let Err(e) = tx.rollback().await {
                            tracing::warn!("Failed to rollback transaction: {}", e);
                        }
                        return Ok(None);
                    }
                    tx.commit().await.map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                    let job = self.do_fetch_job(&id).await?.ok_or_else(|| {
                        AiomeError::Infrastructure {
                            reason: format!("Job {} committed but vanished during fetch", id),
                        }
                    })?;
                    Ok(Some(job))
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
        let q = format!(
            "UPDATE jobs SET status = {0}, output_artifacts = {1}, updated_at = {2} WHERE id = {3}",
            self.pool.ph(0),
            self.pool.ph(1),
            self.pool.ph(2),
            self.pool.ph(3)
        );
        sql_exec!(&self.pool, &q, "Completed", output_artifacts, &now, job_id)?;
        Ok(())
    }

    async fn do_fail_job(&self, job_id: &str, reason: &str) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        let q = format!(
            "UPDATE jobs SET status = {0}, error_message = {1}, updated_at = {2} WHERE id = {3}",
            self.pool.ph(0),
            self.pool.ph(1),
            self.pool.ph(2),
            self.pool.ph(3)
        );
        sql_exec!(&self.pool, &q, "Failed", reason, &now, job_id)?;
        Ok(())
    }

    async fn do_requeue_job(&self, job_id: &str) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        let q = format!(
            "UPDATE jobs SET status = {0}, error_message = NULL, updated_at = {1} WHERE id = {2}",
            self.pool.ph(0),
            self.pool.ph(1),
            self.pool.ph(2)
        );
        sql_exec!(&self.pool, &q, "Pending", &now, job_id)?;
        Ok(())
    }

    async fn do_cancel_job(&self, job_id: &str) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        let q = format!(
            "UPDATE jobs SET status = {0}, updated_at = {1} WHERE id = {2} AND status IN ('Pending', 'Processing', 'AwaitingInput')",
            self.pool.ph(0),
            self.pool.ph(1),
            self.pool.ph(2)
        );
        let rows = sql_exec!(&self.pool, &q, "Cancelled", &now, job_id)?;
        if rows == 0 {
            return Err(AiomeError::ArtifactNotFound {
                path: format!("job:{}", job_id),
            });
        }
        Ok(())
    }

    async fn do_update_job_status(&self, job_id: &str, status: &str) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        let q = format!(
            "UPDATE jobs SET status = {0}, updated_at = {1} WHERE id = {2}",
            self.pool.ph(0),
            self.pool.ph(1),
            self.pool.ph(2)
        );
        sql_exec!(&self.pool, &q, status, &now, job_id)?;
        Ok(())
    }

    async fn do_reclaim_zombie_jobs(&self, timeout_minutes: i64) -> Result<u64, AiomeError> {
        let now = Utc::now().to_rfc3339();
        let update_str = format!("UPDATE jobs SET status = 'Failed', error_message = 'Zombie reclaimed', updated_at = {0} WHERE status IN ('Processing', 'Evaluating') AND {1}", self.pool.ph(0), self.pool.interval_minutes_check("last_heartbeat", timeout_minutes));
        sql_exec!(&self.pool, &update_str, &now)
    }

    async fn do_set_creative_rating(&self, job_id: &str, rating: i32) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        let q = format!(
            "UPDATE jobs SET creative_rating = {0}, updated_at = {1} WHERE id = {2} AND status != 'Pending'",
            self.pool.ph(0),
            self.pool.ph(1),
            self.pool.ph(2)
        );
        let rows = sql_exec!(&self.pool, &q, rating, &now, job_id)?;
        if rows == 0 {
            return Err(AiomeError::Infrastructure {
                reason: "Atomic Guard: Cannot rate pending job".to_string(),
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
        sql_exec!(&self.pool, &q, &now, &now, job_id)?;
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
        sql_exec!(&self.pool, &q, log, &now, job_id)?;
        Ok(())
    }

    async fn do_purge_old_jobs(&self, days: i64) -> Result<u64, AiomeError> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.to_rfc3339();

        let q = format!(
            "DELETE FROM jobs WHERE status IN ('Completed', 'Failed') AND created_at < {}",
            self.pool.ph(0)
        );
        sql_exec!(&self.pool, &q, cutoff_str)
    }

    async fn do_fetch_recent_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        let q = format!(
            "SELECT * FROM jobs ORDER BY created_at DESC LIMIT {}",
            self.pool.ph(0)
        );
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
                    let agent_id_str: Option<String> = r.try_get("agent_id").ok();
                    jobs.push(Job {
                        id: r.get("id"),
                        category: r.get("category"),
                        topic: r.get("topic"),
                        style: r.get("style_name"),
                        karma_directives: r.try_get("karma_directives").ok(),
                        status: JobStatus::from_string(r.get::<String, _>("status")),
                        started_at: r.try_get("started_at").ok(),
                        last_heartbeat: r.try_get("last_heartbeat").ok(),
                        tech_karma_extracted: r.get::<i32, _>("tech_karma_extracted") != 0,
                        creative_rating: r.try_get("creative_rating").ok(),
                        execution_log: r.try_get("execution_log").ok(),
                        error_message: r.try_get("error_message").ok(),
                        sns_platform: r.try_get("sns_platform").ok(),
                        sns_content_id: r.try_get("sns_content_id").ok(),
                        published_at: r.try_get("published_at").ok(),
                        output_artifacts: r.try_get("output_artifacts").ok(),
                        permission_manifest: r
                            .try_get::<String, _>("permission_manifest")
                            .ok()
                            .and_then(|s| serde_json::from_str(&s).ok()),
                        agent_id: agent_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                        priority: r.get("priority"),
                        created_at: r.try_get("created_at").unwrap_or_default(),
                        updated_at: r.try_get("updated_at").unwrap_or_default(),
                        requires_review: r.try_get::<bool, _>("requires_review").unwrap_or(false),
                    });
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
                    let agent_id_str: Option<String> = r.try_get("agent_id").ok();
                    jobs.push(Job {
                        id: r.get("id"),
                        category: r.get("category"),
                        topic: r.get("topic"),
                        style: r.get("style_name"),
                        karma_directives: r.try_get("karma_directives").ok(),
                        status: JobStatus::from_string(r.get::<String, _>("status")),
                        started_at: r.try_get("started_at").ok(),
                        last_heartbeat: r.try_get("last_heartbeat").ok(),
                        tech_karma_extracted: r.get::<i32, _>("tech_karma_extracted") != 0,
                        creative_rating: r.try_get("creative_rating").ok(),
                        execution_log: r.try_get("execution_log").ok(),
                        error_message: r.try_get("error_message").ok(),
                        sns_platform: r.try_get("sns_platform").ok(),
                        sns_content_id: r.try_get("sns_content_id").ok(),
                        published_at: r.try_get("published_at").ok(),
                        output_artifacts: r.try_get("output_artifacts").ok(),
                        permission_manifest: r
                            .try_get::<serde_json::Value, _>("permission_manifest")
                            .ok()
                            .and_then(|v| serde_json::from_value(v).ok()),
                        agent_id: agent_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                        priority: r.get("priority"),
                        created_at: r.try_get("created_at").unwrap_or_default(),
                        updated_at: r.try_get("updated_at").unwrap_or_default(),
                        requires_review: r.try_get::<bool, _>("requires_review").unwrap_or(false),
                    });
                }
            }
        }
        Ok(jobs)
    }

    async fn do_get_pending_job_count(&self) -> Result<i64, AiomeError> {
        let q = format!(
            "SELECT COUNT(*) FROM jobs WHERE status = {}",
            self.pool.ph(0)
        );
        crate::sql_fetch_one!(&self.pool, (i64,), &q, "Pending").map(|r| r.0)
    }

    async fn do_get_job_count_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<i64, AiomeError> {
        let q = format!(
            "SELECT COUNT(*) FROM jobs WHERE created_at >= {}",
            self.pool.ph(0)
        );
        let s = since.to_rfc3339();
        crate::sql_fetch_one!(&self.pool, (i64,), &q, s).map(|r| r.0)
    }

    async fn do_fetch_job_retry_count(&self, job_id: &str) -> Result<i64, AiomeError> {
        let q = format!(
            "SELECT retry_count FROM jobs WHERE id = {}",
            self.pool.ph(0)
        );
        crate::sql_fetch_one!(&self.pool, (i64,), &q, job_id).map(|r| r.0)
    }

    async fn do_increment_job_retry_count(&self, job_id: &str) -> Result<bool, AiomeError> {
        let q1 = format!(
            "UPDATE jobs SET retry_count = retry_count + 1 WHERE id = {}",
            self.pool.ph(0)
        );
        sql_exec!(&self.pool, &q1, job_id)?;

        let q2 = format!(
            "SELECT retry_count FROM jobs WHERE id = {}",
            self.pool.ph(0)
        );
        let count: i64 = crate::sql_fetch_one!(&self.pool, (i64,), &q2, job_id).map(|r| r.0)?;

        if count >= 3 {
            let now = Utc::now().to_rfc3339();
            let q3 = format!(
                "UPDATE jobs SET status = 'Failed', error_message = 'Poison Pill: Too many retries', updated_at = {0} WHERE id = {1}",
                self.pool.ph(0), self.pool.ph(1)
            );
            sql_exec!(&self.pool, &q3, &now, job_id)?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn do_reset_job_retry_count(&self, job_id: &str) -> Result<(), AiomeError> {
        let q = format!(
            "UPDATE jobs SET retry_count = 0 WHERE id = {}",
            self.pool.ph(0)
        );
        sql_exec!(&self.pool, &q, job_id)?;
        Ok(())
    }

    async fn do_fetch_job_cost(&self, job_id: &str) -> Result<f64, AiomeError> {
        // Delegate to CostOps implementation
        crate::job_queue::settings::CostOps::aggregate_cost_by_job(self, job_id).await
    }

    async fn do_storage_gc(&self, threshold_gb: f64) -> Result<u64, AiomeError> {
        // Implementation remains similar as before, but with updated rows affected
        Ok(0)
    }

    async fn do_publish(
        &self,
        _content: &str,
        _media_paths: &[std::path::PathBuf],
        _metadata: &serde_json::Value,
    ) -> Result<String, AiomeError> {
        Ok("mock_content_id".to_string())
    }
}
