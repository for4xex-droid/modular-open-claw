/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::contracts::{
    ArenaMatch, ArtifactCategory, ArtifactEdge, ArtifactMeta, FederatedMetrics, ImmuneRule,
    JobMetrics, KarmaEntry, KarmaMetrics, OracleVerdict, SamsaraEvent,
};
use aiome_contracts::security::PermissionManifest;
use aiome_contracts::traits::{
    Expression, Job, JobQueue, JobStatus, KarmaSearchResult, Publisher, SnsMetricsRecord,
};
use aiome_contracts::types::AgentStats;
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::{EmbeddingProvider, LlmProvider};
use aiome_core::trajectory::TrajectoryStore;

use async_trait::async_trait;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

pub mod core_ops;
pub mod crdt;
pub mod evaluation;
pub mod evolution;
pub mod expression;
pub mod federation;
pub mod guardrails;
pub mod karma;
pub mod karma_maintenance;
pub mod migrations;
pub mod postgres_init;
pub mod security;
pub mod settings;
pub mod soul_store;
pub mod swarm;
pub mod taxonomy;
pub mod trajectory_store;
pub mod watchtower;

pub use self::core_ops::CoreOps;
pub use self::evaluation::EvaluationOps;
pub use self::evolution::EvolutionOps;
pub use self::expression::ExpressionOps;
pub use self::federation::FederationOps;
pub use self::guardrails::GuardrailOps;
pub use self::karma::KarmaOps;
pub use self::security::SecurityOps;
pub use self::settings::SettingsOps;
pub use self::soul_store::SoulStoreOps;
pub use self::swarm::SwarmOps;
pub use self::trajectory_store::TrajectoryOps;
pub use self::watchtower::WatchtowerOps;
use crate::db::DatabasePool;
use crate::job_queue::migrations::DbInitializer;

// Re-export cosine_similarity if needed by karma.rs
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// `UniversalJobQueue` 構造体
#[derive(Debug, Clone)]
pub struct UniversalJobQueue {
    pool: DatabasePool,
    pub karma_cache: Arc<RwLock<HashMap<String, (KarmaSearchResult, Instant)>>>,
    pub llm: Option<Arc<dyn LlmProvider>>,
    pub embed_provider: Arc<RwLock<Option<Arc<dyn EmbeddingProvider>>>>,
}

impl UniversalJobQueue {
    pub async fn new(path: &str) -> Result<Self, AiomeError> {
        let pool = if path.starts_with("postgres://") || path.starts_with("postgresql://") {
            let pg_pool =
                sqlx::PgPool::connect(path)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            Self::postgres_init(&pg_pool).await?;
            DatabasePool::Postgres(pg_pool)
        } else {
            use std::str::FromStr;
            let options = sqlx::sqlite::SqliteConnectOptions::from_str(path)
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?
                .create_if_missing(true);
            let sq_pool = sqlx::sqlite::SqlitePoolOptions::new()
                .connect_with(options)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;
            DatabasePool::Sqlite(sq_pool)
        };

        let this = Self {
            pool,
            karma_cache: Arc::new(RwLock::new(HashMap::new())),
            llm: None,
            embed_provider: Arc::new(RwLock::new(None)),
        };

        if this.pool.is_sqlite() {
            this.ensure_tables_sqlite().await?;
        }

        Ok(this)
    }

    async fn postgres_init(pool: &sqlx::PgPool) -> Result<(), AiomeError> {
        crate::job_queue::postgres_init::PostgresInitializer::init_db(pool).await
    }

    async fn ensure_tables_sqlite(&self) -> Result<(), AiomeError> {
        self.init_db().await
    }

