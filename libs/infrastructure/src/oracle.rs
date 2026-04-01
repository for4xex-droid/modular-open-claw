/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::society_of_thought::SoTEngine;
use aiome_core::contracts::OracleVerdict;
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use aiome_core_contracts::events::CoreEvent;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

/// The Oracle (神託)
pub struct Oracle {
    primary_provider: Arc<dyn LlmProvider>,
    fast_provider: Arc<dyn LlmProvider>, // P-2 用
    multi_providers: Vec<Arc<dyn LlmProvider>>,
    soul_md: String,
    sot_engine: Arc<SoTEngine>,
    core_event_tx: Option<broadcast::Sender<CoreEvent>>,
}

impl Oracle {
    /// 新しいインスタンスを生成する
    pub fn new(provider: Arc<dyn LlmProvider>, soul_md: String) -> Self {
        let fast_provider = provider.clone(); // デフォルトは同じものを使用
        let sot_engine = Arc::new(SoTEngine::new(fast_provider.clone(), provider.clone()));
        Self {
            primary_provider: provider,
            fast_provider,
            multi_providers: Vec::new(),
            soul_md,
            sot_engine,
            core_event_tx: None,
        }
    }

    /// イベント送信チャネルを設定する
    pub fn with_event_tx(mut self, tx: broadcast::Sender<CoreEvent>) -> Self {
        self.core_event_tx = Some(tx);
        self
    }

    /// 明示的に高速モデルを指定して SoT を初期化する
    pub fn with_fast_provider(mut self, fast_provider: Arc<dyn LlmProvider>) -> Self {
        self.fast_provider = fast_provider.clone();
        self.sot_engine = Arc::new(SoTEngine::new(fast_provider, self.primary_provider.clone()));
        self
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

    /// AI-Scientist スタイルのマルチレビュー (ADR-023)
    pub async fn multi_review(
        &self,
        content: &str,
        context: &aiome_core_contracts::contracts::ReviewContext,
        config: aiome_core_contracts::contracts::ReviewConfig,
    ) -> Result<aiome_core_contracts::contracts::MultiReviewResult, AiomeError> {
        // Phase B: SoT (Society of Thought) が有効な場合はそちらに委譲する
        if let Some(sot_config) = config.sot_config {
            if sot_config.enabled {
                info!("🔮 [Oracle] Delegating deliberation to SoT Engine (P-1, P-3, P-10)");

                let budget = context.job_id.as_ref().map(|_| 1.0).unwrap_or(1.0);
                let trigger = aiome_core_contracts::contracts::SoTTrigger::Manual;
                let task_desc = format!("Goal: {:?}\nContent: {}", context.goal, content);

                // P-8: SSE イベントブリッジング
                let mut sot_rx = self.sot_engine.subscribe();
                let event_tx_clone = self.core_event_tx.clone();
                let job_id = context.job_id.clone();

                let bridge_handle = tokio::spawn(async move {
                    while let Ok(sot_ev) = sot_rx.recv().await {
                        if let Some(ref tx) = event_tx_clone {
                            let core_ev = CoreEvent::SoTProgress { event: sot_ev };
                            let _ = tx.send(core_ev);
                        }
                    }
                });

                let result = self
                    .sot_engine
                    .run_session(&task_desc, trigger, sot_config, budget)
                    .await;

                // ブリッジ終了を待つ必要はないが、クリーンアップのために handle は落とす
                drop(bridge_handle);

                let (_session_id, outcome, scores) = result?;

                let avg_score = if scores.is_empty() {
                    5.0
                } else {
                    scores.iter().map(|(_, s)| s).sum::<f64>() / scores.len() as f64
                };

                return Ok(aiome_core_contracts::contracts::MultiReviewResult {
                    overall_score: avg_score as f32,
                    decision: match outcome {
                        aiome_core_contracts::contracts::SoTOutcome::AllCriteriaPassed => {
                            aiome_core_contracts::contracts::ReviewDecision::Accept
                        }
                        _ => aiome_core_contracts::contracts::ReviewDecision::Reject,
                    },
                    reflections: scores
                        .iter()
                        .map(|(n, s)| format!("{}: {}", n, s))
                        .collect(),
                    strengths: vec!["Highly deliberated".to_string()],
                    weaknesses: vec![],
                    sot_artifact_uri: None,
                });
            }
        }

        info!(
            "🔮 [Oracle] Starting standard Multi-Review for job: {:?} (iterations: {})",
            context.job_id, config.num_reflections
        );

        let mut current_content = content.to_string();
        let mut reflections = Vec::new();

        for i in 1..=config.num_reflections {
            info!(
                "🔮 [Oracle] Reflection Round {}/{}",
                i, config.num_reflections
            );

            // 1. Critic Round: 現在の内容を批判し、改善点を挙げる
            let critic_prompt = format!(
                "You are a Critical Reviewer. Analyze the following content and identify strengths and weaknesses.\n\nContext: {:?}\n\nContent:\n{}\n\nOutput strengths and weaknesses clearly.",
                context, current_content
            );
            let critic_resp = self
                .primary_provider
                .complete(
                    &critic_prompt,
                    Some("Analyze and provide critical feedback for improvement."),
                )
                .await?;
            reflections.push(format!("Round {}: {}", i, critic_resp.content));

            // 2. Refine Round: 批判に基づき、内容を洗練させる
            let refine_prompt = format!(
                "You are a Scientific Refiner. Improve the content based on the following feedback.\n\nFeedback:\n{}\n\nOriginal Content:\n{}\n\nOutput only the improved content.",
                critic_resp.content, current_content
            );
            let refine_resp = self
                .primary_provider
                .complete(&refine_prompt, Some("Refine and improve the content."))
                .await?;
            current_content = refine_resp.content;
        }

        // 3. Final Verdict Round: 最終的なスコアと判定を下す
        let final_prompt = format!(
            "Analyze the final refined content and provide a structured verdict.\n\nContent:\n{}\n\nOutput in JSON format with 'overall_score' (1.0-10.0), 'decision' (Accept/Reject/Revise), 'strengths' (list), and 'weaknesses' (list).",
            current_content
        );
        let final_resp = self
            .primary_provider
            .complete(
                &final_prompt,
                Some("Provide the final structured review verdict in JSON."),
            )
            .await?;

        let json_str = crate::concept_manager::extract_json(&final_resp.content)?;
        let mut result =
            serde_json::from_str::<aiome_core_contracts::contracts::MultiReviewResult>(
                json_str.as_str(),
            )
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to parse MultiReview JSON: {}", e),
            })?;

