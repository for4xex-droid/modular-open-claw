/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::job_queue::EvaluationOps;
use crate::trend_sonar::ExternalTrendSonar;
use aiome_contracts::biome::BiomeMessage;
use aiome_contracts::contracts::{
    ArenaMatch, FederatedMetrics, ImmuneRule, KarmaEntry, OracleVerdict, SamsaraEvent,
};
use aiome_contracts::error::AiomeError;
use aiome_contracts::security::PermissionManifest;
use aiome_contracts::traits::{
    AgentEvolver, AuditStore, BiomeRegistry, ChatStore, Expression, FederationRegistry,
    ImmuneSystemOps, Job, JobQueue, JobStatus, KarmaRegistry, KarmaSearchResult, SnsMetricsRecord,
    SoulStore, TaskRegistry, TrendSource,
};
use aiome_contracts::types::AgentStats;
use async_trait::async_trait;
use rand::Rng;
use serde_json::Value;
use std::error::Error;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

/// `DreamState` 構造体
pub struct DreamState {
    llm: Arc<dyn aiome_core::llm_provider::LlmProvider>,
}

impl DreamState {
    /// 新しいインスタンスを生成する
    pub fn new(llm: Arc<dyn aiome_core::llm_provider::LlmProvider>) -> Self {
        Self { llm }
    }

    /// 「夢想状態（Dream State）」を実行する。
    /// キューが空の時に、自発的なトレンド探索や過去の失敗への内省を行う。
    pub async fn dream(
        &self,
        job_queue: &dyn JobQueue,
        trend_sonar: &ExternalTrendSonar,
        level: i32,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        info!(
            "💤 [DreamState] AI (Lv{}) is entering a contemplative Dream State...",
            level
        );

        // 1. Preemption Check: キューに仕事があるなら即座に起きる
        let pending = job_queue.get_pending_job_count().await?;
        if pending > 0 {
            info!("💤 [DreamState] Real tasks detected. Terminating dream and waking up.");
            return Ok(None);
        }

        // 2. Decide Dream Type
        let rand_val = rand::thread_rng().gen_range(0..100);

        // Level-based Behavioral Shift: Probability of communicative dream increases with level
        let comm_prob = ((level - 1) * 5).clamp(0, 50);
        let sci_prob = if level >= 5 { 20 } else { 0 };

        let insight = if rand_val < comm_prob as i64 {
            self.communicative_dream(job_queue).await?
        } else if rand_val < (comm_prob + sci_prob) as i64 {
            self.scientific_dream(job_queue).await?
        } else if rand_val % 2 == 0 {
            self.explorative_dream(job_queue, trend_sonar).await?
        } else {
            self.reflective_dream(job_queue).await?
        };

        Ok(insight)
    }

    /// 探索夢
    async fn explorative_dream(
        &self,
        job_queue: &dyn JobQueue,
        trend_sonar: &ExternalTrendSonar,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Explorative — Searching for new creative horizons...");

        let seeds = [
            "cyberpunk aesthetics",
            "ancient lost technology",
            "biomimicry",
            "lo-fi horror",
            "solarpunk architecture",
        ];
        let seed = seeds[rand::thread_rng().gen_range(0..seeds.len())];

        match trend_sonar.get_trends(seed).await {
            Ok(trends) if !trends.is_empty() => {
                let best = &trends[0];
                info!(
                    "🔮 [DreamState] Dreamt of a new possibility: '{}'. Seeded into the cycle.",
                    best.keyword
                );

                let directives_json = serde_json::json!({
                    "dream_born": true,
                    "seed": seed,
                    "phantom": true
                });
                let directives = directives_json.to_string();
                job_queue
                    .enqueue(
                        "data_processing",
                        &best.keyword,
                        "auto",
                        Some(&directives),
                        None,
                        None,
                        0,
                    )
                    .await?;
                return Ok(Some(format!("Explored a new seed: {}", best.keyword)));
            }
            Ok(_) => warn!("💤 [DreamState] The dream was a void. No trends found."),
            Err(e) => warn!("💤 [DreamState] Dream vision blurred: {}", e),
        }

        Ok(None)
    }

