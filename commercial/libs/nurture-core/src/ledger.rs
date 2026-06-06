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

use crate::coin::CoinWallet;
use crate::points::PointsAccount;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    Purchase,
    Refund,
    Charge,
    SystemFee,
    PointsWithdrawal,
    Burn,
    CloneFork,
    CloneMerge,
    SageMeditation,
    Transfer,
    SurpriseBonus,
    Gift,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub asset_id: Option<Uuid>,
    pub debit_account: ActorId,
    pub credit_account: ActorId,
    pub coin_amount: u64,
    pub points_amount: u64,
    pub entry_type: EntryType,
    pub created_at: DateTime<Utc>,
    pub debit_account_version: Option<u64>,
}

#[async_trait]
pub trait EconomyLedger: Send + Sync {
    async fn record_entry(&self, entry: &LedgerEntry) -> Result<(), NurtureError>;
    async fn record_batch(&self, entries: &[LedgerEntry]) -> Result<(), NurtureError>;
    async fn get_balance(&self, actor: &ActorId) -> Result<CoinWallet, NurtureError>;
    async fn get_points(&self, creator: &ActorId) -> Result<PointsAccount, NurtureError>;
    async fn get_history(
        &self,
        actor: &ActorId,
        limit: u32,
    ) -> Result<Vec<LedgerEntry>, NurtureError>;
    /// transaction_id でエントリを取得する（rollback/verify 用）
    async fn get_entries_by_transaction(
        &self,
        transaction_id: &Uuid,
    ) -> Result<Vec<LedgerEntry>, NurtureError>;
}
