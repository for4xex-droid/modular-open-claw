/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::commerce::{
    CommerceEngine, EscrowRecord, FiatPaymentRails, SubscriptionStatus, Web3PaymentRails,
};
use aiome_core_contracts::error::AiomeError;
use async_trait::async_trait;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use uuid::Uuid;

pub struct PolarCommerceEngine {
    api_key: secrecy::SecretString,
    webhook_secret: secrecy::SecretString,
    base_url: String,
    http_client: reqwest::Client,
}

impl PolarCommerceEngine {
    pub fn new(
        api_key: secrecy::SecretString,
        webhook_secret: secrecy::SecretString,
        base_url: Option<String>,
    ) -> Self {
        Self {
            api_key,
            webhook_secret,
            base_url: base_url.unwrap_or_else(|| "https://api.polar.sh".into()),
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl FiatPaymentRails for PolarCommerceEngine {
    fn verify_signature(&self, svix_payload: &str, sig_header: &str) -> Result<(), AiomeError> {
        let sig = sig_header
            .strip_prefix("v1,")
            .ok_or_else(|| AiomeError::Unauthorized {
                reason: "Invalid Polar signature format (missing v1, prefix)".into(),
            })?;

        let actual_sig =
            base64::prelude::BASE64_STANDARD
                .decode(sig)
                .map_err(|_| AiomeError::Unauthorized {
                    reason: "Invalid Polar signature base64".into(),
                })?;

        let exposed_secret = secrecy::ExposeSecret::expose_secret(&self.webhook_secret);
        let secret = exposed_secret
            .strip_prefix("whsec_")
            .unwrap_or(exposed_secret);
        let decoded_secret = base64::prelude::BASE64_STANDARD
            .decode(secret)
            .map_err(|_| AiomeError::Infrastructure {
                reason: "Invalid Polar webhook secret base64".into(),
            })?;

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(&decoded_secret).map_err(|e| {
            AiomeError::Infrastructure {
                reason: e.to_string(),
            }
        })?;
        mac.update(svix_payload.as_bytes());

        let expected_sig = mac.finalize().into_bytes();

        if expected_sig.len() == actual_sig.len() && expected_sig.ct_eq(&actual_sig).into() {
            Ok(())
        } else {
            Err(AiomeError::Unauthorized {
                reason: "Signature mismatch".into(),
            })
        }
    }

    async fn create_checkout_session(
        &self,
        _agent_id: Uuid,
        _price_id: &str,
        _success_url: &str,
        _cancel_url: &str,
    ) -> Result<String, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "create_checkout_session not implemented for Polar API".into(),
        })
    }

    async fn create_portal_session(
        &self,
        _agent_id: Uuid,
        _return_url: &str,
    ) -> Result<String, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "create_portal_session not implemented for Polar API".into(),
        })
    }

    async fn create_subscription(
        &self,
        agent_id: Uuid,
        plan_id: &str,
    ) -> Result<String, AiomeError> {
        let url = format!("{}/api/v1/checkouts", self.base_url);

        let payload = serde_json::json!({
            "product_id": plan_id,
            "metadata": {
                "actor_id": agent_id.to_string()
            }
        });

        let res = self
            .http_client
            .post(&url)
            .bearer_auth(secrecy::ExposeSecret::expose_secret(&self.api_key))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Polar API Error: {}", e),
            })?;

        if !res.status().is_success() {
            let error_text = res.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Polar Checkout Failed: {}", error_text),
            });
        }

        let data: serde_json::Value = res.json().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Invalid Polar Response: {}", e),
        })?;

        let checkout_url = data["url"]
            .as_str()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "Missing url in Polar response".into(),
            })?;

        Ok(checkout_url.to_string())
    }

    async fn cancel_subscription(
        &self,
        _agent_id: Uuid,
        _subscription_id: &str,
    ) -> Result<(), AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "cancel_subscription not implemented for Polar API".into(),
        })
    }
}

#[async_trait]
impl Web3PaymentRails for PolarCommerceEngine {
    async fn stake(&self, _agent_id: Uuid, _amount: u64) -> Result<(), AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "stake not implemented for Polar API".into(),
        })
    }

    async fn slash(&self, _agent_id: Uuid, _amount: u64, _reason: &str) -> Result<(), AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "slash not implemented for Polar API".into(),
        })
    }
}

