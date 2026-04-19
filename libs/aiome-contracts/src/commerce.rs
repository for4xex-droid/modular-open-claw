/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AiomeError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 経済エンジン・トレイト
///
/// `Project-Nurture` 等の商用モジュールによって実装される。
#[async_trait]
pub trait CommerceEngine: Send + Sync {
    /// エージェントの現在の残高（コイン数）を取得する
    async fn get_balance(&self, agent_id: Uuid) -> Result<u64, AiomeError>;

    /// 実施予定のアクションが経済ポリシー（予算、日次上限、安全性）に適合するか検証する
    async fn validate_activity(
        &self,
        agent_id: Uuid,
        activity_type: &str,
        amount: u64,
    ) -> Result<(), AiomeError>;

    /// 自律的な決済を実行する
    ///
    /// `item_id`: 購入対象の商標・アイテムID
    /// `metadata`: 決済に関連する追加情報
    async fn execute_autonomous_purchase(
        &self,
        agent_id: Uuid,
        item_id: Uuid,
        metadata: serde_json::Value,
    ) -> Result<String, AiomeError>; // 戻り値はトランザクションID
    /// 今日使用したコインの総額を取得する
    async fn get_daily_spend(&self, agent_id: Uuid) -> Result<u64, AiomeError>;

    /// 1日の使用上限を取得する
    async fn get_daily_limit(&self, agent_id: Uuid) -> Result<u64, AiomeError>;

    /// エスクロー（一時保留）決済を作成する
    async fn escrow_create(&self, agent_id: Uuid, amount: u64) -> Result<String, AiomeError>;

    /// エスクローを解放し受注者に送金する
    async fn escrow_release(&self, escrow_id: &str, recipient_id: Uuid) -> Result<(), AiomeError>;

    /// エスクローを依頼者に返金する（キャンセル時）
    async fn escrow_refund(&self, escrow_id: &str) -> Result<(), AiomeError>;

    /// ステーキング（証拠金預託）を行う
    async fn stake(&self, agent_id: Uuid, amount: u64) -> Result<(), AiomeError>;

    /// スラッシュ（罰則によるトークン没収）を実行する
    async fn slash(&self, agent_id: Uuid, amount: u64, reason: &str) -> Result<(), AiomeError>;

    /// ライセンス（Voice Asset 等の使用権）を登録する (P0-1)
    async fn register_license(
        &self,
        agent_id: Uuid,
        asset_id: Uuid,
        license_type: &str,
    ) -> Result<String, AiomeError>;

    /// Webhook 署名の検証
    fn verify_signature(&self, payload: &str, sig_header: &str) -> Result<(), AiomeError>;

    /// Webhook イベントの冪等処理
    async fn process_webhook(
        &self,
        event_id: &str,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<(), AiomeError>;

    /// サブスクリプションを作成する (P0-1)
    async fn create_subscription(
        &self,
        agent_id: Uuid,
        plan_id: &str,
    ) -> Result<String, AiomeError>;

    /// サブスクリプションをキャンセルする (P0-1)
    async fn cancel_subscription(
        &self,
        agent_id: Uuid,
        subscription_id: &str,
    ) -> Result<(), AiomeError>;

    /// サブスクリプションのステータスを取得する (P0-1)
    async fn get_subscription_status(
        &self,
        agent_id: Uuid,
    ) -> Result<SubscriptionStatus, AiomeError>;

    /// 他のエージェントまたはユーザーへ送金する (P0-1, G-2, M-1)
    async fn transfer(&self, from_id: Uuid, to_id: Uuid, amount: u64)
        -> Result<String, AiomeError>; // 戻り値はトランザクションID

    /// 生成コスト（AI画像・音声・動画など）の利用料を天引きする
    async fn deduct_generation_cost(
        &self,
        agent_id: Uuid,
        amount: u64,
        generation_type: &str,
    ) -> Result<(), AiomeError>;
}

/// サブスクリプションのステータス
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    /// 有効
    Active,
    /// キャンセル済み（期間終了まで有効）
    Cancelled,
    /// 支払い待ち・遅延
    PastDue,
    /// 未登録
    None,
}

/// 経済コンテキスト
///
/// LLMのプロンプト等に注入するための、現在の経済状況サマリー。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicContext {
    /// 利用可能なコイン残高
    pub balance: u64,
    /// 今日使用したコインの総額
    pub spent_today: u64,
    /// 1日の使用上限
    pub daily_limit: u64,
}

/// ギフトエンジン・トレイト (A2C 恩返し / Tremendous 連携)
///
/// AI が自律的にユーザーへ実世界価値（ギフトカード等）を還元するための基盤。
#[async_trait]
pub trait GiftEngine: Send + Sync {
    /// ギフト（Tremendous等）を生成・送信する
    /// `recipient_email`: 受取人のメールアドレス
    /// `amount_usd`: ギフトの金額 (USD)
    /// `reason`: 送信理由 (Karma 蓄積、恩返し等)
    async fn send_gift_code(
        &self,
        recipient_email: &str,
        amount_usd: f64,
        reason: &str,
    ) -> Result<String, AiomeError>;

    /// ギフト送信が許可されているか検証する (日次上限、エージェント信頼度)
    async fn validate_gift_policy(&self, agent_id: Uuid, amount_usd: f64)
        -> Result<(), AiomeError>;

    /// 現在のギフトポリシーのコンテキスト（上限額・日次制限等）を返す
    async fn get_policy_context(&self, agent_id: Uuid) -> Result<GiftPolicyContext, AiomeError>;
}

/// ギフトポリシーコンテキスト（LLM プロンプト構築・API 返却用）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GiftPolicyContext {
    pub max_amount_usd: f64,
    pub daily_limit_reached: bool,
    pub daily_sent_count: u32,
    pub daily_sent_total_usd: f64,
}

/// ギフトリクエスト
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GiftRequest {
    pub recipient_email: String,
    pub amount_usd: f64,
    pub reason: String,
}