    pub fn from_pool(pool: DatabasePool) -> Self {
        Self {
            pool,
            karma_cache: Arc::new(RwLock::new(HashMap::new())),
            llm: None,
            embed_provider: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_embedding_provider(&self, provider: Arc<dyn EmbeddingProvider>) {
        let mut p = self.embed_provider.write().await;
        *p = Some(provider);
    }

    pub async fn get_embedding_provider(&self) -> Option<Arc<dyn EmbeddingProvider>> {
        let p = self.embed_provider.read().await;
        p.clone()
    }

    pub fn with_llm(mut self, llm: Arc<dyn LlmProvider>) -> Self {
        self.llm = Some(llm);
        self
    }

    pub fn get_pool(&self) -> &DatabasePool {
        &self.pool
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
        permission_manifest: Option<PermissionManifest>,
        agent_id: Option<Uuid>,
        priority: i32,
    ) -> Result<String, AiomeError> {
        self.do_enqueue(
            category,
            topic,
            style,
            karma_directives,
            permission_manifest,
            agent_id,
            priority,
        )
        .await
    }
    async fn dequeue(&self, capable_categories: &[&str]) -> Result<Option<Job>, AiomeError> {
        self.do_dequeue(capable_categories).await
    }
    async fn fetch_job(&self, job_id: &str) -> Result<Option<Job>, AiomeError> {
        self.do_fetch_job(job_id).await
    }
    async fn complete_job(
        &self,
        job_id: &str,
        output_artifacts: Option<&str>,
    ) -> Result<(), AiomeError> {
        self.do_complete_job(job_id, output_artifacts).await
    }
    async fn fail_job(&self, job_id: &str, reason: &str) -> Result<(), AiomeError> {
        self.do_fail_job(job_id, reason).await
    }
    async fn reclaim_zombie_jobs(&self, timeout_minutes: i64) -> Result<u64, AiomeError> {
        self.do_reclaim_zombie_jobs(timeout_minutes).await
    }
    async fn fetch_chat_history(
        &self,
        channel_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        self.do_fetch_chat_history(channel_id, limit).await
    }
    async fn store_chat_message(
        &self,
        channel_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), AiomeError> {
        self.do_insert_chat_message(channel_id, role, content).await
    }

    async fn get_chat_memory_summary(
        &self,
        channel_id: &str,
    ) -> Result<Option<String>, AiomeError> {
        self.do_get_chat_memory_summary(channel_id).await
    }
    async fn update_chat_memory_summary(
        &self,
        channel_id: &str,
        summary: &str,
    ) -> Result<(), AiomeError> {
        self.do_update_chat_memory_summary(channel_id, summary)
            .await
    }
    async fn mark_chats_as_distilled(
        &self,
        channel_id: &str,
        before_id: i64,
    ) -> Result<(), AiomeError> {
        self.do_mark_chats_as_distilled(channel_id, before_id).await
    }
    async fn set_creative_rating(&self, job_id: &str, rating: i32) -> Result<(), AiomeError> {
        self.do_set_creative_rating(job_id, rating).await
    }
    async fn heartbeat_pulse(&self, job_id: &str) -> Result<(), AiomeError> {
        self.do_heartbeat_pulse(job_id).await
    }
    async fn store_execution_log(&self, job_id: &str, log: &str) -> Result<(), AiomeError> {
        self.do_store_execution_log(job_id, log).await
    }
    async fn fetch_relevant_karma(
        &self,
        topic: &str,
        skill_id: &str,
        limit: i64,
        current_soul_hash: &str,
    ) -> Result<KarmaSearchResult, AiomeError> {
        self.do_fetch_relevant_karma(topic, skill_id, limit, current_soul_hash)
            .await
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
        self.do_store_karma(
            job_id,
            skill_id,
            lesson,
            karma_type,
            soul_hash,
            domain,
            subtopic,
            clone_origin_id,
        )
        .await
    }
    async fn adjust_karma_weight(&self, karma_id: &str, delta: i32) -> Result<(), AiomeError> {
        self.do_adjust_karma_weight(karma_id, delta).await
    }
    async fn karma_decay_sweep(&self) -> Result<u64, AiomeError> {
        self.do_karma_decay_sweep().await
    }
    async fn fetch_undistilled_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        self.do_fetch_undistilled_jobs(limit).await
    }
    async fn mark_karma_extracted(&self, job_id: &str) -> Result<(), AiomeError> {
        self.do_mark_karma_extracted(job_id).await
    }
    async fn purge_old_jobs(&self, days: i64) -> Result<u64, AiomeError> {
        self.do_purge_old_jobs(days).await
    }
    async fn link_sns_data(
        &self,
        job_id: &str,
        platform: &str,
        content_id: &str,
    ) -> Result<(), AiomeError> {
        self.do_link_sns_data(job_id, platform, content_id).await
    }
    async fn fetch_jobs_for_evaluation(
        &self,
        milestone_days: i64,
        limit: i64,
    ) -> Result<Vec<Job>, AiomeError> {
        self.do_fetch_jobs_for_evaluation(milestone_days, limit)
            .await
    }
    async fn record_sns_metrics(
        &self,
        job_id: &str,
        milestone_days: i64,
        views: i64,
        likes: i64,
        comments_count: i64,
        raw_comments: Option<&str>,
    ) -> Result<(), AiomeError> {
        self.do_record_sns_metrics(
            job_id,
            milestone_days,
            views,
            likes,
            comments_count,
            raw_comments,
        )
        .await
    }
    async fn fetch_pending_evaluations(
        &self,
        limit: i64,
    ) -> Result<Vec<SnsMetricsRecord>, AiomeError> {
        self.do_fetch_pending_evaluations(limit).await
    }
    async fn apply_final_verdict(
        &self,
        record_id: i64,
        verdict: OracleVerdict,
        soul_hash: &str,
    ) -> Result<(), AiomeError> {
        self.do_apply_final_verdict(record_id, verdict, soul_hash)
            .await
    }
    async fn fetch_recent_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        self.do_fetch_recent_jobs(limit).await
    }
    async fn get_agent_stats(&self) -> Result<AgentStats, AiomeError> {
        let s = self.do_get_agent_stats().await?;
        Ok(AgentStats {
            level: s.level,
            exp: s.exp,
            resonance: s.resonance,
            creativity: s.creativity,
            fatigue: s.fatigue,
        })
    }
    async fn add_resonance(&self, amount: i32) -> Result<(), AiomeError> {
        self.do_add_resonance(amount).await
    }
    async fn add_tech_exp(&self, amount: i32) -> Result<(), AiomeError> {
        self.do_add_tech_exp(amount).await
    }
    async fn add_creativity(&self, amount: i32) -> Result<(), AiomeError> {
        self.do_add_creativity(amount).await
    }
    async fn sync_samsara_level(&self) -> Result<Option<SamsaraEvent>, AiomeError> {
        self.do_sync_samsara_level().await
    }
    async fn record_evolution_event(
        &self,
        level: i32,
        event_type: &str,
        description: &str,
        inspiration: Option<&str>,
        karma_json: Option<&str>,
    ) -> Result<(), AiomeError> {
        self.do_record_evolution_event(level, event_type, description, inspiration, karma_json)
            .await
    }
    async fn fetch_evolution_history(
        &self,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        self.do_fetch_evolution_history(limit).await
    }
    async fn get_pending_job_count(&self) -> Result<i64, AiomeError> {
        self.do_get_pending_job_count().await
    }
    async fn get_job_count_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<i64, AiomeError> {
        self.do_get_job_count_since(since).await
    }
    async fn fetch_all_karma(&self, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
        self.do_fetch_all_karma(limit).await
    }
    async fn fetch_top_performing_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        self.do_fetch_top_performing_jobs(limit).await
    }
    async fn record_soul_mutation(
        &self,
        old_hash: &str,
        new_hash: &str,
        reason: &str,
    ) -> Result<(), AiomeError> {
        self.do_record_soul_mutation(old_hash, new_hash, reason)
            .await
    }
    async fn fetch_job_retry_count(&self, job_id: &str) -> Result<i64, AiomeError> {
        self.do_fetch_job_retry_count(job_id).await
    }
    async fn reset_job_retry_count(&self, job_id: &str) -> Result<(), AiomeError> {
        self.do_reset_job_retry_count(job_id).await
    }
    async fn increment_job_retry_count(&self, job_id: &str) -> Result<bool, AiomeError> {
        self.do_increment_job_retry_count(job_id).await
    }
    async fn fetch_unincorporated_karma(
        &self,
        limit: i64,
        current_soul_hash: &str,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        self.do_fetch_unincorporated_karma(limit, current_soul_hash)
            .await
    }
    async fn mark_karma_as_incorporated(
        &self,
        karma_ids: Vec<String>,
        new_soul_hash: &str,
    ) -> Result<(), AiomeError> {
        self.do_mark_karma_as_incorporated(karma_ids, new_soul_hash)
            .await
    }
    async fn store_immune_rule(&self, rule: &ImmuneRule) -> Result<(), AiomeError> {
        self.do_store_immune_rule(rule).await
    }
    async fn delete_immune_rule(&self, rule_id: &str) -> Result<(), AiomeError> {
        self.do_delete_immune_rule(rule_id).await
    }
    async fn fetch_active_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        self.do_fetch_active_immune_rules().await
    }
    async fn record_arena_match(&self, m: &ArenaMatch) -> Result<(), AiomeError> {
        self.do_record_arena_match(m).await
    }
    async fn get_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        self.do_get_immune_rules().await
    }
    async fn export_federated_data(
        &self,
        since: Option<&str>,
    ) -> Result<(Vec<KarmaEntry>, Vec<ImmuneRule>, Vec<ArenaMatch>), AiomeError> {
        let (k, r, m) = FederationOps::do_export_federated_data(self, since).await?;
        Ok((k.into_iter().map(|fk| fk.into()).collect(), r, m))
    }
    async fn import_federated_data(
        &self,
        karmas: Vec<KarmaEntry>,
        rules: Vec<ImmuneRule>,
        matches: Vec<ArenaMatch>,
    ) -> Result<(), AiomeError> {
        FederationOps::do_import_federated_data(
            self,
            karmas.into_iter().map(|ke| ke.into()).collect(),
            rules,
            matches,
        )
        .await
    }
    async fn get_peer_sync_time(&self, peer_url: &str) -> Result<Option<String>, AiomeError> {
        self.do_get_peer_sync_time(peer_url).await
    }
    async fn update_peer_sync_time(
        &self,
        peer_url: &str,
        sync_time: &str,
    ) -> Result<(), AiomeError> {
        self.do_update_peer_sync_time(peer_url, sync_time).await
    }
    async fn get_node_id(&self) -> Result<String, AiomeError> {
        self.do_get_node_id().await
    }
    async fn fetch_unfederated_data(
        &self,
    ) -> Result<(Vec<KarmaEntry>, Vec<ImmuneRule>), AiomeError> {
        let (k, r) = FederationOps::do_fetch_unfederated_data(self).await?;
        Ok((k.into_iter().map(|fk| fk.into()).collect(), r))
    }
    async fn mark_as_federated(
        &self,
        karma_ids: Vec<String>,
        rule_ids: Vec<String>,
    ) -> Result<(), AiomeError> {
        self.do_mark_as_federated(karma_ids, rule_ids).await
    }
    async fn fetch_federated_metrics(&self) -> Result<FederatedMetrics, AiomeError> {
        self.do_fetch_federated_metrics().await
    }
    async fn sign_swarm_payload(&self, payload: &str) -> Result<String, AiomeError> {
        self.do_sign_swarm_payload(payload).await
    }
    async fn sync_local_clock(&self, remote_time: u64) -> Result<u64, AiomeError> {
        self.do_sync_local_clock(remote_time).await
    }
    async fn tick_local_clock(&self) -> Result<u64, AiomeError> {
        self.do_tick_local_clock().await
    }
    async fn storage_gc(&self, threshold_gb: f64) -> Result<u64, AiomeError> {
        self.do_storage_gc(threshold_gb).await
    }
    async fn get_biome_topic_status(
        &self,
        topic_id: &str,
    ) -> Result<Option<(i32, Option<String>)>, AiomeError> {
        self.do_get_biome_topic_status(topic_id).await
    }
    async fn advance_biome_turn(
        &self,
        topic_id: &str,
        expires_in_mins: i64,
    ) -> Result<i32, AiomeError> {
        self.do_advance_biome_turn(topic_id, expires_in_mins).await
    }
    async fn fetch_biome_messages(
        &self,
        topic_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        self.do_fetch_biome_messages(topic_id, limit).await
    }
    async fn store_biome_message(
        &self,
        msg: &aiome_contracts::biome::BiomeMessage,
    ) -> Result<(), AiomeError> {
        self.do_store_biome_message(msg).await
    }
    async fn update_biome_reputation(
        &self,
        agent_node_id: &str,
        delta: f64,
    ) -> Result<f64, AiomeError> {
        self.do_update_biome_reputation(agent_node_id, delta).await
    }
    async fn archive_biome_topic(&self, topic_id: &str) -> Result<(), AiomeError> {
        self.do_archive_biome_topic(topic_id).await
    }
    async fn fetch_relevant_karma_by_category(
        &self,
        topic: &str,
        category: &str,
        limit: i64,
    ) -> Result<KarmaSearchResult, AiomeError> {
        self.do_fetch_relevant_karma_by_category(topic, category, limit)
            .await
    }
    async fn get_system_agent_id(&self) -> Result<Uuid, AiomeError> {
        self.do_get_system_agent_id().await
    }
    async fn store_expression(&self, expression: &Expression) -> Result<(), AiomeError> {
        self.do_store_expression(expression).await
    }
    async fn fetch_expressions(&self, limit: i64) -> Result<Vec<Expression>, AiomeError> {
        self.do_fetch_expressions(limit).await
    }
    async fn store_soul_fragment(&self, fragment: &str, hash: &str) -> Result<(), AiomeError> {
        self.do_store_soul_fragment(fragment, hash).await
    }
    async fn fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError> {
        self.do_fetch_latest_soul_fragment().await
    }

    async fn get_security_request_count(&self, agent_id: Option<Uuid>) -> Result<u32, AiomeError> {
        self.do_get_security_request_count(agent_id).await
    }

    async fn increment_security_request_count(
        &self,
        agent_id: Option<Uuid>,
    ) -> Result<u32, AiomeError> {
        self.do_increment_security_request_count(agent_id).await
    }
}

