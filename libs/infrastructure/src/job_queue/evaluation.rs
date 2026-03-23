/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::try_get_opt;
use super::UniversalJobQueue;
use aiome_core::contracts::OracleVerdict;
use aiome_core::error::AiomeError;
use aiome_core::traits::{Job, JobStatus, SnsMetricsRecord};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Row;
use tracing::warn;
use uuid::Uuid;

#[async_trait]
pub trait EvaluationOps {
    async fn do_fetch_jobs_for_evaluation(
        &self,
        milestone_days: i64,
        limit: i64,
    ) -> Result<Vec<Job>, AiomeError>;
    async fn do_record_sns_metrics(
        &self,
        job_id: &str,
        milestone_days: i64,
        views: i64,
        likes: i64,
        comments_count: i64,
        raw_comments: Option<&str>,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_pending_evaluations(
        &self,
        limit: i64,
    ) -> Result<Vec<SnsMetricsRecord>, AiomeError>;
    async fn do_apply_final_verdict(
        &self,
        record_id: i64,
        verdict: OracleVerdict,
        soul_hash: &str,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_top_performing_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError>;
}

#[async_trait]
impl EvaluationOps for UniversalJobQueue {
    async fn do_fetch_jobs_for_evaluation(
        &self,
        milestone_days: i64,
        limit: i64,
    ) -> Result<Vec<Job>, AiomeError> {
        let now_interval = self.pool.now_with_dynamic_days_interval(0);
        let q = format!(
            "SELECT id, category, topic, style_name, karma_directives, permission_manifest, status, started_at, last_heartbeat, 
                     tech_karma_extracted, creative_rating, execution_log, error_message,
                     sns_platform, sns_content_id, published_at, output_artifacts, priority 
              FROM jobs 
              WHERE sns_platform IS NOT NULL 
              AND sns_content_id IS NOT NULL 
              AND published_at IS NOT NULL
              AND published_at <= {0}
              AND id NOT IN (SELECT job_id FROM sns_metrics_history WHERE milestone_days = {1})
              ORDER BY published_at ASC LIMIT {2}",
            now_interval, self.pool.ph(1), self.pool.ph(2)
        );

        let mut jobs = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(&q)
                    .bind(format!("-{}", milestone_days))
                    .bind(milestone_days)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
                    let permission_manifest = r
                        .try_get::<String, _>("permission_manifest")
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok());
                    jobs.push(Job {
                        id: r.get("id"),
                        category: r.get("category"),
                        topic: r.get("topic"),
                        style: r.get("style_name"),
                        karma_directives: try_get_opt(&r, "karma_directives"),
                        status: JobStatus::from_string(r.get("status")),
                        started_at: try_get_opt(&r, "started_at"),
                        last_heartbeat: try_get_opt(&r, "last_heartbeat"),
                        tech_karma_extracted: tech_karma_extracted != 0,
                        creative_rating: r.try_get("creative_rating").ok(),
                        execution_log: try_get_opt(&r, "execution_log"),
                        error_message: try_get_opt(&r, "error_message"),
                        sns_platform: try_get_opt(&r, "sns_platform"),
                        sns_content_id: try_get_opt(&r, "sns_content_id"),
                        published_at: try_get_opt(&r, "published_at"),
                        output_artifacts: try_get_opt(&r, "output_artifacts"),
                        permission_manifest,
                        agent_id: None,
                        priority: r.get("priority"),
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(&q)
                    .bind(-(milestone_days as i32))
                    .bind(milestone_days)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
                    let permission_manifest = r
                        .try_get::<serde_json::Value, _>("permission_manifest")
                        .ok()
                        .and_then(|v| serde_json::from_value(v).ok());
                    jobs.push(Job {
                        id: r.get("id"),
                        category: r.get("category"),
                        topic: r.get("topic"),
                        style: r.get("style_name"),
                        karma_directives: try_get_opt(&r, "karma_directives"),
                        status: JobStatus::from_string(r.get("status")),
                        started_at: try_get_opt(&r, "started_at"),
                        last_heartbeat: try_get_opt(&r, "last_heartbeat"),
                        tech_karma_extracted: tech_karma_extracted != 0,
                        creative_rating: r.try_get("creative_rating").ok(),
                        execution_log: try_get_opt(&r, "execution_log"),
                        error_message: try_get_opt(&r, "error_message"),
                        sns_platform: try_get_opt(&r, "sns_platform"),
                        sns_content_id: try_get_opt(&r, "sns_content_id"),
                        published_at: try_get_opt(&r, "published_at"),
                        output_artifacts: try_get_opt(&r, "output_artifacts"),
                        permission_manifest,
                        agent_id: None,
                        priority: r.get("priority"),
                    });
                }
            }
        }
        Ok(jobs)
    }

