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

impl std::fmt::Display for FailureCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::PlanAdherenceFailure => "PlanAdherenceFailure",
            Self::InventionOfNewInformation => "InventionOfNewInformation",
            Self::InvalidInvocation => "InvalidInvocation",
            Self::MisinterpretationOfOutput => "MisinterpretationOfOutput",
            Self::IntentPlanMisalignment => "IntentPlanMisalignment",
            Self::UnderSpecifiedIntent => "UnderSpecifiedIntent",
            Self::IntentNotSupported => "IntentNotSupported",
            Self::GuardrailsTriggered => "GuardrailsTriggered",
            Self::SystemFailure => "SystemFailure",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for FailureCategory {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PlanAdherenceFailure" => Ok(Self::PlanAdherenceFailure),
            "InventionOfNewInformation" => Ok(Self::InventionOfNewInformation),
            "InvalidInvocation" => Ok(Self::InvalidInvocation),
            "MisinterpretationOfOutput" => Ok(Self::MisinterpretationOfOutput),
            "IntentPlanMisalignment" => Ok(Self::IntentPlanMisalignment),
            "UnderSpecifiedIntent" => Ok(Self::UnderSpecifiedIntent),
            "IntentNotSupported" => Ok(Self::IntentNotSupported),
            "GuardrailsTriggered" => Ok(Self::GuardrailsTriggered),
            "SystemFailure" => Ok(Self::SystemFailure),
            _ => Err(()),
        }
    }
}

/// 📂 ステップカテゴリ (ADR-024)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StepCategory {
    /// 既存の汎用ステップ
    General,
    /// 仮説生成（ADR-023 連携）
    Hypothesis,
    /// ツール選択（①連携）
    ToolSelection,
    /// 計画策定（③連携）
    Planning,
    /// 実行
    Execution,
    /// 自己レビュー（ADR-023 連携）
    Review,
    /// 最終判断
    Decision,
}

impl Default for StepCategory {
    fn default() -> Self {
        Self::General
    }
}

/// 📉 軌跡（Trajectory）の1ステップ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryStep {
    pub step_id: u32,
    pub job_id: Option<String>,
    pub action: String,
    pub tool_name: Option<String>,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    pub timestamp: String,
    pub constraint_violations: Vec<ConstraintViolation>,
    pub is_critical_failure: bool,
    pub failure_category: Option<FailureCategory>,

    // --- ADR-024 拡張フィールド ---
    /// 「なぜこの行動を選んだか」の推論理由
    pub reasoning: Option<String>,
    /// 因果関係の親ステップ ID
    pub parent_step_id: Option<String>,
    /// ステップの種別カテゴリ
    pub step_category: StepCategory,
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