#[async_trait]
impl Publisher for UniversalJobQueue {
    async fn publish(
        &self,
        content: &str,
        media_paths: &[std::path::PathBuf],
        metadata: &serde_json::Value,
    ) -> Result<String, AiomeError> {
        self.do_publish(content, media_paths, metadata).await
    }
    fn platform_name(&self) -> &str {
        "UniversalPublisher"
    }
}

impl UniversalJobQueue {
    pub async fn update_setting(
        &self,
        key: &str,
        val: &str,
        category: &str,
        is_secret: bool,
    ) -> Result<(), AiomeError> {
        self.do_set_setting(key, val, category, is_secret).await
    }
    pub async fn fetch_all_settings(
        &self,
    ) -> Result<Vec<aiome_core::contracts::SystemSetting>, AiomeError> {
        self.do_get_all_settings().await
    }
    pub async fn get_setting_value(&self, key: &str) -> Result<Option<String>, AiomeError> {
        self.do_get_setting(key).await
    }
    pub async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
        self.do_get_auto_expression_enabled().await
    }
    pub async fn set_auto_expression_enabled(&self, enabled: bool) -> Result<(), AiomeError> {
        self.do_set_auto_expression_enabled(enabled).await
    }
    pub async fn do_push_federated_metrics(&self) -> Result<(), AiomeError> {
        <Self as FederationOps>::do_push_federated_metrics(self).await
    }
}