        result.reflections = reflections;
        Ok(result)
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
            _request: aiome_core_contracts::llm::LlmRequest,
        ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
            self.complete("", None).await
        }

        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_oracle_evaluation() {
        use std::sync::Arc;
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
        let verdict = res.expect("Should return verdict");
        assert_eq!(verdict.alignment_score, 0.95);
        assert_eq!(verdict.growth_score, 0.8);
        assert!(verdict.should_evolve);
        assert_eq!(
            verdict
                .classification
                .as_ref()
                .expect("Should have classification")
                .domain,
            "Creative"
        );
    }

    #[tokio::test]
    async fn test_oracle_payload_limit() {
        use std::sync::Arc;
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
        use std::sync::Arc;
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
            .expect("Should return consensus verdict");

        assert!(
            verdict.should_evolve,
            "Consensus should be true (majority: 2/3)"
        );
        assert!(verdict.alignment_score > 0.5);
    }

    #[tokio::test]
    async fn test_oracle_multi_review_green() {
        use std::sync::Arc;
        let mock_json = r#"{
            "overall_score": 8.5,
            "decision": "Accept",
            "reflections": [],
            "strengths": ["Clear logic", "Robust extraction"],
            "weaknesses": ["None"]
        }"#;

        let provider = Arc::new(MockLlmProvider {
            response: format!("```json\n{}\n```", mock_json),
        });

        let oracle = Oracle::new(provider, "Be ethical.".to_string());
        let context = aiome_core_contracts::contracts::ReviewContext {
            job_id: Some("test-job".to_string()),
            topic: "AI Ethics".to_string(),
            goal: Some("Create a fair AI".to_string()),
        };
        let config = aiome_core_contracts::contracts::ReviewConfig {
            num_reflections: 2,
            temperature: 0.1,
            sot_config: None,
        };

        let res = oracle.multi_review("Bad content", &context, config).await;

        assert!(res.is_ok(), "Multi-review failed: {:?}", res.err());
        let result = res.expect("Should have result");
        assert_eq!(result.overall_score, 8.5);
        assert_eq!(
            result.decision,
            aiome_core_contracts::contracts::ReviewDecision::Accept
        );
        assert_eq!(
            result.reflections.len(),
            2,
            "Should have 2 reflection rounds"
        );
    }

    #[tokio::test]
    async fn test_oracle_sot_delegation_green() {
        use std::sync::Arc;
        let provider = Arc::new(MockLlmProvider {
            response: "passed".to_string(),
        });

        let (core_tx, mut core_rx) = broadcast::channel(10);
        let oracle = Oracle::new(provider, "Be ethical.".to_string()).with_event_tx(core_tx);

        let context = aiome_core_contracts::contracts::ReviewContext {
            job_id: Some("sot-test-job".to_string()),
            topic: "Ethics".to_string(),
            goal: Some("Fairness".to_string()),
        };

        let config = aiome_core_contracts::contracts::ReviewConfig {
            num_reflections: 1,
            temperature: 0.1,
            sot_config: Some(aiome_core_contracts::contracts::SoTConfig {
                enabled: true,
                max_rounds: 1,
                ..Default::default()
            }),
        };

        let res = oracle.multi_review("Content", &context, config).await;

        assert!(res.is_ok());
        let result = res.expect("Should have result");
        assert_eq!(
            result.decision,
            aiome_core_contracts::contracts::ReviewDecision::Accept
        );

        // SSE イベントがブリッジされているか確認
        let mut found_progress = false;
        while let Ok(ev) = core_rx.recv().await {
            if let CoreEvent::SoTProgress { .. } = ev {
                found_progress = true;
                break;
            }
        }
        assert!(found_progress, "Should have bridged SoTProgress events");
    }
}
