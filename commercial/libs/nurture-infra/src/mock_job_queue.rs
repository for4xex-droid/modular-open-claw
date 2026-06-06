/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 *
 * Usage of this software is subject to the BSL 1.1 terms.
 * Commercial use requires a separate license agreement.
 */

use aiome_core_contracts::contracts::{
    ArenaMatch, FederatedKarma, ImmuneRule, OracleVerdict, SamsaraEvent,
};
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::traits::{
    AgentEvolver, AuditStore, BiomeRegistry, ChatStore, FederationRegistry, HarnessRegistryOps,
    ImmuneSystemOps, Job, JobQueue, KarmaRegistry, KarmaSearchResult, SettingsOps,
    SnsMetricsRecord, SoulStore, SystemStateOps, TaskRegistry,
};
use async_trait::async_trait;
use nurture_bridge::job_queue::{EvaluationOps, UniversalJobQueue};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug)]
pub struct RealJobQueue {
    inner: Arc<UniversalJobQueue>,
}

impl RealJobQueue {
    pub async fn new(db_path: &str) -> Result<Self, AiomeError> {
        use nurture_bridge::db::DatabasePool;
        use nurture_bridge::job_queue::trajectory_store::SqliteTrajectoryStore;

        let pool = DatabasePool::new_sqlite(db_path).await?;
        let traj_store = Arc::new(SqliteTrajectoryStore::from_db_path(db_path).await?)
            as Arc<dyn aiome_core_contracts::trajectory::TrajectoryStore>;
        let inner = UniversalJobQueue::new(pool, None, traj_store).await?;
        Ok(Self {
            inner: Arc::new(inner),
        })
    }
}

/// テスト環境用エイリアス。
/// 注意: 内部実装は実 DB 接続 (`UniversalJobQueue`) を使用するため、
/// ユニットテストでは in-memory DB パスを指定すること。
#[cfg(any(test, debug_assertions))]
pub type MockJobQueue = RealJobQueue;

#[async_trait]
impl SettingsOps for RealJobQueue {
    async fn do_get_setting(&self, key: &str) -> Result<Option<String>, AiomeError> {
        self.inner.do_get_setting(key).await
    }

    async fn do_set_setting(
        &self,
        key: &str,
        value: &str,
        category: &str,
        is_secret: bool,
    ) -> Result<(), AiomeError> {
        self.inner
            .do_set_setting(key, value, category, is_secret)
            .await
    }

    async fn do_get_all_settings(
        &self,
    ) -> Result<Vec<aiome_core_contracts::contracts::SystemSetting>, AiomeError> {
        self.inner.do_get_all_settings().await
    }

    async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
        self.inner.get_auto_expression_enabled().await
    }

    async fn set_auto_expression_enabled(&self, enabled: bool) -> Result<(), AiomeError> {
        self.inner.set_auto_expression_enabled(enabled).await
    }
}

#[async_trait]
impl JobQueue for RealJobQueue {
    async fn get_system_agent_id(&self) -> Result<Uuid, AiomeError> {
        self.inner.get_system_agent_id().await
    }

    async fn sign_swarm_payload(&self, payload: &str) -> Result<String, AiomeError> {
        self.inner.sign_swarm_payload(payload).await
    }

    async fn tick_local_clock(&self) -> Result<u64, AiomeError> {
        self.inner.tick_local_clock().await
    }

    async fn sync_local_clock(&self, remote_clock: u64) -> Result<u64, AiomeError> {
        self.inner.sync_local_clock(remote_clock).await
    }

    async fn storage_gc(&self, threshold_gb: f64) -> Result<u64, AiomeError> {
        self.inner.storage_gc(threshold_gb).await
    }

    async fn store_expression(
        &self,
        expression: &aiome_core_contracts::expression::Expression,
    ) -> Result<(), AiomeError> {
        self.inner.store_expression(expression).await
    }

    async fn fetch_expressions(
        &self,
        limit: i64,
    ) -> Result<Vec<aiome_core_contracts::expression::Expression>, AiomeError> {
        self.inner.fetch_expressions(limit).await
    }
}

#[async_trait]
impl BiomeRegistry for RealJobQueue {
    async fn get_biome_topic_status(
        &self,
        topic_id: &str,
    ) -> Result<Option<(i32, Option<String>)>, AiomeError> {
        self.inner.get_biome_topic_status(topic_id).await
    }

    async fn advance_biome_turn(
        &self,
        topic_id: &str,
        cooldown_minutes: i64,
    ) -> Result<i32, AiomeError> {
        self.inner
            .advance_biome_turn(topic_id, cooldown_minutes)
            .await
    }

