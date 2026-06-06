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

use commerce_protocol::error::NurtureError;
use commerce_protocol::transaction::{Transaction, TxState};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct EconomyPolicy {
    pub daily_spend_limit: u64,
    pub max_single_purchase: u64,
    pub creator_points_rate: u32,
    pub system_fee_rate: u32,
    pub agency_fee_rate: u32, // B2B AIAA agency fee rate (preferential)
    pub burn_rate: u32,
    pub min_item_price: u64,
    pub max_item_price: u64,
    pub min_transaction_interval_ms: u64,
}

pub type SharedPolicy = std::sync::Arc<tokio::sync::RwLock<EconomyPolicy>>;

impl Default for EconomyPolicy {
    fn default() -> Self {
        Self {
            daily_spend_limit: 10_000,
            max_single_purchase: 5_000,
            creator_points_rate: 7000, // 70%
            system_fee_rate: 3000,     // 30%
            agency_fee_rate: 1000,     // 10% (for B2B AIAA packages)
            burn_rate: 500,            // 5% は永久に焼却 (デフレ圧力)
            min_item_price: 10,
            max_item_price: 100_000,
            min_transaction_interval_ms: 1000, // 高頻度取引防止 (1秒間隔)
        }
    }
}

impl EconomyPolicy {
    /// bps レートの不変条件を検証する。各レートは 10000 (100%) 以下でなければならない。
    pub fn validate(&self) -> Result<(), NurtureError> {
        const MAX_BPS: u32 = 10000;
        if self.burn_rate > MAX_BPS {
            return Err(NurtureError::PolicyViolation(format!(
                "burn_rate {} が上限 {} bps を超過しています",
                self.burn_rate, MAX_BPS
            )));
        }
        if self.system_fee_rate > MAX_BPS {
            return Err(NurtureError::PolicyViolation(format!(
                "system_fee_rate {} が上限 {} bps を超過しています",
                self.system_fee_rate, MAX_BPS
            )));
        }
        if self.creator_points_rate > MAX_BPS {
            return Err(NurtureError::PolicyViolation(format!(
                "creator_points_rate {} が上限 {} bps を超過しています",
                self.creator_points_rate, MAX_BPS
            )));
        }

        // 合算上限の検証 (burn_rate + system_fee_rate)
        let combined_platform_rate = self.burn_rate.saturating_add(self.system_fee_rate);
        if combined_platform_rate > MAX_BPS {
            return Err(NurtureError::PolicyViolation(format!(
                "burn_rate と system_fee_rate の合算 {} が上限 {} bps を超過しています",
                combined_platform_rate, MAX_BPS
            )));
        }

        if self.agency_fee_rate > MAX_BPS {
            return Err(NurtureError::PolicyViolation(format!(
                "agency_fee_rate {} が上限 {} bps を超過しています",
                self.agency_fee_rate, MAX_BPS
            )));
        }

        // 合算上限の検証 (burn_rate + agency_fee_rate)
        let combined_agency_platform_rate = self.burn_rate.saturating_add(self.agency_fee_rate);
        if combined_agency_platform_rate > MAX_BPS {
            return Err(NurtureError::PolicyViolation(format!(
                "burn_rate と agency_fee_rate の合算 {} が上限 {} bps を超過しています",
                combined_agency_platform_rate, MAX_BPS
            )));
        }

        Ok(())
    }
}

