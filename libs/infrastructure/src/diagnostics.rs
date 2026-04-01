/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::Job;
use aiome_core::trajectory::{AgentDiagnosis, FailureCategory, TrajectoryStep};
use chrono::Utc;
use std::sync::Arc;

/// AgentRxの軌跡分析と自己診断
pub struct AgentRxDiagnostics {
    provider: Arc<dyn LlmProvider>,
}

impl AgentRxDiagnostics {
    /// 新しいインスタンスを生成する
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self { provider }
    }

    /// 失敗した軌跡を分析し、根本原因と自己修復ヒントを返す (AgentRx LLM Judge)
    pub async fn diagnose(
        &self,
        trajectory: &[TrajectoryStep],
        job: &Job,
    ) -> Result<AgentDiagnosis, AiomeError> {
        let trajectory_json = serde_json::to_string_pretty(trajectory).unwrap_or_default();
        let job_json = serde_json::to_string_pretty(job).unwrap_or_default();

        let prompt = format!(
            "あなたはAIエージェントの失敗原因を特定するフォレンジックエンジニアです。\n\
             以下の実行軌跡（Trajectory）とジョブ内容を分析し、最初の回復不能な失敗（Critical Failure Step）を特定してください。\n\n\
             ### Job Context\n{}\n\n\
             ### Execution Trajectory\n{}\n\n\
             分析結果を以下の JSON 形式で返してください：\n\
             {{\n  \
               \"critical_failure_step\": ステップID,\n  \
               \"failure_category\": \"FailureCategoryの文字列\",\n  \
               \"root_cause\": \"なぜ失敗したかの技術的・論理的分析\",\n  \
               \"self_repair_hint\": \"次回のリトライで何を修正すべきか（KarmaDirectivesへの指示形式）\"\n\
             }}\n\n\
             FailureCategory一覧: PlanAdherenceFailure, InventionOfNewInformation, InvalidInvocation, MisinterpretationOfOutput, IntentPlanMisalignment, UnderSpecifiedIntent, IntentNotSupported, GuardrailsTriggered, SystemFailure",
            job_json, trajectory_json
        );

        let resp = self
            .provider
            .complete(&prompt, Some("厳格なJSON形式で応答してください。"))
            .await?;

        // JSON抽出（既存のロジックを想定、あるいはシンプルにパース）
        let json_str = crate::concept_manager::extract_json(&resp.content)?;
        let v: serde_json::Value =
            serde_json::from_str(&json_str).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to parse diagnostic JSON: {}", e),
            })?;

        let step_id = v["critical_failure_step"].as_u64().unwrap_or(0) as u32;
        let cat_str = v["failure_category"].as_str().unwrap_or("SystemFailure");
        let category = cat_str.parse().unwrap_or(FailureCategory::SystemFailure);
        let root_cause = v["root_cause"].as_str().unwrap_or("Unknown").to_string();
        let hint = v["self_repair_hint"].as_str().unwrap_or("").to_string();

        // 当該ステップの違反情報を収集
        let evidence = trajectory
            .iter()
            .find(|s| s.step_id == step_id)
            .map(|s| s.constraint_violations.clone())
            .unwrap_or_default();

        Ok(AgentDiagnosis {
            critical_failure_step: step_id,
            category,
            root_cause,
            evidence,
            self_repair_hint: hint,
            diagnosed_at: Utc::now().to_rfc3339(),
        })
    }

    /// 診断結果とエージェントの成長度（AgentStats）に基づき、最適な修復戦略を提案する
    pub fn suggest_repair_strategy(
        &self,
        diagnosis: &AgentDiagnosis,
        stats: &aiome_contracts::types::AgentStats,
        current_retries: u32,
    ) -> crate::repair_strategy::RepairStrategy {
        let max_retries = crate::repair_strategy::RepairCalculator::calculate_max_retries(stats);
        crate::repair_strategy::suggest_strategy(
            &diagnosis.category,
            &diagnosis.self_repair_hint,
            current_retries,
            max_retries,
        )
    }
}
