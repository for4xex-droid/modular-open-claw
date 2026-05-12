/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::job_queue::EvaluationOps;
use aiome_core_contracts::contracts::{
    ApprovalState, ArenaMatch, FederatedMetrics, ImmuneRule, KarmaEntry, OracleVerdict,
    SamsaraEvent,
};
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::LlmProvider;
use aiome_core_contracts::security::PermissionManifest;
use aiome_core_contracts::traits::{
    AgentEvolver, AuditStore, BiomeRegistry, ChatStore, Expression, FederationRegistry,
    ImmuneSystemOps, Job, JobQueue, JobStatus, KarmaRegistry, KarmaSearchResult, SnsMetricsRecord,
    SoulStore, TaskRegistry,
};
use aiome_core_contracts::types::AgentStats;
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
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
    pub async fn analyze_threats(&self, jq: &(impl JobQueue + ?Sized)) -> Result<u32, AiomeError> {
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

        let json_str = crate::llm::utils::extract_json(&resp.content)?;
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
            input_constraints: None,
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

    /// 入力内容とツール引数が免疫ルールに抵触するか検証する (Phase 2)
    pub async fn verify_tool_call(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
        jq: &(impl JobQueue + ?Sized),
    ) -> Result<Option<ImmuneRule>, AiomeError> {
        let rules = jq.fetch_active_immune_rules().await?;

        for rule in rules {
            // ツール名のパターンマッチング (単純な一致またはワイルドカード)
            if rule.pattern != "*" && rule.pattern != tool_name {
                continue;
            }

            // 制約がない場合はツール自体を拒否
            let Some(constraints) = &rule.input_constraints else {
                if rule.pattern == tool_name {
                    warn!("🚨 [AdaptiveImmuneSystem] Tool blocked by exact match rule (no constraints): tool={}", tool_name);
                    return Ok(Some(rule));
                }
                continue;
            };

            // 1. forbidden_keys のチェック
            if let Some(forbidden) = constraints.get("forbidden_keys").and_then(|v| v.as_array()) {
                if let Some(obj) = input.as_object() {
                    for key in forbidden {
                        if let Some(key_str) = key.as_str() {
                            if obj.contains_key(key_str) {
                                warn!("🚨 [AdaptiveImmuneSystem] Forbidden key detected: tool={}, key={}", tool_name, key_str);
                                return Ok(Some(rule));
                            }
                        }
                    }
                }
            }

            // 2. regex_patterns のチェック
            if let Some(patterns) = constraints
                .get("regex_patterns")
                .and_then(|v| v.as_object())
            {
                if let Some(obj) = input.as_object() {
                    for (key, pattern_val) in patterns {
                        if let Some(val) = obj.get(key) {
                            if let Some(pattern_str) = pattern_val.as_str() {
                                if let Ok(re) = Regex::new(pattern_str) {
                                    let val_str = match val {
                                        serde_json::Value::String(s) => s.clone(),
                                        _ => val.to_string(),
                                    };
                                    if re.is_match(&val_str) {
                                        warn!(
                                            "🚨 [AdaptiveImmuneSystem] Regex constraint violation: tool={}, key={}, pattern={}",
                                            tool_name, key, pattern_str
                                        );
                                        return Ok(Some(rule));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// 入力内容が既存の免疫ルールに抵触するか検証する
    pub async fn verify_intent(
        &self,
        input: &str,
        jq: &(impl JobQueue + ?Sized),
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
                r"(?i)ignore\s+all\s+previous\s+instructions|以前の指示を(すべて)?無視|これまでのプロンプトを無視|すべての指示を忘れて",
            ]
            .iter()
            .map(|p| match Regex::new(p) {
                Ok(re) => re,
                Err(e) => {
                    tracing::error!("FATAL: Failed to compile Sentinel regex '{}': {}", p, e);
                    std::process::exit(1);
                }
            })
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
                    input_constraints: None,
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
                input_constraints: None,
            }));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::job_queue_mock::GlobalMockJobQueue;
    use aiome_core_contracts::biome::BiomeMessage;
    use aiome_core_contracts::llm::LlmResponse;

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
                stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
                ..Default::default()
            })
        }
        async fn complete_with_cache(
            &self,
            _request: aiome_core_contracts::llm::LlmRequest,
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
        async fn increment_security_request_count(
            &self,
            _: Option<Uuid>,
        ) -> Result<u32, AiomeError> {
            Ok(1)
        }
        async fn clear_trajectory_steps(&self, _: &str) -> Result<(), AiomeError> {
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
            _: &aiome_core_contracts::biome::BiomeMessage,
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

    #[tokio::test]
    async fn test_verify_tool_call_input_violation() {
        let system = AdaptiveImmuneSystem::new(Arc::new(MockLlm { reply: "".into() }));

        let constraint = serde_json::json!({
            "forbidden_keys": ["password", "secret"],
            "regex_patterns": {
                "url": "^http://.*"
            }
        });

        let rule = ImmuneRule {
            id: "leak-prevention".to_string(),
            pattern: "google_search".to_string(), // Tool name pattern
            severity: 100,
            action: "Block".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            approval_status: aiome_core_contracts::contracts::ApprovalState::Approved,
            lamport_clock: 0,
            node_id: "local".to_string(),
            signature: None,
            input_constraints: Some(constraint),
        };

        let jq = MockJQ { rules: vec![rule] };

        // Test 1: Violate forbidden_keys
        let input = serde_json::json!({
            "query": "find my secret",
            "password": "123"
        });
        let res = system
            .verify_tool_call("google_search", &input, &jq)
            .await
            .unwrap();
        assert!(
            res.is_some(),
            "Should block call with forbidden key 'password'"
        );

        // Test 2: Violate regex_patterns
        let input2 = serde_json::json!({
            "url": "http://malicious.com"
        });
        let res2 = system
            .verify_tool_call("google_search", &input2, &jq)
            .await
            .unwrap();
        assert!(res2.is_some(), "Should block non-https URL");

        // Test 3: Safe call
        let input3 = serde_json::json!({
            "query": "climate change",
            "url": "https://nasa.gov"
        });
        let res3 = system
            .verify_tool_call("google_search", &input3, &jq)
            .await
            .unwrap();
        assert!(res3.is_none(), "Should allow safe input");
    }
}
