/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct BalanceResponse {
    pub balance: u64,
}

#[derive(Serialize)]
pub struct DailyStatsResponse {
    pub spent_today: u64,
    pub daily_limit: u64,
}

#[derive(Deserialize)]
pub struct CoinChargeRequest {
    pub actor_id: Uuid,
    pub amount: u64,
    pub currency: String,
    pub stripe_event_id: String,
    pub idempotency_key: String,
}

#[derive(Deserialize)]
pub struct EscrowCreateRequest {
    pub actor_id: Uuid,
    pub amount: u64,
}

#[derive(Serialize)]
pub struct EscrowCreateResponse {
    pub escrow_id: String,
}

#[derive(Deserialize)]
pub struct EscrowReleaseRequest {
    pub escrow_id: String,
    pub recipient_id: Uuid,
}

#[derive(Deserialize)]
pub struct EscrowRefundRequest {
    pub escrow_id: String,
}

#[derive(Deserialize)]
pub struct DeductCostRequest {
    pub actor_id: Uuid,
    pub asset_id: Option<Uuid>,
    pub amount: u64,
    pub generation_type: String,
}

#[derive(Deserialize)]
pub struct TransferRequest {
    pub from_id: Uuid,
    pub to_id: Uuid,
    pub amount: u64,
    /// Aiome 側で生成された冪等性キー（将来の重複防止用に予約）
    #[serde(default, rename = "idempotency_key")]
    pub _idempotency_key: Option<String>,
}

#[derive(Serialize)]
pub struct TransferResponse {
    pub transaction_id: String,
}

#[derive(Deserialize)]
pub struct InstantRefundRequest {
    pub transaction_id: String,
    pub actor_id: Uuid,
    #[serde(default, rename = "idempotency_key")]
    pub _idempotency_key: Option<String>,
}

#[derive(Deserialize)]
pub struct WithdrawPointsRequest {
    pub actor_id: Uuid,
    pub points: u64,
    #[serde(default, rename = "idempotency_key")]
    pub _idempotency_key: Option<String>,
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct PurchaseS2SRequest {
    pub buyer: Uuid,
    pub item_id: Uuid,
    pub idempotency_key: Option<String>,
}

#[derive(Serialize)]
pub struct PurchaseS2SResponse {
    pub transaction_id: String,
}

#[derive(Deserialize)]
pub struct LoraTrainRequest {
    pub base_model: String,
    pub dataset_id: String,
    pub params: serde_json::Value,
}

#[derive(Serialize)]
pub struct LoraTrainResponse {
    pub job_id: String,
}

#[derive(Deserialize)]
pub struct ValidateActivityRequest {
    pub actor_id: Uuid,
    pub activity_type: String,
    pub amount: u64,
}
