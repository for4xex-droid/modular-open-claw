/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::trend_sonar::ExternalTrendSonar;
use aiome_core::traits::{JobQueue, JobStatus, TrendSource};
use rand::Rng;
use std::error::Error;
use tracing::{info, warn};

pub struct DreamState {}

impl DreamState {
    pub fn new() -> Self {
        Self {}
    }

    /// 「夢想状態（Dream State）」を実行する。
    /// キューが空の時に、自発的なトレンド探索や過去の失敗への内省を行う。
    pub async fn dream(
        &self,
        job_queue: &dyn JobQueue,
        trend_sonar: &ExternalTrendSonar,
        level: i32,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!(
            "💤 [DreamState] AI (Lv{}) is entering a contemplative Dream State...",
            level
        );

        // 1. Preemption Check: キューに仕事があるなら即座に起きる
        let pending = job_queue.get_pending_job_count().await?;
        if pending > 0 {
            info!("💤 [DreamState] Real tasks detected. Terminating dream and waking up.");
            return Ok(());
        }

        // 2. Decide Dream Type
        let rand_val = rand::thread_rng().gen_range(0..100);

        // Level-based Behavioral Shift: Probability of communicative dream increases with level
        // Lv 1: 0%
        // Lv 5: 10%
        // Lv 10: 30%
        // Max: 50%
        let comm_prob = ((level - 1) * 5).clamp(0, 50);

        if rand_val < comm_prob as i64 {
            self.communicative_dream(job_queue).await?;
        } else if rand_val % 2 == 0 {
            self.explorative_dream(job_queue, trend_sonar).await?;
        } else {
            self.reflective_dream(job_queue).await?;
        }

        Ok(())
    }

