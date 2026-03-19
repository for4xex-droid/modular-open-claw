/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::commerce::GiftEngine;
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tracing::{info, warn};
use uuid::Uuid;

/// Tremendous API を使用したギフト送信エンジン
pub struct TremendousGiftEngine {
    api_key: String,
    base_url: String,
    client: Client,
}

impl TremendousGiftEngine {
    /// 新しい TremendousGiftEngine を作成する
    pub fn new(api_key: String, sandbox: bool) -> Self {
        let base_url = if sandbox {
            "https://testflight.tremendous.com/api/v2".to_string()
        } else {
            "https://tremendous.com/api/v2".to_string()
        };
        Self {
            api_key,
            base_url,
            client: aiome_core::http::get_http_client().clone(),
        }
    }
}

#[async_trait]
impl GiftEngine for TremendousGiftEngine {
    async fn send_gift_code(
        &self,
        recipient_email: &str,
        amount_usd: f64,
        reason: &str,
    ) -> Result<String, AiomeError> {
        info!(
            "🎁 [GiftEngine] Processing gift for {} (Amount: ${}, Reason: {})",
            recipient_email, amount_usd, reason
        );

        let payload = json!({
            "payment": {
                "funding_source_id": "BALANCE"
            },
            "rewards": [
                {
                    "campaign_id": null,
                    "delivery": {
                        "method": "EMAIL"
                    },
                    "recipient": {
                        "name": "Aiome User",
                        "email": recipient_email
                    },
                    "value": {
                        "amount": amount_usd,
                        "currency_code": "USD"
                    }
                }
            ]
        });

        let res = self
            .client
            .post(format!("{}/orders", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Tremendous API connection failed: {}", e),
            })?;

        if !res.status().is_success() {
            let error_text = res.text().await.unwrap_or_default();
            warn!("❌ [GiftEngine] Tremendous API Error: {}", error_text);
            return Err(AiomeError::Infrastructure {
                reason: format!("Tremendous API Error: {}", error_text),
            });
        }

        let body: serde_json::Value = res.json().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to parse Tremendous response: {}", e),
        })?;
        let order_id = body["order"]["id"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        info!(
            "✅ [GiftEngine] Gift order created successfully: {}",
            order_id
        );
        Ok(order_id)
    }

    async fn validate_gift_policy(
        &self,
        _agent_id: Uuid,
        amount_usd: f64,
    ) -> Result<(), AiomeError> {
        // Phase 7.2 Safety Constraint: 自律的ギフトは 1件あたり $5.0USD を上限とする (ダークパターン・資産流出防止)
        if amount_usd > 5.0 {
            return Err(AiomeError::SecurityViolation {
                reason: "Autonomous gift amount exceeds safety limit ($5.0)".to_string(),
            });
        }

        // TODO: 将来的には Karma スコア等と連動させ、信頼度の高いエージェントのみ上限を緩和する
        Ok(())
    }
}