    async fn fetch_biome_messages(
        &self,
        topic_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        self.inner.fetch_biome_messages(topic_id, limit).await
    }

    async fn store_biome_message(
        &self,
        message: &aiome_core_contracts::biome::BiomeMessage,
    ) -> Result<(), AiomeError> {
        self.inner.store_biome_message(message).await
    }

    async fn update_biome_reputation(&self, pubkey: &str, delta: f64) -> Result<f64, AiomeError> {
        self.inner.update_biome_reputation(pubkey, delta).await
    }

    async fn archive_biome_topic(&self, topic_id: &str) -> Result<(), AiomeError> {
        self.inner.archive_biome_topic(topic_id).await
    }
}

#[async_trait]
impl AuditStore for RealJobQueue {
    async fn update_trajectory_reward(&self, job_id: &str, reward: f64) -> Result<(), AiomeError> {
        self.inner.update_trajectory_reward(job_id, reward).await
    }

    async fn store_trajectory_step(
        &self,
        step: aiome_core_contracts::trajectory::TrajectoryStep,
    ) -> Result<(), AiomeError> {
        self.inner.store_trajectory_step(step).await
    }
    async fn fetch_trajectory_steps(
        &self,
        job_id: &str,
    ) -> Result<Vec<aiome_core_contracts::trajectory::TrajectoryStep>, AiomeError> {
        self.inner.fetch_trajectory_steps(job_id).await
    }
    async fn clear_trajectory_steps(&self, job_id: &str) -> Result<(), AiomeError> {
        self.inner.clear_trajectory_steps(job_id).await
    }
    async fn fetch_diagnosis(
        &self,
        job_id: &str,
    ) -> Result<Option<aiome_core_contracts::trajectory::AgentDiagnosis>, AiomeError> {
        self.inner.fetch_diagnosis(job_id).await
    }
    async fn store_diagnosis(
        &self,
        job_id: &str,
        diag: aiome_core_contracts::trajectory::AgentDiagnosis,
    ) -> Result<(), AiomeError> {
        self.inner.store_diagnosis(job_id, diag).await
    }
    async fn get_security_request_count(&self, agent_id: Option<Uuid>) -> Result<u32, AiomeError> {
        self.inner.get_security_request_count(agent_id).await
    }
    async fn increment_security_request_count(
        &self,
        agent_id: Option<Uuid>,
    ) -> Result<u32, AiomeError> {
        self.inner.increment_security_request_count(agent_id).await
    }

    async fn store_execution_log(&self, job_id: &str, log: &str) -> Result<(), AiomeError> {
        self.inner.store_execution_log(job_id, log).await
    }
}

#[async_trait]
impl EvaluationOps for RealJobQueue {
    async fn do_fetch_top_performing_jobs(
        &self,
        limit: i64,
    ) -> Result<Vec<aiome_core_contracts::traits::Job>, AiomeError> {
        self.inner.do_fetch_top_performing_jobs(limit).await
    }

    async fn do_link_sns_data(
        &self,
        job_id: &str,
        platform: &str,
        content_id: &str,
    ) -> Result<(), AiomeError> {
        self.inner
            .do_link_sns_data(job_id, platform, content_id)
            .await
    }

    async fn do_fetch_jobs_for_evaluation(
        &self,
        milestone_days: i64,
        limit: i64,
    ) -> Result<Vec<Job>, AiomeError> {
        self.inner
            .do_fetch_jobs_for_evaluation(milestone_days, limit)
            .await
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
        self.inner
            .do_record_sns_metrics(
                job_id,
                milestone_days,
                views,
                likes,
                comments_count,
                raw_comments,
                repost_count,
                quote_count,
                reply_count,
                impression_count,
            )
            .await
    }

    async fn do_fetch_pending_evaluations(
        &self,
        limit: i64,
    ) -> Result<Vec<SnsMetricsRecord>, AiomeError> {
        self.inner.do_fetch_pending_evaluations(limit).await
    }

    async fn do_apply_final_verdict(
        &self,
        record_id: i64,
        verdict: OracleVerdict,
        soul_hash: &str,
    ) -> Result<(), AiomeError> {
        self.inner
            .do_apply_final_verdict(record_id, verdict, soul_hash)
            .await
    }
}

#[async_trait]
impl ImmuneSystemOps for RealJobQueue {
    async fn store_immune_rule(&self, rule: &ImmuneRule) -> Result<(), AiomeError> {
        self.inner.store_immune_rule(rule).await
    }

