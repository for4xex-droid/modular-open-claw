/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::currency::OnChainAmount;
use crate::wallet::AgentWallet;
use aiome_core_contracts::{PaymentProof, X402Negotiator, U256};
use alloy_primitives::Address;
use anyhow::Result;
use async_trait::async_trait;
use std::str::FromStr;
use thiserror::Error;

/// Q2 default network (see `docs/roadmaps/op083_cd_x402_plan.md`).
pub const X402_DEFAULT_NETWORK: &str = "base-sepolia";

/// Resolve `X402_NETWORK` env or default (shared by Client::new and Factory).
/// Validity is enforced by [`validate_x402_network`] in constructors (Q2 pin).
pub fn resolve_x402_network() -> String {
    std::env::var("X402_NETWORK")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| X402_DEFAULT_NETWORK.to_string())
}

/// OP-083 Q2: only `base-sepolia` until Human documents another network.
pub fn validate_x402_network(network: &str) -> Result<()> {
    let network = network.trim();
    if network.is_empty() {
        anyhow::bail!("X402_NETWORK / network must be non-empty");
    }
    if !network.eq_ignore_ascii_case(X402_DEFAULT_NETWORK) {
        anyhow::bail!(
            "X402_NETWORK must be '{}' for OP-083 Q2 (mainnet/other chains out of scope)",
            X402_DEFAULT_NETWORK
        );
    }
    Ok(())
}

/// Fail-closed RPC URL: non-empty http(s) only (no file/data/javascript schemes).
fn validate_rpc_url(rpc_url: &str) -> Result<String> {
    let rpc_url = rpc_url.trim();
    if rpc_url.is_empty() {
        anyhow::bail!("X402_RPC_URL / rpc_url must be non-empty (Q2)");
    }
    let parsed = url::Url::parse(rpc_url)
        .map_err(|_| anyhow::anyhow!("X402_RPC_URL / rpc_url must be a valid http(s) URL (Q2)"))?;
    match parsed.scheme() {
        "http" | "https" => Ok(rpc_url.to_string()),
        other => anyhow::bail!(
            "X402_RPC_URL scheme must be http or https (got '{}')",
            other
        ),
    }
}

#[derive(Debug, Error)]
pub enum X402Error {
    #[error("Budget exhausted. Requested: {requested}, Remaining: {remaining}")]
    BudgetExhausted { requested: U256, remaining: U256 },
    #[error("Failed to parse payment required response")]
    InvalidPaymentResponse,
    #[error("No valid payment headers found in 402 response")]
    MissingHeaders,
    #[error("Payment proof signing failed")]
    SigningFailed,
}

pub struct X402Client {
    /// Retained for future RPC balance queries; C does not broadcast.
    #[allow(dead_code)]
    rpc_url: String,
    budget_cap: U256,
    network: String,
    wallet: AgentWallet,
}

impl X402Client {
    /// Construct with an explicit wallet. Q2: http(s) RPC + `base-sepolia` only.
    pub fn from_wallet(
        rpc_url: String,
        budget_cap: U256,
        network: String,
        wallet: AgentWallet,
    ) -> Result<Self> {
        let rpc_url = validate_rpc_url(&rpc_url)?;
        validate_x402_network(&network)?;
        Ok(Self {
            rpc_url,
            budget_cap,
            network: X402_DEFAULT_NETWORK.to_string(),
            wallet,
        })
    }

    /// Convenience constructor used by legacy call sites / tests.
    pub fn new(rpc_url: String, budget_cap: U256) -> Result<Self> {
        let wallet = AgentWallet::resolve_from_env_or_keychain()?;
        Self::from_wallet(rpc_url, budget_cap, resolve_x402_network(), wallet)
    }

    fn configured_network(&self) -> &str {
        &self.network
    }
}

fn parse_payment_price(price_str: &str) -> Result<OnChainAmount, X402Error> {
    let price_str = price_str.trim();
    if price_str.is_empty() {
        return Err(X402Error::InvalidPaymentResponse);
    }
    let price =
        U256::from_str_radix(price_str, 10).map_err(|_| X402Error::InvalidPaymentResponse)?;
    Ok(OnChainAmount::new(price))
}