    async fn do_record_sns_metrics(
        &self,
        job_id: &str,
        milestone_days: i64,
        views: i64,
        likes: i64,
        comments_count: i64,
        raw_comments: Option<&str>,
    ) -> Result<(), AiomeError> {
        let engagement_rate = if views > 0 {
            (likes as f64 / views as f64) * 100.0
        } else {
            0.0
        };
        let hard_metric_score = if engagement_rate >= 10.0 {
            1.0
        } else if engagement_rate >= 5.0 {
            0.5
        } else if engagement_rate >= 1.0 {
            0.0
        } else {
            -0.5
        };

        let q = format!(
            "INSERT INTO sns_metrics_history (job_id, milestone_days, views, likes, comments_count, raw_comments_json, hard_metric_score, engagement_rate, recorded_at) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6), self.pool.ph(7), self.pool.now_fn()
        );
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                sqlx::query(&q)
                    .bind(job_id)
                    .bind(milestone_days)
                    .bind(views)
                    .bind(likes)
                    .bind(comments_count)
                    .bind(raw_comments)
                    .bind(hard_metric_score)
                    .bind(engagement_rate)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to record SNS metrics: {}", e),
                    })?;
            }
            crate::db::DatabasePool::Postgres(p) => {
                sqlx::query(&q)
                    .bind(job_id)
                    .bind(milestone_days)
                    .bind(views)
                    .bind(likes)
                    .bind(comments_count)
                    .bind(raw_comments)
                    .bind(hard_metric_score)
                    .bind(engagement_rate)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to record SNS metrics: {}", e),
                    })?;
            }
        }
        Ok(())
    }

    async fn do_fetch_pending_evaluations(
        &self,
        limit: i64,
    ) -> Result<Vec<SnsMetricsRecord>, AiomeError> {
        let q = format!("SELECT id, job_id, milestone_days, views, likes, comments_count, raw_comments_json, hard_metric_score, engagement_rate FROM sns_metrics_history WHERE is_finalized = 0 ORDER BY recorded_at ASC LIMIT {}", self.pool.ph(0));
        let mut records = Vec::new();
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
                    records.push(SnsMetricsRecord {
                        id: r.get("id"),
                        job_id: r.get("job_id"),
                        milestone_days: r.get("milestone_days"),
                        views: r.get("views"),
                        likes: r.get("likes"),
                        comments_count: r.get("comments_count"),
                        raw_comments_json: try_get_opt(&r, "raw_comments_json"),
                        hard_metric_score: r.try_get("hard_metric_score").ok(),
                        engagement_rate: r.try_get("engagement_rate").ok(),
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
                    records.push(SnsMetricsRecord {
                        id: r.get("id"),
                        job_id: r.get("job_id"),
                        milestone_days: r.get("milestone_days"),
                        views: r.get("views"),
                        likes: r.get("likes"),
                        comments_count: r.get("comments_count"),
                        raw_comments_json: try_get_opt(&r, "raw_comments_json"),
                        hard_metric_score: r.try_get("hard_metric_score").ok(),
                        engagement_rate: r.try_get("engagement_rate").ok(),
                    });
                }
            }
        }
        Ok(records)
    }

    async fn do_apply_final_verdict(
        &self,
        record_id: i64,
        verdict: OracleVerdict,
        soul_hash: &str,
    ) -> Result<(), AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        let q1 = format!("UPDATE sns_metrics_history SET alignment_score = {0}, growth_score = {1}, lesson = {2}, should_evolve = {3}, oracle_reason = {4}, is_finalized = 1 WHERE id = {5}",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5));

        match &mut tx {
            crate::db::DatabaseTransaction::Sqlite(t) => {
                sqlx::query(&q1)
                    .bind(verdict.alignment_score)
                    .bind(verdict.growth_score)
                    .bind(&verdict.lesson)
                    .bind(verdict.should_evolve as i32)
                    .bind(&verdict.reasoning)
                    .bind(record_id)
                    .execute(&mut **t)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
            crate::db::DatabaseTransaction::Postgres(t) => {
                sqlx::query(&q1)
                    .bind(verdict.alignment_score)
                    .bind(verdict.growth_score)
                    .bind(&verdict.lesson)
                    .bind(verdict.should_evolve as i32)
                    .bind(&verdict.reasoning)
                    .bind(record_id)
                    .execute(&mut **t)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
        }

        let q2 = format!("SELECT j.id, j.topic, j.style_name, h.milestone_days FROM jobs j JOIN sns_metrics_history h ON j.id = h.job_id WHERE h.id = {}", self.pool.ph(0));
        let (job_id, style_name, milestone_days) = match &mut tx {
            crate::db::DatabaseTransaction::Sqlite(t) => {
                let r = sqlx::query(&q2)
                    .bind(record_id)
                    .fetch_one(&mut **t)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                (
                    r.get::<String, _>("id"),
                    r.get::<String, _>("style_name"),
                    r.get::<i64, _>("milestone_days"),
                )
            }
            crate::db::DatabaseTransaction::Postgres(t) => {
                let r = sqlx::query(&q2)
                    .bind(record_id)
                    .fetch_one(&mut **t)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                (
                    r.get::<String, _>("id"),
                    r.get::<String, _>("style_name"),
                    r.get::<i64, _>("milestone_days"),
                )
            }
        };

        if milestone_days == 30 {
            let avg_score = (verdict.alignment_score + verdict.growth_score) / 2.0;
            let weight = ((avg_score * 100.0) as i64).clamp(0, 100);
            let karma_id = Uuid::new_v4().to_string();
            let (domain, subtopic) = verdict
                .classification
                .as_ref()
                .map(|c| (Some(c.domain.as_str()), Some(c.subtopic.as_str())))
                .unwrap_or((None, None));
            let q3 = format!("INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, domain, subtopic, clone_origin_id) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, NULL)",
                self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6), self.pool.now_fn(), self.pool.ph(7), self.pool.ph(8));

            match &mut tx {
                crate::db::DatabaseTransaction::Sqlite(t) => {
                    sqlx::query(&q3)
                        .bind(&karma_id)
                        .bind(&job_id)
                        .bind("Synthesized")
                        .bind(&style_name)
                        .bind(&verdict.lesson)
                        .bind(weight)
                        .bind(soul_hash)
                        .bind(domain.unwrap_or("general"))
                        .bind(subtopic)
                        .execute(&mut **t)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                }
                crate::db::DatabaseTransaction::Postgres(t) => {
                    sqlx::query(&q3)
                        .bind(&karma_id)
                        .bind(&job_id)
                        .bind("Synthesized")
                        .bind(&style_name)
                        .bind(&verdict.lesson)
                        .bind(weight)
                        .bind(soul_hash)
                        .bind(domain.unwrap_or("general"))
                        .bind(subtopic)
                        .execute(&mut **t)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                }
            }
        }

        if verdict.should_evolve {
            let q4 = format!("UPDATE agent_stats SET exp = exp + 10, resonance = resonance + 5, updated_at = {} WHERE id = 1", self.pool.now_fn());
            match &mut tx {
                crate::db::DatabaseTransaction::Sqlite(t) => {
                    let _ = sqlx::query(&q4).execute(&mut **t).await;
                }
                crate::db::DatabaseTransaction::Postgres(t) => {
                    let _ = sqlx::query(&q4).execute(&mut **t).await;
                }
            }
        }

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to commit evaluation: {}", e),
        })?;
        Ok(())
    }

    async fn do_fetch_top_performing_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        let q = format!("SELECT j.* FROM jobs j JOIN sns_metrics_history s ON j.id = s.job_id WHERE s.is_finalized = 1 ORDER BY s.views DESC LIMIT {}", self.pool.ph(0));
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
                    let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
                    let permission_manifest = r
                        .try_get::<String, _>("permission_manifest")
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok());
                    jobs.push(Job {
                        id: r.get("id"),
                        category: r.get("category"),
                        topic: r.get("topic"),
                        style: r.get("style_name"),
                        karma_directives: try_get_opt(&r, "karma_directives"),
                        status: JobStatus::from_string(r.get("status")),
                        started_at: try_get_opt(&r, "started_at"),
                        last_heartbeat: try_get_opt(&r, "last_heartbeat"),
                        tech_karma_extracted: tech_karma_extracted != 0,
                        creative_rating: r.try_get("creative_rating").ok(),
                        execution_log: try_get_opt(&r, "execution_log"),
                        error_message: try_get_opt(&r, "error_message"),
                        sns_platform: try_get_opt(&r, "sns_platform"),
                        sns_content_id: try_get_opt(&r, "sns_content_id"),
                        published_at: try_get_opt(&r, "published_at"),
                        output_artifacts: try_get_opt(&r, "output_artifacts"),
                        permission_manifest,
                        agent_id: None,
                        priority: r.get("priority"),
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
                    let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
                    let permission_manifest = r
                        .try_get::<serde_json::Value, _>("permission_manifest")
                        .ok()
                        .and_then(|v| serde_json::from_value(v).ok());
                    jobs.push(Job {
                        id: r.get("id"),
                        category: r.get("category"),
                        topic: r.get("topic"),
                        style: r.get("style_name"),
                        karma_directives: try_get_opt(&r, "karma_directives"),
                        status: JobStatus::from_string(r.get::<String, _>("status").as_str()),
                        started_at: try_get_opt(&r, "started_at"),
                        last_heartbeat: try_get_opt(&r, "last_heartbeat"),
                        tech_karma_extracted: tech_karma_extracted != 0,
                        creative_rating: r.try_get("creative_rating").ok(),
                        execution_log: try_get_opt(&r, "execution_log"),
                        error_message: try_get_opt(&r, "error_message"),
                        sns_platform: try_get_opt(&r, "sns_platform"),
                        sns_content_id: try_get_opt(&r, "sns_content_id"),
                        published_at: try_get_opt(&r, "published_at"),
                        output_artifacts: try_get_opt(&r, "output_artifacts"),
                        permission_manifest,
                        agent_id: None,
                        priority: r.get("priority"),
                    });
                }
            }
        }
        Ok(jobs)
    }
}
