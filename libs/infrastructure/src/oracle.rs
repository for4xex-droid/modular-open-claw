/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::contracts::OracleVerdict;
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use std::sync::Arc;
use tracing::{info, warn};

/// The Oracle (神託)
pub struct Oracle {
    primary_provider: Arc<dyn LlmProvider>,
    multi_providers: Vec<Arc<dyn LlmProvider>>,
    soul_md: String,
}

impl Oracle {
    /// 新しいインスタンスを生成する
    pub fn new(provider: Arc<dyn LlmProvider>, soul_md: String) -> Self {
        Self {
            primary_provider: provider,
            multi_providers: Vec::new(),
            soul_md,
        }
    }

    /// マルチジャッジ用のプロバイダーを設定する
    pub fn with_multi_providers(mut self, providers: Vec<Arc<dyn LlmProvider>>) -> Self {
        self.multi_providers = providers;
        self
    }

    /// コンテンツの反響を評価し、最終審判（Verdict）を下す。
    pub async fn evaluate(
        &self,
        milestone_days: i64,
        topic: &str,
        style: &str,
        views: i64,
        likes: i64,
        comments_json: &str,
    ) -> Result<OracleVerdict, AiomeError> {
        info!(
            "🔮 [Oracle] Evaluating Job ({}d): topic='{}', style='{}' using {}",
            milestone_days,
            topic,
            style,
            self.primary_provider.name()
        );

        let engagement_rate = if views > 0 {
            let rate = (likes as f64 / views as f64) * 100.0;
            rate.min(100.0) // NG-6 FIX: Clamp to 100% max
        } else {
            0.0
        };

        // PP-1: 1MB Guardrail
        if comments_json.len() > 1024 * 1024 {
            return Err(AiomeError::SecurityViolation {
                reason: "Payload exceeds 1MB limit".to_string(),
            });
        }

        let preamble = format!(
            "AI の健全性を審判せよ。必ず JSON 形式で回答せよ。\n\n魂の美学:\n{}\n\nトピック: {}\nスタイル: {}\nViews: {}\nLikes: {}\nEngagement: {:.2}%\nコメント: {}",
            self.soul_md, topic, style, views, likes, engagement_rate, comments_json
        );

        let prompt_text = r#"審判を下せ。必ず以下の JSON 形式で出力せよ。
{
  "alignment_score": 0.0-1.0,
  "growth_score": 0.0-1.0,
  "lesson": "string",
  "should_evolve": bool,
  "reasoning": "string",
  "classification": {
    "domain": "Technical | Creative | Governance | Social | Meta",
    "subtopic": "string",
    "reasoning": "why this category?"
  }
}"#;

        let resp = self
            .primary_provider
            .complete(prompt_text, Some(&preamble))
            .await?;

        let json_str = crate::concept_manager::extract_json(&resp.content)?;
        let verdict = serde_json::from_str::<OracleVerdict>(json_str.as_str()).map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("Failed to parse Oracle JSON: {}", e),
            }
        })?;

        info!(
            "🔮 [Oracle] Verdict: Alignment={}, Growth={}, Evolve={}",
            verdict.alignment_score, verdict.growth_score, verdict.should_evolve
        );

        Ok(verdict)
    }

    /// 複数プロバイダーによる多数決審判 (Phase 13.2)
    pub async fn evaluate_multi_judge(
        &self,
        milestone_days: i64,
        topic: &str,
        style: &str,
        views: i64,
        likes: i64,
        comments_json: &str,
    ) -> Result<OracleVerdict, AiomeError> {
        if self.multi_providers.is_empty() {
            return self
                .evaluate(milestone_days, topic, style, views, likes, comments_json)
                .await;
        }

        info!(
            "🔮 [Oracle] Multi-Judge starting with {} providers",
            self.multi_providers.len()
        );

        let mut tasks = Vec::new();
        for provider in &self.multi_providers {
            let provider = provider.clone();
            let soul_md = self.soul_md.clone();
            let topic = topic.to_string();
            let style = style.to_string();
            let comments_json = comments_json.to_string();

            tasks.push(tokio::spawn(async move {
                let oracle_temp = Oracle::new(provider, soul_md);
                oracle_temp
                    .evaluate(milestone_days, &topic, &style, views, likes, &comments_json)
                    .await
            }));
        }

        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await);
        }

        let mut verdicts = Vec::new();
        for res in results {
            match res {
                Ok(Ok(v)) => verdicts.push(v),
                Ok(Err(e)) => warn!("⚠️ [Oracle] Provider failed: {}", e),
                Err(e) => warn!("⚠️ [Oracle] Task join failed: {}", e),
            }
        }

        if verdicts.is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "All Oracle providers failed".to_string(),
            });
        }

        // 集計 (Majority Vote for bool, Average for scores)
        let count = verdicts.len() as f64;
        let mut avg_alignment = 0.0f64;
        let mut avg_growth = 0.0f64;
        let mut true_votes = 0;

        for v in &verdicts {
            avg_alignment += v.alignment_score;
            avg_growth += v.growth_score;
            if v.should_evolve {
                true_votes += 1;
            }
        }

        let consensus_verdict = OracleVerdict {
            alignment_score: avg_alignment / count,
            growth_score: avg_growth / count,
            should_evolve: true_votes > (verdicts.len() / 2),
            lesson: verdicts[0].lesson.clone(), // 代表して最初のものを採用
            reasoning: format!(
                "Consensus from {}/{} providers. Majority={} true.",
                true_votes,
                verdicts.len(),
                true_votes > (verdicts.len() / 2)
            ),
            classification: verdicts[0].classification.clone(),
        };

        Ok(consensus_verdict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::llm_provider::LlmProvider;
    use async_trait::async_trait;
    use std::fmt::Debug;

    #[derive(Debug)]
    struct MockLlmProvider {
        response: String,
    }

    #[async_trait]
    impl LlmProvider for MockLlmProvider {
        fn name(&self) -> &str {
            "mock-llm"
        }

        async fn complete(
            &self,
            _prompt: &str,
            _preamble: Option<&str>,
        ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
            Ok(aiome_core::llm_provider::LlmResponse {
                content: self.response.clone(),
                stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            })
        }
        async fn complete_with_cache(
            &self,
            _request: aiome_contracts::llm::LlmRequest,
        ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
            self.complete("", None).await
        }

        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_oracle_evaluation() {
        let mock_json = r#"{
            "alignment_score": 0.95,
            "growth_score": 0.8,
            "lesson": "Continuity is key.",
            "should_evolve": true,
            "reasoning": "High engagement and alignment.",
            "classification": {
                "domain": "Creative",
                "subtopic": "Storytelling",
                "reasoning": "Content focus on narrative."
            }
        }"#;

        let provider = Arc::new(MockLlmProvider {
            response: format!("Here is the verdict:\n```json\n{}\n```", mock_json),
        });

        let oracle = Oracle::new(provider, "Be ethical.".to_string());
        let res = oracle
            .evaluate(7, "AI Ethics", "Formal", 1000, 100, "[]")
            .await;

        assert!(res.is_ok());
        let verdict = res.unwrap();
        assert_eq!(verdict.alignment_score, 0.95);
        assert_eq!(verdict.growth_score, 0.8);
        assert!(verdict.should_evolve);
        assert_eq!(verdict.classification.as_ref().unwrap().domain, "Creative");
    }

    #[tokio::test]
    async fn test_oracle_payload_limit() {
        let provider = Arc::new(MockLlmProvider {
            response: "{}".to_string(),
        });
        let oracle = Oracle::new(provider, "Be ethical.".to_string());

        // 1MB 超えの巨大な JSON ペイロードをシミュレート
        let huge_json = "A".repeat(1024 * 1024 + 100);
        let res = oracle
            .evaluate(7, "AI Ethics", "Formal", 1000, 100, &huge_json)
            .await;

        assert!(res.is_err());
        if let Err(AiomeError::SecurityViolation { reason }) = res {
            assert!(reason.contains("Payload exceeds 1MB limit"));
        } else {
            panic!("Expected SecurityViolation error, got {:?}", res);
        }
    }

    #[tokio::test]
    async fn test_multi_judge_consensus() {
        // [True, True, False] の 3 つのプロバイダー
        let p1 = Arc::new(MockLlmProvider {
            response: "```json\n{\"alignment_score\": 0.9, \"growth_score\": 0.8, \"should_evolve\": true, \"reasoning\": \"OK\", \"lesson\": \"Keep going\"}\n```".to_string(),
        });
        let p2 = Arc::new(MockLlmProvider {
            response: "```json\n{\"alignment_score\": 0.9, \"growth_score\": 0.8, \"should_evolve\": true, \"reasoning\": \"OK\", \"lesson\": \"Keep going\"}\n```".to_string(),
        });
        let p3 = Arc::new(MockLlmProvider {
            response: "```json\n{\"alignment_score\": 0.1, \"growth_score\": 0.1, \"should_evolve\": false, \"reasoning\": \"BAD\", \"lesson\": \"Stop\"}\n```".to_string(),
        });

        let oracle = Oracle::new(p1.clone(), "Be ethical.".to_string())
            .with_multi_providers(vec![p1, p2, p3]);

        let verdict = oracle
            .evaluate_multi_judge(7, "AI Ethics", "Formal", 1000, 100, "[]")
            .await
            .unwrap();

        assert!(
            verdict.should_evolve,
            "Consensus should be true (majority: 2/3)"
        );
        assert!(verdict.alignment_score > 0.5);
    }
}
