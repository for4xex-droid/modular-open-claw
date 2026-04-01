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
        if trajectory.is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "Cannot diagnose empty execution trajectory".to_string(),
            });
        }

        let trajectory_json =
            serde_json::to_string_pretty(trajectory).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to serialize trajectory: {}", e),
            })?;
        let job_json =
            serde_json::to_string_pretty(job).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to serialize job: {}", e),
            })?;

        let prompt = format!(
            "あなたはAIエージェントの失敗原因を特定するシニア・フォレンジックエンジニアです。\n\
             以下の実行軌跡（Trajectory）とジョブ内容を詳細に分析し、エージェントが目標達成に失敗した【根本原因（Root Cause）】と【最初に致命的な失敗が起きたステップ（Critical Failure Step）】を特定してください。\n\
             また、同じ失敗を繰り返さないための【具体的な行動修正・自己修復ヒント（Self-Repair Hint）】を提示してください。ヒントは「ツール A の代わりにツール B を使う」「引数 X ではなく Y を渡す」など、エージェントが直接行動に移せる実行可能なレベルで記述すること。\n\n\
             ### Job Context\n{}\n\n\
             ### Execution Trajectory\n{}\n\n\
             分析結果を以下の厳密な JSON 形式で返してください：\n\
             {{\n  \
               \"critical_failure_step\": ステップID,\n  \
               \"failure_category\": \"PlanAdherenceFailure, InventionOfNewInformation, InvalidInvocation, MisinterpretationOfOutput, IntentPlanMisalignment, UnderSpecifiedIntent, IntentNotSupported, GuardrailsTriggered, または SystemFailure のいずれか\",\n  \
               \"root_cause\": \"なぜ失敗したかの具体的な技術的・論理的分析\",\n  \
               \"self_repair_hint\": \"次回のリトライで何を修正すべきか（具体的かつ行動可能な指示）\"\n\
             }}\n",
            job_json, trajectory_json
        );

        let complete_future = self.provider.complete(
            &prompt,
            Some("厳格なJSON形式で応答してください。マクロな文字列を含めないでください。"),
        );

        // FAIL-SAFE: 30秒のタイムアウトを設定して診断エンジンのハングを防ぐ
        let resp_result =
            tokio::time::timeout(std::time::Duration::from_secs(30), complete_future).await;

        let resp = match resp_result {
            Ok(Ok(response)) => response,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(AiomeError::Infrastructure {
                    reason: "Diagnostics text generation timed out after 30 seconds".to_string(),
                })
            }
        };

        // JSON抽出（既存のロジックを想定、あるいはシンプルにパース）
        let json_str = crate::concept_manager::extract_json(&resp.content)?;
        let v: serde_json::Value =
            serde_json::from_str(&json_str).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to parse diagnostic JSON: {}", e),
            })?;

        let step_id =
            v["critical_failure_step"]
                .as_u64()
                .ok_or_else(|| AiomeError::Infrastructure {
                    reason: "Missing critical_failure_step in diagnosis response".into(),
                })? as u32;
        let cat_str = v["failure_category"].as_str().unwrap_or("SystemFailure");
        let category = cat_str.parse().unwrap_or(FailureCategory::SystemFailure);
        let root_cause = v["root_cause"]
            .as_str()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "Missing root_cause in diagnosis response".into(),
            })?
            .to_string();
        let hint = v["self_repair_hint"]
            .as_str()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "Missing self_repair_hint in diagnosis response".into(),
            })?
            .to_string();

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
        stats: &aiome_core_contracts::types::AgentStats,
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