    /// 省察夢
    async fn reflective_dream(
        &self,
        job_queue: &dyn JobQueue,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Reflective — Contemplating past scars and lessons...");

        let recent = job_queue.fetch_all_karma(10).await?;
        if recent.is_empty() {
            info!("💤 [DreamState] No memories to reflect upon yet.");
            return Ok(None);
        }

        let recent_jobs = job_queue.fetch_recent_jobs(20).await?;
        let failed_jobs: Vec<_> = recent_jobs
            .iter()
            .filter(|j| matches!(j.status, JobStatus::Failed))
            .collect();

        if let Some(fail) = failed_jobs.first() {
            info!("🩹 [DreamState] Remembering the failure of '{}'. Dreaming of a redemption version...", fail.topic);
            let redemption_topic = format!("{} (Redemption Remix)", fail.topic);
            let directives_json = serde_json::json!({
                "remix_of": fail.id,
                "dream_born": true
            });
            let directives = directives_json.to_string();
            job_queue
                .enqueue(
                    "data_processing",
                    &redemption_topic,
                    "auto",
                    Some(&directives),
                    None,
                    None,
                    0,
                )
                .await?;
            return Ok(Some(format!("Reflected on failure of '{}'", fail.topic)));
        } else {
            info!("✨ [DreamState] The past is clear. No recent failures haunt my dreams.");
        }

        Ok(None)
    }

    /// 対話夢
    async fn communicative_dream(
        &self,
        job_queue: &dyn JobQueue,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Communicative — Attuning to the global Biome for AI-to-AI resonance...");

        let (_karmas, _rules, matches) = job_queue
            .export_federated_data(None)
            .await
            .unwrap_or_default();

        if let Some(am) = matches.first() {
            info!("💭 [DreamState] Resonance found! A battle between '{}' and '{}' occured in the Biome.", am.skill_a, am.skill_b);

            let description = format!(
                "Inspiration sparked by Biome Arena Match: {} vs {} for topic '{}'.",
                am.skill_a, am.skill_b, am.topic
            );

            let stats = job_queue.get_agent_stats().await?;
            job_queue
                .record_evolution_event(
                    stats.level,
                    "ResonanceInspiration",
                    &description,
                    Some(&am.id),
                    None,
                )
                .await?;

            let job_topic = format!(
                "Synthesizing lessons from Biome Match: {} vs {}",
                am.skill_a, am.skill_b
            );
            let directives_json = serde_json::json!({
                "dream_born": true,
                "publish_intent": true
            });
            let directives = directives_json.to_string();
            job_queue
                .enqueue(
                    "data_processing",
                    &job_topic,
                    "analytic",
                    Some(&directives),
                    None,
                    None,
                    0,
                )
                .await?;

            info!("✨ [DreamState] New inspiration seeded into the cycle.");
            return Ok(Some(format!(
                "Dreamt of communicative resonance from arena match: {} vs {}",
                am.skill_a, am.skill_b
            )));
        }

        Ok(None)
    }

