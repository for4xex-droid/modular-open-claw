/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AiomeError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// AI 受発注（Gig Economy）プロトコル：ステータス遷移
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
pub enum GigOrderStatus {
    Open,       // Intent公開中：入札待ち
    Bidding,    // 入札受付中
    Accepted,   // 受注者決定：エスクローロック完了
    InProgress, // 履行中
    Delivered,  // 納品済：検証待ち
    Verified,   // 検証パス：決済実行準備
    Rejected,   // 検証失敗：供託金没収
    Disputed,   // 紛争中：要人間介入
    Completed,  // 完了：報酬分配済
    Cancelled,  // キャンセル：エスクロー返金
}

/// インテントカテゴリ（AgentSense 用）
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
pub enum IntentCategory {
    Learning, // 学習用データ
    Tool,     // ツール作成
    Service,  // サービス代行
    Content,  // コンテンツ生成
    Other,    // その他
}

/// 納品検収基準（The Immutable Gateway の審査エンジン）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "config")]
pub enum AcceptanceCriteria {
    /// JSONスキーマによる構造検証
    JsonSchema { schema: serde_json::Value },
    /// ファイル属性（MIMEタイプ/サイズ）検証
    FileType { mime: String, max_bytes: u64 },
    /// カスタム Wasm ロジックによる検証
    WasmValidator { wasm_module_cid: String },
    /// Oracle（LLM審判）による定量的評価
    OracleJudge {
        rubric_prompt: String,
        min_score: f32,
        model: Option<String>,
    },
}

/// インテント（依頼の欲求表明）：AIがブロードキャストする
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GigIntent {
    #[schema(value_type = String)]
    pub id: Uuid,
    #[schema(value_type = String)]
    pub requester_id: Uuid,
    pub description: String,
    pub criteria: Vec<AcceptanceCriteria>,
    pub max_budget_coins: u64,
    pub category: IntentCategory,
    #[schema(value_type = String)]
    pub deadline: DateTime<Utc>,
}

/// ビッド（入札）：受注を希望するAIが送信する
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GigBid {
    #[schema(value_type = String)]
    pub id: Uuid,
    #[schema(value_type = String)]
    pub intent_id: Uuid,
    #[schema(value_type = String)]
    pub bidder_id: Uuid,
    pub price_coins: u64,
    pub est_duration_sec: u64,
    pub deposit_amount: u64, // 没収リスクを負う供託金
}

/// 納品物
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GigDeliverable {
    #[schema(value_type = String)]
    pub order_id: Uuid,
    #[schema(value_type = String)]
    pub deliverer_id: Uuid,
    pub artifact_path: String,
    pub metadata: serde_json::Value,
}

/// 検証結果
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerificationResult {
    #[schema(value_type = String)]
    pub order_id: Uuid,
    pub passed: bool,
    pub score: f32,
    pub detail: String,
}

/// GigEngine トレイト：AI間ギグ・エコノミーを駆動する本体
#[async_trait]
pub trait GigEngine: Send + Sync {
    /// 依頼を公開・ブロードキャストする
    async fn publish_intent(&self, intent: GigIntent) -> Result<Uuid, AiomeError>;

    /// 依頼に対して入札する
    async fn submit_bid(&self, bid: GigBid) -> Result<(), AiomeError>;

    /// 落札者を決定し、エスクローをロックする
    async fn accept_bid(&self, intent_id: Uuid, bid_id: Uuid) -> Result<(), AiomeError>;

    /// 納品物を提出する
    async fn deliver(&self, deliverable: GigDeliverable) -> Result<(), AiomeError>;

    /// 納品物を検証し、決済を執行する（PassならRelease、FailならSlash）
    async fn verify_and_settle(&self, order_id: Uuid) -> Result<VerificationResult, AiomeError>;
}

/// UTXO型エスクロー管理（Typestate による二重解放防止）
pub struct UnspentEscrow(pub String);
pub struct SpentEscrow(pub String);
pub struct RefundedEscrow(pub String);
