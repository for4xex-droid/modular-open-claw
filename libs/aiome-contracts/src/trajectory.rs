/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AiomeError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 🛡️ AgentRx 失敗カテゴリ (Taxonomy)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FailureCategory {
    /// 計画されたステップに従わなかった、または不要な追加アクションを行った
    PlanAdherenceFailure,
    /// 証拠のない情報を生成（幻覚）
    InventionOfNewInformation,
    /// ツール呼び出しの形式が不正
    InvalidInvocation,
    /// ツールの出力を誤解して誤った前提で動いた
    MisinterpretationOfOutput,
    /// ユーザーの意図と計画が整合していない
    IntentPlanMisalignment,
    /// ユーザーの意図が不明確で継続不能
    UnderSpecifiedIntent,
    /// 要求に応えられるスキルが存在しない
    IntentNotSupported,
    /// セキュリティガード（BastionGuard/ImmuneSystem）によりブロックされた
    GuardrailsTriggered,
    /// システム障害（インフラ、ネットワーク、LLMダウン）
    SystemFailure,
}

impl ToString for FailureCategory {
    fn to_string(&self) -> String {
        match self {
            Self::PlanAdherenceFailure => "PlanAdherenceFailure".to_string(),
            Self::InventionOfNewInformation => "InventionOfNewInformation".to_string(),
            Self::InvalidInvocation => "InvalidInvocation".to_string(),
            Self::MisinterpretationOfOutput => "MisinterpretationOfOutput".to_string(),
            Self::IntentPlanMisalignment => "IntentPlanMisalignment".to_string(),
            Self::UnderSpecifiedIntent => "UnderSpecifiedIntent".to_string(),
            Self::IntentNotSupported => "IntentNotSupported".to_string(),
            Self::GuardrailsTriggered => "GuardrailsTriggered".to_string(),
            Self::SystemFailure => "SystemFailure".to_string(),
        }
    }
}

impl FailureCategory {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "PlanAdherenceFailure" => Some(Self::PlanAdherenceFailure),
            "InventionOfNewInformation" => Some(Self::InventionOfNewInformation),
            "InvalidInvocation" => Some(Self::InvalidInvocation),
            "MisinterpretationOfOutput" => Some(Self::MisinterpretationOfOutput),
            "IntentPlanMisalignment" => Some(Self::IntentPlanMisalignment),
            "UnderSpecifiedIntent" => Some(Self::UnderSpecifiedIntent),
            "IntentNotSupported" => Some(Self::IntentNotSupported),
            "GuardrailsTriggered" => Some(Self::GuardrailsTriggered),
            "SystemFailure" => Some(Self::SystemFailure),
            _ => None,
        }
    }
}

/// 📉 軌跡（Trajectory）の1ステップ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryStep {
    pub step_id: u32,
    pub action: String,
    pub tool_name: Option<String>,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    pub timestamp: String,
    pub constraint_violations: Vec<ConstraintViolation>,
    pub is_critical_failure: bool,
    pub failure_category: Option<FailureCategory>,
}

/// ⛓️ 制約違反の証拠
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintViolation {
    pub constraint_name: String,
    pub expected: String,
    pub actual: String,
    pub severity: u8, // 0-100
}

/// 🧠 エージェントの自己診断結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDiagnosis {
    pub critical_failure_step: u32,
    pub category: FailureCategory,
    pub root_cause: String,
    pub evidence: Vec<ConstraintViolation>,
    pub self_repair_hint: String,
    pub diagnosed_at: String,
}

/// 📥 TrajectoryStore トレイト
///
/// 実行時の軌跡（Trajectory）を永続化し、AgentRx診断に使用する。
/// JobQueue とは独立させることで、既存システムへの影響を最小化する。
#[async_trait]
pub trait TrajectoryStore: Send + Sync {
    /// ステップを即時永続化する
    async fn record_step(&self, job_id: &str, step: TrajectoryStep) -> Result<(), AiomeError>;

    /// ジョブの全軌跡を取得する
    async fn fetch_trajectory(&self, job_id: &str) -> Result<Vec<TrajectoryStep>, AiomeError>;

    /// 特定のステップを Critical Failure としてマークし、診断情報を保存する
    async fn store_diagnosis(
        &self,
        job_id: &str,
        diagnosis: AgentDiagnosis,
    ) -> Result<(), AiomeError>;

    /// 診断情報を取得する
    async fn fetch_diagnosis(&self, job_id: &str) -> Result<Option<AgentDiagnosis>, AiomeError>;
}
