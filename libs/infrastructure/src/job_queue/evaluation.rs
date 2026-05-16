/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::UniversalJobQueue;
use crate::{sql_exec, sql_fetch_all, sql_fetch_one, sql_fetch_optional};
use aiome_core::contracts::OracleVerdict;
use aiome_core::error::AiomeError;
use aiome_core::traits::{Job, JobQueue, JobStatus, SnsMetricsRecord};
use async_trait::async_trait;
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
        repost_count: Option<i64>,
        quote_count: Option<i64>,
        reply_count: Option<i64>,
        impression_count: Option<i64>,
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
    async fn do_link_sns_data(
        &self,
        job_id: &str,
        platform: &str,
        content_id: &str,
    ) -> Result<(), AiomeError>;
}

#[async_trait]
impl EvaluationOps for UniversalJobQueue {
    async fn do_link_sns_data(
        &self,
        job_id: &str,
        platform: &str,
        content_id: &str,
    ) -> Result<(), AiomeError> {
        const Q_SQLITE: &str = "UPDATE jobs SET sns_platform = ?, sns_content_id = ?, published_at = datetime('now') WHERE id = ?";
        const Q_PG: &str = "UPDATE jobs SET sns_platform = $1, sns_content_id = $2, published_at = NOW() WHERE id = $3";

        crate::sql_exec!(
            &self.pool,
            sqlite: Q_SQLITE,
            pg: Q_PG,
            platform,
            content_id,
            job_id
        )?;
        Ok(())
    }

