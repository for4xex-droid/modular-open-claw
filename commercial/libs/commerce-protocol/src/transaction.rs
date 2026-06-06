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
use std::marker::PhantomData;
use uuid::Uuid;

/// Transaction 状態マーカー用のトレイト
pub trait TxState: Send + Sync {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Initiated;
impl TxState for Initiated {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authorized;
impl TxState for Authorized {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settled;
impl TxState for Settled {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Failed;
impl TxState for Failed {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Refunded;
impl TxState for Refunded {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cancelled;
impl TxState for Cancelled {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction<S: TxState> {
    pub id: Uuid,
    pub buyer: ActorId,
    pub seller: ActorId,
    pub item: ItemDescriptor,
    pub amount_coins: u64,
    pub creator_points_earned: u64,
    pub initiated_at: DateTime<Utc>,
    pub settled_at: Option<DateTime<Utc>>,
    pub debit_account_version: Option<u64>,
    #[serde(skip)]
    _state: PhantomData<S>,
}

impl Transaction<Initiated> {
    pub fn new(
        buyer: ActorId,
        seller: ActorId,
        item: ItemDescriptor,
        creator_points_rate: u32,
    ) -> Self {
        let amount_coins = match &item.sale_mode {
            crate::offer::SaleMode::Subscription { price_coins, .. } => *price_coins,
            crate::offer::SaleMode::Instant => match item.price {
                PriceTag::Fixed(c) => c,
                PriceTag::Negotiable { min, .. } => min, // デフォルトは最小価格
                PriceTag::Free => 0,
            },
        };
        // 自己売買（ウォッシュトレード / Points Alchemy）防止
        let creator_points_earned = if buyer == seller {
            0
        } else {
            (u128::from(amount_coins) * u128::from(creator_points_rate) / 10000) as u64
        };

        Self {
            id: Uuid::new_v4(),
            buyer,
            seller,
            item,
            amount_coins,
            creator_points_earned,
            initiated_at: Utc::now(),
            settled_at: None,
            debit_account_version: None,
            _state: PhantomData,
        }
    }

    pub fn authorize(self) -> Transaction<Authorized> {
        Transaction {
            id: self.id,
            buyer: self.buyer,
            seller: self.seller,
            item: self.item,
            amount_coins: self.amount_coins,
            creator_points_earned: self.creator_points_earned,
            initiated_at: self.initiated_at,
            settled_at: None,
            debit_account_version: self.debit_account_version,
            _state: PhantomData,
        }
    }

    pub fn cancel(self) -> Transaction<Cancelled> {
        Transaction {
            id: self.id,
            buyer: self.buyer,
            seller: self.seller,
            item: self.item,
            amount_coins: self.amount_coins,
            creator_points_earned: self.creator_points_earned,
            initiated_at: self.initiated_at,
            settled_at: None,
            debit_account_version: self.debit_account_version,
            _state: PhantomData,
        }
    }
}

impl Transaction<Authorized> {
    pub fn settle(self) -> Transaction<Settled> {
        Transaction {
            id: self.id,
            buyer: self.buyer,
            seller: self.seller,
            item: self.item,
            amount_coins: self.amount_coins,
            creator_points_earned: self.creator_points_earned,
            initiated_at: self.initiated_at,
            settled_at: Some(Utc::now()),
            debit_account_version: self.debit_account_version,
            _state: PhantomData,
        }
    }

    pub fn fail(self) -> Transaction<Failed> {
        Transaction {
            id: self.id,
            buyer: self.buyer,
            seller: self.seller,
            item: self.item,
            amount_coins: self.amount_coins,
            creator_points_earned: self.creator_points_earned,
            initiated_at: self.initiated_at,
            settled_at: None,
            debit_account_version: self.debit_account_version,
            _state: PhantomData,
        }
    }

    pub fn cancel(self) -> Transaction<Cancelled> {
        Transaction {
            id: self.id,
            buyer: self.buyer,
            seller: self.seller,
            item: self.item,
            amount_coins: self.amount_coins,
            creator_points_earned: self.creator_points_earned,
            initiated_at: self.initiated_at,
            settled_at: None,
            debit_account_version: self.debit_account_version,
            _state: PhantomData,
        }
    }
}

impl Transaction<Settled> {
    pub fn refund(self) -> Transaction<Refunded> {
        Transaction {
            id: self.id,
            buyer: self.buyer,
            seller: self.seller,
            item: self.item,
            amount_coins: self.amount_coins,
            creator_points_earned: self.creator_points_earned,
            initiated_at: self.initiated_at,
            settled_at: self.settled_at,
            debit_account_version: self.debit_account_version,
            _state: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commodity::{CommodityKind, PriceTag};

    fn mock_item() -> ItemDescriptor {
        ItemDescriptor {
            id: Uuid::new_v4(),
            kind: CommodityKind::VrmAvatar,
            name: "Test Asset".to_string(),
            description: "Test description".to_string(),
            price: PriceTag::Fixed(100),
            creator_id: ActorId(Uuid::new_v4()),
            sale_mode: crate::offer::SaleMode::Instant,
            drm_enabled: false,
            created_at: Utc::now(),
            metadata: serde_json::json!({}),
            content_hash: None,
        }
    }

    #[test]
    fn test_transaction_lifecycle() {
        let buyer = ActorId(Uuid::new_v4());
        let seller = ActorId(Uuid::new_v4());
        let item = mock_item();

        // 1. Initiated
        let tx = Transaction::new(buyer, seller, item.clone(), 1000);
        assert_eq!(tx.amount_coins, 100);
        assert_eq!(tx.creator_points_earned, 10);
        assert!(tx.settled_at.is_none());

        // 2. Authorized
        let tx = tx.authorize();
        assert!(tx.settled_at.is_none());

        // 3. Settled
        let tx = tx.settle();
        assert!(tx.settled_at.is_some());

        // 4. Refunded
        let tx = tx.refund();
        assert_eq!(tx.amount_coins, 100);
    }

    #[test]
    fn test_transaction_cancel() {
        let buyer = ActorId(Uuid::new_v4());
        let seller = ActorId(Uuid::new_v4());
        let item = mock_item();

        let tx = Transaction::new(buyer, seller, item, 1000);
        let _tx_cancelled = tx.cancel();
    }
}
