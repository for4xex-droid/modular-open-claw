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

use crate::error::NurtureError;
use crate::transaction::{Authorized, Transaction};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementReceipt {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub coin_debited: u64,
    pub points_credited: u64,
    pub settled_at: DateTime<Utc>,
}

#[async_trait]
pub trait SettlementProtocol: Send + Sync {
    async fn settle(&self, tx: &Transaction<Authorized>)
        -> Result<SettlementReceipt, NurtureError>;
    async fn rollback(&self, receipt: &SettlementReceipt) -> Result<(), NurtureError>;
    async fn verify(&self, receipt: &SettlementReceipt) -> Result<bool, NurtureError>;
}