    async fn do_fetch_jobs_for_evaluation(
        &self,
        milestone_days: i64,
        limit: i64,
    ) -> Result<Vec<Job>, AiomeError> {
        const Q_SQLITE: &str = "SELECT * FROM jobs
              WHERE sns_platform IS NOT NULL
              AND sns_content_id IS NOT NULL
              AND published_at IS NOT NULL
              AND published_at <= datetime('now', ? || ' days')
              AND id NOT IN (SELECT job_id FROM sns_metrics_history WHERE milestone_days = ?)
              ORDER BY published_at ASC LIMIT ?";

        const Q_PG: &str = "SELECT * FROM jobs
              WHERE sns_platform IS NOT NULL
              AND sns_content_id IS NOT NULL
              AND published_at IS NOT NULL
              AND published_at <= NOW() + ($1 * INTERVAL '1 day')
              AND id NOT IN (SELECT job_id FROM sns_metrics_history WHERE milestone_days = $1)
              ORDER BY published_at ASC LIMIT $2";

        let mut jobs = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(Q_SQLITE)
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
                        karma_directives: r.try_get("karma_directives").ok(),
                        status: JobStatus::from_string(r.get::<String, _>("status")),
                        started_at: r.try_get("started_at").ok(),
                        last_heartbeat: r.try_get("last_heartbeat").ok(),
                        tech_karma_extracted: tech_karma_extracted != 0,
                        creative_rating: r.try_get("creative_rating").ok(),
                        execution_log: r.try_get("execution_log").ok(),
                        error_message: r.try_get("error_message").ok(),
                        sns_platform: r.try_get("sns_platform").ok(),
                        sns_content_id: r.try_get("sns_content_id").ok(),
                        published_at: r.try_get("published_at").ok(),
                        output_artifacts: r.try_get("output_artifacts").ok(),
                        permission_manifest,
                        agent_id: None,
                        priority: r.get("priority"),
                        created_at: r.try_get("created_at").unwrap_or_default(),
                        updated_at: r.try_get("updated_at").unwrap_or_default(),
                        requires_review: r.try_get::<bool, _>("requires_review").unwrap_or(false),
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(Q_PG)
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
                        karma_directives: r.try_get("karma_directives").ok(),
                        status: JobStatus::from_string(r.get::<String, _>("status")),
                        started_at: r.try_get("started_at").ok(),
                        last_heartbeat: r.try_get("last_heartbeat").ok(),
                        tech_karma_extracted: tech_karma_extracted != 0,
                        creative_rating: r.try_get("creative_rating").ok(),
                        execution_log: r.try_get("execution_log").ok(),
                        error_message: r.try_get("error_message").ok(),
                        sns_platform: r.try_get("sns_platform").ok(),
                        sns_content_id: r.try_get("sns_content_id").ok(),
                        published_at: r.try_get("published_at").ok(),
                        output_artifacts: r.try_get("output_artifacts").ok(),
                        permission_manifest,
                        agent_id: None,
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

    async fn do_record_sns_metrics(
        &self,
        job_id: &str,
        milestone_days: i64,
        views: i64,
        likes: i64,
        comments_count: i64,
        raw_comments: Option<&str>,
        repost_count: Option<i64>,
        quote_count: Option<i64>,
        reply_count: Option<i64>,
        impression_count: Option<i64>,
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

        const Q_SQLITE: &str = "INSERT INTO sns_metrics_history (job_id, milestone_days, views, likes, comments_count, raw_comments_json, hard_metric_score, engagement_rate, repost_count, quote_count, reply_count, impression_count, recorded_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))";
        const Q_PG: &str = "INSERT INTO sns_metrics_history (job_id, milestone_days, views, likes, comments_count, raw_comments_json, hard_metric_score, engagement_rate, repost_count, quote_count, reply_count, impression_count, recorded_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())";

        crate::sql_exec!(
            &self.pool,
            sqlite: Q_SQLITE,
            pg: Q_PG,
            job_id,
            milestone_days,
            views,
            likes,
            comments_count,
            raw_comments,
            hard_metric_score,
            engagement_rate,
            repost_count.unwrap_or(0),
            quote_count.unwrap_or(0),
            reply_count.unwrap_or(0),
            impression_count.unwrap_or(0)
        )?;
        Ok(())
    }

    async fn do_fetch_pending_evaluations(
        &self,
        limit: i64,
    ) -> Result<Vec<SnsMetricsRecord>, AiomeError> {
        const Q_SQLITE: &str = "SELECT id, job_id, milestone_days, views, likes, comments_count, raw_comments_json, hard_metric_score, engagement_rate FROM sns_metrics_history WHERE is_finalized = 0 ORDER BY recorded_at ASC LIMIT ?";
        const Q_PG: &str = "SELECT id, job_id, milestone_days, views, likes, comments_count, raw_comments_json, hard_metric_score, engagement_rate FROM sns_metrics_history WHERE is_finalized = 0 ORDER BY recorded_at ASC LIMIT $1";

        let mut records = Vec::new();
        let rows = crate::sql_fetch_all!(
            &self.pool,
            (
                i64,
                String,
                i64,
                i64,
                i64,
                i64,
                Option<String>,
                Option<f64>,
                Option<f64>
            ),
            sqlite: Q_SQLITE,
            pg: Q_PG,
            limit
        )
        .unwrap_or_default();
        for r in rows {
            records.push(SnsMetricsRecord {
                id: r.0,
                job_id: r.1,
                milestone_days: r.2,
                views: r.3,
                likes: r.4,
                comments_count: r.5,
                raw_comments_json: r.6,
                hard_metric_score: r.7,
                engagement_rate: r.8,
            });
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
        const Q1_SQLITE: &str = "UPDATE sns_metrics_history SET alignment_score = ?, growth_score = ?, lesson = ?, should_evolve = ?, oracle_reason = ?, is_finalized = 1 WHERE id = ?";
        const Q1_PG: &str = "UPDATE sns_metrics_history SET alignment_score = $1, growth_score = $2, lesson = $3, should_evolve = $4, oracle_reason = $5, is_finalized = 1 WHERE id = $6";

        crate::sql_tx_exec!(
            &mut tx,
            sqlite: Q1_SQLITE,
            pg: Q1_PG,
            verdict.alignment_score,
            verdict.growth_score,
            &verdict.lesson,
            verdict.should_evolve as i32,
            &verdict.reasoning,
            record_id
        )?;

        const Q2_SQLITE: &str = "SELECT j.id, j.topic, j.style_name, h.milestone_days FROM jobs j JOIN sns_metrics_history h ON j.id = h.job_id WHERE h.id = ?";
        const Q2_PG: &str = "SELECT j.id, j.topic, j.style_name, h.milestone_days FROM jobs j JOIN sns_metrics_history h ON j.id = h.job_id WHERE h.id = $1";

        let (job_id, style_name, milestone_days) = crate::sql_tx_fetch_one!(
            &mut tx,
            (String, String, String, i64),
            sqlite: Q2_SQLITE,
            pg: Q2_PG,
            record_id
        )
        .map(|r| (r.0, r.2, r.3))?;

        if milestone_days == 30 {
            let avg_score = (verdict.alignment_score + verdict.growth_score) / 2.0;
            let weight = ((avg_score * 100.0) as i64).clamp(0, 100);
            let karma_id = Uuid::new_v4().to_string();
            let (domain, subtopic) = verdict
                .classification
                .as_ref()
                .map(|c| (Some(c.domain.as_str()), Some(c.subtopic.as_str())))
                .unwrap_or((None, None));
            const Q3_SQLITE: &str = "INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, domain, subtopic) VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'), ?, ?)";
            const Q3_PG: &str = "INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, domain, subtopic) VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), $8, $9)";

            crate::sql_tx_exec!(
                &mut tx,
                sqlite: Q3_SQLITE,
                pg: Q3_PG,
                &karma_id,
                &job_id,
                "Synthesized",
                &style_name,
                &verdict.lesson,
                weight,
                soul_hash,
                domain.unwrap_or("general"),
                subtopic
            )?;
        }

        if verdict.should_evolve {
            const Q4_SQLITE: &str = "UPDATE agent_stats SET exp = exp + 10, resonance = resonance + 5, updated_at = datetime('now') WHERE id = 1";
            const Q4_PG: &str = "UPDATE agent_stats SET exp = exp + 10, resonance = resonance + 5, updated_at = NOW() WHERE id = 1";

            if let Err(e) = crate::sql_tx_exec!(
                &mut tx,
                sqlite: Q4_SQLITE,
                pg: Q4_PG
            ) {
                tracing::warn!("⚠️ [Evaluation] Failed to update agent_stats: {:?}", e);
            }
        }
        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to commit evaluation: {}", e),
        })?;
        Ok(())
    }

