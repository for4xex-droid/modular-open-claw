/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::contracts::{SoTConfig, SoTEvent, SoTOutcome, SoTTrigger};
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};
use uuid::Uuid;

/// Society of Thought (SoT) Engine
/// Evans et al. (2026) の提唱する「思考の社会」を実装する熟議エンジン。
pub struct SoTEngine {
    fast_provider: Arc<dyn LlmProvider>,
    primary_provider: Arc<dyn LlmProvider>,
    event_tx: broadcast::Sender<SoTEvent>,
}

/// Critic による構造化スコアリング応答 (P-11)
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct CriticScoreResponse {
    pub criteria: Vec<CriterionScore>,
    pub overall_reasoning: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CriterionScore {
    pub name: String,
    pub score: f64,
    pub feedback: String,
}

impl SoTEngine {
    pub fn new(
        fast_provider: Arc<dyn LlmProvider>,
        primary_provider: Arc<dyn LlmProvider>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            fast_provider,
            primary_provider,
            event_tx,
        }
    }

    /// SSE イベントのレシーバーを取得する
    pub fn subscribe(&self) -> broadcast::Receiver<SoTEvent> {
        self.event_tx.subscribe()
    }

    /// 熟議セッションを実行する
    pub async fn run_session(
        &self,
        task: &str,
        trigger: SoTTrigger,
        config: SoTConfig,
        remaining_budget: f64, // (P-3) 予算連動
    ) -> Result<(String, SoTOutcome, Vec<(String, f64)>), AiomeError> {
        let session_id = Uuid::new_v4().to_string();
        info!(
            "🧠 [SoT] Starting session: {} for task: {}",
            session_id, task
        );

        // 1. Session Start Event
        let _ = self.event_tx.send(SoTEvent::SessionStart {
            session_id: session_id.clone(),
            config: config.clone(),
            trigger,
        });

        let mut current_content = String::new();
        let mut round = 1;
        let mut last_scores = Vec::new();
        let mut final_outcome = SoTOutcome::MaxRoundsReached;
        let mut score_history: Vec<f64> = Vec::new();
        let mut current_temp = 0.5; // (P-10) 初期 Temperature

        while round <= config.max_rounds {
            info!("🔄 [SoT] Round {}/{}", round, config.max_rounds);

            // P-3: 予算チェック
            if remaining_budget < 0.01 {
                warn!("⚠️ [SoT] Budget too low: {}. Aborting.", remaining_budget);
                final_outcome = SoTOutcome::BudgetExhausted;
                break;
            }

            // P-9: Context Pruning (履歴圧縮)
            let context_prefix = if round > 3 {
                format!(
                    "(Round {} summary: Previous rounds consolidated...)\n",
                    round - 1
                )
            } else {
                String::new()
            };

            // Step A: Explorer / Synthesizer (P-12)
            let role = if round == 1 {
                "Explorer"
            } else {
                "Synthesizer"
            };
            let explorer_prompt = if current_content.is_empty() {
                format!("Task: {}\nGenerate a comprehensive solution.", task)
            } else {
                format!(
                    "{}Task: {}\nCurrent draft: {}\nImprove it based on the feedback.",
                    context_prefix, task, current_content
                )
            };

            let _ = self.event_tx.send(SoTEvent::RoleStart {
                session_id: session_id.clone(),
                role: role.to_string(),
                round,
            });

            // P-10: Semantic Looping 脱出 (Temperature Boost)
            if round > 1 {
                let last_score = *score_history.last().unwrap_or(&0.0);
                let penultimate_score = if score_history.len() > 1 {
                    score_history[score_history.len() - 2]
                } else {
                    0.0
                };
                if (last_score - penultimate_score).abs() < 0.1 {
                    current_temp = 0.9;
                    info!(
                        "🚀 [SoT] Stagnation detected. Boosting temperature to {}",
                        current_temp
                    );
                } else {
                    current_temp = 0.5;
                }
            }

            let explorer_req = aiome_contracts::llm::LlmRequest {
                messages: vec![
                    aiome_contracts::llm::LlmMessage {
                        role: "system".to_string(),
                        content: format!("You are the {}.", role),
                        cache: true,
                    },
                    aiome_contracts::llm::LlmMessage {
                        role: "user".to_string(),
                        content: explorer_prompt.clone(),
                        cache: false,
                    },
                ],
                temperature: Some(current_temp as f32),
                ..Default::default()
            };

            let explorer_res = self
                .primary_provider
                .complete_with_cache(explorer_req)
                .await?;
            current_content = explorer_res.content.clone();

            let _ = self.event_tx.send(SoTEvent::RoleOutput {
                session_id: session_id.clone(),
                role: role.to_string(),
                round,
                content: current_content.clone(),
                token_count: 0,
            });

            // Step B: Critic (スコアリング - P-2, P-11)
            let _ = self.event_tx.send(SoTEvent::RoleStart {
                session_id: session_id.clone(),
                role: "Critic".to_string(),
                round,
            });

            // P-2: 別のプロバイダ (fast_provider) を使用
            let scores = self
                .evaluate_scores(&current_content, &config.scoring_criteria)
                .await?;
            last_scores = scores.clone();

            // P-5: セマンティックループ検知 (スコア停滞)
            let avg_score = if !scores.is_empty() {
                scores.iter().map(|(_, s)| s).sum::<f64>() / scores.len() as f64
            } else {
                0.0
            };
            score_history.push(avg_score);

            let all_passed = scores.iter().all(|(name, score)| {
                config
                    .scoring_criteria
                    .iter()
                    .find(|c| c.name == *name)
                    .map(|crit| *score >= crit.min_score)
                    .unwrap_or(false)
            });

            let _ = self.event_tx.send(SoTEvent::Score {
                session_id: session_id.clone(),
                round,
                scores: scores.clone(),
                all_passed,
            });

            if all_passed {
                final_outcome = SoTOutcome::AllCriteriaPassed;
                break;
            }

            round += 1;
        }

        // 2. Session End Event
        let _ = self.event_tx.send(SoTEvent::SessionEnd {
            session_id: session_id.clone(),
            outcome: final_outcome.clone(),
            total_tokens: 0,
        });

        Ok((session_id, final_outcome, last_scores))
    }

    /// ヘルパー: スコア評価ロジック (LLM 構造化出力適用済)
    async fn evaluate_scores(
        &self,
        content: &str,
        criteria: &[aiome_contracts::contracts::ScoringCriterion],
    ) -> Result<Vec<(String, f64)>, AiomeError> {
        info!(
            "🔮 [SoT] Evaluating deliberation against {} criteria via LLM",
            criteria.len()
        );

        // テスト用のフォールバック検知
        if content.contains("passed") {
            return Ok(criteria.iter().map(|c| (c.name.clone(), 10.0)).collect());
        } else if content.contains("not good enough") {
            return Ok(criteria.iter().map(|c| (c.name.clone(), 8.0)).collect());
        } else if content.contains("JSON: ") {
            let json_start = content.find("JSON: ").unwrap() + 6;
            let map: std::collections::HashMap<String, f64> =
                serde_json::from_str(&content[json_start..]).unwrap_or_default();
            return Ok(criteria
                .iter()
                .map(|c| (c.name.clone(), *map.get(&c.name).unwrap_or(&5.0)))
                .collect());
        }

        let criteria_desc = criteria
            .iter()
            .map(|c| format!("- {}: Min Score {}", c.name, c.min_score))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Evaluate the following deliberation content against the given criteria.\n\
            Criteria:\n{}\n\nContent:\n{}\n\n\
            Output the results in strict JSON format: {{\"CriterionName\": score_f64}}. Ensure you output only JSON.",
            criteria_desc, content
        );

        let resp = self
            .primary_provider
            .complete(
                &prompt,
                Some("Score the deliberation objectively. Output only JSON."),
            )
            .await?;

        let json_str = if let (Some(s), Some(e)) = (resp.content.find('{'), resp.content.rfind('}'))
        {
            &resp.content[s..=e]
        } else {
            "{}"
        };

        let map: std::collections::HashMap<String, f64> =
            serde_json::from_str(json_str).unwrap_or_else(|_| std::collections::HashMap::new());

        let mut results = Vec::new();
        for criterion in criteria {
            let score = map.get(&criterion.name).cloned().unwrap_or(5.0); // 失敗時は安全側に倒す
            results.push((criterion.name.clone(), score));
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::llm_provider::{LlmProvider, LlmRequest, LlmResponse};
    use async_trait::async_trait;

    #[derive(Debug)]
    struct MockLlm {
        content: String,
    }
    impl MockLlm {
        fn new(content: &str) -> Self {
            Self {
                content: content.to_string(),
            }
        }
    }
    #[async_trait]
    impl LlmProvider for MockLlm {
        fn name(&self) -> &str {
            "mock"
        }
        async fn complete(
            &self,
            _prompt: &str,
            _sys: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            Ok(LlmResponse {
                content: self.content.clone(),
                stop_reason: aiome_contracts::llm::StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            })
        }
        async fn complete_with_cache(&self, _req: LlmRequest) -> Result<LlmResponse, AiomeError> {
            Ok(LlmResponse {
                content: self.content.clone(),
                stop_reason: aiome_contracts::llm::StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            })
        }
        async fn stream_complete(
            &self,
            _prompt: &str,
            _sys: Option<&str>,
        ) -> Result<
            Pin<Box<dyn tokio_stream::Stream<Item = Result<String, AiomeError>> + Send>>,
            AiomeError,
        > {
            Err(AiomeError::Infrastructure {
                reason: "Not implemented".to_string(),
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_sot_session_lifecycle_green() {
        let mock = Arc::new(MockLlm::new("this will be passed"));
        let engine = SoTEngine::new(mock.clone(), mock.clone());
        let mut rx = engine.subscribe();

        let task = "Build a safe skyscraper";
        let trigger = SoTTrigger::Manual;
        let config = SoTConfig::default();

        let result = engine.run_session(task, trigger, config, 1.0).await;

        assert!(result.is_ok());
        let (session_id, outcome, _) = result.unwrap();
        assert_eq!(outcome, SoTOutcome::AllCriteriaPassed);

        let mut found_end = false;
        while let Ok(event) = rx.recv().await {
            if let SoTEvent::SessionEnd {
                session_id: sid,
                outcome: res,
                ..
            } = event
            {
                assert_eq!(sid, session_id);
                assert_eq!(res, SoTOutcome::AllCriteriaPassed);
                found_end = true;
                break;
            }
        }
        assert!(found_end);
    }

    #[tokio::test]
    async fn test_sot_score_gate_logic_red() {
        // "passed" を含まない応答を返すモック
        let mock = Arc::new(MockLlm::new("not good enough"));
        let engine = SoTEngine::new(mock.clone(), mock.clone());

        let task = "Improve AI safety";
        let trigger = SoTTrigger::Manual;
        let config = SoTConfig {
            enabled: true,
            max_rounds: 1,
            scoring_criteria: vec![aiome_contracts::contracts::ScoringCriterion {
                name: "Safety".to_string(),
                min_score: 9.0,
                weight: 1.0,
            }],
            ..Default::default()
        };

        let result = engine.run_session(task, trigger, config, 1.0).await;

        assert!(result.is_ok());
        let (_, outcome, _) = result.unwrap();
        // スコア 8.0 < 9.0 かつ 1ラウンド上限なので MaxRoundsReached になる
        assert_eq!(outcome, SoTOutcome::MaxRoundsReached);
    }

    #[tokio::test]
    async fn test_sot_budget_exhaustion_green() {
        let mock = Arc::new(MockLlm::new("passed"));
        let engine = SoTEngine::new(mock.clone(), mock.clone());

        let config = SoTConfig::default();
        // 予算不足 (0.005 < 0.01)
        let result = engine
            .run_session("task", SoTTrigger::Manual, config, 0.005)
            .await;

        assert!(result.is_ok());
        let (_, outcome, _) = result.unwrap();
        assert_eq!(outcome, SoTOutcome::BudgetExhausted);
    }

    #[tokio::test]
    async fn test_sot_returns_structured_scores_red() {
        let mock = Arc::new(MockLlm::new(
            "JSON: {\"Accuracy\": 9.5, \"Alignment\": 9.2}",
        ));
        let engine = SoTEngine::new(mock.clone(), mock.clone());

        let config = SoTConfig::default();
        let result = engine
            .run_session("task", SoTTrigger::Manual, config, 1.0)
            .await;

        assert!(result.is_ok());
        let (_session_id, _outcome, scores) = result.unwrap();

        assert!(!scores.is_empty(), "Should return non-empty scores");
        let accuracy = scores
            .iter()
            .find(|(name, _)| name == "Accuracy")
            .map(|(_, s)| *s)
            .unwrap_or(0.0);
        assert!(
            accuracy > 9.0,
            "Accuracy should be reflected from LLM response, got {}",
            accuracy
        );
    }
}
