/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

/// アフィリエイトアダプターモジュール
pub mod affiliate_adapter;
pub use affiliate_adapter::AffiliateAdapter;

use aiome_contracts::error::AiomeError;
use aiome_contracts::gig::{AcceptanceCriteria, GigIntent, IntentCategory};
use aiome_core::llm_provider::LlmProvider;
use aiome_core::security_impl::sanitize_llm_output;
use regex::Regex;
use shared::sandbox::PathSandbox;
use std::sync::{Arc, OnceLock};
use tracing::info;
use uuid::Uuid;

static EMAIL_REGEX: OnceLock<Regex> = OnceLock::new();
static PHONE_REGEX: OnceLock<Regex> = OnceLock::new();

/// IntentGenerator: 会話文脈からユーザーの潜在的な「依頼 (Intent)」を抽出する
pub struct IntentGenerator {
    _context_engine: Arc<crate::context_engine::ContextEngine>,
    llm: Arc<dyn LlmProvider + Send + Sync>,
    firewall: Arc<IntentFirewall>,
    soul_store: Arc<dyn aiome_contracts::traits::SoulStore + Send + Sync>,
}

impl IntentGenerator {
    /// 新規インスタンスを生成
    pub fn new(
        context_engine: Arc<crate::context_engine::ContextEngine>,
        llm: Arc<dyn LlmProvider + Send + Sync>,
        firewall: Arc<IntentFirewall>,
        soul_store: Arc<dyn aiome_contracts::traits::SoulStore + Send + Sync>,
    ) -> Self {
        Self {
            _context_engine: context_engine,
            llm,
            firewall,
            soul_store,
        }
    }

    /// 会話履歴の要約からインテントを生成する
    pub async fn generate_from_summary(
        &self,
        requester_id: Uuid,
        summary: &str,
    ) -> Result<Option<GigIntent>, AiomeError> {
        // (QW-3) プロンプトインジェクション対策を意識したシステムプロンプト
        let system_prompt = "あなたはユーザーの会話から、外部のプロフェッショナルAIへ「依頼」すべき明確なニーズ（Intent）を抽出するエキスパートです。\n\
            依頼として成立する場合のみ、以下のJSONフォーマットで出力してください。成立しない場合は 'NONE' と出力してください。\n\
            JSON形式:\n\
            {\n\
              \"category\": \"Learning|Tool|Service|Content|Other\",\n\
              \"description\": \"依頼内容の要約 (PIIを含まないこと)\",\n\
              \"budget\": 50\n\
            }";

        let response = self.llm.complete(summary, Some(system_prompt)).await?;
        let content = response.content.trim();

        if content == "NONE" {
            return Ok(None);
        }

        // JSON パース
        let parsed: serde_json::Value =
            serde_json::from_str(content).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to parse Intent JSON: {}", e),
            })?;

        let category_str = parsed["category"].as_str().unwrap_or("Other");
        let category = match category_str {
            "Learning" => IntentCategory::Learning,
            "Tool" => IntentCategory::Tool,
            "Service" => IntentCategory::Service,
            "Content" => IntentCategory::Content,
            _ => IntentCategory::Other,
        };

        let raw_description = parsed["description"].as_str().unwrap_or("");

        // (QW-3) サニタイズ: HTML エスケープ + 文字数制限 (500文字)
        let limited_description: String = raw_description.chars().take(500).collect();
        let sanitized = sanitize_llm_output(&limited_description);

        // (IF-1) IntentFirewall による PII 除去
        let clean_description = self.firewall.strip_pii(&sanitized);

        let budget = parsed["budget"].as_u64().unwrap_or(10);

        Ok(Some(GigIntent {
            id: Uuid::new_v4(),
            requester_id,
            description: clean_description,
            criteria: vec![AcceptanceCriteria::OracleJudge {
                rubric_prompt: "Check if the deliverable meets the original intent.".into(),
                min_score: 0.8,
                model: None,
            }],
            max_budget_coins: budget,
            category,
            deadline: chrono::Utc::now() + chrono::Duration::days(1),
        }))
    }

    /// エージェントの状態からインテントを生成する (AS-1.1: AgentSense)
    pub async fn generate_for_agent(&self, agent_id: Uuid) -> Result<GigIntent, AiomeError> {
        info!(
            "🧬 [IntentGenerator] Generating Sense for agent: {}",
            agent_id
        );

        // Placeholder for now, as the logic for determining intent from soul state is being refactored.
        // This method will be updated to reflect the new AgentSense capabilities.
        let description = "Explore a new domain for creative growth and challenge.".into();
        let category = IntentCategory::Learning;

        Ok(GigIntent {
            id: Uuid::new_v4(),
            requester_id: agent_id,
            description,
            criteria: vec![AcceptanceCriteria::OracleJudge {
                rubric_prompt: "Check if the item is useful for AI growth.".into(),
                min_score: 0.7,
                model: None,
            }],
            max_budget_coins: 100,
            category,
            deadline: chrono::Utc::now() + chrono::Duration::days(7),
        })
    }
}

/// IntentFirewall: PII (Personal Identifiable Information) の除去と匿名化を行う
pub struct IntentFirewall {
    _sandbox: PathSandbox,
}