pub fn validate_transaction<S: TxState>(
    policy: &EconomyPolicy,
    tx: &Transaction<S>,
) -> Result<(), NurtureError> {
    // ポリシー自体の不変条件を先に検証 (bps レート上限チェック)
    policy.validate()?;

    // 無料アイテム (0コイン) はポリシーバイパス (プロモーション・初回特典等)
    if tx.amount_coins > 0 && tx.amount_coins < policy.min_item_price {
        return Err(NurtureError::PolicyViolation(format!(
            "取引額が最低価格 {} を下回っています",
            policy.min_item_price
        )));
    }
    if tx.amount_coins > policy.max_item_price {
        return Err(NurtureError::PolicyViolation(format!(
            "取引額が最大価格 {} を超過しています",
            policy.max_item_price
        )));
    }
    if tx.amount_coins > policy.max_single_purchase {
        return Err(NurtureError::PolicyViolation(format!(
            "一回の購入上限 {} を超えています",
            policy.max_single_purchase
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use commerce_protocol::commodity::{CommodityKind, ItemDescriptor, PriceTag};
    use commerce_protocol::identity::ActorId;
    use commerce_protocol::offer::SaleMode;
    use uuid::Uuid;

    fn test_item(price: u64) -> ItemDescriptor {
        ItemDescriptor {
            id: Uuid::new_v4(),
            kind: CommodityKind::VrmAvatar,
            name: "Test Item".to_string(),
            description: "Test".to_string(),
            price: PriceTag::Fixed(price),
            creator_id: ActorId(Uuid::new_v4()),
            sale_mode: SaleMode::Instant,
            drm_enabled: false,
            created_at: Utc::now(),
            metadata: serde_json::Value::Null,
            content_hash: None,
        }
    }

    #[test]
    fn test_validate_success() {
        let policy = EconomyPolicy::default();
        let item = test_item(100);
        let tx = Transaction::new(
            ActorId(Uuid::new_v4()),
            ActorId(Uuid::new_v4()),
            item,
            policy.creator_points_rate,
        );
        assert!(validate_transaction(&policy, &tx).is_ok());
    }

    #[test]
    fn test_validate_min_price() {
        let policy = EconomyPolicy::default();
        let item = test_item(5);
        let tx = Transaction::new(
            ActorId(Uuid::new_v4()),
            ActorId(Uuid::new_v4()),
            item,
            policy.creator_points_rate,
        );
        let result = validate_transaction(&policy, &tx);
        assert!(matches!(result, Err(NurtureError::PolicyViolation(_))));
    }

    #[test]
    fn test_validate_max_price() {
        let policy = EconomyPolicy {
            max_item_price: 1000,
            ..EconomyPolicy::default()
        };
        let item = test_item(2000);
        let tx = Transaction::new(
            ActorId(Uuid::new_v4()),
            ActorId(Uuid::new_v4()),
            item,
            policy.creator_points_rate,
        );
        let result = validate_transaction(&policy, &tx);
        assert!(matches!(result, Err(NurtureError::PolicyViolation(_))));
    }

    #[test]
    fn test_validate_single_purchase_limit() {
        let policy = EconomyPolicy {
            max_single_purchase: 500,
            ..EconomyPolicy::default()
        };
        let item = test_item(600);
        let tx = Transaction::new(
            ActorId(Uuid::new_v4()),
            ActorId(Uuid::new_v4()),
            item,
            policy.creator_points_rate,
        );
        let result = validate_transaction(&policy, &tx);
        assert!(matches!(result, Err(NurtureError::PolicyViolation(_))));
    }

    #[test]
    fn test_policy_validate_bps_boundaries() {
        // 正常系: 境界値ぴったり (10000 bps)
        let policy = EconomyPolicy {
            burn_rate: 5000,
            system_fee_rate: 5000,
            creator_points_rate: 10000,
            ..EconomyPolicy::default()
        };
        assert!(policy.validate().is_ok(), "100% rate should be valid");

        // 異常系: 上限突破 (10001 bps)
        let mut invalid_policy = policy.clone();
        invalid_policy.burn_rate = 10001;
        assert!(
            invalid_policy.validate().is_err(),
            "burn_rate > 10000 should fail"
        );

        let mut invalid_policy2 = policy.clone();
        invalid_policy2.system_fee_rate = 10001;
        assert!(
            invalid_policy2.validate().is_err(),
            "system_fee_rate > 10000 should fail"
        );

        let mut invalid_policy3 = policy.clone();
        invalid_policy3.creator_points_rate = 10001;
        assert!(
            invalid_policy3.validate().is_err(),
            "creator_points_rate > 10000 should fail"
        );

        // 異常系: 合算上限突破 (burn + fee > 10000 bps)
        let mut invalid_policy4 = policy.clone();
        invalid_policy4.burn_rate = 6000;
        invalid_policy4.system_fee_rate = 5000;
        assert!(
            invalid_policy4.validate().is_err(),
            "burn_rate + system_fee_rate > 10000 should fail"
        );

        let mut invalid_policy5 = policy.clone();
        invalid_policy5.agency_fee_rate = 10001;
        assert!(
            invalid_policy5.validate().is_err(),
            "agency_fee_rate > 10000 should fail"
        );

        let mut invalid_policy6 = policy;
        invalid_policy6.burn_rate = 6000;
        invalid_policy6.agency_fee_rate = 5000;
        assert!(
            invalid_policy6.validate().is_err(),
            "burn_rate + agency_fee_rate > 10000 should fail"
        );
    }
}
