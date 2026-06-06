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
use commerce_protocol::identity::ActorId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorPoints {
    pub balance: u64,
    pub lifetime_earned: u64,
    pub lifetime_withdrawn: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsAccount {
    pub creator: ActorId,
    pub points: CreatorPoints,
    pub conversion_rate: u32,
}

impl PointsAccount {
    pub fn earn(&mut self, amount: u64) -> Result<(), NurtureError> {
        self.points.balance =
            self.points
                .balance
                .checked_add(amount)
                .ok_or_else(|| NurtureError::Ledger {
                    reason: "Points balance overflow".to_string(),
                })?;
        self.points.lifetime_earned =
            self.points
                .lifetime_earned
                .checked_add(amount)
                .ok_or_else(|| NurtureError::Ledger {
                    reason: "Lifetime earned overflow".to_string(),
                })?;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: u64) -> Result<(), NurtureError> {
        if self.points.balance < amount {
            return Err(NurtureError::InsufficientBalance {
                required: amount,
                available: self.points.balance,
            });
        }

        self.points.balance -= amount;
        self.points.lifetime_withdrawn = self
            .points
            .lifetime_withdrawn
            .checked_add(amount)
            .ok_or_else(|| NurtureError::Ledger {
                reason: "Lifetime withdrawn overflow".to_string(),
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_account(balance: u64) -> PointsAccount {
        PointsAccount {
            creator: ActorId(Uuid::new_v4()),
            points: CreatorPoints {
                balance,
                lifetime_earned: balance,
                lifetime_withdrawn: 0,
            },
            conversion_rate: 10000,
        }
    }

    #[test]
    fn test_earn() {
        let mut account = test_account(100);
        account.earn(50).unwrap();
        assert_eq!(account.points.balance, 150);
        assert_eq!(account.points.lifetime_earned, 150);
    }

    #[test]
    fn test_withdraw_success() {
        let mut account = test_account(100);
        account.withdraw(40).unwrap();
        assert_eq!(account.points.balance, 60);
        assert_eq!(account.points.lifetime_withdrawn, 40);
    }

    #[test]
    fn test_withdraw_insufficient() {
        let mut account = test_account(100);
        let result = account.withdraw(150);
        assert!(matches!(
            result,
            Err(NurtureError::InsufficientBalance { .. })
        ));
    }

    #[test]
    fn test_earn_overflow() {
        let mut account = test_account(u64::MAX);
        let result = account.earn(1);
        assert!(result.is_err());
    }

    #[test]
    fn test_withdraw_overflow_protection() {
        let mut account = PointsAccount {
            creator: ActorId(Uuid::new_v4()),
            points: CreatorPoints {
                balance: 100,
                lifetime_earned: 100,
                lifetime_withdrawn: u64::MAX,
            },
            conversion_rate: 10000,
        };
        let result = account.withdraw(1);
        assert!(result.is_err());
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_earn_preserves_total(initial_balance in 0u64..1000000, earn_amount in 0u64..1000000) {
            let mut account = test_account(initial_balance);
            let prev_balance = account.points.balance;
            let expected_lifetime_earned = account.points.lifetime_earned + earn_amount;

            assert!(account.earn(earn_amount).is_ok());

            prop_assert_eq!(account.points.balance, prev_balance + earn_amount);
            prop_assert_eq!(account.points.lifetime_earned, expected_lifetime_earned);
        }

        #[test]
        fn proptest_withdraw_preserves_total(
            initial_balance in 1u64..1000000,
            withdraw_amount in 1u64..1000000
        ) {
            let mut account = test_account(initial_balance);
            let prev_balance = account.points.balance;
            let prev_withdrawn = account.points.lifetime_withdrawn;

            if withdraw_amount <= initial_balance {
                assert!(account.withdraw(withdraw_amount).is_ok());
                // 合計保存則
                prop_assert_eq!(
                    account.points.balance + account.points.lifetime_withdrawn,
                    prev_balance + prev_withdrawn
                );
            } else {
                assert!(account.withdraw(withdraw_amount).is_err());
            }
        }
    }
}
