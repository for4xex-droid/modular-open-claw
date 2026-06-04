/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::{PaymentProof, X402Negotiator, U256};
use alloy_signer_local::PrivateKeySigner;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum X402Error {
    #[error("Budget exhausted. Requested: {requested}, Remaining: {remaining}")]
    BudgetExhausted { requested: U256, remaining: U256 },
    #[error("Failed to parse payment required response")]
    InvalidPaymentResponse,
    #[error("No valid payment headers found in 402 response")]
    MissingHeaders,
}

pub struct X402Client {
    #[allow(dead_code)]
    rpc_url: String,
    budget_cap: U256,
    signer: PrivateKeySigner,
}

impl X402Client {
    pub fn new(rpc_url: String, budget_cap: U256) -> Result<Self> {
        // [重要: 秘密鍵保護]
        // テスト/デバッグビルドではモックキーを使用
        // 本番ビルドでは keyring クレートにより OS Keychain から取得（未実装のためエラー）
        #[cfg(any(test, debug_assertions))]
        let private_key_hex =
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string();

        #[cfg(not(any(test, debug_assertions)))]
        return Err(anyhow::anyhow!(
            "X402Client private key management not implemented for production. Use OS Keychain integration."
        ));

        let signer =
            PrivateKeySigner::from_str(&private_key_hex).context("Invalid private key format")?;

        Ok(Self {
            rpc_url,
            budget_cap,
            signer,
        })
    }
}

#[async_trait]
impl X402Negotiator for X402Client {
    async fn negotiate(&self, response: &reqwest::Response) -> Result<PaymentProof> {
        let headers = response.headers();

        let price_str = headers
            .get("X-Payment-Price")
            .and_then(|h| h.to_str().ok())
            .ok_or(X402Error::MissingHeaders)?;

        let price =
            U256::from_str_radix(price_str, 10).map_err(|_| X402Error::InvalidPaymentResponse)?;

        if price > self.budget_cap {
            return Err(X402Error::BudgetExhausted {
                requested: price,
                remaining: self.budget_cap,
            }
            .into());
        }

        // REFACTOR: Here we will eventually use self.signer to construct a real transaction.
        // For now, we return a mocked transaction hash to ensure the structural TDD logic holds.
        let mocked_tx_hash = format!("0x_mock_tx_signed_by_{}", self.signer.address());

        Ok(PaymentProof {
            transaction_hash: mocked_tx_hash,
        })
    }

    async fn balance(&self) -> Result<U256> {
        Ok(self.budget_cap)
    }
}

impl From<X402Error> for aiome_core_contracts::error::AiomeError {
    fn from(err: X402Error) -> Self {
        Self::Infrastructure {
            reason: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_x402_extracts_headers_and_pays_under_budget() {
        let budget = U256::from(10_000_000_000u64); // 10 USDC (assuming 6 decimals)
        let client = X402Client::new("http://localhost".to_string(), budget).unwrap();

        // We need a dummy response with X-Payment headers
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/protected"))
            .respond_with(
                ResponseTemplate::new(402)
                    .insert_header("X-Payment-Price", "1000000") // 1 USDC
                    .insert_header("X-Payment-Currency", "USDC")
                    .insert_header(
                        "X-Payment-Recipient",
                        "0x00000000219ab540356cBB839Cbe05303d7705Fa",
                    )
                    .insert_header("X-Payment-Network", "base-sepolia"),
            )
            .mount(&mock_server)
            .await;

        let req_client = aiome_core::http::get_http_client();
        let resp = req_client
            .get(format!("{}/protected", mock_server.uri()))
            .send()
            .await
            .unwrap();

        // Act & Assert
        let proof = client.negotiate(&resp).await;
        assert!(
            proof.is_ok(),
            "Should successfully construct and sign payment"
        );
        assert!(!proof.unwrap().transaction_hash.is_empty());
    }

    #[tokio::test]
    async fn test_x402_rejects_over_budget() {
        let budget = U256::from(1_000_000u64); // 1 USDC limit
        let client = X402Client::new("http://localhost".to_string(), budget).unwrap();

        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/protected"))
            .respond_with(
                ResponseTemplate::new(402)
                    .insert_header("X-Payment-Price", "5000000") // 5 USDC
                    .insert_header("X-Payment-Currency", "USDC")
                    .insert_header(
                        "X-Payment-Recipient",
                        "0x00000000219ab540356cBB839Cbe05303d7705Fa",
                    ),
            )
            .mount(&mock_server)
            .await;

        let req_client = aiome_core::http::get_http_client();
        let resp = req_client
            .get(format!("{}/protected", mock_server.uri()))
            .send()
            .await
            .unwrap();

        // Act & Assert
        let proof = client.negotiate(&resp).await;
        assert!(proof.is_err(), "Should error out due to budget exhausted");
        if let Err(e) = proof {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("Budget exhausted"),
                "Error should be BudgetExhausted"
            );
        }
    }

    #[test]
    fn test_x402_to_aiome_error() {
        use aiome_core_contracts::error::AiomeError;

        let err = X402Error::MissingHeaders;
        let aiome_err: AiomeError = err.into();
        assert!(matches!(aiome_err, AiomeError::Infrastructure { .. }));

        let err = X402Error::BudgetExhausted {
            requested: U256::from(100u64),
            remaining: U256::from(50u64),
        };
        let aiome_err: AiomeError = err.into();
        assert!(matches!(aiome_err, AiomeError::Infrastructure { .. }));
    }
}
