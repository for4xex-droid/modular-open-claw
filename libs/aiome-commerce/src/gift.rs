/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use aiome_core_contracts::audit::AuditLogger;
use aiome_core_contracts::commerce::GiftEngine;
use async_trait::async_trait;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde_json::json;
use shared::db::DatabasePool;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

/// Tremendous API を使用したギフト送信エンジン
pub struct TremendousGiftEngine {
    api_key: SecretString,
    base_url: String,
    client: Client,
    pool: DatabasePool,
    audit_logger: Arc<dyn AuditLogger>,
}

impl TremendousGiftEngine {
    /// 新しい TremendousGiftEngine を作成する
    pub fn new(
        api_key: SecretString,
        sandbox: bool,
        pool: DatabasePool,
        audit_logger: Arc<dyn AuditLogger>,
    ) -> Result<Self, aiome_core::error::AiomeError> {
        // Double check for production safety (😈 Demon's Advocate Gate 4)
        #[cfg(debug_assertions)]
        if !sandbox {
            return Err(aiome_core::error::AiomeError::SecurityViolation {
                reason: "Attempting to use PRODUCTION Tremendous API in a DEBUG build! This is strictly forbidden to prevent accidental real fund usage during development/testing.".to_string(),
            });
        }

        let base_url = if sandbox {
            "https://testflight.tremendous.com/api/v2".to_string()
        } else {
            "https://tremendous.com/api/v2".to_string()
        };
        Ok(Self {
            api_key,
            base_url,
            client: aiome_core::http::get_http_client().clone(),
            pool,
            audit_logger,
        })
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
            .bearer_auth(self.api_key.expose_secret())
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

        // Log the successful transaction asynchronously
        let _ = self
            .audit_logger
            .log_event(
                "GIFT_SEND",
                "GiftEngine",
                &json!({
                    "record_id": order_id.clone(),
                    "recipient_email": recipient_email,
                    "amount_usd": amount_usd,
                    "reason": safe_reason,
                }),
            )
            .await;

        Ok(order_id)
    }