fn parse_payment_recipient(raw: &str) -> Result<Address, X402Error> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(X402Error::MissingHeaders);
    }
    let addr = Address::from_str(raw).map_err(|_| X402Error::InvalidPaymentResponse)?;
    if addr == Address::ZERO {
        return Err(X402Error::InvalidPaymentResponse);
    }
    Ok(addr)
}

fn assert_network_allowed(header_network: &str, configured: &str) -> Result<(), X402Error> {
    let header_network = header_network.trim();
    if header_network.is_empty() {
        return Err(X402Error::MissingHeaders);
    }
    if !header_network.eq_ignore_ascii_case(configured.trim()) {
        return Err(X402Error::InvalidPaymentResponse);
    }
    Ok(())
}

#[async_trait]
impl X402Negotiator for X402Client {
    async fn negotiate(
        &self,
        response: &reqwest::Response,
    ) -> Result<PaymentProof, aiome_core_contracts::error::AiomeError> {
        let headers = response.headers();

        let price_str = headers
            .get("X-Payment-Price")
            .and_then(|h| h.to_str().ok())
            .ok_or(X402Error::MissingHeaders)?;
        let amount = parse_payment_price(price_str)?;
        let price = amount.as_u256();

        if price > self.budget_cap {
            return Err(X402Error::BudgetExhausted {
                requested: price,
                remaining: self.budget_cap,
            }
            .into());
        }

        let recipient_raw = headers
            .get("X-Payment-Recipient")
            .and_then(|h| h.to_str().ok())
            .ok_or(X402Error::MissingHeaders)?;
        let recipient = parse_payment_recipient(recipient_raw)?;

        let network_raw = headers
            .get("X-Payment-Network")
            .and_then(|h| h.to_str().ok())
            .ok_or(X402Error::MissingHeaders)?;
        assert_network_allowed(network_raw, self.configured_network())?;

        // OP-083-C: real EIP-191 signature over payment intent. No RPC broadcast.
        let payload = format!(
            "x402-payment-v1|{}|{}|{}|{}",
            self.configured_network(),
            recipient,
            price,
            self.wallet.address()
        );
        let sig_hex = self
            .wallet
            .sign_message_hex(payload.as_bytes())
            .map_err(|e| {
                tracing::error!(error = %e, "x402 negotiate signing failed");
                X402Error::SigningFailed
            })?;

        if sig_hex.contains("mock_tx") {
            return Err(X402Error::SigningFailed.into());
        }

        // Field name is historical (`transaction_hash`); OP-083-C stores EIP-191
        // signature hex here. No on-chain broadcast / no real tx hash.
        Ok(PaymentProof {
            transaction_hash: sig_hex,
        })
    }

    async fn balance(&self) -> Result<U256, aiome_core_contracts::error::AiomeError> {
        // C: do not claim on-chain balance; expose configured budget cap only.
        Ok(self.budget_cap)
    }
}

