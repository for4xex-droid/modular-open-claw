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

use crate::commodity::{ItemDescriptor, PriceTag};
use crate::identity::ActorId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OfferStatus {
    Draft,
    Active,
    Suspended,
    SoldOut,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaleMode {
    Instant,
    Subscription {
        interval_days: u32,
        price_coins: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Offer {
    pub id: Uuid,
    pub seller: ActorId,
    pub item: ItemDescriptor,
    pub price: PriceTag,
    pub status: OfferStatus,
    pub sale_mode: SaleMode,
    pub stock: Option<u32>,
    pub listed_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}