    async fn delete_immune_rule(&self, rule_id: &str) -> Result<(), AiomeError> {
        self.inner.delete_immune_rule(rule_id).await
    }

    async fn fetch_active_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        self.inner.fetch_active_immune_rules().await
    }

    async fn record_arena_match(&self, match_data: &ArenaMatch) -> Result<(), AiomeError> {
        self.inner.record_arena_match(match_data).await
    }

    async fn get_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        self.inner.get_immune_rules().await
    }

    async fn fetch_arena_matches(&self, limit: i64) -> Result<Vec<ArenaMatch>, AiomeError> {
        self.inner.fetch_arena_matches(limit).await
    }
}

#[async_trait]
impl TaskRegistry for RealJobQueue {
    async fn append_job_karma_directives(
        &self,
        job_id: &str,
        hint: &str,
    ) -> Result<(), AiomeError> {
        self.inner.append_job_karma_directives(job_id, hint).await
    }

    async fn requeue_job(&self, job_id: &str) -> Result<(), AiomeError> {
        self.inner.requeue_job(job_id).await
    }
    async fn cancel_job(&self, job_id: &str) -> Result<(), AiomeError> {
        self.inner.cancel_job(job_id).await
    }
    async fn update_job_status(
        &self,
        job_id: &str,
        status: aiome_core_contracts::traits::JobStatus,
    ) -> Result<(), AiomeError> {
        self.inner.update_job_status(job_id, status).await
    }

    async fn enqueue(
        &self,
        category: &str,
        topic: &str,
        style: &str,
        karma_directives: Option<&str>,
        permission_manifest: Option<aiome_core_contracts::security::PermissionManifest>,
        specific_agent_id: Option<Uuid>,
        priority: i32,
    ) -> Result<String, AiomeError> {
        self.inner
            .enqueue(
                category,
                topic,
                style,
                karma_directives,
                permission_manifest,
                specific_agent_id,
                priority,
            )
            .await
    }

    async fn fetch_job(&self, job_id: &str) -> Result<Option<Job>, AiomeError> {
        self.inner.fetch_job(job_id).await
    }

    async fn dequeue(&self, capable_categories: &[&str]) -> Result<Option<Job>, AiomeError> {
        self.inner.dequeue(capable_categories).await
    }

    async fn complete_job(
        &self,
        job_id: &str,
        output_artifacts: Option<&str>,
    ) -> Result<(), AiomeError> {
        self.inner.complete_job(job_id, output_artifacts).await
    }

    async fn fail_job(&self, job_id: &str, reason: &str) -> Result<(), AiomeError> {
        self.inner.fail_job(job_id, reason).await
    }

    async fn reclaim_zombie_jobs(&self, timeout_minutes: i64) -> Result<u64, AiomeError> {
        self.inner.reclaim_zombie_jobs(timeout_minutes).await
    }

    async fn set_creative_rating(&self, job_id: &str, rating: i32) -> Result<(), AiomeError> {
        self.inner.set_creative_rating(job_id, rating).await
    }

    async fn heartbeat_pulse(&self, job_id: &str) -> Result<(), AiomeError> {
        self.inner.heartbeat_pulse(job_id).await
    }

    async fn purge_old_jobs(&self, days: i64) -> Result<u64, AiomeError> {
        self.inner.purge_old_jobs(days).await
    }

    async fn fetch_recent_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        self.inner.fetch_recent_jobs(limit).await
    }

    async fn get_pending_job_count(&self) -> Result<i64, AiomeError> {
        self.inner.get_pending_job_count().await
    }

    async fn get_job_count_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<i64, AiomeError> {
        self.inner.get_job_count_since(since).await
    }

    async fn fetch_top_performing_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        self.inner.fetch_top_performing_jobs(limit).await
    }

    async fn fetch_job_retry_count(&self, job_id: &str) -> Result<i64, AiomeError> {
        self.inner.fetch_job_retry_count(job_id).await
    }

    async fn increment_job_retry_count(&self, job_id: &str) -> Result<bool, AiomeError> {
        self.inner.increment_job_retry_count(job_id).await
    }

    async fn reset_job_retry_count(&self, job_id: &str) -> Result<(), AiomeError> {
        self.inner.reset_job_retry_count(job_id).await
    }

    async fn fetch_job_cost(&self, job_id: &str) -> Result<f64, AiomeError> {
        self.inner.fetch_job_cost(job_id).await
    }
}

#[async_trait]
impl ChatStore for RealJobQueue {
    async fn get_chat_memory_summary(
        &self,
        channel_id: &str,
    ) -> Result<Option<(String, Option<String>)>, AiomeError> {
        self.inner.get_chat_memory_summary(channel_id).await
    }

