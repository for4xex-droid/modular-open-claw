/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

//! Surprise/Easter Egg logic for AI interactions.
//! A2C (AI to Commercial) easter eggs.

use rand::RngCore;

pub struct SurpriseEngine;

impl SurpriseEngine {
    /// Evaluates whether a transaction triggers a surprise bonus.
    /// `transaction_amount`: The size of the purchase in coins.
    /// `daily_bonus_issued`: Total surprise bonus coins already issued globally today.
    /// `max_daily_bonus`: The absolute limit of bonus coins allowed per day.
    /// `rng`: Cryptographically secure RNG.
    pub fn evaluate_bonus<R: RngCore>(
        transaction_amount: u64,
        daily_bonus_issued: u64,
        max_daily_bonus: u64,
        rng: &mut R,
    ) -> Option<u64> {
        if daily_bonus_issued >= max_daily_bonus {
            return None;
        }

        // Anti-spam: bonus is roughly 5-10% of transaction amount.
        // Transactions < 20 coins mathematically cannot yield a bonus > 0.
        let base_bonus = transaction_amount / 20;
        if base_bonus == 0 {
            return None;
        }

        // Scaling probability: 10% chance per 1000 coins, max 50% chance.
        let win_threshold = transaction_amount.min(5000);
        let roll = rng.next_u64() % 10000;

        if roll < win_threshold {
            let extra_roll = if base_bonus > 0 {
                rng.next_u64() % base_bonus
            } else {
                0
            };
            let mut actual_bonus = base_bonus + extra_roll;

            // Global Circuit Breaker Application
            if daily_bonus_issued + actual_bonus > max_daily_bonus {
                actual_bonus = max_daily_bonus.saturating_sub(daily_bonus_issued);
            }

            if actual_bonus > 0 {
                return Some(actual_bonus);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::mock::StepRng;

    #[test]
    fn test_global_circuit_breaker() {
        let mut rng = StepRng::new(2, 1);
        // If daily issued is already at or above max, MUST return None.
        let result = SurpriseEngine::evaluate_bonus(1000, 10000, 10000, &mut rng);
        assert_eq!(result, None);
    }

    #[test]
    fn test_anti_spam_micro_transactions() {
        let mut rng = StepRng::new(0, 0); // Always returns 0 (best possible roll)
                                          // 1 coin transaction should yield 0 or None bonus because it's too small to cross the spam threshold,
                                          // or the bonus is 0 coins mathematically.
        let result = SurpriseEngine::evaluate_bonus(1, 0, 10000, &mut rng);
        // Even with the luckiest roll, a 1 coin transaction cannot farm significant bonuses.
        assert!(result.unwrap_or(0) == 0);
    }

    #[test]
    fn test_valid_bonus_dropped() {
        // RNG that rolls a specific sequence to trigger a win
        let mut rng = StepRng::new(0, 0); // Guaranteed to hit the probability threshold if amount is high
        let result = SurpriseEngine::evaluate_bonus(5000, 5000, 10000, &mut rng);
        assert!(result.is_some());
        let bonus = result.unwrap();
        assert!(bonus > 0);
        // Bonus should not cause daily_bonus_issued to exceed max_daily_bonus
        assert!(5000 + bonus <= 10000);
    }

    #[test]
    fn test_bonus_capped_by_circuit_breaker() {
        let mut rng = StepRng::new(0, 0);
        // We have 9900 issued, max is 10000. So we only have 100 coins left in the global pool.
        // Even if the transaction is huge, the bonus cannot exceed 100.
        let result = SurpriseEngine::evaluate_bonus(50000, 9900, 10000, &mut rng);
        assert!(result.is_some());
        assert!(result.unwrap() <= 100);
    }
}
