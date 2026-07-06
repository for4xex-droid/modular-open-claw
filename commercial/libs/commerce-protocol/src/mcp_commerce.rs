/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 *
 * Usage of this software is subject to the BSL 1.1 terms.
 * Commercial use requires a separate license agreement.
 */

use crate::commodity::{CommodityKind, ItemDescriptor};
use crate::identity::ActorId;
use crate::settlement::SettlementReceipt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSearchRequest {
    pub query: Option<String>,
    pub kind: Option<CommodityKind>,
    pub max_price: Option<u64>,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSearchResponse {
    pub items: Vec<ItemDescriptor>,
    pub total_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyRequest {
    pub item_id: Uuid,
    pub buyer: ActorId,
    pub idempotency_key: Option<String>,
    pub use_escrow: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyResponse {
    pub transaction_id: Uuid,
    pub receipt: SettlementReceipt,
    pub license_id: Option<Uuid>,
    pub escrow_id: Option<String>,
    #[serde(default)]
    pub surprise_bonus: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxExecRequest {
    pub code: String,
    pub input_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxExecResponse {
    pub output_data: serde_json::Value,
}
