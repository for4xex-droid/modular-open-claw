/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::error::AiomeError;
use aiome_contracts::llm::LlmProvider;
use aiome_contracts::traits::JobQueue;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tracing::{info, warn};

pub struct SoulMutator {
    llm: Arc<dyn LlmProvider>,
    base_dir: PathBuf,
}

impl SoulMutator {
    pub fn new(llm: Arc<dyn LlmProvider>, base_dir: PathBuf) -> Self {
        Self { llm, base_dir }
    }

    pub async fn transmute(&self, jq: &dyn JobQueue) -> Result<bool, AiomeError> {
        let soul_path = self.base_dir.join("SOUL.md");
        let soul_content =
            fs::read_to_string(&soul_path)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to read SOUL.md: {}", e),
                })?;

        let prompt = format!(
            "Current Soul:\n{}\n\nMutate this soul to reflect recent development and lessons learned.",
            soul_content
        );

        let response = self.llm.complete(&prompt, None).await?;
        let mutated_soul = response.content;

        // Verify drift
        if mutated_soul.len() > soul_content.len() * 2 {
            warn!("Significant soul drift detected! Possible hallucination.");
            return Ok(false);
        }

        fs::write(&soul_path, mutated_soul)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to write mutated soul: {}", e),
            })?;

        info!("Soul transmutation successful.");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_contracts::contracts::{
        ArenaMatch, FederatedMetrics, ImmuneRule, KarmaEntry, OracleVerdict, SamsaraEvent,
    };
    use aiome_contracts::llm::{LlmResponse, StopReason};
    use aiome_contracts::traits::{
        Expression, Job, JobStatus, KarmaSearchResult, SnsMetricsRecord, SoulStore,
    };
    use aiome_contracts::types::AgentStats;
    use serde_json::json;
    use uuid::Uuid;

    #[derive(Debug)]
    struct MockLlm {
        mutation_response: String,
    }

    #[async_trait]
    impl LlmProvider for MockLlm {
        async fn complete(&self, _: &str, _: Option<&str>) -> Result<LlmResponse, AiomeError> {
            Ok(LlmResponse {
                content: self.mutation_response.clone(),
                stop_reason: StopReason::EndTurn,
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "mock"
        }
    }

    #[derive(Debug)]
    struct MockJobQueue {
        karma_lessons: Vec<String>,
        stats: AgentStats,
    }

    #[async_trait]
    impl JobQueue for MockJobQueue {
        #[allow(clippy::too_many_arguments)]
        async fn enqueue(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: Option<&str>,
            _: Option<aiome_contracts::security::PermissionManifest>,
            _: Option<Uuid>,
            _: i32,
        ) -> Result<String, AiomeError> {
            Ok("mock".into())
        }
        async fn dequeue(&self, _: &[&str]) -> Result<Option<Job>, AiomeError> {
            Ok(None)
        }
        async fn fetch_job(&self, id: &str) -> Result<Option<Job>, AiomeError> {
            Ok(None)
        }
        async fn complete_job(&self, _: &str, _: Option<&str>) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fail_job(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn reclaim_zombie_jobs(&self, _: i64) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn fetch_chat_history(
            &self,
            _: &str,
            _: i64,
        ) -> Result<Vec<serde_json::Value>, AiomeError> {
            Ok(vec![])
        }
        async fn store_chat_message(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn get_chat_memory_summary(&self, _: &str) -> Result<Option<String>, AiomeError> {
            Ok(None)
        }
        async fn update_chat_memory_summary(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn mark_chats_as_distilled(&self, _: &str, _: i64) -> Result<(), AiomeError> {
            Ok(())
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
        async fn fetch_relevant_karma(
            &self,
            _: &str,
            _: &str,
            _: i64,
            _: &str,
        ) -> Result<KarmaSearchResult, AiomeError> {
            Ok(KarmaSearchResult::empty())
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
        async fn fetch_undistilled_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
        }
        async fn mark_karma_extracted(&self, _: &str) -> Result<(), AiomeError> {
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
        async fn fetch_pending_evaluations(
            &self,
            _: i64,
        ) -> Result<Vec<SnsMetricsRecord>, AiomeError> {
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
        async fn fetch_recent_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
        }
        async fn get_agent_stats(&self) -> Result<AgentStats, AiomeError> {
            Ok(self.stats.clone())
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
        async fn fetch_evolution_history(
            &self,
            _: i64,
        ) -> Result<Vec<serde_json::Value>, AiomeError> {
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
        async fn fetch_all_karma(&self, _l: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
            Ok(self
                .karma_lessons
                .iter()
                .map(|l| json!({"lesson": l}))
                .collect())
        }
        async fn fetch_top_performing_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
        }
        async fn record_soul_mutation(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_job_retry_count(&self, _: &str) -> Result<i64, AiomeError> {
            Ok(0)
        }
        async fn reset_job_retry_count(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn increment_job_retry_count(&self, _: &str) -> Result<bool, AiomeError> {
            Ok(true)
        }
        async fn fetch_unincorporated_karma(
            &self,
            _: i64,
            _: &str,
        ) -> Result<Vec<serde_json::Value>, AiomeError> {
            Ok(vec![])
        }
        async fn mark_karma_as_incorporated(
            &self,
            _: Vec<String>,
            _: &str,
        ) -> Result<(), AiomeError> {
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
        async fn get_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
            Ok(vec![])
        }
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
        async fn mark_as_federated(
            &self,
            _: Vec<String>,
            _: Vec<String>,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_federated_metrics(&self) -> Result<FederatedMetrics, AiomeError> {
            Ok(FederatedMetrics::default())
        }
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
        async fn store_biome_message(
            &self,
            _: &aiome_contracts::biome::BiomeMessage,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn update_biome_reputation(&self, _: &str, _: f64) -> Result<f64, AiomeError> {
            Ok(0.0)
        }
        async fn archive_biome_topic(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn get_system_agent_id(&self) -> Result<Uuid, AiomeError> {
            Ok(Uuid::nil())
        }
        async fn fetch_relevant_karma_by_category(
            &self,
            _: &str,
            _: &str,
            _: i64,
        ) -> Result<KarmaSearchResult, AiomeError> {
            Ok(KarmaSearchResult::empty())
        }
        async fn store_expression(&self, _: &Expression) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_expressions(&self, _: i64) -> Result<Vec<Expression>, AiomeError> {
            Ok(vec![])
        }
        async fn store_soul_fragment(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError> {
            Ok(None)
        }

        async fn get_security_request_count(&self, _: Option<Uuid>) -> Result<u32, AiomeError> {
            Ok(0)
        }
        async fn increment_security_request_count(
            &self,
            _: Option<Uuid>,
        ) -> Result<u32, AiomeError> {
            Ok(1)
        }
    }

    #[async_trait]
    impl SoulStore for MockJobQueue {
        async fn load_soul(&self, _: &str) -> Result<Option<serde_json::Value>, AiomeError> {
            Ok(None)
        }
        async fn store_soul_fragment(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_soul_transmutation() {
        let temp_dir = tempfile::tempdir().unwrap().into_path();
        let _ = fs::create_dir_all(&temp_dir).await;
        fs::write(temp_dir.join("SOUL.md"), "A B C D E")
            .await
            .unwrap();

        let llm = Arc::new(MockLlm {
            mutation_response: "X Y Z".to_string(),
        });
        let mutator = SoulMutator::new(llm, temp_dir.clone());
        let jq = MockJobQueue {
            karma_lessons: vec!["Test lesson".into()],
            stats: AgentStats::default(),
        };

        let res = mutator.transmute(&jq).await;
        assert!(res.is_ok());

        let content = fs::read_to_string(temp_dir.join("SOUL.md")).await.unwrap();
        assert_eq!(content, "X Y Z");
    }
}
