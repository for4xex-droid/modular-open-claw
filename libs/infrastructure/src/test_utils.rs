/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

//! # Test Utilities
//!
//! Provides mock implementations and helper functions for infrastructure testing.
use aiome_contracts::biome::BiomeMessage;
use aiome_contracts::contracts::{
    ArenaMatch, FederatedKarma, FederatedMetrics, ImmuneRule, OracleVerdict, SamsaraEvent,
};
use aiome_contracts::error::AiomeError;
use aiome_contracts::traits::{Job, JobQueue, JobStatus, KarmaSearchResult, SnsMetricsRecord};
use aiome_contracts::types::AgentStats;
use async_trait::async_trait;
use uuid::Uuid;

/// テストおよび開発用の JobQueue モック実装。
#[cfg(any(test, debug_assertions))]
pub struct MockJobQueue;

#[cfg(any(test, debug_assertions))]
#[async_trait]
impl JobQueue for MockJobQueue {
    async fn get_pending_job_count(&self) -> Result<i64, AiomeError> {
        Ok(0)
    }
    async fn enqueue(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: Option<aiome_core::security::PermissionManifest>,
        _: Option<Uuid>,
        _: i32,
    ) -> Result<String, AiomeError> {
        Ok("mock_id".into())
    }
    async fn fetch_all_karma(&self, _: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
        Ok(vec![])
    }
    async fn fetch_recent_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
        Ok(vec![])
    }
    async fn get_agent_stats(&self) -> Result<AgentStats, AiomeError> {
        Ok(AgentStats {
            level: 1,
            exp: 0,
            resonance: 0,
            creativity: 0,
            fatigue: 0,
        })
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
    async fn fetch_evolution_history(&self, _: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
        Ok(vec![])
    }
    async fn export_federated_data(
        &self,
        _: Option<&str>,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>, Vec<ArenaMatch>), AiomeError> {
        Ok((vec![], vec![], vec![]))
    }
    async fn dequeue(&self, _: &[&str]) -> Result<Option<Job>, AiomeError> {
        Ok(None)
    }
    async fn fetch_job(&self, _: &str) -> Result<Option<Job>, AiomeError> {
        Ok(None)
    }
    async fn complete_job(&self, _: &str, _: Option<&str>) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fail_job(&self, _: &str, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_relevant_karma(
        &self,
        _: &str,
        _: &str,
        _: i64,
        _: &str,
    ) -> Result<KarmaSearchResult, AiomeError> {
        Ok(KarmaSearchResult {
            entries: vec![],
            is_ood: false,
            max_score: 0.0,
        })
    }
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
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn adjust_karma_weight(&self, _: &str, _: i32) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn karma_decay_sweep(&self) -> Result<u64, AiomeError> {
        Ok(0)
    }
    async fn reclaim_zombie_jobs(&self, _: i64) -> Result<u64, AiomeError> {
        Ok(0)
    }
    async fn set_creative_rating(&self, _: &str, _: i32) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn heartbeat_pulse(&self, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn store_execution_log(&self, _: &str, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_undistilled_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
        Ok(vec![])
    }
    async fn mark_karma_extracted(&self, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_job_retry_count(&self, _: &str) -> Result<i64, AiomeError> {
        Ok(0)
    }
    async fn increment_job_retry_count(&self, _: &str) -> Result<bool, AiomeError> {
        Ok(true)
    }
    async fn reset_job_retry_count(&self, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn store_immune_rule(&self, _: &ImmuneRule) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn delete_immune_rule(&self, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_active_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        Ok(vec![])
    }
    async fn record_arena_match(&self, _: &ArenaMatch) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn import_federated_data(
        &self,
        _: Vec<FederatedKarma>,
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
    async fn fetch_federated_metrics(&self) -> Result<FederatedMetrics, AiomeError> {
        Ok(FederatedMetrics::default())
    }
    async fn get_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        Ok(vec![])
    }
    async fn get_node_id(&self) -> Result<String, AiomeError> {
        Ok("mock-node".into())
    }
    async fn sign_swarm_payload(&self, _: &str) -> Result<String, AiomeError> {
        Ok("mock-sig".into())
    }
    async fn tick_local_clock(&self) -> Result<u64, AiomeError> {
        Ok(0)
    }
    async fn sync_local_clock(&self, _: u64) -> Result<u64, AiomeError> {
        Ok(0)
    }
    async fn storage_gc(&self, _: f64) -> Result<u64, AiomeError> {
        Ok(0)
    }
    async fn store_chat_message(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_chat_history(
        &self,
        _: &str,
        _: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        Ok(vec![])
    }
    async fn store_expression(
        &self,
        _: &aiome_core::expression::Expression,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_expressions(
        &self,
        _: i64,
    ) -> Result<Vec<aiome_core::expression::Expression>, AiomeError> {
        Ok(vec![])
    }
    async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
        Ok(false)
    }
    async fn set_auto_expression_enabled(&self, _: bool) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn record_soul_mutation(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn purge_old_jobs(&self, _: i64) -> Result<u64, AiomeError> {
        Ok(0)
    }
    async fn link_sns_data(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_jobs_for_evaluation(&self, _: i64, _: i64) -> Result<Vec<Job>, AiomeError> {
        Ok(vec![])
    }
    async fn record_sns_metrics(
        &self,
        _: &str,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: Option<&str>,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn fetch_pending_evaluations(&self, _: i64) -> Result<Vec<SnsMetricsRecord>, AiomeError> {
        Ok(vec![])
    }
    async fn apply_final_verdict(
        &self,
        _: i64,
        _: OracleVerdict,
        _: &str,
    ) -> Result<(), AiomeError> {
        Ok(())
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
    async fn get_biome_topic_status(
        &self,
        _: &str,
    ) -> Result<Option<(i32, Option<String>)>, AiomeError> {
        Ok(None)
    }
    async fn advance_biome_turn(&self, _: &str, _: i64) -> Result<i32, AiomeError> {
        Ok(0)
    }
    async fn fetch_biome_messages(
        &self,
        _: &str,
        _: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        Ok(vec![])
    }
    async fn store_biome_message(&self, _: &BiomeMessage) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn update_biome_reputation(&self, _: &str, _: f64) -> Result<f64, AiomeError> {
        Ok(0.0)
    }
    async fn archive_biome_topic(&self, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn get_job_count_since(
        &self,
        _: chrono::DateTime<chrono::Utc>,
    ) -> Result<i64, AiomeError> {
        Ok(0)
    }
    async fn fetch_top_performing_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
        Ok(vec![])
    }
    async fn fetch_unincorporated_karma(
        &self,
        _: i64,
        _: &str,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        Ok(vec![])
    }
    async fn mark_karma_as_incorporated(&self, _: Vec<String>, _: &str) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn get_system_agent_id(&self) -> Result<Uuid, AiomeError> {
        Ok(Uuid::new_v4())
    }
}