    async fn update_chat_memory_summary(
        &self,
        channel_id: &str,
        summary: &str,
        last_interaction_id: Option<&str>,
    ) -> Result<(), AiomeError> {
        self.inner
            .update_chat_memory_summary(channel_id, summary, last_interaction_id)
            .await
    }

    async fn mark_chats_as_distilled(
        &self,
        channel_id: &str,
        last_id: i64,
    ) -> Result<(), AiomeError> {
        self.inner
            .mark_chats_as_distilled(channel_id, last_id)
            .await
    }

    async fn store_chat_message(
        &self,
        channel_id: &str,
        role: &str,
        content: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<(), AiomeError> {
        self.inner
            .store_chat_message(channel_id, role, content, metadata)
            .await
    }

    async fn fetch_chat_history(
        &self,
        channel_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        self.inner.fetch_chat_history(channel_id, limit).await
    }

    async fn purge_old_distilled_chats(&self, days: i64) -> Result<u64, AiomeError> {
        self.inner.purge_old_distilled_chats(days).await
    }
}

#[async_trait]
impl AgentEvolver for RealJobQueue {
    async fn get_agent_stats(&self) -> Result<nurture_bridge::watchtower::AgentStats, AiomeError> {
        self.inner.get_agent_stats().await
    }

    async fn add_resonance(&self, amount: i32) -> Result<(), AiomeError> {
        self.inner.add_resonance(amount).await
    }

    async fn add_tech_exp(&self, amount: i32) -> Result<(), AiomeError> {
        self.inner.add_tech_exp(amount).await
    }

    async fn add_creativity(&self, amount: i32) -> Result<(), AiomeError> {
        self.inner.add_creativity(amount).await
    }

    async fn sync_samsara_level(&self) -> Result<Option<SamsaraEvent>, AiomeError> {
        self.inner.sync_samsara_level().await
    }

    async fn record_evolution_event(
        &self,
        level: i32,
        event_type: &str,
        description: &str,
        inspiration: Option<&str>,
        karma_json: Option<&str>,
    ) -> Result<(), AiomeError> {
        self.inner
            .record_evolution_event(level, event_type, description, inspiration, karma_json)
            .await
    }

    async fn fetch_evolution_history(
        &self,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        self.inner.fetch_evolution_history(limit).await
    }

    async fn record_soul_mutation(
        &self,
        old_hash: &str,
        new_hash: &str,
        reason: &str,
    ) -> Result<(), AiomeError> {
        self.inner
            .record_soul_mutation(old_hash, new_hash, reason)
            .await
    }
}

#[async_trait]
impl KarmaRegistry for RealJobQueue {
    async fn fetch_relevant_karma_by_category(
        &self,
        topic: &str,
        category: &str,
        limit: i64,
    ) -> Result<KarmaSearchResult, AiomeError> {
        self.inner
            .fetch_relevant_karma_by_category(topic, category, limit)
            .await
    }

    async fn fetch_relevant_karma(
        &self,
        topic: &str,
        skill_id: &str,
        limit: i64,
        current_soul_hash: &str,
    ) -> Result<KarmaSearchResult, AiomeError> {
        self.inner
            .fetch_relevant_karma(topic, skill_id, limit, current_soul_hash)
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
        is_private: bool,
    ) -> Result<(), AiomeError> {
        self.inner
            .store_karma(
                job_id,
                skill_id,
                lesson,
                karma_type,
                soul_hash,
                domain,
                subtopic,
                clone_origin_id,
                is_private,
            )
            .await
    }

    async fn adjust_karma_weight(&self, karma_id: &str, delta: i32) -> Result<(), AiomeError> {
        self.inner.adjust_karma_weight(karma_id, delta).await
    }

    async fn karma_decay_sweep(&self) -> Result<u64, AiomeError> {
        self.inner.karma_decay_sweep().await
    }

    async fn fetch_all_karma(&self, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
        self.inner.fetch_all_karma(limit).await
    }

    async fn fetch_undistilled_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        self.inner.fetch_undistilled_jobs(limit).await
    }

    async fn mark_karma_extracted(&self, job_id: &str) -> Result<(), AiomeError> {
        self.inner.mark_karma_extracted(job_id).await
    }

    async fn fetch_unincorporated_karma(
        &self,
        limit: i64,
        current_soul_hash: &str,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        self.inner
            .fetch_unincorporated_karma(limit, current_soul_hash)
            .await
    }

