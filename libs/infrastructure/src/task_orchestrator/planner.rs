/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::error::AiomeError;
use aiome_contracts::traits::StrategicPlanner;
use aiome_contracts::trajectory::TrajectoryStep;
use aiome_core::llm_provider::LlmProvider;
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

        // response.content をパースする
        if let Ok(json_steps) = serde_json::from_str::<Vec<serde_json::Value>>(&response.content) {
            for v in json_steps {
                steps.push(TrajectoryStep {
                    step_id: current_step_id,
                    job_id: context["job_id"].as_str().map(|s| s.to_string()),
                    parent_step_id: None,
                    step_category: match v["step_category"].as_str().unwrap_or("Execution") {
                        "Planning" => aiome_contracts::trajectory::StepCategory::Planning,
                        "Verification" => aiome_contracts::trajectory::StepCategory::Review,
                        _ => aiome_contracts::trajectory::StepCategory::Execution,
                    },
                    action: v["description"].as_str().unwrap_or("").to_string(),
                    tool_name: None,
                    input: v["input"].clone(),
                    output: serde_json::Value::Null,
                    timestamp: Utc::now().to_rfc3339(),
                    constraint_violations: vec![],
                    is_critical_failure: false,
                    failure_category: None,
                    reasoning: v["reasoning"].as_str().map(|s| s.to_string()),
                    completion_criteria: None,
                    interaction_id: None,
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
                step_category: aiome_contracts::trajectory::StepCategory::Decision,
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
            });
        }

        Ok(steps)
    }
}
