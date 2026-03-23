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
use sqlx::SqlitePool;
use tracing::{info, warn};
use uuid::Uuid;

/// Tremendous API を使用したギフト送信エンジン
pub struct TremendousGiftEngine {
    api_key: String,
    base_url: String,
    client: Client,
    pool: SqlitePool,
}

impl TremendousGiftEngine {
    /// 新しい TremendousGiftEngine を作成する
    pub fn new(api_key: String, sandbox: bool, pool: SqlitePool) -> Self {
        // Double check for production safety (😈 Demon's Advocate Gate 4)
        #[cfg(debug_assertions)]
        if !sandbox {
            panic!("🚨 [SECURITY] Attempting to use PRODUCTION Tremendous API in a DEBUG build! This is strictly forbidden to prevent accidental real fund usage during development/testing.");
        }

        let base_url = if sandbox {
            "https://testflight.tremendous.com/api/v2".to_string()
        } else {
            "https://tremendous.com/api/v2".to_string()
        };
        Self {
            api_key,
            base_url,
            client: aiome_core::http::get_http_client().clone(),
            pool,
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
        // Quick Win: [Expert-Review Gate 4] reason length limit
        let safe_reason = reason.chars().take(256).collect::<String>();

        info!(
            "🎁 [GiftEngine] Processing gift for {} (Amount: ${}, Reason: {})",
            recipient_email, amount_usd, safe_reason
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
        agent_id: Uuid,
        amount_usd: f64,
    ) -> Result<(), AiomeError> {
        let context = self.get_policy_context(agent_id).await?;

        // 1. 個別ギフト上限チェック ($5.0)
        if amount_usd > context.max_amount_usd {
            return Err(AiomeError::SecurityViolation {
                reason: format!(
                    "Autonomous gift amount exceeds safety limit (${})",
                    context.max_amount_usd
                ),
            });
        }

        // 2. 日次上限チェック
        if context.daily_limit_reached {
            return Err(AiomeError::SecurityViolation {
                reason: "Daily autonomous gift limit reached for this agent".to_string(),
            });
        }

        // 余裕を持たせたチェック (合計が $20.0 を超えないか)
        if context.daily_sent_total_usd + amount_usd > 20.0 {
            return Err(AiomeError::SecurityViolation {
                reason: "Proposed gift would exceed daily aggregate safety limit ($20.0)"
                    .to_string(),
            });
        }

        Ok(())
    }

    async fn get_policy_context(
        &self,
        agent_id: Uuid,
    ) -> Result<aiome_contracts::commerce::GiftPolicyContext, AiomeError> {
        use sqlx::Row;

        // Phase 15.2: 監査ログから今日（JST/UTC 混合に注意だが現行はサーバーローカル/UTC）の送信実績を取得
        let row = sqlx::query(
            "SELECT 
                COUNT(*) as count, 
                SUM(json_extract(new_data, '$.amount_usd')) as total
             FROM audit_ledger_global
             WHERE table_name = 'gift_transactions'
               AND operation = 'SEND'
               AND timestamp >= datetime('now', 'start of day')
               AND json_extract(new_data, '$.agent_id') = ?",
        )
        .bind(agent_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to fetch gift audit: {}", e),
        })?;

        let count: i64 = row.get("count");
        let total: f64 = row.get::<Option<f64>, _>("total").unwrap_or(0.0);

        // とりあえず日次 5 件 or 合計 $20.0 を上限とする
        let daily_limit_reached = count >= 5 || total >= 20.0;

        Ok(aiome_contracts::commerce::GiftPolicyContext {
            max_amount_usd: 5.0,
            daily_limit_reached,
            daily_sent_count: count as u32,
            daily_sent_total_usd: total,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_engine() -> TremendousGiftEngine {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        // 基本的なマイグレーション (audit_ledger_global)
        sqlx::query("CREATE TABLE audit_ledger_global (id INTEGER PRIMARY KEY, table_name TEXT, operation TEXT, record_id TEXT, new_data TEXT, current_hash TEXT, timestamp TEXT DEFAULT (datetime('now')))").execute(&pool).await.unwrap();
        TremendousGiftEngine::new("test_key".into(), true, pool)
    }

    #[tokio::test]
    async fn test_validate_gift_policy_within_limit() {
        let engine = setup_test_engine().await;
        let result = engine.validate_gift_policy(Uuid::new_v4(), 5.0).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_gift_policy_exceeds_limit() {
        let engine = setup_test_engine().await;
        let result = engine.validate_gift_policy(Uuid::new_v4(), 5.01).await;
        assert!(result.is_err());
        if let Err(AiomeError::SecurityViolation { reason }) = result {
            assert!(reason.contains("safety limit"));
        } else {
            panic!("Expected SecurityViolation");
        }
    }

    #[tokio::test]
    async fn test_sandbox_url_selection() {
        // sandbox=true の場合は testflight URL になることを確認
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let sandbox_engine = TremendousGiftEngine::new("key".into(), true, pool);
        assert!(sandbox_engine.base_url.contains("testflight"));
    }
}
