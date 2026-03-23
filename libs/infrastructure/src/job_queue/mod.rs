/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::contracts::{ArenaMatch, FederatedKarma, ImmuneRule, OracleVerdict};
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::EmbeddingProvider;
use aiome_core::traits::KarmaSearchResult;
use aiome_core::traits::{Job, JobQueue, SnsMetricsRecord};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tracing::info;

macro_rules! sql_exec {
    ($pool:expr, $q:expr $(, $bind:expr)*) => {
        match $pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let mut q = sqlx::query($q);
                $(q = q.bind($bind);)*
                q.execute(p).await.map(|r| r.rows_affected())
            }
            crate::db::DatabasePool::Postgres(p) => {
                let mut q = sqlx::query($q);
                $(q = q.bind($bind);)*
                q.execute(p).await.map(|r| r.rows_affected())
            }
        }
    };
}

#[cfg(test)]
pub(crate) mod tests;

mod core_ops;
/// `crdt` モジュール
pub mod crdt;
mod evaluation;
mod evolution;
mod expression;
/// `federation` モジュール
pub mod federation;
mod guardrails;
mod karma;
mod karma_maintenance;
mod migrations;
mod postgres_init;
/// `settings` モジュール
pub mod settings;
mod swarm;
mod taxonomy;
mod trajectory_store;
mod watchtower;

use core_ops::CoreOps;
use crdt::CrdtOps;
use evaluation::EvaluationOps;
use evolution::EvolutionOps;
use expression::ExpressionOps;
use federation::FederationOps;
use guardrails::GuardrailOps;
use karma::KarmaOps;
use migrations::DbInitializer;
use settings::SettingsOps;
use swarm::SwarmOps;
use trajectory_store::TrajectoryOps;
use watchtower::WatchtowerOps;

/// Job Queue that utilizes multi-backend (SQLite/Postgres) database.
#[derive(Clone, Debug)]
pub struct UniversalJobQueue {
    pub(crate) pool: crate::db::DatabasePool,
    pub(crate) embed_provider: Arc<tokio::sync::RwLock<Option<Arc<dyn EmbeddingProvider>>>>,
    pub(crate) karma_cache: Arc<tokio::sync::RwLock<HashMap<String, (KarmaSearchResult, Instant)>>>,
}

impl UniversalJobQueue {
    /// `get_pool` を実行する
    pub fn get_pool(&self) -> &crate::db::DatabasePool {
        &self.pool
    }

    /// `get_embedding_provider` を実行する
    pub async fn get_embedding_provider(&self) -> Option<Arc<dyn EmbeddingProvider>> {
        self.embed_provider.read().await.clone()
    }

    /// Connects to the SQLite database by default (Legacy compatibility)
    pub async fn new(db_path: &str) -> Result<Self, AiomeError> {
        if db_path.starts_with("postgres://") || db_path.starts_with("postgresql://") {
            Self::new_postgres(db_path).await
        } else {
            Self::new_sqlite(db_path).await
        }
    }

    /// Try to initialize the backend with a SQLite pool explicitly
    pub async fn new_sqlite(db_path: &str) -> Result<Self, AiomeError> {
        let pool = crate::db::DatabasePool::new_sqlite(db_path).await?;
        let instance = Self {
            pool,
            embed_provider: Arc::new(tokio::sync::RwLock::new(None)),
            karma_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        };
        DbInitializer::init_db(&instance).await?;
        Ok(instance)
    }

    /// Try to initialize the backend with a PostgreSQL pool explicitly
    pub async fn new_postgres(url: &str) -> Result<Self, AiomeError> {
        let pool = crate::db::DatabasePool::new_postgres(url).await?;
        let instance = Self {
            pool,
            embed_provider: Arc::new(tokio::sync::RwLock::new(None)),
            karma_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        };

        // Initialize Postgres schema (Initial Phase)
        if let crate::db::DatabasePool::Postgres(ref p) = instance.pool {
            postgres_init::PostgresInitializer::init_db(p).await?;
        }

        Ok(instance)
    }

    /// `set_embedding_provider` を実行する
    pub async fn set_embedding_provider(&self, provider: Arc<dyn EmbeddingProvider>) {
        let mut w = self.embed_provider.write().await;
        *w = Some(provider);
    }

