/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! Agent-side signing wallet for x402 (OP-083-C).
//!
//! Distinct from Nurture `CoinWallet` (fiat/points ledger). This type only
//! holds an EVM signing key for payment proofs — it does not broadcast txs.

use alloy_primitives::Address;
use alloy_signer::SignerSync;
use alloy_signer_local::PrivateKeySigner;
use anyhow::{Context, Result};
use std::str::FromStr;

/// Signing wallet for autonomous x402 negotiation (Layer: Web3 rails).
pub struct AgentWallet {
    signer: PrivateKeySigner,
}

impl AgentWallet {
    /// Build from a hex-encoded secp256k1 private key (`0x` prefix optional).
    pub fn from_hex_key(private_key_hex: &str) -> Result<Self> {
        let signer = PrivateKeySigner::from_str(private_key_hex.trim())
            .context("Invalid X402 signer private key format")?;
        Ok(Self { signer })
    }

    /// Resolve signer: env `X402_SIGNER_KEY` → macOS Keychain → debug mock / release fail-closed.
    ///
    /// When the key is taken from the process environment, it is scrubbed immediately
    /// after copy (same Zero-Trust pattern as `JWT_PRIVATE_KEY_B64`).
    pub fn resolve_from_env_or_keychain() -> Result<Self> {
        if let Ok(key) = std::env::var("X402_SIGNER_KEY") {
            let key = key.trim().to_string();
            shared::security::scrub_env("X402_SIGNER_KEY");
            if !key.is_empty() {
                return Self::from_hex_key(&key);
            }
        }

        if let Some(key) = shared::security::get_keychain_secret("X402_SIGNER_KEY") {
            let key = key.trim().to_string();
            if !key.is_empty() {
                return Self::from_hex_key(&key);
            }
        }

        #[cfg(any(test, debug_assertions))]
        {
            tracing::warn!(
                "⚠️ [X402] X402_SIGNER_KEY unset; using debug mock signer (not for production)"
            );
            Self::from_hex_key("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
        }

        #[cfg(not(any(test, debug_assertions)))]
        {
            anyhow::bail!(
                "X402_SIGNER_KEY must be set via Vault/env or macOS Keychain in production (fail-closed)"
            )
        }
    }

    pub fn address(&self) -> Address {
        self.signer.address()
    }

    /// EIP-191 personal_sign over `message`. Returns `0x`-prefixed hex signature.
    /// Does **not** submit any transaction (OP-083-C: broadcast out of scope).
    pub fn sign_message_hex(&self, message: &[u8]) -> Result<String> {
        let sig = self
            .signer
            .sign_message_sync(message)
            .context("Failed to sign x402 payment proof")?;
        Ok(format!("0x{}", hex::encode(sig.as_bytes())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    const MOCK_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn agent_wallet_signs_without_mock_prefix() {
        let wallet = AgentWallet::from_hex_key(MOCK_KEY).unwrap();
        let sig = wallet.sign_message_hex(b"x402-test").unwrap();
        assert!(sig.starts_with("0x"));
        assert!(!sig.contains("mock_tx"));
        assert!(sig.len() > 10);
    }

    #[test]
    fn invalid_hex_key_rejected() {
        match AgentWallet::from_hex_key("not-a-key") {
            Err(e) => assert!(e.to_string().contains("Invalid X402 signer"), "got: {}", e),
            Ok(_) => panic!("expected invalid key rejection"),
        }
    }

    #[test]
    #[serial]
    fn resolve_scrubs_env_key() {
        let prev = std::env::var("X402_SIGNER_KEY").ok();
        std::env::set_var("X402_SIGNER_KEY", MOCK_KEY);
        let wallet = AgentWallet::resolve_from_env_or_keychain().unwrap();
        assert!(std::env::var("X402_SIGNER_KEY").is_err());
        assert_eq!(
            wallet.address(),
            AgentWallet::from_hex_key(MOCK_KEY).unwrap().address()
        );
        if let Some(v) = prev {
            std::env::set_var("X402_SIGNER_KEY", v);
        }
    }
}