    async fn do_fetch_top_performing_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        const Q_SQLITE: &str = "SELECT j.* FROM jobs j JOIN sns_metrics_history s ON j.id = s.job_id WHERE s.is_finalized = 1 ORDER BY s.views DESC LIMIT ?";
        const Q_PG: &str = "SELECT j.* FROM jobs j JOIN sns_metrics_history s ON j.id = s.job_id WHERE s.is_finalized = 1 ORDER BY s.views DESC LIMIT $1";

        let mut jobs = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(Q_SQLITE)
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
                        karma_directives: r.try_get("karma_directives").ok(),
                        status: JobStatus::from_string(r.get::<String, _>("status")),
                        started_at: r.try_get("started_at").ok(),
                        last_heartbeat: r.try_get("last_heartbeat").ok(),
                        tech_karma_extracted: tech_karma_extracted != 0,
                        creative_rating: r.try_get("creative_rating").ok(),
                        execution_log: r.try_get("execution_log").ok(),
                        error_message: r.try_get("error_message").ok(),
                        sns_platform: r.try_get("sns_platform").ok(),
                        sns_content_id: r.try_get("sns_content_id").ok(),
                        published_at: r.try_get("published_at").ok(),
                        output_artifacts: r.try_get("output_artifacts").ok(),
                        permission_manifest,
                        agent_id: None,
                        priority: r.get("priority"),
                        created_at: r.try_get("created_at").unwrap_or_default(),
                        updated_at: r.try_get("updated_at").unwrap_or_default(),
                        requires_review: r.try_get::<bool, _>("requires_review").unwrap_or(false),
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(Q_PG)
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
                        karma_directives: r.try_get("karma_directives").ok(),
                        status: JobStatus::from_string(r.get::<String, _>("status")),
                        started_at: r.try_get("started_at").ok(),
                        last_heartbeat: r.try_get("last_heartbeat").ok(),
                        tech_karma_extracted: tech_karma_extracted != 0,
                        creative_rating: r.try_get("creative_rating").ok(),
                        execution_log: r.try_get("execution_log").ok(),
                        error_message: r.try_get("error_message").ok(),
                        sns_platform: r.try_get("sns_platform").ok(),
                        sns_content_id: r.try_get("sns_content_id").ok(),
                        published_at: r.try_get("published_at").ok(),
                        output_artifacts: r.try_get("output_artifacts").ok(),
                        permission_manifest,
                        agent_id: None,
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
}
