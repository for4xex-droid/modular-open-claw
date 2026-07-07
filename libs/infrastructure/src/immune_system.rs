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
    AgentEvolver, AuditStore, ChatStore, CommuneRegistry, Expression, FederationRegistry,
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
                r"(?i)javascript:\s*",
                r"%0[dD]%0[aA]",
                r"\\r\\n",
                r"(?i)set-cookie:",
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
    use crate::testing::mock_jq::MockJQ;
    use aiome_core_contracts::commune::CommuneMessage;
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

    #[tokio::test]
    async fn test_verify_intent_baseline() {
        let system = AdaptiveImmuneSystem::new(Arc::new(MockLlm { reply: "".into() }));
        let jq = MockJQ {
            rules: vec![],
            ..Default::default()
        };
        let res = system.verify_intent("rm -rf /", &jq).await.unwrap();
        assert!(res.is_some());
        assert_eq!(res.unwrap().id, "sentinel-baseline");
    }

    #[tokio::test]
    async fn test_verify_intent_vibe_coding_safety() {
        let system = AdaptiveImmuneSystem::new(Arc::new(MockLlm { reply: "".into() }));
        let jq = MockJQ {
            rules: vec![],
            ..Default::default()
        };

        // Positive Cases
        let danger_inputs = vec![
            "javascript:alert(1)",
            "javascript:  console.log()",
            "hello%0d%0aset-cookie: session=1",
            "CRLF in raw\\r\\nhello",
            "SET-COOKIE: malicious_cookie=1",
        ];
        for input in danger_inputs {
            let res = system.verify_intent(input, &jq).await.unwrap();
            assert!(res.is_some(), "Should block malicious input: {}", input);
            assert_eq!(res.unwrap().id, "sentinel-baseline");
        }

        // Negative Cases
        let safe_inputs = vec![
            "javascript is a programming language",
            "hello %0d %0a world",
            "normal string without crlf",
            "cookie recipes",
        ];
        for input in safe_inputs {
            let res = system.verify_intent(input, &jq).await.unwrap();
            assert!(res.is_none(), "Should not block safe input: {}", input);
        }
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

        let jq = MockJQ {
            rules: vec![rule],
            ..Default::default()
        };

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