    async fn validate_gift_policy(
        &self,
        agent_id: Uuid,
        amount_usd: f64,
    ) -> Result<(), AiomeError> {
        // 0. 入力サニタイズ: NaN, Infinity, ゼロ, 負数を拒否
        if !amount_usd.is_finite() || amount_usd <= 0.0 {
            return Err(AiomeError::SecurityViolation {
                reason: format!(
                    "Invalid gift amount: {}. Must be a positive finite number.",
                    amount_usd
                ),
            });
        }

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
    ) -> Result<aiome_core_contracts::commerce::GiftPolicyContext, AiomeError> {
        let agent_str = agent_id.to_string();

        let q = format!(
            "SELECT
                COUNT(*) as count,
                COALESCE(SUM(CAST(new_data->>'amount_usd' AS FLOAT)), 0.0) as total
             FROM audit_ledger_global
             WHERE table_name = 'gift_transactions'
               AND operation = 'SEND'
               AND timestamp >= CURRENT_DATE
               AND new_data->>'agent_id' = {}",
            self.pool.ph(0)
        );

        // Use generic SQL fetch for dual-dialect support (SQLite & PostgreSQL)
        let row_opt =
            shared::sql_fetch_optional!(&self.pool, (i64, f64), &q, &agent_str).map_err(|e| {
                AiomeError::Infrastructure {
                    reason: format!("Failed to fetch gift audit: {}", e),
                }
            })?;

        let (count, total) = row_opt.unwrap_or((0, 0.0));

        // とりあえず日次 5 件 or 合計 $20.0 を上限とする
        let daily_limit_reached = count >= 5 || total >= 20.0;

        Ok(aiome_core_contracts::commerce::GiftPolicyContext {
            max_amount_usd: 5.0,
            daily_limit_reached,
            daily_sent_count: u32::try_from(count).unwrap_or(u32::MAX),
            daily_sent_total_usd: total,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core_contracts::audit::AuditLogger;
    use shared::db::DatabasePool;
    use sqlx::SqlitePool;

    struct MockAuditLogger;

    #[async_trait::async_trait]
    impl AuditLogger for MockAuditLogger {
        async fn log_event(
            &self,
            _t: &str,
            _a: &str,
            _d: &serde_json::Value,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn log_violation(
            &self,
            _t: &str,
            _d: &str,
            _c: &serde_json::Value,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    async fn setup_test_engine() -> TremendousGiftEngine {
        let pool = if let Ok(pg_url) = std::env::var("TEST_POSTGRES_URL") {
            let pg_pool = sqlx::postgres::PgPoolOptions::new()
                .connect(&pg_url)
                .await
                .unwrap();
            sqlx::query("CREATE TABLE IF NOT EXISTS audit_ledger_global (id SERIAL PRIMARY KEY, table_name TEXT, operation TEXT, record_id TEXT, new_data JSONB, current_hash TEXT, timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP)").execute(&pg_pool).await.unwrap();
            sqlx::query("DELETE FROM audit_ledger_global")
                .execute(&pg_pool)
                .await
                .unwrap();
            DatabasePool::Postgres(pg_pool)
        } else {
            let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
            sqlx::query("CREATE TABLE audit_ledger_global (id INTEGER PRIMARY KEY, table_name TEXT, operation TEXT, record_id TEXT, new_data TEXT, current_hash TEXT, timestamp TEXT DEFAULT (datetime('now')))").execute(&p).await.unwrap();
            DatabasePool::Sqlite(p)
        };

        let logger = std::sync::Arc::new(MockAuditLogger);
        TremendousGiftEngine::new(
            SecretString::from("test_key".to_string()),
            true,
            pool,
            logger,
        )
        .unwrap()
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
            panic!("Expected SecurityViolation but got {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_validate_gift_policy_rejects_nan() {
        let engine = setup_test_engine().await;
        let result = engine.validate_gift_policy(Uuid::new_v4(), f64::NAN).await;
        assert!(result.is_err());
        if let Err(AiomeError::SecurityViolation { reason }) = result {
            assert!(reason.contains("Invalid gift amount"));
        } else {
            panic!("Expected SecurityViolation for NaN but got {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_validate_gift_policy_rejects_negative() {
        let engine = setup_test_engine().await;
        let result = engine.validate_gift_policy(Uuid::new_v4(), -1.0).await;
        assert!(result.is_err());
        if let Err(AiomeError::SecurityViolation { reason }) = result {
            assert!(reason.contains("Invalid gift amount"));
        } else {
            panic!(
                "Expected SecurityViolation for negative but got {:?}",
                result
            );
        }
    }

    #[tokio::test]
    async fn test_validate_gift_policy_rejects_zero() {
        let engine = setup_test_engine().await;
        let result = engine.validate_gift_policy(Uuid::new_v4(), 0.0).await;
        assert!(result.is_err());
        if let Err(AiomeError::SecurityViolation { reason }) = result {
            assert!(reason.contains("Invalid gift amount"));
        } else {
            panic!("Expected SecurityViolation for zero but got {:?}", result);
        }
    }
    #[tokio::test]
    async fn test_sandbox_url_selection() {
        // sandbox=true の場合は testflight URL になることを確認
        let pool = if let Ok(pg_url) = std::env::var("TEST_POSTGRES_URL") {
            let pg_pool = sqlx::postgres::PgPoolOptions::new()
                .connect(&pg_url)
                .await
                .unwrap();
            DatabasePool::Postgres(pg_pool)
        } else {
            DatabasePool::Sqlite(
                SqlitePool::connect("sqlite::memory:")
                    .await
                    .unwrap_or_else(|e| panic!("sqlite fail: {}", e)),
            )
        };
        let logger = std::sync::Arc::new(MockAuditLogger);
        let sandbox_engine =
            TremendousGiftEngine::new(SecretString::from("key".to_string()), true, pool, logger)
                .unwrap();
        assert!(sandbox_engine.base_url.contains("testflight"));
    }
}
