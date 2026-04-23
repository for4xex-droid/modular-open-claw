/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core_contracts::commerce::{CommerceEngine, EscrowRecord, SubscriptionStatus};
use aiome_core_contracts::error::AiomeError;
use async_trait::async_trait;
use uuid::Uuid;

pub struct PolarCommerceEngine {
    api_key: String,
    webhook_secret: String,
    base_url: String,
    http_client: reqwest::Client,
}

impl PolarCommerceEngine {
    pub fn new(api_key: String, webhook_secret: String, base_url: Option<String>) -> Self {
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
        Err(AiomeError::Infrastructure {
            reason: "validate_activity not implemented for Polar API".into(),
        })
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

    async fn escrow_create(&self, _agent_id: Uuid, _amount: u64) -> Result<String, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "escrow_create not implemented for Polar API".into(),
        })
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

    fn verify_signature(&self, _payload: &str, _sig_header: &str) -> Result<(), AiomeError> {
        Err(AiomeError::Unauthorized {
            reason: "Not implemented".into(),
        })
    }

    async fn process_webhook(
        &self,
        _event_id: &str,
        _event_type: &str,
        _payload: &serde_json::Value,
    ) -> Result<(), AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "process_webhook not implemented for Polar API".into(),
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
            .bearer_auth(&self.api_key)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_polar_create_subscription() {
        let mock_server = MockServer::start().await;
        // Mock Polar checkout creation
        Mock::given(method("POST"))
            .and(path("/api/v1/checkouts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chk_123",
                "url": "https://polar.sh/checkout/chk_123",
                "metadata": {
                    "actor_id": "00000000-0000-0000-0000-000000000000" // Expect actor_id to be passed
                }
            })))
            .mount(&mock_server)
            .await;

        let engine = PolarCommerceEngine::new(
            "test_api_key".into(),
            "test_webhook_secret".into(),
            Some(mock_server.uri()),
        );

        let result = engine.create_subscription(Uuid::nil(), "plan_123").await;
        assert!(
            result.is_ok(),
            "Expected Polar Checkout URL to be generated, got {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), "https://polar.sh/checkout/chk_123");
    }
}