#[async_trait]
impl CommerceEngine for PolarCommerceEngine {
    async fn get_balance(&self, _agent_id: Uuid) -> Result<u64, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "get_balance not implemented for Polar API".into(),
        })
    }

    async fn validate_activity(
        &self,
        _agent_id: Uuid,
        _activity_type: &str,
        _amount: u64,
    ) -> Result<(), AiomeError> {
        // Polar doesn't support pre-validation, so we always succeed here and let checkout handle it
        Ok(())
    }

    async fn execute_autonomous_purchase(
        &self,
        _agent_id: Uuid,
        _item_id: Uuid,
        _metadata: serde_json::Value,
    ) -> Result<String, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "execute_autonomous_purchase not implemented for Polar API".into(),
        })
    }

    async fn get_daily_spend(&self, _agent_id: Uuid) -> Result<u64, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "get_daily_spend not implemented for Polar API".into(),
        })
    }

    async fn get_daily_limit(&self, _agent_id: Uuid) -> Result<u64, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "get_daily_limit not implemented for Polar API".into(),
        })
    }

    async fn escrow_create(&self, agent_id: Uuid, amount: u64) -> Result<String, AiomeError> {
        let url = format!("{}/api/v1/checkouts", self.base_url);

        let payload = serde_json::json!({
            "product_id": "gig_escrow", // This should be a dynamic ID or handled via metadata
            "metadata": {
                "actor_id": agent_id.to_string(),
                "amount": amount.to_string(),
                "type": "gig_escrow"
            }
        });

        let res = self
            .http_client
            .post(&url)
            .bearer_auth(secrecy::ExposeSecret::expose_secret(&self.api_key))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Polar API Error: {}", e),
            })?;

        if !res.status().is_success() {
            let error_text = res.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Polar Escrow Creation Failed: {}", error_text),
            });
        }

        let data: serde_json::Value = res.json().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Invalid Polar Response: {}", e),
        })?;

        let checkout_url = data["url"]
            .as_str()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "Missing url in Polar response".into(),
            })?;

        Ok(checkout_url.to_string())
    }

    async fn list_escrows(&self, _agent_id: Uuid) -> Result<Vec<EscrowRecord>, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "list_escrows not implemented for Polar API".into(),
        })
    }

    async fn escrow_release(
        &self,
        _escrow_id: &str,
        _recipient_id: Uuid,
    ) -> Result<(), AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "escrow_release not implemented for Polar API".into(),
        })
    }

    async fn escrow_refund(&self, _escrow_id: &str) -> Result<(), AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "escrow_refund not implemented for Polar API".into(),
        })
    }

    async fn register_license(
        &self,
        _agent_id: Uuid,
        _asset_id: Uuid,
        _transaction_id: &str,
        _license_type: &str,
    ) -> Result<String, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "register_license not implemented for Polar API".into(),
        })
    }

    async fn get_subscription_status(
        &self,
        _agent_id: Uuid,
    ) -> Result<SubscriptionStatus, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "get_subscription_status not implemented for Polar API".into(),
        })
    }

    async fn transfer(
        &self,
        _from_id: Uuid,
        _to_id: Uuid,
        _amount: u64,
    ) -> Result<String, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "transfer not implemented for Polar API".into(),
        })
    }

    async fn deduct_generation_cost(
        &self,
        _agent_id: Uuid,
        _asset_id: Option<Uuid>,
        _amount: u64,
        _generation_type: &str,
    ) -> Result<(), AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "deduct_generation_cost not implemented for Polar API".into(),
        })
    }

    async fn instant_refund(
        &self,
        _transaction_id: &str,
        _agent_id: Uuid,
    ) -> Result<(), AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "instant_refund not implemented for Polar API".into(),
        })
    }

    async fn withdraw_points(&self, _agent_id: Uuid, _amount: u64) -> Result<(), AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "withdraw_points not implemented for Polar API".into(),
        })
    }

    async fn get_points(
        &self,
        _agent_id: Uuid,
    ) -> Result<aiome_core_contracts::commerce::PointsBalance, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "get_points not implemented for Polar API".into(),
        })
    }

    async fn get_transaction_history(
        &self,
        _agent_id: Uuid,
        _limit: u32,
    ) -> Result<Vec<aiome_core_contracts::commerce::TransactionRecord>, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "get_transaction_history not implemented for Polar API".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_polar_create_subscription() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/checkouts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chk_123",
                "url": "https://polar.sh/checkout/chk_123",
                "metadata": {
                    "actor_id": "00000000-0000-0000-0000-000000000000"
                }
            })))
            .mount(&mock_server)
            .await;

        let engine = PolarCommerceEngine::new(
            secrecy::SecretString::from("test_api_key".to_string()),
            secrecy::SecretString::from("test_webhook_secret".to_string()),
            Some(mock_server.uri()),
        );

        let result = engine.create_subscription(Uuid::nil(), "plan_123").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://polar.sh/checkout/chk_123");
    }

    #[test]
    fn test_polar_verify_signature_success() {
        use base64::Engine;
        let raw_secret = b"test_secret_for_hmac_123456789";
        let b64_secret = base64::prelude::BASE64_STANDARD.encode(raw_secret);
        let secret = format!("whsec_{}", b64_secret);

        let payload = "msg_123.1614556800.{\"test\":true}";
        let engine = PolarCommerceEngine::new(
            secrecy::SecretString::from("key".to_string()),
            secrecy::SecretString::from(secret),
            None,
        );

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(raw_secret).unwrap();
        mac.update(payload.as_bytes());
        let b64_sig = base64::prelude::BASE64_STANDARD.encode(mac.finalize().into_bytes());
        let signature = format!("v1,{}", b64_sig);

        let result = engine.verify_signature(payload, &signature);
        assert!(result.is_ok(), "Expected OK, got: {:?}", result);
    }

    #[tokio::test]
    async fn test_polar_escrow_create_success() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/checkouts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chk_escrow_123",
                "url": "https://polar.sh/checkout/escrow_123"
            })))
            .mount(&mock_server)
            .await;

        let engine = PolarCommerceEngine::new(
            secrecy::SecretString::from("key".to_string()),
            secrecy::SecretString::from("secret".to_string()),
            Some(mock_server.uri()),
        );

        let result = engine.escrow_create(Uuid::nil(), 1000).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://polar.sh/checkout/escrow_123");
    }
}
