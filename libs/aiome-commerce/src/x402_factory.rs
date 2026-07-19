/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! Factory for `X402Client` — intentionally separate from `CommerceEngineFactory`
//! (OP-083-C: do not mix ProviderType::X402 into fiat factory).

use crate::wallet::AgentWallet;
use crate::x402::{resolve_x402_network, X402Client};
use aiome_core_contracts::{X402Negotiator, U256};
use anyhow::{Context, Result};
use std::sync::Arc;

/// Builds an `Arc<dyn X402Negotiator>` from environment / Keychain.
pub struct X402ClientFactory;

impl X402ClientFactory {
    /// Create from env:
    /// - `X402_RPC_URL` (required, fail-closed if empty)
    /// - `X402_NETWORK` via [`resolve_x402_network`]
    /// - signer via [`AgentWallet::resolve_from_env_or_keychain`]
    pub fn create_from_env(budget_cap: U256) -> Result<Arc<dyn X402Negotiator>> {
        let rpc_url = std::env::var("X402_RPC_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .context("X402_RPC_URL must be set (Q2)")?;

        let wallet = AgentWallet::resolve_from_env_or_keychain()?;
        let client = X402Client::from_wallet(rpc_url, budget_cap, resolve_x402_network(), wallet)
            .context("Failed to construct X402Client")?;
        Ok(Arc::new(client))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn factory_fail_closed_without_rpc_url() {
        let prev = std::env::var("X402_RPC_URL").ok();
        std::env::remove_var("X402_RPC_URL");
        match X402ClientFactory::create_from_env(U256::from(1u64)) {
            Err(e) => assert!(
                e.to_string().contains("X402_RPC_URL"),
                "expected RPC fail-closed, got: {}",
                e
            ),
            Ok(_) => panic!("expected fail-closed without X402_RPC_URL"),
        }
        match prev {
            Some(v) => std::env::set_var("X402_RPC_URL", v),
            None => std::env::remove_var("X402_RPC_URL"),
        }
    }

    #[test]
    #[serial]
    fn factory_builds_with_rpc_and_debug_signer() {
        let prev_rpc = std::env::var("X402_RPC_URL").ok();
        let prev_key = std::env::var("X402_SIGNER_KEY").ok();
        std::env::set_var("X402_RPC_URL", "https://sepolia.base.org");
        std::env::remove_var("X402_SIGNER_KEY");
        let client = X402ClientFactory::create_from_env(U256::from(1_000_000u64));
        assert!(
            client.is_ok(),
            "debug signer path must build: {:?}",
            client.err()
        );
        match prev_rpc {
            Some(v) => std::env::set_var("X402_RPC_URL", v),
            None => std::env::remove_var("X402_RPC_URL"),
        }
        match prev_key {
            Some(v) => std::env::set_var("X402_SIGNER_KEY", v),
            None => std::env::remove_var("X402_SIGNER_KEY"),
        }
    }
}
