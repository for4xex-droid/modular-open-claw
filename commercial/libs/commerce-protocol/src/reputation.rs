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

use crate::identity::ActorId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    Unverified,
    Verified,
    Trusted,
    Premium,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationScore {
    pub actor_id: ActorId,
    pub total_sales: u64,
    pub total_revenue_coins: u64,
    pub rating_sum: f64,
    pub rating_count: u64,
    pub trust_level: TrustLevel,
    pub violation_count: u32,
}

impl ReputationScore {
    /// ポリシー違反（CSAM, 不正アクセス等）が発生した際にペナルティを適用する。
    /// violation_count を加算し、閾値に応じて TrustLevel をダウングレードする。
    pub fn apply_penalty(&mut self, severity: u32) {
        self.violation_count = self.violation_count.saturating_add(severity);

        // TrustLevel のダウングレード閾値
        self.trust_level = match self.violation_count {
            0 => self.trust_level.clone(),
            1..=2 => {
                // 軽微: Premium → Trusted, それ以外は維持
                match self.trust_level {
                    TrustLevel::Premium => TrustLevel::Trusted,
                    _ => self.trust_level.clone(),
                }
            }
            3..=5 => {
                // 中程度: Trusted 以上 → Verified にダウングレード
                match self.trust_level {
                    TrustLevel::Premium | TrustLevel::Trusted => TrustLevel::Verified,
                    _ => self.trust_level.clone(),
                }
            }
            _ => {
                // 重大: 全て Unverified にダウングレード
                TrustLevel::Unverified
            }
        };
    }

    /// 現在の TrustLevel に基づく手数料率の倍率を返す。
    /// 違反が多いほど手数料が高くなる（1.0 = 通常、最大 2.0）。
    pub fn fee_multiplier(&self) -> f64 {
        match self.trust_level {
            TrustLevel::Premium => 0.8,
            TrustLevel::Trusted => 1.0,
            TrustLevel::Verified => 1.3,
            TrustLevel::Unverified => 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_reputation(trust: TrustLevel) -> ReputationScore {
        ReputationScore {
            actor_id: ActorId(Uuid::new_v4()),
            total_sales: 0,
            total_revenue_coins: 0,
            rating_sum: 0.0,
            rating_count: 0,
            trust_level: trust,
            violation_count: 0,
        }
    }

    #[test]
    fn test_penalty_downgrades_premium() {
        let mut rep = test_reputation(TrustLevel::Premium);
        rep.apply_penalty(1);
        assert_eq!(rep.trust_level, TrustLevel::Trusted);
    }

    #[test]
    fn test_penalty_severe_downgrades_to_unverified() {
        let mut rep = test_reputation(TrustLevel::Premium);
        rep.apply_penalty(6);
        assert_eq!(rep.trust_level, TrustLevel::Unverified);
    }

    #[test]
    fn test_fee_multiplier() {
        let rep = test_reputation(TrustLevel::Unverified);
        assert_eq!(rep.fee_multiplier(), 2.0);
    }
}