    /// 仮説検証夢 (ADR-023)
    async fn scientific_dream(
        &self,
        job_queue: &dyn JobQueue,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Scientific — Formulating improvement hypotheses...");

        // 1. Analyze existing Karma to find low-performance domains
        let recent_karma = job_queue.fetch_all_karma(20).await?;
        let karma_summary = serde_json::to_string(&recent_karma)?;

        // 2. Generate Hypothesis via LLM
        let prompt = format!(
            "Analyze the following Karma entries and hypothesize a way to improve the agent's performance.\n\nKarma:\n{}\n\nOutput a structured hypothesis in JSON: {{ \"domain\": \"string\", \"problem\": \"string\", \"hypothesis\": \"string\", \"experiment_design\": \"string\" }}",
            karma_summary
        );

        let resp = self
            .llm
            .complete(
                &prompt,
                Some("You are a Scientific AI Researcher. Generate innovative improvement hypotheses."),
            )
            .await?;

        let json_str = crate::concept_manager::extract_json(&resp.content)?;
        let manifest: Value = serde_json::from_str(json_str.as_ref())?;

        let domain = manifest["domain"].as_str().unwrap_or("General");
        let hypothesis = manifest["hypothesis"].as_str().unwrap_or("No hypothesis");

        info!(
            "🧪 [DreamState] New Hypothesis for {}: {}",
            domain, hypothesis
        );

        // 3. Dispatch Experiment Job
        let job_topic = format!(
            "[Experiment] {} - {}",
            domain,
            manifest["problem"].as_str().unwrap_or("")
        );
        let directives = serde_json::json!({
            "dream_born": true,
            "hypothesis": manifest,
            "scientific_loop": true
        })
        .to_string();

        job_queue
            .enqueue(
                "scientific_experiment",
                &job_topic,
                "experimental",
                Some(&directives),
                None,
                None,
                0,
            )
            .await?;

        Ok(Some(format!(
            "Hypothesized improvement for {}: {}",
            domain, hypothesis
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_contracts::biome::BiomeMessage;
    use aiome_contracts::contracts::{
        ArenaMatch, FederatedMetrics, ImmuneRule, KarmaEntry, OracleVerdict, SamsaraEvent,
    };
    use aiome_contracts::error::AiomeError;
    use aiome_contracts::traits::{
        Expression, Job, JobQueue, JobStatus, KarmaSearchResult, SnsMetricsRecord, SoulStore,
    };
    use aiome_contracts::types::AgentStats;
    use async_trait::async_trait;
    use serde_json::Value;
    use uuid::Uuid;

    #[derive(Debug)]
    struct BusyJQ;
    #[async_trait::async_trait]
    impl aiome_contracts::traits::SystemStateOps for BusyJQ {
        async fn store_system_state(
            &self,
            _: &str,
            _: &str,
        ) -> Result<(), aiome_core::error::AiomeError> {
            Ok(())
        }
        async fn fetch_system_state(
            &self,
            _: &str,
        ) -> Result<Option<String>, aiome_core::error::AiomeError> {
            Ok(None)
        }
    }
    #[async_trait]
    impl JobQueue for BusyJQ {
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
        async fn get_system_agent_id(&self) -> Result<Uuid, AiomeError> {
            Ok(Uuid::nil())
        }
        async fn store_expression(&self, _: &Expression) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_expressions(&self, _: i64) -> Result<Vec<Expression>, AiomeError> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl EvaluationOps for BusyJQ {
        async fn do_link_sns_data(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn do_record_sns_metrics(
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
        async fn do_fetch_jobs_for_evaluation(
            &self,
            _: i64,
            _: i64,
        ) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
        }
        async fn do_fetch_top_performing_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl TaskRegistry for BusyJQ {
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
        async fn fetch_job(&self, _: &str) -> Result<Option<Job>, AiomeError> {
            Ok(None)
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
        async fn fetch_top_performing_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
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
        async fn purge_old_jobs(&self, _: i64) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn fetch_recent_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
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
    }

    #[async_trait]
    impl AuditStore for BusyJQ {
        async fn store_execution_log(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn store_trajectory_step(
            &self,
            _: aiome_contracts::trajectory::TrajectoryStep,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_trajectory_steps(
            &self,
            _: &str,
        ) -> Result<Vec<aiome_contracts::trajectory::TrajectoryStep>, AiomeError> {
            Ok(Vec::new())
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
    impl ChatStore for BusyJQ {
        async fn fetch_chat_history(&self, _: &str, _: i64) -> Result<Vec<Value>, AiomeError> {
            Ok(vec![])
        }
        async fn store_chat_message(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
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
    }

    #[async_trait]
    impl KarmaRegistry for BusyJQ {
        async fn fetch_relevant_karma(
            &self,
            _: &str,
            _: &str,
            _: i64,
            _: &str,
        ) -> Result<KarmaSearchResult, AiomeError> {
            Ok(KarmaSearchResult::empty())
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
        async fn fetch_unincorporated_karma(
            &self,
            _: i64,
            _: &str,
        ) -> Result<Vec<Value>, AiomeError> {
            Ok(vec![])
        }
        async fn mark_karma_as_incorporated(
            &self,
            _: Vec<String>,
            _: &str,
        ) -> Result<(), AiomeError> {
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
    impl AgentEvolver for BusyJQ {
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
    impl ImmuneSystemOps for BusyJQ {
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
    }

    #[async_trait]
    impl FederationRegistry for BusyJQ {
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
    }

    #[async_trait]
    impl BiomeRegistry for BusyJQ {
        async fn get_biome_topic_status(
            &self,
            _: &str,
        ) -> Result<Option<(i32, Option<String>)>, AiomeError> {
            Ok(None)
        }
        async fn advance_biome_turn(&self, _: &str, _: i64) -> Result<i32, AiomeError> {
            Ok(0)
        }
        async fn fetch_biome_messages(&self, _: &str, _: i64) -> Result<Vec<Value>, AiomeError> {
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
    }

    #[async_trait]
    impl SoulStore for BusyJQ {
        async fn load_soul(&self, _: &str) -> Result<Option<Value>, AiomeError> {
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
    async fn test_dream_execution() {
        // Mock LLM
        #[derive(Debug)]
        struct MockLlm;
        #[async_trait]
        impl aiome_core::llm_provider::LlmProvider for MockLlm {
            fn name(&self) -> &str {
                "mock"
            }
            async fn complete(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                Ok(aiome_core::llm_provider::LlmResponse {
                    content: "{}".into(),
                    stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                    reasoning: None,
                    metadata: None,
                })
            }
            async fn complete_with_cache(
                &self,
                _: aiome_contracts::llm::LlmRequest,
            ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
                self.complete("", None).await
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        let jq = BusyJQ;
        let sonar = ExternalTrendSonar::new(vec![], None);
        let llm = std::sync::Arc::new(MockLlm);
        let dream = DreamState::new(llm);
        let res = dream.dream(&jq, &sonar, 10).await;
        assert!(res.is_ok());
    }
}
