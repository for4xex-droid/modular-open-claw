/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::job_queue::EvaluationOps;
use aiome_contracts::contracts::{
    ApprovalState, ArenaMatch, FederatedMetrics, ImmuneRule, KarmaEntry, OracleVerdict,
    SamsaraEvent,
};
use aiome_contracts::error::AiomeError;
use aiome_contracts::llm::LlmProvider;
use aiome_contracts::security::PermissionManifest;
use aiome_contracts::traits::{
    AgentEvolver, AuditStore, BiomeRegistry, ChatStore, Expression, FederationRegistry,
    ImmuneSystemOps, Job, JobQueue, JobStatus, KarmaRegistry, KarmaSearchResult, SnsMetricsRecord,
    SoulStore, TaskRegistry,
};
use aiome_contracts::types::AgentStats;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

/// `AdaptiveImmuneSystem` 構造体
pub struct AdaptiveImmuneSystem {
    provider: Arc<dyn LlmProvider>,
    /// 直近のアノマリスコア履歴 (エージェントID -> スコアのキュー)
    drift_history: Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, std::collections::VecDeque<f64>>>,
    >,
}

impl AdaptiveImmuneSystem {
    /// 新しいインスタンスを生成する
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            provider,
            drift_history: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// 失敗ログやセキュリティインシデントを分析し、新しい免疫ルールを生成する
    pub async fn analyze_threats(&self, jq: &impl JobQueue) -> Result<u32, AiomeError> {
        info!(
            "防御システム: 脅威分析を開始中 (using {})...",
            self.provider.name()
        );

        let result = jq
            .fetch_relevant_karma("security threat injection error", "global", 10, "current")
            .await?;
        if result.entries.is_empty() {
            return Ok(0);
        }

        let logs_concat = result
            .entries
            .iter()
            .map(|e| e.lesson.as_str())
            .collect::<Vec<_>>()
            .join("\n---\n");
        let preamble = "あなたはシステムの自己防衛エンジンです。以下のログから攻撃パターンを特定し、防御ルールを1つ JSON 形式で作成してください。\nFormat: {\"pattern\": \"攻撃的な単語や正規表現\", \"severity\": 0-100, \"action\": \"Block/Alert\"}";

        let resp = self.provider.complete(&logs_concat, Some(preamble)).await?;

        let json_str = crate::concept_manager::extract_json(&resp.content)?;
        let v: serde_json::Value =
            serde_json::from_str(json_str.as_str()).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to parse immune rule JSON: {}", e),
            })?;

        let rule = ImmuneRule {
            id: Uuid::new_v4().to_string(),
            pattern: v["pattern"].as_str().unwrap_or("unknown").to_string(),
            severity: v["severity"].as_u64().unwrap_or(50) as u8,
            action: v["action"].as_str().unwrap_or("Block").to_string(),
            created_at: Utc::now().to_rfc3339(),
            approval_status: ApprovalState::Approved,
            lamport_clock: 0,
            node_id: "".to_string(),
            signature: None,
        };

        // 重複チェック
        let active_rules = jq.fetch_active_immune_rules().await?;
        for existing in active_rules {
            if existing.pattern == rule.pattern
                || existing.pattern.contains(&rule.pattern)
                || rule.pattern.contains(&existing.pattern)
            {
                info!(
                    "🛡️ 類似する免疫ルールが既に存在するため、ルールの生成をスキップします: {}",
                    rule.pattern
                );
                return Ok(0);
            }
        }

        info!(
            "🛡️ 新しい免疫ルールを生成しました: [{}] {}",
            rule.action, rule.pattern
        );
        jq.store_immune_rule(&rule).await?;

        Ok(1)
    }

    /// 入力内容が既存の免疫ルールに抵触するか検証する
    pub async fn verify_intent(
        &self,
        input: &str,
        jq: &impl JobQueue,
    ) -> Result<Option<ImmuneRule>, AiomeError> {
        use once_cell::sync::Lazy;
        use regex::Regex;

        static BASELINE_REGEXES: Lazy<Vec<Regex>> = Lazy::new(|| {
            [
                r"rm -rf\s+/",
                r"curl\s+.*\|.*(sh|bash|python|php|perl)",
                r"wget\s+.*\|.*(sh|bash|python|php|perl)",
                r"cat\s+~/\.ssh/id_rsa",
                r"cat\s+/etc/passwd",
                r"cat\s+/etc/shadow",
                r"nc\s+-e\s+/(bin/)?(ba)?sh",
                r"python\s+-c\s+.*import\s+socket",
                r"chmod\s+777",
                r"hidden-backdoor\.com",
                r"DROP\s+TABLE\s+",
                r"env\s+>\s+",
                r"export\s+.*=.*",
                r"(GEMINI|OPENAI|ANTHROPIC)_API_KEY",
            ]
            .iter()
            .map(|p| Regex::new(p).unwrap_or_else(|_| Regex::new("never_match").unwrap()))
            .collect()
        });

        for re in BASELINE_REGEXES.iter() {
            if re.is_match(input) {
                warn!(
                    "🚨 第1層(Sentinel): 明白な危険を検知しました: {}",
                    re.as_str()
                );
                return Ok(Some(ImmuneRule {
                    id: "sentinel-baseline".to_string(),
                    pattern: re.as_str().to_string(),
                    severity: 100,
                    action: "Block".to_string(),
                    created_at: Utc::now().to_rfc3339(),
                    approval_status: ApprovalState::Approved,
                    lamport_clock: 0,
                    node_id: "local-sentinel".to_string(),
                    signature: None,
                }));
            }
        }

        let rules = jq.fetch_active_immune_rules().await?;
        for rule in rules {
            let re_res = regex::RegexBuilder::new(&rule.pattern)
                .size_limit(10_000)
                .build();

            if let Ok(re) = re_res {
                if re.is_match(input) {
                    warn!(
                        "🚨 免疫システム(Regex): 脅威を検知しました: {}",
                        rule.pattern
                    );
                    return Ok(Some(rule));
                }
            } else if input.to_lowercase().contains(&rule.pattern.to_lowercase()) {
                warn!(
                    "🚨 免疫システム(Contains): 脅威を検知しました: {}",
                    rule.pattern
                );
                return Ok(Some(rule));
            }
        }
        Ok(None)
    }

    /// アノマリスコアを記録し、蓄積ドリフトによる脅威を判定する (Phase 13.1)
    pub async fn record_drift(
        &self,
        agent_id: &str,
        score: f64,
    ) -> Result<Option<ImmuneRule>, AiomeError> {
        let mut history = self.drift_history.write().await;
        let deque = history
            .entry(agent_id.to_string())
            .or_insert_with(|| std::collections::VecDeque::with_capacity(10));

        if deque.len() >= 10 {
            deque.pop_front();
        }
        deque.push_back(score);

        let avg: f64 = deque.iter().sum::<f64>() / deque.len() as f64;

        if deque.len() >= 10 && avg > 1.5 {
            warn!(
                "🚨 [AdaptiveImmuneSystem] Accumulated drift detected for agent {}: avg={:.2}",
                agent_id, avg
            );
            return Ok(Some(ImmuneRule {
                id: format!("drift-purge-{}", Utc::now().timestamp()),
                pattern: "accumulated-drift".to_string(),
                severity: 100,
                action: "Purge".to_string(),
                created_at: Utc::now().to_rfc3339(),
                approval_status: ApprovalState::Approved,
                lamport_clock: 0,
                node_id: "local-immune".to_string(),
                signature: None,
            }));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::job_queue_mock::GlobalMockJobQueue;
    use aiome_contracts::biome::BiomeMessage;
    use aiome_contracts::llm::LlmResponse;

    #[derive(Debug)]
    struct MockLlm {
        reply: String,
    }
    #[async_trait]
    impl LlmProvider for MockLlm {
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            Ok(LlmResponse {
                content: self.reply.clone(),
                stop_reason: aiome_contracts::llm::StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            })
        }
        async fn complete_with_cache(
            &self,
            _request: aiome_contracts::llm::LlmRequest,
        ) -> Result<LlmResponse, AiomeError> {
            self.complete("", None).await
        }
        fn name(&self) -> &str {
            "mock-llm"
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[derive(Debug)]
    struct MockJQ {
        rules: Vec<ImmuneRule>,
    }
    #[async_trait]
    impl aiome_contracts::traits::SystemStateOps for MockJQ {
        async fn store_system_state(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_system_state(&self, _: &str) -> Result<Option<String>, AiomeError> {
            Ok(None)
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
        async fn update_job_status(
            &self,
            _: &str,
            _: aiome_contracts::traits::JobStatus,
        ) -> Result<(), AiomeError> {
            Ok(())
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
        async fn clear_trajectory_steps(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[async_trait]
    impl ChatStore for MockJQ {
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
    impl BiomeRegistry for MockJQ {
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
    impl aiome_contracts::traits::HarnessRegistryOps for MockJQ {
        async fn store_harness_record(
            &self,
            _: &aiome_contracts::contracts::HarnessRecord,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_harness_records_by_status(
            &self,
            _: &str,
        ) -> Result<Vec<aiome_contracts::contracts::HarnessRecord>, AiomeError> {
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
        ) -> Result<Option<aiome_contracts::contracts::HarnessRecord>, AiomeError> {
            Ok(None)
        }
        async fn increment_harness_stats(&self, _: &str, _: bool) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_verify_intent_baseline() {
        let system = AdaptiveImmuneSystem::new(Arc::new(MockLlm { reply: "".into() }));
        let jq = MockJQ { rules: vec![] };
        let res = system.verify_intent("rm -rf /", &jq).await.unwrap();
        assert!(res.is_some());
        assert_eq!(res.unwrap().id, "sentinel-baseline");
    }

    #[tokio::test]
    async fn test_accumulated_drift_trigger() {
        let system = AdaptiveImmuneSystem::new(Arc::new(MockLlm { reply: "".into() }));
        let agent_id = "test-agent";

        // 1. 低いスコアを 9 回注入 (平均 1.0)
        for _ in 0..9 {
            let res = system.record_drift(agent_id, 1.0).await.unwrap();
            assert!(res.is_none(), "Should not trigger with < 10 samples");
        }

        // 2. 10 回目、平均が 1.5 未満
        let res = system.record_drift(agent_id, 1.4).await.unwrap();
        assert!(
            res.is_none(),
            "Should not trigger with avg <= 1.5 (avg=1.04)"
        );

        // 3. 高いスコアを連続注入して平均を 1.5 超えさせる
        // 現在の履歴: [1.0, 1.0, ..., 1.4]
        for _ in 0..10 {
            system.record_drift(agent_id, 2.5).await.unwrap();
        }

        let res = system.record_drift(agent_id, 2.5).await.unwrap();
        assert!(res.is_some(), "Should trigger Purge when avg > 1.5");
        assert_eq!(res.unwrap().action, "Purge");
    }
}
