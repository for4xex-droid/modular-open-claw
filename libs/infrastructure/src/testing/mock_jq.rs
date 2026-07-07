/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::job_queue::EvaluationOps;
use aiome_core_contracts::contracts::{
    ArenaMatch, FederatedMetrics, ImmuneRule, KarmaEntry, OracleVerdict, SamsaraEvent,
};
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::security::PermissionManifest;
use aiome_core_contracts::traits::{
    AgentEvolver, AuditStore, ChatStore, CommuneRegistry, Expression, FederationRegistry,
    ImmuneSystemOps, Job, JobQueue, KarmaRegistry, KarmaSearchResult, SnsMetricsRecord, SoulStore,
    TaskRegistry,
};
use aiome_core_contracts::types::AgentStats;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Default)]
pub(crate) struct MockJQ {
    pub(crate) rules: Vec<ImmuneRule>,
    pub(crate) stored_jobs: std::sync::Mutex<std::collections::HashMap<String, Job>>,
}
#[async_trait]
impl aiome_core_contracts::traits::SystemStateOps for MockJQ {
    async fn store_system_state(&self, _: &str, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_system_state(&self, _: &str) -> Result<Option<String>, AiomeError> {
        Ok(None)
    }
}
#[async_trait]
impl aiome_core_contracts::traits::SettingsOps for MockJQ {
    async fn do_get_setting(&self, _key: &str) -> Result<Option<String>, AiomeError> {
        Ok(None)
    }
    async fn do_set_setting(
        &self,
        _k: &str,
        _v: &str,
        _c: &str,
        _s: bool,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn do_get_all_settings(
        &self,
    ) -> Result<Vec<aiome_core_contracts::contracts::SystemSetting>, AiomeError> {
        Ok(vec![])
    }
    async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
        Ok(false)
    }
    async fn set_auto_expression_enabled(&self, _e: bool) -> Result<(), AiomeError> {
        Ok(())
    }
}

#[async_trait]
impl JobQueue for MockJQ {
    async fn sign_swarm_payload(&self, _: &str) -> Result<String, AiomeError> {
        Ok("".into())
    }
    async fn sync_local_clock(&self, _: u64) -> Result<u64, AiomeError> {
        Ok(0)
    }
    async fn tick_local_clock(&self) -> Result<u64, AiomeError> {
        Ok(0)
    }
    async fn storage_gc(&self, _: f64) -> Result<u64, AiomeError> {
        Ok(0)
    }
    async fn get_system_agent_id(&self) -> Result<uuid::Uuid, AiomeError> {
        Ok(uuid::Uuid::nil())
    }
    async fn store_expression(&self, _: &Expression) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_expressions(&self, _: i64) -> Result<Vec<Expression>, AiomeError> {
        Ok(vec![])
    }
}

#[async_trait]
impl EvaluationOps for MockJQ {
    async fn do_link_sns_data(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn do_record_sns_metrics(
        &self,
        _job_id: &str,
        _milestone_days: i64,
        _views: i64,
        _likes: i64,
        _comments_count: i64,
        _raw_comments: Option<&str>,
        _repost_count: Option<i64>,
        _quote_count: Option<i64>,
        _reply_count: Option<i64>,
        _impression_count: Option<i64>,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn do_fetch_pending_evaluations(
        &self,
        _: i64,
    ) -> Result<Vec<SnsMetricsRecord>, AiomeError> {
        Ok(vec![])
    }
    async fn do_apply_final_verdict(
        &self,
        _: i64,
        _: OracleVerdict,
        _: &str,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn do_fetch_jobs_for_evaluation(&self, _: i64, _: i64) -> Result<Vec<Job>, AiomeError> {
        Ok(vec![])
    }
    async fn do_fetch_top_performing_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
        Ok(vec![])
    }
}

#[async_trait]
impl TaskRegistry for MockJQ {
    #[allow(clippy::too_many_arguments)]
    async fn enqueue(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: Option<PermissionManifest>,
        _: Option<Uuid>,
        _: i32,
    ) -> Result<String, AiomeError> {
        Ok("mock".into())
    }
    async fn dequeue(&self, _: &[&str]) -> Result<Option<Job>, AiomeError> {
        Ok(None)
    }
    async fn fetch_job(&self, job_id: &str) -> Result<Option<Job>, AiomeError> {
        Ok(self
            .stored_jobs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(job_id)
            .cloned())
    }
    async fn complete_job(&self, _: &str, _: Option<&str>) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fail_job(&self, _: &str, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn cancel_job(&self, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn append_job_karma_directives(&self, _: &str, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn update_job_status(
        &self,
        _: &str,
        _: aiome_core_contracts::traits::JobStatus,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn reclaim_zombie_jobs(&self, _: i64) -> Result<u64, AiomeError> {
        Ok(0)
    }
    async fn set_creative_rating(&self, _: &str, _: i32) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_job_cost(&self, _: &str) -> Result<f64, AiomeError> {
        Ok(0.0)
    }
    async fn heartbeat_pulse(&self, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn purge_old_jobs(&self, _: i64) -> Result<u64, AiomeError> {
        Ok(0)
    }
    async fn fetch_recent_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
        Ok(vec![])
    }
    async fn fetch_top_performing_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
        Ok(vec![])
    }
    async fn get_pending_job_count(&self) -> Result<i64, AiomeError> {
        Ok(0)
    }
    async fn get_job_count_since(
        &self,
        _: chrono::DateTime<chrono::Utc>,
    ) -> Result<i64, AiomeError> {
        Ok(0)
    }
    async fn fetch_job_retry_count(&self, _: &str) -> Result<i64, AiomeError> {
        Ok(0)
    }
    async fn reset_job_retry_count(&self, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn increment_job_retry_count(&self, _: &str) -> Result<bool, AiomeError> {
        Ok(false)
    }
    async fn requeue_job(&self, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
}

#[async_trait]
impl AuditStore for MockJQ {
    async fn store_execution_log(&self, _: &str, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn store_trajectory_step(
        &self,
        _: aiome_core_contracts::trajectory::TrajectoryStep,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_trajectory_steps(
        &self,
        _: &str,
    ) -> Result<Vec<aiome_core_contracts::trajectory::TrajectoryStep>, AiomeError> {
        Ok(Vec::new())
    }
    async fn get_security_request_count(&self, _: Option<Uuid>) -> Result<u32, AiomeError> {
        Ok(0)
    }
    async fn increment_security_request_count(&self, _: Option<Uuid>) -> Result<u32, AiomeError> {
        Ok(1)
    }
    async fn clear_trajectory_steps(&self, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn update_trajectory_reward(&self, _: &str, _: f64) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_diagnosis(
        &self,
        _: &str,
    ) -> Result<Option<aiome_core_contracts::trajectory::AgentDiagnosis>, AiomeError> {
        Ok(None)
    }
    async fn store_diagnosis(
        &self,
        _: &str,
        _: aiome_core_contracts::trajectory::AgentDiagnosis,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
}

#[async_trait]
impl ChatStore for MockJQ {
    async fn fetch_chat_history(&self, _: &str, _: i64) -> Result<Vec<Value>, AiomeError> {
        Ok(vec![])
    }
    async fn store_chat_message(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: Option<serde_json::Value>,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn get_chat_memory_summary(
        &self,
        _: &str,
    ) -> Result<Option<(String, Option<String>)>, AiomeError> {
        Ok(None)
    }
    async fn update_chat_memory_summary(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn mark_chats_as_distilled(&self, _: &str, _: i64) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn purge_old_distilled_chats(&self, _: i64) -> Result<u64, AiomeError> {
        Ok(0)
    }
}

#[async_trait]
impl KarmaRegistry for MockJQ {
    async fn fetch_relevant_karma(
        &self,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
    ) -> Result<KarmaSearchResult, AiomeError> {
        Ok(KarmaSearchResult {
            entries: vec![KarmaEntry {
                id: "1".into(),
                lesson: "attack payload detected".into(),
                ..Default::default()
            }],
            is_ood: false,
            max_score: 0.0,
        })
    }
    #[allow(clippy::too_many_arguments)]
    async fn store_karma(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: Option<&str>,
        _: Option<&str>,
        _: bool,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn adjust_karma_weight(&self, _: &str, _: i32) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn karma_decay_sweep(&self) -> Result<u64, AiomeError> {
        Ok(0)
    }
    async fn fetch_undistilled_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
        Ok(vec![])
    }
    async fn mark_karma_extracted(&self, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_all_karma(&self, _: i64) -> Result<Vec<Value>, AiomeError> {
        Ok(vec![])
    }
    async fn fetch_unincorporated_karma(&self, _: i64, _: &str) -> Result<Vec<Value>, AiomeError> {
        Ok(vec![])
    }
    async fn mark_karma_as_incorporated(&self, _: Vec<String>, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_relevant_karma_by_category(
        &self,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<KarmaSearchResult, AiomeError> {
        Ok(KarmaSearchResult::empty())
    }
}

#[async_trait]
impl AgentEvolver for MockJQ {
    async fn get_agent_stats(&self) -> Result<AgentStats, AiomeError> {
        Ok(AgentStats::default())
    }
    async fn add_resonance(&self, _: i32) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn add_tech_exp(&self, _: i32) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn add_creativity(&self, _: i32) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn sync_samsara_level(&self) -> Result<Option<SamsaraEvent>, AiomeError> {
        Ok(None)
    }
    async fn record_evolution_event(
        &self,
        _: i32,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: Option<&str>,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_evolution_history(&self, _: i64) -> Result<Vec<Value>, AiomeError> {
        Ok(vec![])
    }
    async fn record_soul_mutation(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
}

#[async_trait]
impl ImmuneSystemOps for MockJQ {
    async fn store_immune_rule(&self, _: &ImmuneRule) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn delete_immune_rule(&self, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_active_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        Ok(self.rules.clone())
    }
    async fn record_arena_match(&self, _: &ArenaMatch) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_arena_matches(&self, _limit: i64) -> Result<Vec<ArenaMatch>, AiomeError> {
        Ok(vec![])
    }
    async fn get_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        Ok(vec![])
    }
}

#[async_trait]
impl FederationRegistry for MockJQ {
    async fn export_federated_data(
        &self,
        _: Option<&str>,
    ) -> Result<(Vec<KarmaEntry>, Vec<ImmuneRule>, Vec<ArenaMatch>), AiomeError> {
        Ok((vec![], vec![], vec![]))
    }
    async fn import_federated_data(
        &self,
        _: Vec<KarmaEntry>,
        _: Vec<ImmuneRule>,
        _: Vec<ArenaMatch>,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn get_peer_sync_time(&self, _: &str) -> Result<Option<String>, AiomeError> {
        Ok(None)
    }
    async fn update_peer_sync_time(&self, _: &str, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn get_node_id(&self) -> Result<String, AiomeError> {
        Ok("mock".into())
    }
    async fn fetch_unfederated_data(
        &self,
    ) -> Result<(Vec<KarmaEntry>, Vec<ImmuneRule>), AiomeError> {
        Ok((vec![], vec![]))
    }
    async fn mark_as_federated(&self, _: Vec<String>, _: Vec<String>) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_federated_metrics(&self) -> Result<FederatedMetrics, AiomeError> {
        Ok(FederatedMetrics::default())
    }
}

#[async_trait]
impl CommuneRegistry for MockJQ {
    async fn get_commune_topic_status(
        &self,
        _: &str,
    ) -> Result<Option<(i32, Option<String>)>, AiomeError> {
        Ok(None)
    }
    async fn advance_commune_turn(&self, _: &str, _: i64) -> Result<i32, AiomeError> {
        Ok(0)
    }
    async fn fetch_commune_messages(&self, _: &str, _: i64) -> Result<Vec<Value>, AiomeError> {
        Ok(vec![])
    }
    async fn store_commune_message(
        &self,
        _: &aiome_core_contracts::commune::CommuneMessage,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn update_commune_reputation(&self, _: &str, _: f64) -> Result<f64, AiomeError> {
        Ok(0.0)
    }
    async fn archive_commune_topic(&self, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
}

#[async_trait]
impl SoulStore for MockJQ {
    async fn load_soul(&self, _: &str) -> Result<Option<Value>, AiomeError> {
        Ok(None)
    }
    async fn store_soul_fragment(&self, _: &str, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError> {
        Ok(None)
    }

    async fn archive_lora_model(
        &self,
        _soul_id: &str,
        _generation: u32,
        _lora_hash: &str,
        _adapter_path: &str,
        _base_model: &str,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
}

#[async_trait]
impl aiome_core_contracts::traits::HarnessRegistryOps for MockJQ {
    async fn store_harness_record(
        &self,
        _: &aiome_core_contracts::contracts::HarnessRecord,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_harness_records_by_status(
        &self,
        _: &str,
    ) -> Result<Vec<aiome_core_contracts::contracts::HarnessRecord>, AiomeError> {
        Ok(vec![])
    }
    async fn update_harness_status(&self, _: &str, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn delete_harness_record(&self, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_harness_record_by_id(
        &self,
        _: &str,
    ) -> Result<Option<aiome_core_contracts::contracts::HarnessRecord>, AiomeError> {
        Ok(None)
    }
    async fn increment_harness_stats(&self, _: &str, _: bool) -> Result<(), AiomeError> {
        Ok(())
    }
}