    /// 埋め込みプロバイダを設定する
    pub fn with_embeddings(self, provider: Arc<dyn EmbeddingProvider>) -> Self {
        Self {
            pool: self.pool,
            embed_provider: Arc::new(tokio::sync::RwLock::new(Some(provider))),
            karma_cache: self.karma_cache,
        }
    }
}

#[async_trait]
impl JobQueue for UniversalJobQueue {
    async fn enqueue(
        &self,
        category: &str,
        topic: &str,
        style: &str,
        karma_directives: Option<&str>,
        permission_manifest: Option<aiome_core::security::PermissionManifest>,
        agent_id: Option<uuid::Uuid>,
        priority: i32,
    ) -> Result<String, AiomeError> {
        Box::pin(self.do_enqueue(
            category,
            topic,
            style,
            karma_directives,
            permission_manifest,
            agent_id,
            priority,
        ))
        .await
    }

    async fn fetch_job(&self, job_id: &str) -> Result<Option<Job>, AiomeError> {
        Box::pin(self.do_fetch_job(job_id)).await
    }

    async fn dequeue(&self, capable_categories: &[&str]) -> Result<Option<Job>, AiomeError> {
        Box::pin(self.do_dequeue(capable_categories)).await
    }

    async fn complete_job(
        &self,
        job_id: &str,
        output_artifacts: Option<&str>,
    ) -> Result<(), AiomeError> {
        Box::pin(self.do_complete_job(job_id, output_artifacts)).await
    }

    async fn fail_job(&self, job_id: &str, reason: &str) -> Result<(), AiomeError> {
        Box::pin(self.do_fail_job(job_id, reason)).await
    }

    async fn reclaim_zombie_jobs(&self, timeout_minutes: i64) -> Result<u64, AiomeError> {
        Box::pin(self.do_reclaim_zombie_jobs(timeout_minutes)).await
    }

    async fn set_creative_rating(&self, job_id: &str, rating: i32) -> Result<(), AiomeError> {
        Box::pin(self.do_set_creative_rating(job_id, rating)).await
    }

    async fn heartbeat_pulse(&self, job_id: &str) -> Result<(), AiomeError> {
        Box::pin(self.do_heartbeat_pulse(job_id)).await
    }

    async fn store_execution_log(&self, job_id: &str, log: &str) -> Result<(), AiomeError> {
        Box::pin(self.do_store_execution_log(job_id, log)).await
    }

    async fn fetch_relevant_karma(
        &self,
        topic: &str,
        skill_id: &str,
        limit: i64,
        current_soul_hash: &str,
    ) -> Result<aiome_core::traits::KarmaSearchResult, AiomeError> {
        Box::pin(self.do_fetch_relevant_karma(topic, skill_id, limit, current_soul_hash)).await
    }

    async fn store_karma(
        &self,
        job_id: &str,
        skill_id: &str,
        lesson: &str,
        karma_type: &str,
        soul_hash: &str,
        domain: Option<&str>,
        subtopic: Option<&str>,
        clone_origin_id: Option<&str>,
    ) -> Result<(), AiomeError> {
        Box::pin(self.do_store_karma(
            job_id,
            skill_id,
            lesson,
            karma_type,
            soul_hash,
            domain,
            subtopic,
            clone_origin_id,
        ))
        .await
    }

    async fn adjust_karma_weight(&self, karma_id: &str, delta: i32) -> Result<(), AiomeError> {
        Box::pin(self.do_adjust_karma_weight(karma_id, delta)).await
    }

    async fn karma_decay_sweep(&self) -> Result<u64, AiomeError> {
        Box::pin(self.do_karma_decay_sweep()).await
    }

