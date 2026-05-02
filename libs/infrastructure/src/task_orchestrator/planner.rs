/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::llm_provider::LlmProvider;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::traits::StrategicPlanner;
use aiome_core_contracts::trajectory::TrajectoryStep;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;

/// デフォルトの戦略的計画エンジン
pub struct DefaultStrategicPlanner {
    llm: Arc<dyn LlmProvider>,
}

impl DefaultStrategicPlanner {
    /// 新しいインスタンスを生成する
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self { llm }
    }
}

#[async_trait]
impl StrategicPlanner for DefaultStrategicPlanner {
    async fn plan_goal(
        &self,
        goal: &str,
        context: serde_json::Value,
    ) -> Result<Vec<TrajectoryStep>, AiomeError> {
        // Phase 13-C: Goal Decomposition via LLM
        let prompt = format!(
            "Goal: {}\nContext: {}\n\nこのゴールを達成するための具体的なステップ（JSON形式のリスト）に分解してください。各ステップには 'description', 'step_category' (Planning, Execution, Verification), 'reasoning' を含めてください。",
            goal, context
        );

        let response = self.llm.complete(&prompt, Some("You are a Strategic Planning Agent. Break down complex goals into logical, executable steps.")).await?;

        let mut steps = Vec::new();
        let mut current_step_id = 1;

        // response.content から JSON 部分を抽出する
        let content = response.content.trim();
        let json_str = if content.contains("```json") {
            content
                .split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(content)
                .trim()
        } else if content.contains("```") {
            content
                .split("```")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(content)
                .trim()
        } else {
            content
        };

        if let Ok(json_steps) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
            for v in json_steps {
                steps.push(TrajectoryStep {
                    step_id: current_step_id,
                    job_id: context["job_id"].as_str().map(|s| s.to_string()),
                    parent_step_id: None,
                    step_category: match v["step_category"].as_str().unwrap_or("Execution") {
                        "Planning" => aiome_core_contracts::trajectory::StepCategory::Planning,
                        "Verification" => aiome_core_contracts::trajectory::StepCategory::Review,
                        _ => aiome_core_contracts::trajectory::StepCategory::Execution,
                    },
                    action: v["description"].as_str().unwrap_or("").to_string(),
                    tool_name: v["tool_name"].as_str().map(|s| s.to_string()),
                    input: v["input"].clone(),
                    output: serde_json::Value::Null,
                    timestamp: Utc::now().to_rfc3339(),
                    constraint_violations: vec![],
                    is_critical_failure: false,
                    failure_category: None,
                    reasoning: v["reasoning"].as_str().map(|s| s.to_string()),
                    completion_criteria: None,
                    interaction_id: None,
                    verified_invariants: vec![],
                    verification_time_us: None,
                    state_hash: None,
                    parent_state_hash: None,
                });
                current_step_id += 1;
            }
        }

        // フォールバック
        if steps.is_empty() {
            steps.push(TrajectoryStep {
                step_id: current_step_id,
                job_id: context["job_id"].as_str().map(|s| s.to_string()),
                parent_step_id: None,
                step_category: aiome_core_contracts::trajectory::StepCategory::Decision,
                action: "Final Decision".into(),
                tool_name: None,
                input: serde_json::Value::Null,
                output: serde_json::Value::Null,
                timestamp: Utc::now().to_rfc3339(),
                constraint_violations: vec![],
                is_critical_failure: false,
                failure_category: None,
                reasoning: Some("Evaluating all steps and context for the final output.".into()),
                completion_criteria: None,
                interaction_id: None,
                verified_invariants: vec![],
                verification_time_us: None,
                state_hash: None,
                parent_state_hash: None,
            });
        }

        Ok(steps)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core_contracts::trajectory::StepCategory;
    use std::sync::Arc;

    #[derive(Debug)]
    struct MockLlm {
        response: String,
    }

    #[async_trait]
    impl LlmProvider for MockLlm {
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<aiome_core_contracts::llm::LlmResponse, AiomeError> {
            Ok(aiome_core_contracts::llm::LlmResponse {
                content: self.response.clone(),
                stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "mock"
        }
        async fn complete_with_cache(
            &self,
            _req: aiome_core_contracts::llm::LlmRequest,
        ) -> Result<aiome_core_contracts::llm::LlmResponse, AiomeError> {
            Err(AiomeError::Infrastructure {
                reason: "Not yet implemented".into(),
            })
        }
    }

    #[tokio::test]
    async fn test_plan_goal_decomposition_markdown_red() {
        // LLM が Markdown コードブロックでレスポンスを返してきた場合 (堅牢性のテスト)
        let mock_response = format!(
            "Here is the plan:\n```json\n{}\n```",
            json!([
                {
                    "description": "Research AI trends",
                    "step_category": "Planning",
                    "reasoning": "Need to gather data first"
                },
                {
                    "description": "Write summary report",
                    "step_category": "Execution",
                    "reasoning": "Synthesize the gathered information"
                }
            ])
        );

        let mock_llm = Arc::new(MockLlm {
            response: mock_response,
        });
        let planner = DefaultStrategicPlanner::new(mock_llm);

        let steps = planner
            .plan_goal("Analyze AI trends and write a report", json!({}))
            .await
            .unwrap(); // allow-anti-pattern

        // 3. 現状の実装では Markdown をパースできず、フォールバック（1ステップ）になるはず (RED)
        assert_eq!(
            steps.len(),
            2,
            "Should have extracted 2 steps from markdown, but got {:?}",
            steps
        );
        assert_eq!(steps[0].step_category, StepCategory::Planning);
    }
}