impl From<X402Error> for aiome_core_contracts::error::AiomeError {
    fn from(err: X402Error) -> Self {
        // OP-051 / CWE-209: keep client-facing reasons opaque; details stay in logs.
        match &err {
            X402Error::BudgetExhausted { .. } => {
                tracing::warn!(error = %err, "x402 budget exhausted");
                Self::Infrastructure {
                    reason: "Payment budget exhausted".to_string(),
                }
            }
            X402Error::MissingHeaders | X402Error::InvalidPaymentResponse => Self::Validation {
                reason: "Invalid payment required response".to_string(),
            },
            X402Error::SigningFailed => {
                tracing::error!(error = %err, "x402 signing failed");
                Self::Infrastructure {
                    reason: "Payment proof signing failed".to_string(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn mock_wallet() -> AgentWallet {
        AgentWallet::from_hex_key(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap()
    }

    async fn negotiate_with_headers(
        client: &X402Client,
        price: &str,
        recipient: Option<&str>,
        network: Option<&str>,
    ) -> Result<PaymentProof, aiome_core_contracts::error::AiomeError> {
        let mock_server = MockServer::start().await;
        let mut tmpl = ResponseTemplate::new(402)
            .insert_header("X-Payment-Price", price)
            .insert_header("X-Payment-Currency", "USDC");
        if let Some(r) = recipient {
            tmpl = tmpl.insert_header("X-Payment-Recipient", r);
        }
        if let Some(n) = network {
            tmpl = tmpl.insert_header("X-Payment-Network", n);
        }
        Mock::given(method("GET"))
            .and(path("/protected"))
            .respond_with(tmpl)
            .mount(&mock_server)
            .await;

        let req_client = aiome_core::http::get_http_client();
        let resp = req_client
            .get(format!("{}/protected", mock_server.uri()))
            .send()
            .await
            .unwrap();
        client.negotiate(&resp).await
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_x402_extracts_headers_and_pays_under_budget() {
        let budget = U256::from(10_000_000_000u64);
        let client = X402Client::new("http://localhost".to_string(), budget).unwrap();

        let proof = negotiate_with_headers(
            &client,
            "1000000",
            Some("0x00000000219ab540356cBB839Cbe05303d7705Fa"),
            Some("base-sepolia"),
        )
        .await;
        assert!(proof.is_ok(), "Should successfully sign payment");
        let hash = proof.unwrap().transaction_hash;
        assert!(!hash.contains("mock_tx"));
        assert!(hash.starts_with("0x"));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_x402_rejects_over_budget() {
        let budget = U256::from(1_000_000u64);
        let client = X402Client::new("http://localhost".to_string(), budget).unwrap();

        let proof = negotiate_with_headers(
            &client,
            "5000000",
            Some("0x00000000219ab540356cBB839Cbe05303d7705Fa"),
            Some("base-sepolia"),
        )
        .await;
        assert!(proof.is_err());
        let msg = proof.unwrap_err().to_string().to_lowercase();
        assert!(msg.contains("budget"));
        assert!(!msg.contains("5000000"));
    }

    #[tokio::test]
    async fn test_x402_rejects_empty_recipient() {
        let client = X402Client::from_wallet(
            "http://localhost".into(),
            U256::from(10_000_000u64),
            X402_DEFAULT_NETWORK.into(),
            mock_wallet(),
        )
        .unwrap();

        let err = negotiate_with_headers(&client, "1000", Some(""), Some("base-sepolia"))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            aiome_core_contracts::error::AiomeError::Validation { .. }
        ));
    }

    #[tokio::test]
    async fn test_x402_rejects_missing_recipient() {
        let client = X402Client::from_wallet(
            "http://localhost".into(),
            U256::from(10_000_000u64),
            X402_DEFAULT_NETWORK.into(),
            mock_wallet(),
        )
        .unwrap();

        let err = negotiate_with_headers(&client, "1000", None, Some("base-sepolia"))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            aiome_core_contracts::error::AiomeError::Validation { .. }
        ));
    }

    #[tokio::test]
    async fn test_x402_rejects_network_mismatch() {
        let client = X402Client::from_wallet(
            "http://localhost".into(),
            U256::from(10_000_000u64),
            X402_DEFAULT_NETWORK.into(),
            mock_wallet(),
        )
        .unwrap();

        let err = negotiate_with_headers(
            &client,
            "1000",
            Some("0x00000000219ab540356cBB839Cbe05303d7705Fa"),
            Some("ethereum-mainnet"),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            aiome_core_contracts::error::AiomeError::Validation { .. }
        ));
    }

    #[tokio::test]
    async fn test_x402_rejects_invalid_price() {
        let client = X402Client::from_wallet(
            "http://localhost".into(),
            U256::from(10_000_000u64),
            X402_DEFAULT_NETWORK.into(),
            mock_wallet(),
        )
        .unwrap();

        let err = negotiate_with_headers(
            &client,
            "not-a-number",
            Some("0x00000000219ab540356cBB839Cbe05303d7705Fa"),
            Some("base-sepolia"),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            aiome_core_contracts::error::AiomeError::Validation { .. }
        ));
    }

    #[test]
    fn test_from_wallet_rejects_empty_rpc() {
        match X402Client::from_wallet(
            "".into(),
            U256::from(1u64),
            X402_DEFAULT_NETWORK.into(),
            mock_wallet(),
        ) {
            Err(e) => assert!(e.to_string().contains("non-empty")),
            Ok(_) => panic!("expected fail-closed"),
        }
    }

    #[test]
    fn test_from_wallet_rejects_non_http_rpc() {
        for bad in ["file:///tmp/rpc", "javascript:alert(1)", "not-a-url"] {
            match X402Client::from_wallet(
                bad.into(),
                U256::from(1u64),
                X402_DEFAULT_NETWORK.into(),
                mock_wallet(),
            ) {
                Err(_) => {}
                Ok(_) => panic!("expected reject for rpc={bad}"),
            }
        }
    }

    #[test]
    fn test_from_wallet_rejects_non_q2_network() {
        match X402Client::from_wallet(
            "https://sepolia.base.org".into(),
            U256::from(1u64),
            "ethereum-mainnet".into(),
            mock_wallet(),
        ) {
            Err(e) => assert!(
                e.to_string().contains("base-sepolia") || e.to_string().contains("out of scope"),
                "got: {e}"
            ),
            Ok(_) => panic!("expected Q2 network pin"),
        }
    }

    #[tokio::test]
    async fn test_x402_rejects_zero_recipient() {
        let client = X402Client::from_wallet(
            "http://localhost".into(),
            U256::from(10_000_000u64),
            X402_DEFAULT_NETWORK.into(),
            mock_wallet(),
        )
        .unwrap();

        let err = negotiate_with_headers(
            &client,
            "1000",
            Some("0x0000000000000000000000000000000000000000"),
            Some("base-sepolia"),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            aiome_core_contracts::error::AiomeError::Validation { .. }
        ));
    }

    #[tokio::test]
    async fn test_x402_rejects_missing_network() {
        let client = X402Client::from_wallet(
            "http://localhost".into(),
            U256::from(10_000_000u64),
            X402_DEFAULT_NETWORK.into(),
            mock_wallet(),
        )
        .unwrap();

        let err = negotiate_with_headers(
            &client,
            "1000",
            Some("0x00000000219ab540356cBB839Cbe05303d7705Fa"),
            None,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            aiome_core_contracts::error::AiomeError::Validation { .. }
        ));
    }