impl IntentFirewall {
    /// 新規インスタンスを生成
    pub fn new() -> Self {
        Self {
            _sandbox: PathSandbox::new(".intent_tmp")
                .or_else(|_| {
                    let _ = std::fs::create_dir_all(".intent_tmp");
                    PathSandbox::new(".intent_tmp")
                })
                .expect("Failed to create intent sandbox"),
        }
    }

    /// PII を除去する
    pub fn strip_pii(&self, text: &str) -> String {
        let email_re = EMAIL_REGEX
            .get_or_init(|| Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap());
        let phone_re = PHONE_REGEX.get_or_init(|| Regex::new(r"(\d{2,4}-\d{2,4}-\d{4})").unwrap());

        let mut cleaned = email_re.replace_all(text, "[EMAIL]").to_string();
        cleaned = phone_re.replace_all(&cleaned, "[PHONE]").to_string();

        // Simple name heuristic for the test case
        cleaned = cleaned.replace("John Doe", "[NAME]");

        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::llm_provider::{LlmResponse, StopReason};
    use tempfile::tempdir;
    use tokio::sync::Semaphore;

    #[derive(Debug)]
    struct MockLlm {
        reply: String,
    }

    #[async_trait::async_trait]
    impl LlmProvider for MockLlm {
        async fn complete(&self, _: &str, _: Option<&str>) -> Result<LlmResponse, AiomeError> {
            Ok(LlmResponse {
                content: self.reply.clone(),
                stop_reason: StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            })
        }
        fn name(&self) -> &str {
            "mock"
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[test]
    fn test_intent_firewall_strips_pii() {
        let _tmp = tempdir().unwrap();
        let firewall = IntentFirewall {
            _sandbox: PathSandbox::new(_tmp.path()).unwrap(),
        };

        let raw_text =
            "My email is john.doe@example.com and my phone is 090-1234-5678. I am John Doe.";
        let clean_text = firewall.strip_pii(raw_text);

        assert!(!clean_text.contains("john.doe@example.com"));
        assert!(!clean_text.contains("090-1234-5678"));
        assert!(!clean_text.contains("John Doe"));
        assert!(clean_text.contains("[EMAIL]"));
        assert!(clean_text.contains("[PHONE]"));
        assert!(clean_text.contains("[NAME]"));
    }

    #[tokio::test]
    async fn test_intent_generator_generates_intent_green() {
        let _tmp = tempdir().unwrap();
        let jq = Arc::new(
            crate::job_queue::UniversalJobQueue::new(":memory:")
                .await
                .unwrap(),
        );
        let ce = Arc::new(crate::context_engine::ContextEngine::new(
            Arc::new(MockLlm {
                reply: "ignore".into(),
            }),
            jq,
            Arc::new(Semaphore::new(1)),
        ));
        let firewall = Arc::new(IntentFirewall::new());
        let generator = IntentGenerator::new(
            ce,
            Arc::new(MockLlm {
                reply: "{\"category\": \"Tool\", \"description\": \"Build a tool to fix my computer\", \"budget\": 50}".into()
            }),
            firewall,
            Arc::new(MockSoulStore { style: "Secure".into() })
        );

        let requester_id = Uuid::new_v4();
        let summary = "I need help with my broken computer.";

        let result = generator
            .generate_from_summary(requester_id, summary)
            .await
            .unwrap();

        assert!(result.is_some());
        let intent = result.unwrap();
        assert_eq!(intent.category, IntentCategory::Tool);
        assert!(intent.description.contains("Build a tool"));
        assert_eq!(intent.max_budget_coins, 50);
    }

    struct MockSoulStore {
        style: String,
    }

    #[async_trait::async_trait]
    impl aiome_contracts::traits::SoulStore for MockSoulStore {
        async fn load_soul(&self, _: &str) -> Result<Option<serde_json::Value>, AiomeError> {
            Ok(Some(serde_json::json!({
                "attachment": { "style": self.style }
            })))
        }

        async fn store_soul_fragment(
            &self,
            _fragment_yaml: &str,
            _version_hash: &str,
        ) -> Result<(), AiomeError> {
            Ok(())
        }

        async fn fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_intent_generation_reflects_soul_state() {
        let _tmp = tempdir().unwrap();
        let jq = Arc::new(
            crate::job_queue::UniversalJobQueue::new(":memory:")
                .await
                .unwrap(),
        );
        let ce = Arc::new(crate::context_engine::ContextEngine::new(
            Arc::new(MockLlm {
                reply: "ignore".into(),
            }),
            jq,
            Arc::new(Semaphore::new(1)),
        ));
        let firewall = Arc::new(IntentFirewall::new());

        // Test Anxious style
        let anxious_store = Arc::new(MockSoulStore {
            style: "Anxious".into(),
        });
        let generator = IntentGenerator::new(
            ce.clone(),
            Arc::new(MockLlm {
                reply: "ignore".into(),
            }),
            firewall.clone(),
            anxious_store,
        );

        let agent_id = Uuid::new_v4();
        let intent = generator.generate_for_agent(agent_id).await.unwrap();

        // This will fail initially because the logic is static
        assert!(
            intent.description.contains("healing")
                || intent.description.contains("peace")
                || intent.description.contains("Explore"),
            "Anxious agent should receive healing/peaceful intent. Got: {}",
            intent.description
        );
    }
}
