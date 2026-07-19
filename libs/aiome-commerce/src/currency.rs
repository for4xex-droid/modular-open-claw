/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! u64 ↔ U256 helpers for Web3 / x402 boundaries (OP-083-D).
//!
//! Does **not** replace AiomeCoin or Nurture CoinWallet.

use aiome_core_contracts::U256;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CurrencyConversionError {
    #[error("OnChainAmount exceeds u64::MAX (would truncate)")]
    Overflow,
}

/// Canonical on-chain amount wrapper at the x402 / Web3 boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnChainAmount(U256);

impl OnChainAmount {
    pub fn new(value: U256) -> Self {
        Self(value)
    }

    pub fn from_u64(value: u64) -> Self {
        Self(U256::from(value))
    }

    pub fn as_u256(&self) -> U256 {
        self.0
    }

    /// Convert to u64. Fails if the value does not fit (no silent truncate).
    pub fn try_to_u64(&self) -> Result<u64, CurrencyConversionError> {
        if self.0 > U256::from(u64::MAX) {
            return Err(CurrencyConversionError::Overflow);
        }
        u64::try_from(self.0).map_err(|_| CurrencyConversionError::Overflow)
    }
}

impl From<u64> for OnChainAmount {
    fn from(value: u64) -> Self {
        Self::from_u64(value)
    }
}

impl From<OnChainAmount> for U256 {
    fn from(value: OnChainAmount) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_u64() {
        let amt = OnChainAmount::from_u64(42);
        assert_eq!(amt.try_to_u64().unwrap(), 42);
        assert_eq!(amt.as_u256(), U256::from(42u64));
    }

    #[test]
    fn overflow_rejected() {
        let huge = OnChainAmount::new(U256::from(u64::MAX) + U256::from(1u64));
        assert_eq!(huge.try_to_u64(), Err(CurrencyConversionError::Overflow));
    }

    #[test]
    fn max_u64_ok() {
        let amt = OnChainAmount::from_u64(u64::MAX);
        assert_eq!(amt.try_to_u64().unwrap(), u64::MAX);
    }
}