    #[test]
    fn test_x402_to_aiome_error() {
        use aiome_core_contracts::error::AiomeError;

        let err = X402Error::MissingHeaders;
        let aiome_err: AiomeError = err.into();
        assert!(matches!(aiome_err, AiomeError::Validation { .. }));

        let err = X402Error::BudgetExhausted {
            requested: U256::from(100u64),
            remaining: U256::from(50u64),
        };
        let aiome_err: AiomeError = err.into();
        match aiome_err {
            AiomeError::Infrastructure { reason } => {
                assert!(reason.contains("budget") || reason.contains("Budget"));
                assert!(!reason.contains("100"));
            }
            other => panic!("Expected Infrastructure, got {:?}", other),
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_new_rejects_empty_rpc() {
        match X402Client::new("".to_string(), U256::from(1u64)) {
            Err(e) => assert!(e.to_string().contains("non-empty")),
            Ok(_) => panic!("expected fail-closed on empty rpc_url"),
        }
    }

    #[tokio::test]
    async fn test_x402_rejects_invalid_recipient_address() {
        let client = X402Client::from_wallet(
            "http://localhost".into(),
            U256::from(10_000_000u64),
            X402_DEFAULT_NETWORK.into(),
            mock_wallet(),
        )
        .unwrap();

        let err = negotiate_with_headers(
            &client,
            "1000",
            Some("not-an-eth-address"),
            Some("base-sepolia"),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            aiome_core_contracts::error::AiomeError::Validation { .. }
        ));
    }

    #[test]
    fn parse_helpers_edge_cases() {
        assert!(parse_payment_recipient("").is_err());
        assert!(parse_payment_recipient("not-an-address").is_err());
        assert!(parse_payment_recipient("0x0000000000000000000000000000000000000000").is_err());
        assert!(parse_payment_price("abc").is_err());
        assert!(parse_payment_price("").is_err());
        assert!(assert_network_allowed("base-sepolia", "base-sepolia").is_ok());
        assert!(assert_network_allowed("BASE-SEPOLIA", "base-sepolia").is_ok());
        assert!(assert_network_allowed("", "base-sepolia").is_err());
        assert!(assert_network_allowed("mainnet", "base-sepolia").is_err());
        assert!(validate_x402_network("base-sepolia").is_ok());
        assert!(validate_x402_network("BASE-SEPOLIA").is_ok());
        assert!(validate_x402_network("ethereum-mainnet").is_err());
        assert!(validate_rpc_url("https://sepolia.base.org").is_ok());
        assert!(validate_rpc_url("file:///etc/passwd").is_err());
    }
}