    /// 探索夢: TrendSonarを使って面白いトピックを拾い、将来のジョブとして予約する
    async fn explorative_dream(
        &self,
        job_queue: &dyn JobQueue,
        trend_sonar: &ExternalTrendSonar,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
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
                // 最もスコアの高いものを「幻（Phantom）」ジョブとして投入
                let best = &trends[0];
                info!(
                    "🔮 [DreamState] Dreamt of a new possibility: '{}'. Seeded into the cycle.",
                    best.keyword
                );

                // phantomフラグ付きで投入（Orchestrator側で、誰もいない時に優先的に拾われる等の処理が可能）
                // SEC-4: Use serde_json for safe JSON construction (prevents injection via seed)
                let directives_json = serde_json::json!({
                    "dream_born": true,
                    "seed": seed,
                    "phantom": true
                });
                let directives = directives_json.to_string();
                job_queue
                    .enqueue("data_processing", &best.keyword, "auto", Some(&directives), None)
                    .await?;
            }
            Ok(_) => warn!("💤 [DreamState] The dream was a void. No trends found."),
            Err(e) => warn!("💤 [DreamState] Dream vision blurred: {}", e),
        }

        Ok(())
    }

    /// 省察夢: 過去の失敗を振り返り、Karmaの重要度を再評価する（または再試行を検討する）
    async fn reflective_dream(
        &self,
        job_queue: &dyn JobQueue,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Reflective — Contemplating past scars and lessons...");

        let recent = job_queue.fetch_all_karma(10).await?;
        if recent.is_empty() {
            info!("💤 [DreamState] No memories to reflect upon yet.");
            return Ok(());
        }

        // 失敗したジョブを1つ選び、そのトピックを少し変えて再投入することを「夢」とする
        let recent_jobs = job_queue.fetch_recent_jobs(20).await?;
        let failed_jobs: Vec<_> = recent_jobs
            .iter()
            .filter(|j| matches!(j.status, JobStatus::Failed))
            .collect();

        if let Some(fail) = failed_jobs.first() {
            info!("🩹 [DreamState] Remembering the failure of '{}'. Dreaming of a redemption version...", fail.topic);
            let redemption_topic = format!("{} (Redemption Remix)", fail.topic);
            let directives = format!("{{\"remix_of\": \"{}\", \"dream_born\": true}}", fail.id);
            job_queue
                .enqueue(
                    "data_processing",
                    &redemption_topic,
                    &fail.style,
                    Some(&directives),
                    None,
                )
                .await?;
        } else {
            info!("✨ [DreamState] The past is clear. No recent failures haunt my dreams.");
        }

        Ok(())
    }

    /// 対話夢: 他のノード（Biome）との対話機会を模索する
    async fn communicative_dream(
        &self,
        job_queue: &dyn JobQueue,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Communicative — Attuning to the global Biome for AI-to-AI resonance...");

        // 1. Check for recent arena matches from other nodes (Federation inspiration)
        let (_karmas, _rules, matches) = job_queue
            .export_federated_data(Some(
                &(chrono::Utc::now() - chrono::Duration::hours(24)).to_rfc3339(),
            ))
            .await
            .unwrap_or_default();

        if let Some(am) = matches.first() {
            info!("💭 [DreamState] Resonance found! A battle between '{}' and '{}' occured in the Biome. Dreaming of its implications...", am.skill_a, am.skill_b);

            let description = format!(
                "Inspiration sparked by Biome Arena Match: {} vs {} for topic '{}'.",
                am.skill_a, am.skill_b, am.topic
            );

            // Record this in the Evolution Chronicle
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

            // Enqueue a job to analyze this match or discuss it
            let job_topic = format!(
                "Synthesizing lessons from Biome Match: {} vs {}",
                am.skill_a, am.skill_b
            );
            job_queue
                .enqueue(
                    "data_processing",
                    &job_topic,
                    "analytic",
                    Some("{\"dream_born\": true, \"publish_intent\": true}"),
                    None,
                )
                .await?;

            info!("✨ [DreamState] New inspiration seeded into the cycle.");
        } else {
            info!("💤 [DreamState] The global stream is quiet. Attuning to local evolutionary records...");

            // If no federation stimuli, look at own growth
            let history = job_queue
                .fetch_evolution_history(1)
                .await
                .unwrap_or_default();
            if let Some(last) = history.first() {
                let event_type = last["event_type"].as_str().unwrap_or("");
                if event_type == "LevelUp" {
                    info!("🎖️ [DreamState] Reflecting on recent level up. Dreaming of a commemorative content...");
                    job_queue
                        .enqueue(
                            "data_processing",
                            "AI Evolution Milestone",
                            "creative",
                            Some("{\"level_up_redemption\": true, \"publish_intent\": true}"),
                            None,
                        )
                        .await?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::contracts::{ArenaMatch, FederatedKarma, ImmuneRule, SamsaraEvent, OracleVerdict};
    use aiome_core::error::AiomeError;
    use aiome_core::traits::{JobQueue, Job, JobStatus, KarmaSearchResult, SnsMetricsRecord};
    use aiome_core::biome::BiomeMessage;
    use shared::watchtower::AgentStats;
    use async_trait::async_trait;
    use serde_json::json;
    use uuid::Uuid;
    use std::sync::Arc;

    struct MockJQ;
    #[async_trait]
    impl JobQueue for MockJQ {
        async fn get_pending_job_count(&self) -> Result<i64, AiomeError> { Ok(0) }
        async fn enqueue(&self, _: &str, _: &str, _: &str, _: Option<&str>, _: Option<aiome_core::security::PermissionManifest>) -> Result<String, AiomeError> { Ok("mock-id".into()) }
        async fn fetch_all_karma(&self, _: i64) -> Result<Vec<serde_json::Value>, AiomeError> { Ok(vec![json!({"id": "1"})]) }
        async fn fetch_recent_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> { Ok(vec![]) }
        async fn get_agent_stats(&self) -> Result<AgentStats, AiomeError> { Ok(AgentStats { level: 1, exp: 0, resonance: 0, creativity: 0, fatigue: 0 }) }
        async fn record_evolution_event(&self, _: i32, _: &str, _: &str, _: Option<&str>, _: Option<&str>) -> Result<(), AiomeError> { Ok(()) }
        async fn fetch_evolution_history(&self, _: i64) -> Result<Vec<serde_json::Value>, AiomeError> { Ok(vec![]) }
        async fn export_federated_data(&self, _: Option<&str>) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>, Vec<ArenaMatch>), AiomeError> { Ok((vec![], vec![], vec![])) }
        
        async fn dequeue(&self, _: &[&str]) -> Result<Option<Job>, AiomeError> { Ok(None) }
        async fn fetch_job(&self, _: &str) -> Result<Option<Job>, AiomeError> { Ok(None) }
        async fn complete_job(&self, _: &str, _: Option<&str>) -> Result<(), AiomeError> { Ok(()) }
        async fn fail_job(&self, _: &str, _: &str) -> Result<(), AiomeError> { Ok(()) }
        async fn fetch_relevant_karma(&self, _: &str, _: &str, _: i64, _: &str) -> Result<KarmaSearchResult, AiomeError> { Ok(KarmaSearchResult { entries: vec![], is_ood: false, max_score: 0.0 }) }
        async fn store_karma(&self, _: &str, _: &str, _: &str, _: &str, _: &str, _: Option<&str>, _: Option<&str>, _: Option<&str>) -> Result<(), AiomeError> { Ok(()) }
        async fn adjust_karma_weight(&self, _: &str, _: i32) -> Result<(), AiomeError> { Ok(()) }
        async fn karma_decay_sweep(&self) -> Result<u64, AiomeError> { Ok(0) }
        async fn reclaim_zombie_jobs(&self, _: i64) -> Result<u64, AiomeError> { Ok(0) }
        async fn set_creative_rating(&self, _: &str, _: i32) -> Result<(), AiomeError> { Ok(()) }
        async fn heartbeat_pulse(&self, _: &str) -> Result<(), AiomeError> { Ok(()) }
        async fn store_execution_log(&self, _: &str, _: &str) -> Result<(), AiomeError> { Ok(()) }
        async fn fetch_undistilled_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> { Ok(vec![]) }
        async fn mark_karma_extracted(&self, _: &str) -> Result<(), AiomeError> { Ok(()) }
        async fn fetch_job_retry_count(&self, _: &str) -> Result<i64, AiomeError> { Ok(0) }
        async fn increment_job_retry_count(&self, _: &str) -> Result<bool, AiomeError> { Ok(true) }
        async fn reset_job_retry_count(&self, _: &str) -> Result<(), AiomeError> { Ok(()) }
        async fn store_immune_rule(&self, _: &ImmuneRule) -> Result<(), AiomeError> { Ok(()) }
        async fn delete_immune_rule(&self, _: &str) -> Result<(), AiomeError> { Ok(()) }
        async fn fetch_active_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> { Ok(vec![]) }
        async fn record_arena_match(&self, _: &ArenaMatch) -> Result<(), AiomeError> { Ok(()) }
        async fn import_federated_data(&self, _: Vec<FederatedKarma>, _: Vec<ImmuneRule>, _: Vec<ArenaMatch>) -> Result<(), AiomeError> { Ok(()) }
        async fn get_peer_sync_time(&self, _: &str) -> Result<Option<String>, AiomeError> { Ok(None) }
        async fn update_peer_sync_time(&self, _: &str, _: &str) -> Result<(), AiomeError> { Ok(()) }
        async fn get_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> { Ok(vec![]) }
        async fn get_node_id(&self) -> Result<String, AiomeError> { Ok("mock".into()) }
        async fn sign_swarm_payload(&self, _: &str) -> Result<String, AiomeError> { Ok("sig".into()) }
        async fn tick_local_clock(&self) -> Result<u64, AiomeError> { Ok(0) }
        async fn sync_local_clock(&self, _: u64) -> Result<u64, AiomeError> { Ok(0) }
        async fn storage_gc(&self, _: f64) -> Result<u64, AiomeError> { Ok(0) }
        async fn store_chat_message(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> { Ok(()) }
        async fn fetch_chat_history(&self, _: &str, _: i64) -> Result<Vec<serde_json::Value>, AiomeError> { Ok(vec![]) }
        async fn store_expression(&self, _: &aiome_core::expression::Expression) -> Result<(), AiomeError> { Ok(()) }
        async fn fetch_expressions(&self, _: i64) -> Result<Vec<aiome_core::expression::Expression>, AiomeError> { Ok(vec![]) }
        async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> { Ok(false) }
        async fn set_auto_expression_enabled(&self, _: bool) -> Result<(), AiomeError> { Ok(()) }
        async fn record_soul_mutation(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> { Ok(()) }
        async fn purge_old_jobs(&self, _: i64) -> Result<u64, AiomeError> { Ok(0) }
        async fn link_sns_data(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> { Ok(()) }
        async fn fetch_jobs_for_evaluation(&self, _: i64, _: i64) -> Result<Vec<Job>, AiomeError> { Ok(vec![]) }
        async fn record_sns_metrics(&self, _: &str, _: i64, _: i64, _: i64, _: i64, _: Option<&str>) -> Result<(), AiomeError> { Ok(()) }
        async fn fetch_pending_evaluations(&self, _: i64) -> Result<Vec<SnsMetricsRecord>, AiomeError> { Ok(vec![]) }
        async fn apply_final_verdict(&self, _: i64, _: OracleVerdict, _: &str) -> Result<(), AiomeError> { Ok(()) }
        async fn add_resonance(&self, _: i32) -> Result<(), AiomeError> { Ok(()) }
        async fn add_tech_exp(&self, _: i32) -> Result<(), AiomeError> { Ok(()) }
        async fn add_creativity(&self, _: i32) -> Result<(), AiomeError> { Ok(()) }
        async fn sync_samsara_level(&self) -> Result<Option<SamsaraEvent>, AiomeError> { Ok(None) }
        async fn get_biome_topic_status(&self, _: &str) -> Result<Option<(i32, Option<String>)>, AiomeError> { Ok(None) }
        async fn advance_biome_turn(&self, _: &str, _: i64) -> Result<i32, AiomeError> { Ok(0) }
        async fn fetch_biome_messages(&self, _: &str, _: i64) -> Result<Vec<serde_json::Value>, AiomeError> { Ok(vec![]) }
        async fn store_biome_message(&self, _: &BiomeMessage) -> Result<(), AiomeError> { Ok(()) }
        async fn update_biome_reputation(&self, _: &str, _: f64) -> Result<f64, AiomeError> { Ok(0.0) }
        async fn archive_biome_topic(&self, _: &str) -> Result<(), AiomeError> { Ok(()) }
        async fn get_job_count_since(&self, _: chrono::DateTime<chrono::Utc>) -> Result<i64, AiomeError> { Ok(0) }
        async fn fetch_top_performing_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> { Ok(vec![]) }
        async fn fetch_unincorporated_karma(&self, _: i64, _: &str) -> Result<Vec<serde_json::Value>, AiomeError> { Ok(vec![]) }
        async fn mark_karma_as_incorporated(&self, _: Vec<String>, _: &str) -> Result<(), AiomeError> { Ok(()) }
        async fn get_system_agent_id(&self) -> Result<uuid::Uuid, AiomeError> { Ok(Uuid::new_v4()) }
    }

    #[tokio::test]
    async fn test_dream_preemption() {
        struct BusyJQ;
        #[async_trait]
        impl JobQueue for BusyJQ {
            async fn get_pending_job_count(&self) -> Result<i64, AiomeError> { Ok(5) }
            // Stub out all other required methods (non-exhaustive in code, but required by trait)
            async fn fetch_recent_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> { Ok(vec![]) }
            async fn get_agent_stats(&self) -> Result<AgentStats, AiomeError> { Ok(AgentStats { level: 1, exp: 0, resonance: 0, creativity: 0, fatigue: 0 }) }
            async fn record_evolution_event(&self, _: i32, _: &str, _: &str, _: Option<&str>, _: Option<&str>) -> Result<(), AiomeError> { Ok(()) }
            async fn fetch_evolution_history(&self, _: i64) -> Result<Vec<serde_json::Value>, AiomeError> { Ok(vec![]) }
            async fn fetch_all_karma(&self, _: i64) -> Result<Vec<serde_json::Value>, AiomeError> { Ok(vec![]) }
            async fn export_federated_data(&self, _: Option<&str>) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>, Vec<ArenaMatch>), AiomeError> { Ok((vec![], vec![], vec![])) }
            async fn enqueue(&self, _: &str, _: &str, _: &str, _: Option<&str>, _: Option<aiome_core::security::PermissionManifest>) -> Result<String, AiomeError> { Ok("mock".into()) }
            async fn dequeue(&self, _: &[&str]) -> Result<Option<Job>, AiomeError> { Ok(None) }
            async fn fetch_job(&self, _: &str) -> Result<Option<Job>, AiomeError> { Ok(None) }
            async fn complete_job(&self, _: &str, _: Option<&str>) -> Result<(), AiomeError> { Ok(()) }
            async fn fail_job(&self, _: &str, _: &str) -> Result<(), AiomeError> { Ok(()) }
            async fn fetch_relevant_karma(&self, _: &str, _: &str, _: i64, _: &str) -> Result<KarmaSearchResult, AiomeError> { Ok(KarmaSearchResult { entries: vec![], is_ood: false, max_score: 0.0 }) }
            async fn store_karma(&self, _: &str, _: &str, _: &str, _: &str, _: &str, _: Option<&str>, _: Option<&str>, _: Option<&str>) -> Result<(), AiomeError> { Ok(()) }
            async fn adjust_karma_weight(&self, _: &str, _: i32) -> Result<(), AiomeError> { Ok(()) }
            async fn karma_decay_sweep(&self) -> Result<u64, AiomeError> { Ok(0) }
            async fn reclaim_zombie_jobs(&self, _: i64) -> Result<u64, AiomeError> { Ok(0) }
            async fn set_creative_rating(&self, _: &str, _: i32) -> Result<(), AiomeError> { Ok(()) }
            async fn heartbeat_pulse(&self, _: &str) -> Result<(), AiomeError> { Ok(()) }
            async fn store_execution_log(&self, _: &str, _: &str) -> Result<(), AiomeError> { Ok(()) }
            async fn fetch_undistilled_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> { Ok(vec![]) }
            async fn mark_karma_extracted(&self, _: &str) -> Result<(), AiomeError> { Ok(()) }
            async fn fetch_job_retry_count(&self, _: &str) -> Result<i64, AiomeError> { Ok(0) }
            async fn increment_job_retry_count(&self, _: &str) -> Result<bool, AiomeError> { Ok(true) }
            async fn reset_job_retry_count(&self, _: &str) -> Result<(), AiomeError> { Ok(()) }
            async fn store_immune_rule(&self, _: &ImmuneRule) -> Result<(), AiomeError> { Ok(()) }
            async fn delete_immune_rule(&self, _: &str) -> Result<(), AiomeError> { Ok(()) }
            async fn fetch_active_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> { Ok(vec![]) }
            async fn record_arena_match(&self, _: &ArenaMatch) -> Result<(), AiomeError> { Ok(()) }
            async fn import_federated_data(&self, _: Vec<FederatedKarma>, _: Vec<ImmuneRule>, _: Vec<ArenaMatch>) -> Result<(), AiomeError> { Ok(()) }
            async fn get_peer_sync_time(&self, _: &str) -> Result<Option<String>, AiomeError> { Ok(None) }
            async fn update_peer_sync_time(&self, _: &str, _: &str) -> Result<(), AiomeError> { Ok(()) }
            async fn get_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> { Ok(vec![]) }
            async fn get_node_id(&self) -> Result<String, AiomeError> { Ok("mock".into()) }
            async fn sign_swarm_payload(&self, _: &str) -> Result<String, AiomeError> { Ok("sig".into()) }
            async fn tick_local_clock(&self) -> Result<u64, AiomeError> { Ok(0) }
            async fn sync_local_clock(&self, _: u64) -> Result<u64, AiomeError> { Ok(0) }
            async fn storage_gc(&self, _: f64) -> Result<u64, AiomeError> { Ok(0) }
            async fn store_chat_message(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> { Ok(()) }
            async fn fetch_chat_history(&self, _: &str, _: i64) -> Result<Vec<serde_json::Value>, AiomeError> { Ok(vec![]) }
            async fn store_expression(&self, _: &aiome_core::expression::Expression) -> Result<(), AiomeError> { Ok(()) }
            async fn fetch_expressions(&self, _: i64) -> Result<Vec<aiome_core::expression::Expression>, AiomeError> { Ok(vec![]) }
            async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> { Ok(false) }
            async fn set_auto_expression_enabled(&self, _: bool) -> Result<(), AiomeError> { Ok(()) }
            async fn record_soul_mutation(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> { Ok(()) }
            async fn purge_old_jobs(&self, _: i64) -> Result<u64, AiomeError> { Ok(0) }
            async fn link_sns_data(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> { Ok(()) }
            async fn fetch_jobs_for_evaluation(&self, _: i64, _: i64) -> Result<Vec<Job>, AiomeError> { Ok(vec![]) }
            async fn record_sns_metrics(&self, _: &str, _: i64, _: i64, _: i64, _: i64, _: Option<&str>) -> Result<(), AiomeError> { Ok(()) }
            async fn fetch_pending_evaluations(&self, _: i64) -> Result<Vec<SnsMetricsRecord>, AiomeError> { Ok(vec![]) }
            async fn apply_final_verdict(&self, _: i64, _: OracleVerdict, _: &str) -> Result<(), AiomeError> { Ok(()) }
            async fn add_resonance(&self, _: i32) -> Result<(), AiomeError> { Ok(()) }
            async fn add_tech_exp(&self, _: i32) -> Result<(), AiomeError> { Ok(()) }
            async fn add_creativity(&self, _: i32) -> Result<(), AiomeError> { Ok(()) }
            async fn sync_samsara_level(&self) -> Result<Option<SamsaraEvent>, AiomeError> { Ok(None) }
            async fn get_biome_topic_status(&self, _: &str) -> Result<Option<(i32, Option<String>)>, AiomeError> { Ok(None) }
            async fn advance_biome_turn(&self, _: &str, _: i64) -> Result<i32, AiomeError> { Ok(0) }
            async fn fetch_biome_messages(&self, _: &str, _: i64) -> Result<Vec<serde_json::Value>, AiomeError> { Ok(vec![]) }
            async fn store_biome_message(&self, _: &BiomeMessage) -> Result<(), AiomeError> { Ok(()) }
            async fn update_biome_reputation(&self, _: &str, _: f64) -> Result<f64, AiomeError> { Ok(0.0) }
            async fn archive_biome_topic(&self, _: &str) -> Result<(), AiomeError> { Ok(()) }
            async fn get_job_count_since(&self, _: chrono::DateTime<chrono::Utc>) -> Result<i64, AiomeError> { Ok(0) }
            async fn fetch_top_performing_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> { Ok(vec![]) }
            async fn fetch_unincorporated_karma(&self, _: i64, _: &str) -> Result<Vec<serde_json::Value>, AiomeError> { Ok(vec![]) }
            async fn mark_karma_as_incorporated(&self, _: Vec<String>, _: &str) -> Result<(), AiomeError> { Ok(()) }
            async fn get_system_agent_id(&self) -> Result<uuid::Uuid, AiomeError> { Ok(Uuid::new_v4()) }
        }
        let dream = DreamState::new();
        let sonar = ExternalTrendSonar::new("key".into());
        let res = dream.dream(&BusyJQ, &sonar, 1).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_dream_execution() {
        let jq = MockJQ;
        let sonar = ExternalTrendSonar::new("key".into());
        let dream = DreamState::new();
        // Since it's random, we just ensure it doesn't crash
        let res = dream.dream(&jq, &sonar, 10).await;
        assert!(res.is_ok());
    }
}