    async fn fetch_undistilled_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        Box::pin(self.do_fetch_undistilled_jobs(limit)).await
    }

    async fn mark_karma_extracted(&self, job_id: &str) -> Result<(), AiomeError> {
        Box::pin(self.do_mark_karma_extracted(job_id)).await
    }

    async fn purge_old_jobs(&self, days: i64) -> Result<u64, AiomeError> {
        Box::pin(self.do_purge_old_jobs(days)).await
    }

    async fn link_sns_data(
        &self,
        job_id: &str,
        platform: &str,
        content_id: &str,
    ) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        let q = format!("UPDATE jobs SET sns_platform = {0}, sns_content_id = {1}, published_at = {2}, updated_at = {3} WHERE id = {4}", self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4));
        sql_exec!(&self.pool, &q, platform, content_id, &now, &now, job_id).map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("Failed to link SNS data for job {}: {}", job_id, e),
            }
        })?;
        Ok(())
    }

    async fn fetch_jobs_for_evaluation(
        &self,
        milestone_days: i64,
        limit: i64,
    ) -> Result<Vec<Job>, AiomeError> {
        Box::pin(self.do_fetch_jobs_for_evaluation(milestone_days, limit)).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn record_sns_metrics(
        &self,
        job_id: &str,
        milestone_days: i64,
        views: i64,
        likes: i64,
        comments_count: i64,
        raw_comments: Option<&str>,
    ) -> Result<(), AiomeError> {
        Box::pin(self.do_record_sns_metrics(
            job_id,
            milestone_days,
            views,
            likes,
            comments_count,
            raw_comments,
        ))
        .await
    }

    async fn fetch_pending_evaluations(
        &self,
        limit: i64,
    ) -> Result<Vec<SnsMetricsRecord>, AiomeError> {
        Box::pin(self.do_fetch_pending_evaluations(limit)).await
    }

    async fn apply_final_verdict(
        &self,
        record_id: i64,
        verdict: OracleVerdict,
        soul_hash: &str,
    ) -> Result<(), AiomeError> {
        Box::pin(self.do_apply_final_verdict(record_id, verdict, soul_hash)).await
    }

    async fn fetch_recent_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        Box::pin(self.do_fetch_recent_jobs(limit)).await
    }

    async fn get_agent_stats(&self) -> Result<shared::watchtower::AgentStats, AiomeError> {
        Box::pin(self.do_get_agent_stats()).await
    }

    async fn add_resonance(&self, amount: i32) -> Result<(), AiomeError> {
        Box::pin(self.do_add_resonance(amount)).await
    }

    async fn add_tech_exp(&self, amount: i32) -> Result<(), AiomeError> {
        Box::pin(self.do_add_tech_exp(amount)).await
    }

    async fn add_creativity(&self, amount: i32) -> Result<(), AiomeError> {
        Box::pin(self.do_add_creativity(amount)).await
    }

    async fn sync_samsara_level(
        &self,
    ) -> Result<Option<aiome_core::contracts::SamsaraEvent>, AiomeError> {
        Box::pin(self.do_sync_samsara_level()).await
    }

    async fn record_evolution_event(
        &self,
        level: i32,
        event_type: &str,
        description: &str,
        inspiration: Option<&str>,
        karma_json: Option<&str>,
    ) -> Result<(), AiomeError> {
        Box::pin(self.do_record_evolution_event(
            level,
            event_type,
            description,
            inspiration,
            karma_json,
        ))
        .await
    }

    async fn fetch_evolution_history(
        &self,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        Box::pin(self.do_fetch_evolution_history(limit)).await
    }

    async fn get_pending_job_count(&self) -> Result<i64, AiomeError> {
        Box::pin(self.do_get_pending_job_count()).await
    }

    async fn get_job_count_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<i64, AiomeError> {
        Box::pin(self.do_get_job_count_since(since)).await
    }

    async fn fetch_all_karma(&self, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
        Box::pin(self.do_fetch_all_karma(limit)).await
    }

    async fn fetch_top_performing_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        Box::pin(self.do_fetch_top_performing_jobs(limit)).await
    }

    async fn record_soul_mutation(
        &self,
        old_hash: &str,
        new_hash: &str,
        reason: &str,
    ) -> Result<(), AiomeError> {
        Box::pin(self.do_record_soul_mutation(old_hash, new_hash, reason)).await
    }

    async fn fetch_job_retry_count(&self, job_id: &str) -> Result<i64, AiomeError> {
        Box::pin(self.do_fetch_job_retry_count(job_id)).await
    }

    async fn reset_job_retry_count(&self, job_id: &str) -> Result<(), AiomeError> {
        Box::pin(self.do_reset_job_retry_count(job_id)).await
    }

    async fn increment_job_retry_count(&self, job_id: &str) -> Result<bool, AiomeError> {
        Box::pin(self.do_increment_job_retry_count(job_id)).await
    }

    async fn fetch_unincorporated_karma(
        &self,
        limit: i64,
        current_soul_hash: &str,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        Box::pin(self.do_fetch_unincorporated_karma(limit, current_soul_hash)).await
    }

    async fn mark_karma_as_incorporated(
        &self,
        karma_ids: Vec<String>,
        new_soul_hash: &str,
    ) -> Result<(), AiomeError> {
        Box::pin(self.do_mark_karma_as_incorporated(karma_ids, new_soul_hash)).await
    }

    async fn store_immune_rule(&self, rule: &ImmuneRule) -> Result<(), AiomeError> {
        Box::pin(self.do_store_immune_rule(rule)).await
    }

    async fn delete_immune_rule(&self, rule_id: &str) -> Result<(), AiomeError> {
        Box::pin(self.do_delete_immune_rule(rule_id)).await
    }

    async fn fetch_active_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        Box::pin(self.do_fetch_active_immune_rules()).await
    }

    async fn record_arena_match(&self, match_data: &ArenaMatch) -> Result<(), AiomeError> {
        Box::pin(self.do_record_arena_match(match_data)).await
    }

    async fn export_federated_data(
        &self,
        since: Option<&str>,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>, Vec<ArenaMatch>), AiomeError> {
        Box::pin(self.do_export_federated_data(since)).await
    }

    async fn import_federated_data(
        &self,
        karmas: Vec<FederatedKarma>,
        rules: Vec<ImmuneRule>,
        matches: Vec<ArenaMatch>,
    ) -> Result<(), AiomeError> {
        Box::pin(self.do_import_federated_data(karmas, rules, matches)).await
    }

    async fn get_peer_sync_time(&self, peer_url: &str) -> Result<Option<String>, AiomeError> {
        Box::pin(self.do_get_peer_sync_time(peer_url)).await
    }

    async fn fetch_federated_metrics(
        &self,
    ) -> Result<aiome_core::contracts::FederatedMetrics, AiomeError> {
        Box::pin(self.do_fetch_federated_metrics()).await
    }

    async fn update_peer_sync_time(
        &self,
        peer_url: &str,
        sync_time: &str,
    ) -> Result<(), AiomeError> {
        Box::pin(self.do_update_peer_sync_time(peer_url, sync_time)).await
    }

    async fn get_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        Box::pin(self.do_get_immune_rules()).await
    }

    async fn get_node_id(&self) -> Result<String, AiomeError> {
        Box::pin(self.do_get_node_id()).await
    }

    async fn sign_swarm_payload(&self, payload: &str) -> Result<String, AiomeError> {
        Box::pin(self.do_sign_swarm_payload(payload)).await
    }

    async fn sync_local_clock(&self, remote_clock: u64) -> Result<u64, AiomeError> {
        Box::pin(self.do_sync_local_clock(remote_clock)).await
    }

    async fn get_system_agent_id(&self) -> Result<uuid::Uuid, AiomeError> {
        Box::pin(self.do_get_system_agent_id()).await
    }

    async fn tick_local_clock(&self) -> Result<u64, AiomeError> {
        Box::pin(self.do_tick_local_clock()).await
    }

    async fn storage_gc(&self, threshold_gb: f64) -> Result<u64, AiomeError> {
        Box::pin(self.do_storage_gc(threshold_gb)).await
    }

    // --- Chat & Memory (The Soul Persistence) ---
    async fn store_chat_message(
        &self,
        channel_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), AiomeError> {
        Box::pin(self.do_insert_chat_message(channel_id, role, content)).await
    }

    async fn fetch_chat_history(
        &self,
        channel_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        Box::pin(self.do_fetch_chat_history(channel_id, limit)).await
    }

    async fn get_biome_topic_status(
        &self,
        topic_id: &str,
    ) -> Result<Option<(i32, Option<String>)>, AiomeError> {
        let row: Option<(i32, Option<String>)> = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query(
                "SELECT turn_count, cooldown_until FROM biome_topics WHERE topic_id = ?",
            )
            .bind(topic_id)
            .fetch_optional(p)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?
            .map(|r| {
                (
                    r.get("turn_count"),
                    r.get::<Option<String>, _>("cooldown_until"),
                )
            }),
            crate::db::DatabasePool::Postgres(p) => sqlx::query(
                "SELECT turn_count, cooldown_until FROM biome_topics WHERE topic_id = $1",
            )
            .bind(topic_id)
            .fetch_optional(p)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?
            .map(|r| {
                (
                    r.get("turn_count"),
                    r.get::<Option<String>, _>("cooldown_until"),
                )
            }),
        };

        Ok(row)
    }

    async fn advance_biome_turn(
        &self,
        topic_id: &str,
        cooldown_minutes: i64,
    ) -> Result<i32, AiomeError> {
        let now = chrono::Utc::now();
        let cooldown_until = (now + chrono::Duration::minutes(cooldown_minutes)).to_rfc3339();

        let q = format!("INSERT INTO biome_topics (topic_id, peer_pubkey, status, turn_count, cooldown_until, updated_at) VALUES ({0}, 'unknown_peer', 'Active', 1, {1}, {2}) ON CONFLICT(topic_id) DO UPDATE SET turn_count = biome_topics.turn_count + 1, cooldown_until = EXCLUDED.cooldown_until, updated_at = EXCLUDED.updated_at RETURNING turn_count", self.pool.ph(0), self.pool.ph(1), self.pool.now_fn());
        let turn_count: i32 = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar(&q)
                .bind(topic_id)
                .bind(&cooldown_until)
                .fetch_one(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar(&q)
                .bind(topic_id)
                .bind(&cooldown_until)
                .fetch_one(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
        };

        Ok(turn_count)
    }

    async fn fetch_biome_messages(
        &self,
        topic_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        let messages = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query("SELECT * FROM biome_messages WHERE topic_id = ? ORDER BY created_at DESC LIMIT ?")
                    .bind(topic_id).bind(limit).fetch_all(p).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
                rows.into_iter()
                    .map(|row| {
                        serde_json::json!({
                            "id": row.get::<i64, _>("id"),
                            "sender_pubkey": row.get::<String, _>("sender_pubkey"),
                            "recipient_pubkey": row.get::<String, _>("recipient_pubkey"),
                            "topic_id": row.get::<String, _>("topic_id"),
                            "content": row.get::<String, _>("content"),
                            "karma_root_cid": row.get::<String, _>("karma_root_cid"),
                            "signature": row.get::<String, _>("signature"),
                            "lamport_clock": row.get::<i64, _>("lamport_clock"),
                            "encryption": row.get::<String, _>("encryption"),
                            "created_at": row.get::<Option<String>, _>("created_at"),
                        })
                    })
                    .collect()
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query("SELECT * FROM biome_messages WHERE topic_id = $1 ORDER BY created_at DESC LIMIT $2")
                    .bind(topic_id).bind(limit).fetch_all(p).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
                rows.into_iter()
                    .map(|row| {
                        serde_json::json!({
                            "id": row.get::<i64, _>("id"),
                            "sender_pubkey": row.get::<String, _>("sender_pubkey"),
                            "recipient_pubkey": row.get::<String, _>("recipient_pubkey"),
                            "topic_id": row.get::<String, _>("topic_id"),
                            "content": row.get::<String, _>("content"),
                            "karma_root_cid": row.get::<String, _>("karma_root_cid"),
                            "signature": row.get::<String, _>("signature"),
                            "lamport_clock": row.get::<i64, _>("lamport_clock"),
                            "encryption": row.get::<String, _>("encryption"),
                            "created_at": row.get::<Option<String>, _>("created_at"),
                        })
                    })
                    .collect()
            }
        };

        Ok(messages)
    }

    async fn store_biome_message(
        &self,
        message: &aiome_core::biome::BiomeMessage,
    ) -> Result<(), AiomeError> {
        let q = format!("INSERT INTO biome_messages (sender_pubkey, recipient_pubkey, topic_id, content, karma_root_cid, signature, lamport_clock, encryption) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6), self.pool.ph(7));
        sql_exec!(
            &self.pool,
            &q,
            &message.sender_pubkey,
            &message.recipient_pubkey,
            &message.topic_id,
            &message.content,
            &message.karma_root_cid,
            &message.signature,
            message.lamport_clock as i64,
            &message.encryption
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;
        Ok(())
    }

    async fn update_biome_reputation(&self, pubkey: &str, delta: f64) -> Result<f64, AiomeError> {
        let q = format!("UPDATE biome_peers SET reputation_score = MAX(0, MIN(100, reputation_score + {0})) WHERE pubkey = {1} RETURNING reputation_score", self.pool.ph(0), self.pool.ph(1));
        let score: f64 = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar(&q)
                .bind(delta)
                .bind(pubkey)
                .fetch_one(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
            crate::db::DatabasePool::Postgres(p) => {
                let q_pg = "UPDATE biome_peers SET reputation_score = GREATEST(0, LEAST(100, reputation_score + $1)) WHERE pubkey = $2 RETURNING reputation_score";
                sqlx::query_scalar(q_pg)
                    .bind(delta)
                    .bind(pubkey)
                    .fetch_one(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?
            }
        };

        Ok(score)
    }

    async fn archive_biome_topic(&self, topic_id: &str) -> Result<(), AiomeError> {
        let q = format!(
            "UPDATE biome_topics SET status = 'Archived', updated_at = {0} WHERE topic_id = {1}",
            self.pool.now_fn(),
            self.pool.ph(0)
        );
        sql_exec!(&self.pool, &q, topic_id).map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;
        Ok(())
    }

    // --- Expression Engine (V4) ---
    async fn store_expression(
        &self,
        expression: &aiome_core::expression::Expression,
    ) -> Result<(), AiomeError> {
        <Self as ExpressionOps>::store_expression(self, expression).await
    }

    async fn fetch_expressions(
        &self,
        limit: i64,
    ) -> Result<Vec<aiome_core::expression::Expression>, AiomeError> {
        <Self as ExpressionOps>::fetch_expressions(self, limit).await
    }

    async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
        <Self as ExpressionOps>::get_auto_expression_enabled(self).await
    }

    async fn set_auto_expression_enabled(&self, enabled: bool) -> Result<(), AiomeError> {
        <Self as ExpressionOps>::set_auto_expression_enabled(self, enabled).await
    }
}

// Inherent methods (Watchtower / Chat extension)
impl UniversalJobQueue {
    /// `insert_chat_message` を実行する
    pub async fn insert_chat_message(
        &self,
        channel_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), AiomeError> {
        Box::pin(self.do_insert_chat_message(channel_id, role, content)).await
    }

    /// `fetch_chat_history` を実行する
    pub async fn fetch_chat_history(
        &self,
        channel_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        Box::pin(self.do_fetch_chat_history(channel_id, limit)).await
    }

    /// `get_chat_memory_summary` を実行する
    pub async fn get_chat_memory_summary(
        &self,
        channel_id: &str,
    ) -> Result<Option<String>, AiomeError> {
        Box::pin(self.do_get_chat_memory_summary(channel_id)).await
    }

    /// `update_chat_memory_summary` を実行する
    pub async fn update_chat_memory_summary(
        &self,
        channel_id: &str,
        summary: &str,
    ) -> Result<(), AiomeError> {
        Box::pin(self.do_update_chat_memory_summary(channel_id, summary)).await
    }

    /// `fetch_undistilled_chats_by_channel` を実行する
    pub async fn fetch_undistilled_chats_by_channel(
        &self,
    ) -> Result<std::collections::HashMap<String, Vec<(i64, String, String)>>, AiomeError> {
        Box::pin(self.do_fetch_undistilled_chats_by_channel()).await
    }

    /// `mark_chats_as_distilled` を実行する
    pub async fn mark_chats_as_distilled(
        &self,
        channel_id: &str,
        up_to_id: i64,
    ) -> Result<(), AiomeError> {
        Box::pin(self.do_mark_chats_as_distilled(channel_id, up_to_id)).await
    }

    /// `purge_old_distilled_chats` を実行する
    pub async fn purge_old_distilled_chats(&self, days: i64) -> Result<u64, AiomeError> {
        Box::pin(self.do_purge_old_distilled_chats(days)).await
    }

    /// `fetch_skills_for_distillation` を実行する
    pub async fn fetch_skills_for_distillation(
        &self,
        threshold: i64,
    ) -> Result<Vec<String>, AiomeError> {
        Box::pin(self.do_fetch_skills_for_distillation(threshold)).await
    }

    /// `fetch_raw_karma_for_skill` を実行する
    pub async fn fetch_raw_karma_for_skill(
        &self,
        skill: &str,
    ) -> Result<Vec<(String, String)>, AiomeError> {
        Box::pin(self.do_fetch_raw_karma_for_skill(skill)).await
    }

    /// `apply_distilled_karma` を実行する
    pub async fn apply_distilled_karma(
        &self,
        skill: &str,
        distilled_lesson: &str,
        old_karma_ids: &[String],
        soul_hash: &str,
        domain: Option<&str>,
        subtopic: Option<&str>,
        clone_origin_id: Option<&str>,
    ) -> Result<(), AiomeError> {
        Box::pin(self.do_apply_distilled_karma(
            skill,
            distilled_lesson,
            old_karma_ids,
            soul_hash,
            domain,
            subtopic,
            clone_origin_id,
        ))
        .await
    }

    /// `increment_oracle_retry_count` を実行する
    pub async fn increment_oracle_retry_count(&self, record_id: i64) -> Result<bool, AiomeError> {
        Box::pin(self.do_increment_oracle_retry_count(record_id)).await
    }

    /// `get_global_api_failures` を実行する
    pub async fn get_global_api_failures(&self) -> Result<i64, AiomeError> {
        Box::pin(self.do_get_global_api_failures()).await
    }

    /// `record_global_api_failure` を実行する
    pub async fn record_global_api_failure(&self) -> Result<i64, AiomeError> {
        Box::pin(self.do_record_global_api_failure()).await
    }

    /// `record_global_api_success` を実行する
    pub async fn record_global_api_success(&self) -> Result<(), AiomeError> {
        Box::pin(self.do_record_global_api_success()).await
    }

    /// `fetch_unfederated_data` を実行する
    pub async fn fetch_unfederated_data(
        &self,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>), AiomeError> {
        Box::pin(self.do_fetch_unfederated_data()).await
    }

    /// `mark_as_federated` を実行する
    pub async fn mark_as_federated(
        &self,
        karma_ids: Vec<String>,
        rule_ids: Vec<String>,
    ) -> Result<(), AiomeError> {
        Box::pin(self.do_mark_as_federated(karma_ids, rule_ids)).await
    }

    // Settings
    /// `get_setting_value` を実行する
    pub async fn get_setting_value(&self, key: &str) -> Result<Option<String>, AiomeError> {
        self.get_setting(key).await
    }

    /// `update_setting` を実行する
    pub async fn update_setting(
        &self,
        key: &str,
        value: &str,
        category: &str,
        is_secret: bool,
    ) -> Result<(), AiomeError> {
        self.set_setting(key, value, category, is_secret).await
    }

    /// `fetch_all_settings` を実行する
    pub async fn fetch_all_settings(
        &self,
    ) -> Result<Vec<aiome_core::contracts::SystemSetting>, AiomeError> {
        self.get_all_settings().await
    }

    /// `run_karma_tier_maintenance` を実行する
    pub async fn run_karma_tier_maintenance(&self) -> Result<(), AiomeError> {
        karma_maintenance::run_karma_tier_maintenance(&self.pool).await
    }
}

// Helper function for safer column access
pub(crate) fn try_get_opt<'r, R, T>(row: &'r R, col: &str) -> Option<T>
where
    R: sqlx::Row,
    T: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'a> &'a str: sqlx::ColumnIndex<R>,
{
    row.try_get(col).ok()
}

pub(crate) fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot_product / (norm_a * norm_b)
}
