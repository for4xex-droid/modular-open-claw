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

use chrono::{DateTime, Utc};
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiomeCoin {
    pub balance: u64,
    pub lifetime_charged: u64,
    pub lifetime_spent: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinWallet {
    pub owner: ActorId,
    pub coin: AiomeCoin,
    pub daily_limit: u64,
    pub spent_today: u64,
    pub monthly_limit: u64,
    pub spent_this_month: u64,
    pub last_reset: DateTime<Utc>,
    pub last_transaction_at: Option<DateTime<Utc>>,
    pub version: u64,
}

impl CoinWallet {
    pub fn charge(&mut self, amount: u64) -> Result<(), NurtureError> {
        self.coin.balance =
            self.coin
                .balance
                .checked_add(amount)
                .ok_or_else(|| NurtureError::Ledger {
                    reason: "Coin balance overflow".to_string(),
                })?;
        self.coin.lifetime_charged =
            self.coin
                .lifetime_charged
                .checked_add(amount)
                .ok_or_else(|| NurtureError::Ledger {
                    reason: "Lifetime charged overflow".to_string(),
                })?;
        Ok(())
    }

    pub fn spend(&mut self, amount: u64) -> Result<(), NurtureError> {
        self.check_daily_limit(amount)?;
        self.check_monthly_limit(amount)?;

        if self.coin.balance < amount {
            return Err(NurtureError::InsufficientBalance {
                required: amount,
                available: self.coin.balance,
            });
        }

        self.coin.balance -= amount;
        self.coin.lifetime_spent =
            self.coin
                .lifetime_spent
                .checked_add(amount)
                .ok_or_else(|| NurtureError::Ledger {
                    reason: "Lifetime spent overflow".to_string(),
                })?;
        self.spent_today =
            self.spent_today
                .checked_add(amount)
                .ok_or_else(|| NurtureError::Ledger {
                    reason: "Spent today overflow".to_string(),
                })?;
        self.spent_this_month =
            self.spent_this_month
                .checked_add(amount)
                .ok_or_else(|| NurtureError::Ledger {
                    reason: "Spent this month overflow".to_string(),
                })?;
        Ok(())
    }

    pub fn check_monthly_limit(&self, amount: u64) -> Result<(), NurtureError> {
        if self.monthly_limit == 0 {
            return Ok(());
        }
        let new_spent = self.spent_this_month.saturating_add(amount);
        if new_spent > self.monthly_limit {
            return Err(NurtureError::MonthlyLimitExceeded {
                limit: self.monthly_limit,
                current: new_spent,
            });
        }
        Ok(())
    }

    pub fn check_daily_limit(&self, amount: u64) -> Result<(), NurtureError> {
        let new_spent_today = self.spent_today.saturating_add(amount);
        if new_spent_today > self.daily_limit {
            return Err(NurtureError::DailyLimitExceeded {
                limit: self.daily_limit,
                current: new_spent_today,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_wallet(balance: u64, limit: u64) -> CoinWallet {
        CoinWallet {
            owner: ActorId(Uuid::new_v4()),
            coin: AiomeCoin {
                balance,
                lifetime_charged: balance,
                lifetime_spent: 0,
            },
            daily_limit: limit,
            spent_today: 0,
            monthly_limit: limit,
            spent_this_month: 0,
            last_reset: Utc::now(),
            last_transaction_at: None,
            version: 0,
        }
    }

    #[test]
    fn test_charge() {
        let mut wallet = test_wallet(100, 1000);
        wallet.charge(50).unwrap();
        assert_eq!(wallet.coin.balance, 150);
        assert_eq!(wallet.coin.lifetime_charged, 150);
    }

    #[test]
    fn test_spend_success() {
        let mut wallet = test_wallet(100, 1000);
        wallet.spend(40).unwrap();
        assert_eq!(wallet.coin.balance, 60);
        assert_eq!(wallet.coin.lifetime_spent, 40);
        assert_eq!(wallet.spent_today, 40);
    }

    #[test]
    fn test_spend_insufficient() {
        let mut wallet = test_wallet(100, 1000);
        let result = wallet.spend(150);
        assert!(matches!(
            result,
            Err(NurtureError::InsufficientBalance { .. })
        ));
    }

    #[test]
    fn test_monthly_limit() {
        let mut wallet = CoinWallet {
            owner: ActorId(Uuid::new_v4()),
            coin: AiomeCoin {
                balance: 1000,
                lifetime_charged: 1000,
                lifetime_spent: 0,
            },
            daily_limit: 10_000,
            spent_today: 0,
            monthly_limit: 100,
            spent_this_month: 0,
            last_reset: Utc::now(),
            last_transaction_at: None,
            version: 0,
        };
        wallet.spend(60).unwrap();
        let result = wallet.spend(50);
        assert!(matches!(
            result,
            Err(NurtureError::MonthlyLimitExceeded { .. })
        ));
    }

    #[test]
    fn test_daily_limit() {
        let mut wallet = test_wallet(1000, 100);
        wallet.spend(60).unwrap();
        let result = wallet.spend(50);
        assert!(matches!(
            result,
            Err(NurtureError::DailyLimitExceeded { .. })
        ));
    }

    #[test]
    fn test_charge_overflow() {
        let mut wallet = test_wallet(u64::MAX, 1000);
        let result = wallet.charge(1);
        assert!(result.is_err());
    }

    #[test]
    fn test_spend_overflow_protection() {
        let mut wallet = CoinWallet {
            owner: ActorId(Uuid::new_v4()),
            coin: AiomeCoin {
                balance: 100,
                lifetime_charged: 100,
                lifetime_spent: u64::MAX,
            },
            daily_limit: 10000,
            spent_today: 0,
            monthly_limit: 0,
            spent_this_month: 0,
            last_reset: Utc::now(),
            last_transaction_at: None,
            version: 0,
        };
        let result = wallet.spend(1);
        assert!(result.is_err());
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_charge_preserves_total(initial_balance in 0u64..1000000, charge_amount in 0u64..1000000) {
            let mut wallet = test_wallet(initial_balance, 5000000);
            let prev_total = wallet.coin.balance;
            let expected_lifetime_charged = wallet.coin.lifetime_charged + charge_amount;

            assert!(wallet.charge(charge_amount).is_ok());

            // 合計保存則: (残高変化 = charge_amount) AND (lifetime_charged は増加分と一致)
            prop_assert_eq!(wallet.coin.balance, prev_total + charge_amount);
            prop_assert_eq!(wallet.coin.lifetime_charged, expected_lifetime_charged);
        }

        #[test]
        fn proptest_spend_preserves_total(
            initial_balance in 1u64..1000000,
            spend_amount in 1u64..1000000
        ) {
            let mut wallet = test_wallet(initial_balance, initial_balance + 2000000);
            let prev_balance = wallet.coin.balance;
            let prev_spent = wallet.coin.lifetime_spent;

            if spend_amount <= initial_balance {
                assert!(wallet.spend(spend_amount).is_ok());
                // 合計保存則: (減った残高部分) + (増えた消費履歴) == 以前の残高 + 以前の消費履歴
                prop_assert_eq!(wallet.coin.balance + wallet.coin.lifetime_spent, prev_balance + prev_spent);
                prop_assert_eq!(wallet.spent_today, spend_amount);
            } else {
                assert!(wallet.spend(spend_amount).is_err());
            }
        }
    }
}