    async fn mark_karma_as_incorporated(
        &self,
        karma_ids: Vec<String>,
        new_soul_hash: &str,
    ) -> Result<(), AiomeError> {
        self.inner
            .mark_karma_as_incorporated(karma_ids, new_soul_hash)
            .await
    }
}

#[async_trait]
impl FederationRegistry for RealJobQueue {
    async fn fetch_unfederated_data(
        &self,
    ) -> Result<
        (
            Vec<aiome_core_contracts::contracts::KarmaEntry>,
            Vec<aiome_core_contracts::contracts::ImmuneRule>,
        ),
        AiomeError,
    > {
        self.inner.fetch_unfederated_data().await
    }
    async fn mark_as_federated(
        &self,
        karma_ids: Vec<String>,
        rule_ids: Vec<String>,
    ) -> Result<(), AiomeError> {
        self.inner.mark_as_federated(karma_ids, rule_ids).await
    }
    async fn fetch_federated_metrics(
        &self,
    ) -> Result<aiome_core_contracts::contracts::FederatedMetrics, AiomeError> {
        self.inner.fetch_federated_metrics().await
    }

    async fn export_federated_data(
        &self,
        since: Option<&str>,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>, Vec<ArenaMatch>), AiomeError> {
        self.inner.export_federated_data(since).await
    }

    async fn import_federated_data(
        &self,
        karmas: Vec<FederatedKarma>,
        rules: Vec<ImmuneRule>,
        matches: Vec<ArenaMatch>,
    ) -> Result<(), AiomeError> {
        self.inner
            .import_federated_data(karmas, rules, matches)
            .await
    }

    async fn get_peer_sync_time(&self, peer_url: &str) -> Result<Option<String>, AiomeError> {
        self.inner.get_peer_sync_time(peer_url).await
    }

    async fn update_peer_sync_time(
        &self,
        peer_url: &str,
        sync_time: &str,
    ) -> Result<(), AiomeError> {
        self.inner.update_peer_sync_time(peer_url, sync_time).await
    }

    async fn get_node_id(&self) -> Result<String, AiomeError> {
        self.inner.get_node_id().await
    }
}

#[async_trait]
impl HarnessRegistryOps for RealJobQueue {
    async fn store_harness_record(
        &self,
        record: &aiome_core_contracts::contracts::HarnessRecord,
    ) -> Result<(), AiomeError> {
        self.inner.store_harness_record(record).await
    }
    async fn fetch_harness_records_by_status(
        &self,
        status: &str,
    ) -> Result<Vec<aiome_core_contracts::contracts::HarnessRecord>, AiomeError> {
        self.inner.fetch_harness_records_by_status(status).await
    }
    async fn update_harness_status(&self, id: &str, status: &str) -> Result<(), AiomeError> {
        self.inner.update_harness_status(id, status).await
    }
    async fn delete_harness_record(&self, id: &str) -> Result<(), AiomeError> {
        self.inner.delete_harness_record(id).await
    }
    async fn fetch_harness_record_by_id(
        &self,
        id: &str,
    ) -> Result<Option<aiome_core_contracts::contracts::HarnessRecord>, AiomeError> {
        self.inner.fetch_harness_record_by_id(id).await
    }
    async fn increment_harness_stats(&self, id: &str, fire: bool) -> Result<(), AiomeError> {
        self.inner.increment_harness_stats(id, fire).await
    }
}

#[async_trait]
impl SoulStore for RealJobQueue {
    async fn load_soul(&self, id: &str) -> Result<Option<serde_json::Value>, AiomeError> {
        self.inner.load_soul(id).await
    }

    async fn store_soul_fragment(
        &self,
        fragment_yaml: &str,
        version_hash: &str,
    ) -> Result<(), AiomeError> {
        self.inner
            .store_soul_fragment(fragment_yaml, version_hash)
            .await
    }

    async fn fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError> {
        self.inner.fetch_latest_soul_fragment().await
    }

    async fn archive_lora_model(
        &self,
        soul_id: &str,
        generation: u32,
        lora_hash: &str,
        adapter_path: &str,
        base_model: &str,
    ) -> Result<(), AiomeError> {
        self.inner
            .archive_lora_model(soul_id, generation, lora_hash, adapter_path, base_model)
            .await
    }
}

#[async_trait]
impl SystemStateOps for RealJobQueue {
    async fn store_system_state(&self, key: &str, value: &str) -> Result<(), AiomeError> {
        self.inner.store_system_state(key, value).await
    }
    async fn fetch_system_state(&self, key: &str) -> Result<Option<String>, AiomeError> {
        self.inner.fetch_system_state(key).await
    }
}
