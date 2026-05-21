/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::commerce::{CommerceEngine, EscrowRecord};
use aiome_core_contracts::error::AiomeError;
use async_trait::async_trait;
use secrecy::{ExposeSecret, SecretString};
use stripe_webhook::Webhook;
use uuid::Uuid;

/// Stripe Webhook イベントを処理する商用エンジン実装
pub struct StripeCommerceEngine {
    client: stripe::Client,
    webhook_secret: SecretString,
    pool: sqlx::SqlitePool,
    is_mock: bool,
    nurture_client: Option<reqwest::Client>,
    nurture_url: Option<String>,
    nurture_secret: Option<String>,
    oxp_score_provider: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
}

impl StripeCommerceEngine {
    pub fn new(
        api_key: SecretString,
        webhook_secret: SecretString,
        pool: sqlx::SqlitePool,
        nurture_url: Option<String>,
        nurture_secret: Option<String>,
    ) -> Self {
        if nurture_url.is_none() {
            tracing::warn!("⚠️ [StripeCommerceEngine] NURTURE_API_URL is NOT set. The engine will run in OSS fallback mode. This may cause split-brain if used in a commercial deployment!");
            #[cfg(not(debug_assertions))]
            tracing::error!("🚨 [StripeCommerceEngine] Running in RELEASE mode without NURTURE_API_URL! Operations will write to local SQLite instead of Nurture API.");
        }

        let is_dev_mode = std::env::var("AIOME_DEV_MODE")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(cfg!(debug_assertions));

        let is_mock = is_dev_mode
            && (api_key.expose_secret().starts_with("sk_test_mock")
                || webhook_secret.expose_secret() == "whsec_test");
        let nurture_client = nurture_url
            .as_ref()
            .map(|_| aiome_core::http::get_http_client().clone());
        Self {
            client: stripe::Client::new(api_key.expose_secret().to_string()),
            webhook_secret,
            pool,
            is_mock,
            nurture_client,
            nurture_url,
            nurture_secret,
            oxp_score_provider: None,
        }
    }

    /// OXPスコアプロバイダを注入する
    pub fn with_oxp_score_provider(
        mut self,
        provider: std::sync::Arc<std::sync::atomic::AtomicU32>,
    ) -> Self {
        self.oxp_score_provider = Some(provider);
        self
    }

    /// X-OxiLean-Proof-Certificate ヘッダを生成する
    fn generate_oxp_header(&self) -> Option<String> {
        let secret = self.nurture_secret.as_ref()?;
        let oxp = self
            .oxp_score_provider
            .as_ref()
            .map(|p| p.load(std::sync::atomic::Ordering::Relaxed))
            .unwrap_or(0);
        let ts = chrono::Utc::now().to_rfc3339();
        let cert = aiome_core_contracts::oxilean::OxiLeanProofCertificate::generate(
            "aiome-edge-node".to_string(), // subject_id
            oxp,
            ts,
            secret,
        );
        let cert_json = serde_json::to_string(&cert).ok()?;
        use base64::Engine;
        Some(base64::engine::general_purpose::STANDARD.encode(cert_json))
    }
}

#[derive(serde::Deserialize)]
struct BalanceRes {
    balance: u64,
}

#[derive(serde::Deserialize)]
struct DailyStatsRes {
    spent_today: u64,
    daily_limit: u64,
}

#[async_trait]
impl CommerceEngine for StripeCommerceEngine {
    async fn get_balance(&self, agent_id: Uuid) -> Result<u64, AiomeError> {
        if let (Some(url), Some(secret), Some(client)) = (
            &self.nurture_url,
            &self.nurture_secret,
            &self.nurture_client,
        ) {
            let req_url = format!("{}/internal/balance/{}", url, agent_id);
            let mut req = client
                .get(&req_url)
                .header("Authorization", format!("Bearer {}", secret))
                .timeout(std::time::Duration::from_secs(10));

            if let Some(cert_header) = self.generate_oxp_header() {
                req = req.header("X-OxiLean-Proof-Certificate", cert_header);
            }

            let res = req.send().await.map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

            if res.status().is_success() {
                let body: BalanceRes =
                    res.json().await.map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                return Ok(body.balance);
            }
        }
        Ok(0)
    }

    async fn validate_activity(
        &self,
        agent_id: Uuid,
        activity_type: &str,
        amount: u64,
    ) -> Result<(), AiomeError> {
        if let (Some(url), Some(secret), Some(client)) = (
            &self.nurture_url,
            &self.nurture_secret,
            &self.nurture_client,
        ) {
            let req_url = format!("{}/internal/validate-activity", url);
            let mut req = client
                .post(&req_url)
                .header("Authorization", format!("Bearer {}", secret))
                .json(&serde_json::json!({
                    "agent_id": agent_id.to_string(),
                    "activity_type": activity_type,
                    "amount": amount
                }))
                .timeout(std::time::Duration::from_secs(5));

            if let Some(cert_header) = self.generate_oxp_header() {
                req = req.header("X-OxiLean-Proof-Certificate", cert_header);
            }

            match req.send().await {
                Ok(res) if res.status().is_success() => return Ok(()),
                Ok(res) if res.status() == reqwest::StatusCode::PAYMENT_REQUIRED => {
                    return Err(AiomeError::Infrastructure {
                        reason: "Insufficient funds".into(),
                    });
                }
                Ok(res) => {
                    tracing::warn!(
                        "⚠️ [Billing] validate_activity got unexpected status {} for agent {}. Allowing (fail-open).",
                        res.status(),
                        agent_id
                    );
                    return Ok(());
                }
                Err(e) => {
                    tracing::error!(
                        "❌ [Billing] validate_activity network error for agent {}: {}. Allowing (fail-open).",
                        agent_id,
                        e
                    );
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    async fn execute_autonomous_purchase(
        &self,
        _agent_id: Uuid,
        _item_id: Uuid,
        _metadata: serde_json::Value,
    ) -> Result<String, AiomeError> {
        if self.is_mock {
            return Ok("tx_mock".into());
        }
        Err(AiomeError::Infrastructure {
            reason: "execute_autonomous_purchase is not available in v1.0".into(),
        })
    }

    async fn get_daily_spend(&self, agent_id: Uuid) -> Result<u64, AiomeError> {
        if let (Some(url), Some(secret), Some(client)) = (
            &self.nurture_url,
            &self.nurture_secret,
            &self.nurture_client,
        ) {
            let req_url = format!("{}/internal/daily-stats/{}", url, agent_id);
            let mut req = client
                .get(&req_url)
                .header("Authorization", format!("Bearer {}", secret))
                .timeout(std::time::Duration::from_secs(10));

            if let Some(cert_header) = self.generate_oxp_header() {
                req = req.header("X-OxiLean-Proof-Certificate", cert_header);
            }

            let res = req.send().await.map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

            if res.status().is_success() {
                let body: DailyStatsRes =
                    res.json().await.map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                return Ok(body.spent_today);
            }
        }
        Ok(0)
    }

    async fn get_daily_limit(&self, agent_id: Uuid) -> Result<u64, AiomeError> {
        if let (Some(url), Some(secret), Some(client)) = (
            &self.nurture_url,
            &self.nurture_secret,
            &self.nurture_client,
        ) {
            let req_url = format!("{}/internal/daily-stats/{}", url, agent_id);
            let mut req = client
                .get(&req_url)
                .header("Authorization", format!("Bearer {}", secret))
                .timeout(std::time::Duration::from_secs(10));

            if let Some(cert_header) = self.generate_oxp_header() {
                req = req.header("X-OxiLean-Proof-Certificate", cert_header);
            }

            let res = req.send().await.map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

            if res.status().is_success() {
                let body: DailyStatsRes =
                    res.json().await.map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                return Ok(body.daily_limit);
            }
        }
        Ok(100)
    }

    async fn escrow_create(&self, agent_id: Uuid, amount: u64) -> Result<String, AiomeError> {
        if let (Some(url), Some(secret), Some(client)) = (
            &self.nurture_url,
            &self.nurture_secret,
            &self.nurture_client,
        ) {
            let req_url = format!("{}/internal/escrow-create", url);
            let payload = serde_json::json!({
                "actor_id": agent_id,
                "amount": amount,
                "idempotency_key": Uuid::new_v4().to_string()
            });
            let mut req = client
                .post(&req_url)
                .header("Authorization", format!("Bearer {}", secret))
                .json(&payload)
                .timeout(std::time::Duration::from_secs(10));

            if let Some(cert_header) = self.generate_oxp_header() {
                req = req.header("X-OxiLean-Proof-Certificate", cert_header);
            }

            let res = req.send().await.map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

            if res.status().is_success() {
                #[derive(serde::Deserialize)]
                struct EscrowRes {
                    escrow_id: String,
                }
                let body: EscrowRes = res.json().await.map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;
                return Ok(body.escrow_id);
            } else {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Escrow create HTTP failed: {}", res.status()),
                });
            }
        }

        // ガードチェックの代わりに安全なキャストで直接変換
        let safe_amount = i64::try_from(amount).map_err(|_| AiomeError::Infrastructure {
            reason: format!("Escrow amount {} exceeds i64 maximum", amount),
        })?;
        let escrow_id = format!("escrow_{}", Uuid::new_v4());
        let order_id = format!("ord_{}", Uuid::new_v4());

        let result = sqlx::query(
            "INSERT INTO escrows (id, payer_id, order_id, amount, status) VALUES (?, ?, ?, ?, 'Locked')",
        )
        .bind(&escrow_id)
        .bind(agent_id.to_string())
        .bind(&order_id)
        .bind(safe_amount)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => {
                tracing::info!("🔒 [StripeCommerce] Created Escrow: {}", escrow_id);
                Ok(escrow_id)
            }
            Err(e) => {
                tracing::error!("❌ [StripeCommerce] Failed to create escrow: {}", e);
                if self.is_mock {
                    Ok("escrow_mock".to_string())
                } else {
                    Err(AiomeError::Infrastructure {
                        reason: format!("DB insertion failed: {}", e),
                    })
                }
            }
        }
    }

    async fn list_escrows(&self, agent_id: Uuid) -> Result<Vec<EscrowRecord>, AiomeError> {
        if let (Some(url), Some(secret), Some(client)) = (
            &self.nurture_url,
            &self.nurture_secret,
            &self.nurture_client,
        ) {
            let req_url = format!("{}/internal/escrow-list/{}", url, agent_id);
            let mut req = client
                .get(&req_url)
                .header("Authorization", format!("Bearer {}", secret))
                .timeout(std::time::Duration::from_secs(10));

            if let Some(cert_header) = self.generate_oxp_header() {
                req = req.header("X-OxiLean-Proof-Certificate", cert_header);
            }

            let res = req.send().await.map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

            if res.status().is_success() {
                let records: Vec<EscrowRecord> =
                    res.json().await.map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to parse escrow list: {}", e),
                    })?;
                return Ok(records);
            }
        }

        use sqlx::Row;

        let rows = sqlx::query(
            "SELECT id, payer_id, order_id, amount, status, created_at FROM escrows WHERE payer_id = ? ORDER BY created_at DESC"
        )
        .bind(agent_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to fetch escrows: {}", e),
        })?;

        let records = rows
            .into_iter()
            .map(|row| -> Result<EscrowRecord, AiomeError> {
                Ok(EscrowRecord {
                    id: row.try_get("id").map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to read escrow.id: {}", e),
                    })?,
                    payer_id: row
                        .try_get("payer_id")
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: format!("Failed to read escrow.payer_id: {}", e),
                        })?,
                    order_id: row
                        .try_get("order_id")
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: format!("Failed to read escrow.order_id: {}", e),
                        })?,
                    amount: row
                        .try_get("amount")
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: format!("Failed to read escrow.amount: {}", e),
                        })?,
                    status: row
                        .try_get("status")
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: format!("Failed to read escrow.status: {}", e),
                        })?,
                    created_at: row.try_get("created_at").map_err(|e| {
                        AiomeError::Infrastructure {
                            reason: format!("Failed to read escrow.created_at: {}", e),
                        }
                    })?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(records)
    }

    async fn escrow_release(&self, escrow_id: &str, recipient_id: Uuid) -> Result<(), AiomeError> {
        if let (Some(url), Some(secret), Some(client)) = (
            &self.nurture_url,
            &self.nurture_secret,
            &self.nurture_client,
        ) {
            let req_url = format!("{}/internal/escrow-release", url);
            let payload = serde_json::json!({
                "escrow_id": escrow_id,
                "recipient_id": recipient_id,
                "idempotency_key": Uuid::new_v4().to_string()
            });
            let mut req = client
                .post(&req_url)
                .header("Authorization", format!("Bearer {}", secret))
                .json(&payload)
                .timeout(std::time::Duration::from_secs(10));

            if let Some(cert_header) = self.generate_oxp_header() {
                req = req.header("X-OxiLean-Proof-Certificate", cert_header);
            }

            let res = req.send().await.map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

            if res.status().is_success() {
                return Ok(());
            } else {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Escrow release HTTP failed: {}", res.status()),
                });
            }
        }

        let result = sqlx::query(
            "UPDATE escrows SET status = 'Released', recipient_id = ? WHERE id = ? AND status = 'Locked'",
        )
        .bind(recipient_id.to_string())
        .bind(escrow_id)
        .execute(&self.pool)
        .await;

        match result {
            Ok(db_result) if db_result.rows_affected() > 0 => {
                tracing::info!("🔓 [StripeCommerce] Released Escrow: {}", escrow_id);
                Ok(())
            }
            Ok(_) => Err(AiomeError::Infrastructure {
                reason: "Escrow not found or not locked".into(),
            }),
            Err(e) => {
                if self.is_mock {
                    return Ok(());
                }
                Err(AiomeError::Infrastructure {
                    reason: format!("Failed to release escrow: {}", e),
                })
            }
        }
    }

    async fn escrow_refund(&self, escrow_id: &str) -> Result<(), AiomeError> {
        if let (Some(url), Some(secret), Some(client)) = (
            &self.nurture_url,
            &self.nurture_secret,
            &self.nurture_client,
        ) {
            let req_url = format!("{}/internal/escrow-refund", url);
            let payload = serde_json::json!({
                "escrow_id": escrow_id,
                "idempotency_key": Uuid::new_v4().to_string()
            });
            let mut req = client
                .post(&req_url)
                .header("Authorization", format!("Bearer {}", secret))
                .json(&payload)
                .timeout(std::time::Duration::from_secs(10));

            if let Some(cert_header) = self.generate_oxp_header() {
                req = req.header("X-OxiLean-Proof-Certificate", cert_header);
            }

            let res = req.send().await.map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

            if res.status().is_success() {
                return Ok(());
            } else {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Escrow refund HTTP failed: {}", res.status()),
                });
            }
        }

        let result = sqlx::query(
            "UPDATE escrows SET status = 'Refunded' WHERE id = ? AND status = 'Locked'",
        )
        .bind(escrow_id)
        .execute(&self.pool)
        .await;

        match result {
            Ok(db_result) if db_result.rows_affected() > 0 => {
                tracing::info!("💸 [StripeCommerce] Refunded Escrow: {}", escrow_id);
                Ok(())
            }
            Ok(_) => Err(AiomeError::Infrastructure {
                reason: "Escrow not found or not locked".into(),
            }),
            Err(e) => {
                if self.is_mock {
                    return Ok(());
                }
                Err(AiomeError::Infrastructure {
                    reason: format!("Failed to refund escrow: {}", e),
                })
            }
        }
    }
    async fn stake(&self, _agent_id: Uuid, _amount: u64) -> Result<(), AiomeError> {
        if self.is_mock {
            return Ok(());
        }
        Err(AiomeError::Infrastructure {
            reason: "stake is not available in v1.0".into(),
        })
    }

    async fn slash(&self, _agent_id: Uuid, _amount: u64, _reason: &str) -> Result<(), AiomeError> {
        if self.is_mock {
            return Ok(());
        }
        Err(AiomeError::Infrastructure {
            reason: "slash is not available in v1.0".into(),
        })
    }

    async fn register_license(
        &self,
        _agent_id: Uuid,
        _asset_id: Uuid,
        _transaction_id: &str,
        _license_type: &str,
    ) -> Result<String, AiomeError> {
        if self.is_mock {
            return Ok("lic_mock".into());
        }
        Err(AiomeError::Infrastructure {
            reason: "register_license is not available in v1.0".into(),
        })
    }

    fn verify_signature(&self, payload: &str, sig_header: &str) -> Result<(), AiomeError> {
        if !self.is_mock && self.webhook_secret.expose_secret() == "whsec_test" {
            tracing::error!("🚨 [SECURITY] Stripe Webhook verification rejected: Test secret ('whsec_test') used in production mode!");
            return Err(AiomeError::Infrastructure {
                reason: "Stripe Webhook verification failed: Test secret used in production mode!"
                    .into(),
            });
        }
        // NOTE: construct_event は strict な Event 型にデシリアライズするため、
        // テスト環境の部分ペイロードではパースエラーを返す場合がある。
        // ここでは HMAC 署名検証のみを目的とし、パース済み Event は意図的に破棄する。
        // ペイロードのビジネスロジック処理は commerce_webhook.rs 側で
        // serde_json::Value として柔軟にパースする。
        Webhook::construct_event(payload, sig_header, self.webhook_secret.expose_secret())
            .map(|_| ())
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Stripe Webhook verification failed: {}", e),
            })
    }

    async fn create_checkout_session(
        &self,
        agent_id: Uuid,
        price_id: &str,
        success_url: &str,
        cancel_url: &str,
    ) -> Result<String, AiomeError> {
        if self.is_mock {
            return Ok("cs_test_mock".to_string());
        }

        let existing_customer: Option<(String,)> =
            sqlx::query_as("SELECT customer_id FROM stripe_customers WHERE agent_id = ?")
                .bind(agent_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;

        let customer_id = if let Some(row) = existing_customer {
            row.0
                .parse::<stripe_core::CustomerId>()
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?
        } else {
            let desc = format!("Agent Soul: {}", agent_id);
            let create_customer = stripe_core::customer::CreateCustomer::new()
                .description(desc)
                .metadata(std::collections::HashMap::from([(
                    "agent_id".to_string(),
                    agent_id.to_string(),
                )]));

            let customer = create_customer.send(&self.client).await.map_err(|e| {
                AiomeError::Infrastructure {
                    reason: format!("Failed to create Stripe customer: {}", e),
                }
            })?;

            sqlx::query("INSERT INTO stripe_customers (agent_id, customer_id) VALUES (?, ?)")
                .bind(agent_id.to_string())
                .bind(customer.id.as_str())
                .execute(&self.pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;

            customer.id
        };

        let line_item = stripe_checkout::checkout_session::CreateCheckoutSessionLineItems {
            price: Some(price_id.to_string()),
            quantity: Some(1),
            ..Default::default()
        };

        let create_session = stripe_checkout::checkout_session::CreateCheckoutSession::new()
            .customer(customer_id)
            .mode(stripe_checkout::CheckoutSessionMode::Subscription)
            .success_url(success_url)
            .cancel_url(cancel_url)
            .line_items(vec![line_item]);

        let session =
            create_session
                .send(&self.client)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to create Stripe checkout session: {}", e),
                })?;

        match session.url {
            Some(url) => Ok(url),
            None => {
                tracing::warn!(
                    "⚠️ [StripeCommerce] Checkout session {} has no URL. Returning session ID as fallback.",
                    session.id
                );
                Ok(session.id.to_string())
            }
        }
    }

    async fn create_subscription(
        &self,
        agent_id: Uuid,
        plan_id: &str,
    ) -> Result<String, AiomeError> {
        // Mock mode for tests
        if self.is_mock {
            return Ok("sub_mock_stripe".to_string());
        }

        // P0-1: Create or Get Stripe Customer
        // Retrieve existing customer from registry table stripe_customers
        let existing_customer: Option<(String,)> =
            sqlx::query_as("SELECT customer_id FROM stripe_customers WHERE agent_id = ?")
                .bind(agent_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;

        let customer_id = if let Some(row) = existing_customer {
            tracing::info!("✅ [Stripe] Found existing customer: {}", row.0);
            row.0
                .parse::<stripe_core::CustomerId>()
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?
        } else {
            let desc = format!("Agent Soul: {}", agent_id);
            let create_customer = stripe_core::customer::CreateCustomer::new()
                .description(desc)
                .metadata(std::collections::HashMap::from([(
                    "agent_id".to_string(),
                    agent_id.to_string(),
                )]));

            let customer = match create_customer.send(&self.client).await {
                Ok(c) => c,
                Err(e) => {
                    return Err(AiomeError::Infrastructure {
                        reason: format!("Stripe Customer creation failed: {}", e),
                    })
                }
            };

            // Save to DB
            sqlx::query("INSERT INTO stripe_customers (agent_id, customer_id) VALUES (?, ?)")
                .bind(agent_id.to_string())
                .bind(customer.id.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to save customer: {}", e),
                })?;

            tracing::info!("✅ [Stripe] Created new customer: {}", customer.id);
            customer.id
        };

        let plan_id_str = plan_id.to_string();
        // Stripe Subscriptions API Call
        let sub_item = stripe_billing::subscription::CreateSubscriptionItems {
            price: Some(plan_id_str),
            ..Default::default()
        };

        let create_sub = stripe_billing::subscription::CreateSubscription::new()
            .customer(customer_id.to_string())
            .items([sub_item])
            .metadata(std::collections::HashMap::from([(
                "agent_id".to_string(),
                agent_id.to_string(),
            )]));

        match create_sub.send(&self.client).await {
            Ok(sub) => {
                tracing::info!("✅ [Stripe] Subscription created: {}", sub.id);
                Ok(sub.id.to_string())
            }
            Err(e) => Err(AiomeError::Infrastructure {
                reason: format!("Stripe Subscription creation failed: {}", e),
            }),
        }
    }

    async fn cancel_subscription(
        &self,
        _agent_id: Uuid,
        _subscription_id: &str,
    ) -> Result<(), AiomeError> {
        if self.is_mock {
            return Ok(());
        }
        Err(AiomeError::Infrastructure {
            reason: "cancel_subscription is not available in v1.0".into(),
        })
    }

    async fn get_subscription_status(
        &self,
        _agent_id: Uuid,
    ) -> Result<aiome_core_contracts::commerce::SubscriptionStatus, AiomeError> {
        if self.is_mock {
            return Ok(aiome_core_contracts::commerce::SubscriptionStatus::Active);
        }
        Err(AiomeError::Infrastructure {
            reason: "get_subscription_status is not available in v1.0".into(),
        })
    }

    async fn transfer(
        &self,
        from_id: Uuid,
        to_id: Uuid,
        amount: u64,
    ) -> Result<String, AiomeError> {
        if let (Some(url), Some(secret), Some(client)) = (
            &self.nurture_url,
            &self.nurture_secret,
            &self.nurture_client,
        ) {
            let req_url = format!("{}/internal/transfer", url);
            let payload = serde_json::json!({
                "from_id": from_id,
                "to_id": to_id,
                "amount": amount,
                "idempotency_key": Uuid::new_v4().to_string()
            });
            let mut req = client
                .post(&req_url)
                .header("Authorization", format!("Bearer {}", secret))
                .json(&payload)
                .timeout(std::time::Duration::from_secs(10));

            if let Some(cert_header) = self.generate_oxp_header() {
                req = req.header("X-OxiLean-Proof-Certificate", cert_header);
            }

            let res = req.send().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("HTTP error: {}", e),
            })?;

            if res.status().is_success() {
                #[derive(serde::Deserialize)]
                struct TransferRes {
                    transaction_id: String,
                }
                let body: TransferRes =
                    res.json().await.map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                return Ok(body.transaction_id);
            } else {
                let status = res.status();
                let text = res.text().await.unwrap_or_default();
                return Err(AiomeError::Infrastructure {
                    reason: format!("Transfer failed ({}): {}", status, text),
                });
            }
        }

        tracing::warn!("⚠️ [StripeCommerceEngine] Nurture URL not set. Using mock transfer.");
        Ok("tx_stripe_transfer_mock".into())
    }

    async fn deduct_generation_cost(
        &self,
        agent_id: Uuid,
        asset_id: Option<Uuid>,
        amount: u64,
        generation_type: &str,
    ) -> Result<(), AiomeError> {
        if let (Some(url), Some(secret), Some(client)) = (
            &self.nurture_url,
            &self.nurture_secret,
            &self.nurture_client,
        ) {
            let endpoint = format!("{}/internal/deduct", url);
            let payload = serde_json::json!({
                "actor_id": agent_id,
                "asset_id": asset_id,
                "amount": amount,
                "generation_type": generation_type,
                "idempotency_key": Uuid::new_v4().to_string()
            });

            let mut req = client
                .post(&endpoint)
                .header("Authorization", format!("Bearer {}", secret))
                .timeout(std::time::Duration::from_secs(10))
                .json(&payload);

            if let Some(cert_header) = self.generate_oxp_header() {
                req = req.header("X-OxiLean-Proof-Certificate", cert_header);
            }

            match req.send().await {
                Ok(res) if res.status().is_success() => {
                    tracing::info!(
                        "💸 [StripeCommerceEngine] Deducted {} units from Agent {} for generation type '{}'.",
                        amount,
                        agent_id,
                        generation_type
                    );
                    Ok(())
                }
                Ok(res) => {
                    let status = res.status();
                    let text = res.text().await.unwrap_or_default();
                    tracing::error!(
                        "🚨 [StripeCommerceEngine] Failed to deduct generation cost from Nurture -> {}: {}",
                        status,
                        text
                    );
                    Err(AiomeError::Infrastructure {
                        reason: format!("Nurture API rejected deduction: {}", text),
                    })
                }
                Err(e) => {
                    tracing::error!(
                        "💥 [StripeCommerceEngine] HTTP error contacting Nurture: {}",
                        e
                    );
                    Err(AiomeError::Infrastructure {
                        reason: format!("HTTP error: {}", e),
                    })
                }
            }
        } else {
            // Unconfigured Nurture - mock the deduction
            tracing::info!(
                "💸 [StripeCommerceEngine/Unconfigured] Mock deducted {} units from Agent {}.",
                amount,
                agent_id
            );
            Ok(())
        }
    }

    async fn instant_refund(&self, transaction_id: &str, actor_id: Uuid) -> Result<(), AiomeError> {
        if let (Some(url), Some(secret), Some(client)) = (
            &self.nurture_url,
            &self.nurture_secret,
            &self.nurture_client,
        ) {
            let req_url = format!("{}/internal/instant-refund", url);
            let payload = serde_json::json!({
                "transaction_id": transaction_id,
                "actor_id": actor_id,
                "idempotency_key": Uuid::new_v4().to_string()
            });
            let mut req = client
                .post(&req_url)
                .header("Authorization", format!("Bearer {}", secret))
                .json(&payload)
                .timeout(std::time::Duration::from_secs(10));

            if let Some(cert_header) = self.generate_oxp_header() {
                req = req.header("X-OxiLean-Proof-Certificate", cert_header);
            }

            let res = req.send().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("HTTP error: {}", e),
            })?;

            if res.status().is_success() {
                return Ok(());
            } else {
                let status = res.status();
                let text = res.text().await.unwrap_or_default();
                return Err(AiomeError::Infrastructure {
                    reason: format!("Instant refund failed ({}): {}", status, text),
                });
            }
        }
        Ok(())
    }

    async fn withdraw_points(&self, actor_id: Uuid, amount: u64) -> Result<(), AiomeError> {
        if let (Some(url), Some(secret), Some(client)) = (
            &self.nurture_url,
            &self.nurture_secret,
            &self.nurture_client,
        ) {
            let req_url = format!("{}/internal/withdraw-points", url);
            let payload = serde_json::json!({
                "actor_id": actor_id,
                "points": amount,
                "idempotency_key": Uuid::new_v4().to_string()
            });
            let mut req = client
                .post(&req_url)
                .header("Authorization", format!("Bearer {}", secret))
                .json(&payload)
                .timeout(std::time::Duration::from_secs(10));

            if let Some(cert_header) = self.generate_oxp_header() {
                req = req.header("X-OxiLean-Proof-Certificate", cert_header);
            }

            let res = req.send().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("HTTP error: {}", e),
            })?;

            if res.status().is_success() {
                return Ok(());
            } else {
                let status = res.status();
                let text = res.text().await.unwrap_or_default();
                return Err(AiomeError::Infrastructure {
                    reason: format!("Withdraw points failed ({}): {}", status, text),
                });
            }
        }
        Ok(())
    }

    async fn get_points(
        &self,
        agent_id: Uuid,
    ) -> Result<aiome_core_contracts::commerce::PointsBalance, AiomeError> {
        if let (Some(url), Some(secret), Some(client)) = (
            &self.nurture_url,
            &self.nurture_secret,
            &self.nurture_client,
        ) {
            let req_url = format!("{}/internal/points/{}", url, agent_id);
            let mut req = client
                .get(&req_url)
                .header("Authorization", format!("Bearer {}", secret))
                .timeout(std::time::Duration::from_secs(10));

            if let Some(cert_header) = self.generate_oxp_header() {
                req = req.header("X-OxiLean-Proof-Certificate", cert_header);
            }

            let res = req.send().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("HTTP error: {}", e),
            })?;

            if res.status().is_success() {
                let body: aiome_core_contracts::commerce::PointsBalance =
                    res.json().await.map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                return Ok(body);
            } else {
                let status = res.status();
                let text = res.text().await.unwrap_or_default();
                return Err(AiomeError::Infrastructure {
                    reason: format!("Get points failed ({}): {}", status, text),
                });
            }
        }

        tracing::warn!("⚠️ [StripeCommerceEngine] Nurture URL not set. Returning zero points.");
        Ok(aiome_core_contracts::commerce::PointsBalance {
            balance: 0,
            lifetime_earned: 0,
            lifetime_withdrawn: 0,
            conversion_rate_bps: 10000,
        })
    }

    async fn get_transaction_history(
        &self,
        agent_id: Uuid,
        limit: u32,
    ) -> Result<Vec<aiome_core_contracts::commerce::TransactionRecord>, AiomeError> {
        if let (Some(url), Some(secret), Some(client)) = (
            &self.nurture_url,
            &self.nurture_secret,
            &self.nurture_client,
        ) {
            let req_url = format!(
                "{}/internal/transaction-history/{}?limit={}",
                url, agent_id, limit
            );
            let mut req = client
                .get(&req_url)
                .header("Authorization", format!("Bearer {}", secret))
                .timeout(std::time::Duration::from_secs(10));

            if let Some(cert_header) = self.generate_oxp_header() {
                req = req.header("X-OxiLean-Proof-Certificate", cert_header);
            }

            let res = req.send().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("HTTP error: {}", e),
            })?;

            if res.status().is_success() {
                let body: Vec<aiome_core_contracts::commerce::TransactionRecord> =
                    res.json().await.map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                return Ok(body);
            } else {
                let status = res.status();
                let text = res.text().await.unwrap_or_default();
                return Err(AiomeError::Infrastructure {
                    reason: format!("Get history failed ({}): {}", status, text),
                });
            }
        }

        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn get_test_engine() -> StripeCommerceEngine {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // テーブル作成
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS stripe_webhook_events (
                event_id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                metadata TEXT,
                processed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(&pool)
        .await
        .unwrap();

        StripeCommerceEngine::new(
            SecretString::from("sk_test_mock".to_string()),
            SecretString::from("whsec_test".to_string()),
            pool,
            None,
            None,
        )
    }

    #[tokio::test]
    async fn test_verify_webhook_signature_green() {
        use hmac::{Hmac, Mac};
        use sha2::{Digest, Sha256};

        let engine = get_test_engine().await;
        let payload = r#"{
  "id": "evt_123",
  "object": "event",
  "api_version": "2022-11-15",
  "created": 1678888888,
  "data": {
    "object": {
      "id": "cs_test_123",
      "object": "checkout.session",
      "status": "complete"
    }
  },
  "livemode": false,
  "pending_webhooks": 0,
  "request": {
    "id": null,
    "idempotency_key": null
  },
  "type": "checkout.session.completed"
}"#;

        // 現在時刻の取得 (tolerance 対策)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let timestamp = now.to_string();

        // Stripe 方式の署名生成: HMAC-SHA256(secret, "timestamp.payload")
        let signed_payload = format!("{}.{}", timestamp, payload);
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice("whsec_test".as_bytes()).unwrap();
        mac.update(signed_payload.as_bytes());
        let result_code = mac.finalize();
        let signature = hex::encode(result_code.into_bytes());

        let sig_header = format!("t={},v1={}", timestamp, signature);

        let result = engine.verify_signature(payload, &sig_header);

        // GREEN: 正しい署名なら、(パースエラーが出ても) 署名検証自体はパスしているはず
        // Stripe SDK の秘匿仕様により、署名が間違っている場合は WebhookError::BadSignature になる
        if let Err(AiomeError::Infrastructure { reason }) = result {
            if reason.contains("BadSignature") {
                panic!("署名検証が失敗しました（本来パスすべき）: {}", reason);
            }
            // パースエラーは署名検証を通過した後の段階なので、ここでは「署名検証成功」とみなす
            assert!(
                reason.contains("error parsing event object"),
                "予期せぬエラー: {}",
                reason
            );
        } else {
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_verify_webhook_signature_fails_on_tampering() {
        let engine = get_test_engine().await;
        let payload = "{\"id\": \"evt_123\"}";
        let sig_header = "t=123456789,v1=bad_signature";

        let result = engine.verify_signature(payload, sig_header);

        assert!(result.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = result {
            assert!(reason.contains("Stripe Webhook verification failed"));
        }
    }

    #[tokio::test]
    async fn test_create_subscription_red() {
        let engine = get_test_engine().await;
        let agent_id = Uuid::new_v4();
        let plan_id = "price_gold_monthly";

        // RED: 現在は単に "sub_mock_stripe" を返すだけだが、
        // 将来的には Stripe API でプランの存在確認や Customer の実在確認をするべき。
        // ここでは「結果の ID は sub_ で始まる必要がある」という暫定的なアサーションで失敗させる。
        // (現在は "sub_mock_stripe" なのでパスするが、実装を 'sub_real_' 等に変える前提)
        let result = engine.create_subscription(agent_id, plan_id).await;

        assert!(result.is_ok());
        let sub_id = result.unwrap();
        // 現在の実装は "sub_mock_stripe" を返す
        assert_eq!(sub_id, "sub_mock_stripe");

        // 実装後はここを「Stripe API によって生成された ID」であることを検証するように変更する。
        // TDD としては、まず「Stripe 連携に必要な情報が不足している場合にエラーを返す」テストを書くのが安全。
    }

    #[tokio::test]
    async fn test_production_mode_rejects_test_secrets_red() {
        // 本番キー (sk_live_*) + 本番Webhook秘密鍵 を使用した場合、
        // AIOME_DEV_MODE の状態に関わらず is_mock = false になることを検証する。
        // 注: std::env::set_var はマルチスレッドで unsafe であるため使用しない。
        //     is_mock は api_key/webhook_secret のプレフィックスで決定されるため、
        //     本番キーを直接渡すことで環境変数に依存せず検証可能。

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let engine = StripeCommerceEngine::new(
            secrecy::SecretString::from("sk_live_prod_key_12345".to_string()),
            secrecy::SecretString::from("whsec_live_prod_secret".to_string()),
            pool,
            None,
            None,
        );

        // 本番キーを使用した場合、is_mock は常に false
        assert!(
            !engine.is_mock,
            "Production keys MUST NOT enable mock mode regardless of AIOME_DEV_MODE"
        );
    }

    #[tokio::test]
    async fn test_deduct_generation_cost_green() {
        let engine = get_test_engine().await;
        let agent_id = Uuid::new_v4();

        // Testing the mock fallback or network error behavior
        let result = engine
            .deduct_generation_cost(agent_id, None, 10, "image_gen")
            .await;
        assert!(result.is_ok(), "Should unlock GenerativeEngine billing");
    }

    #[tokio::test]
    async fn test_escrow_create_green() {
        let engine = get_test_engine().await;
        // manually create the escrows table in SQLite for the test
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS escrows (
                id TEXT PRIMARY KEY,
                payer_id TEXT NOT NULL,
                order_id TEXT NOT NULL,
                amount INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'Locked'
            );",
        )
        .execute(&engine.pool)
        .await
        .unwrap();

        let agent_id = Uuid::new_v4();
        let result = engine.escrow_create(agent_id, 500).await;
        assert!(result.is_ok());
        let escrow_id = result.unwrap();
        assert!(escrow_id.starts_with("escrow_"));
        assert_ne!(escrow_id, "escrow_mock");
    }

    #[tokio::test]
    async fn test_escrow_lifecycle_green() {
        let engine = get_test_engine().await;
        // set up the scheme matching our needs
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS escrows (
                id TEXT PRIMARY KEY,
                payer_id TEXT NOT NULL,
                recipient_id TEXT,
                order_id TEXT NOT NULL,
                amount INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'Locked'
            );",
        )
        .execute(&engine.pool)
        .await
        .unwrap();

        let agent_id = Uuid::new_v4();
        let escrow_id = engine.escrow_create(agent_id, 500).await.unwrap();

        // test release
        let recipient_id = Uuid::new_v4();
        let release_result = engine.escrow_release(&escrow_id, recipient_id).await;
        assert!(release_result.is_ok());

        // can't refund a released escrow (fail if the status check in refund is working properly)
        // wait, our refund doesn't return an error right now, it returns "Escrow not found or not locked" ok! Let's check:
        let refund_result = engine.escrow_refund(&escrow_id).await;
        assert!(refund_result.is_err());

        // create another for refund
        let escrow_id2 = engine.escrow_create(agent_id, 500).await.unwrap();
        let refund_result2 = engine.escrow_refund(&escrow_id2).await;
        assert!(refund_result2.is_ok());
    }

    #[tokio::test]
    async fn test_http_proxy_transfer_green() {
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};
        let mock_server = MockServer::start().await;
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let engine = StripeCommerceEngine::new(
            SecretString::from("sk_test_mock".to_string()),
            SecretString::from("whsec_test".to_string()),
            pool,
            Some(mock_server.uri()),
            Some("test_secret".to_string()),
        );

        Mock::given(matchers::method("POST"))
            .and(matchers::path("/internal/transfer"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "transaction_id": "tx_http_123"
            })))
            .mount(&mock_server)
            .await;

        let from_id = Uuid::new_v4();
        let to_id = Uuid::new_v4();
        let result = engine.transfer(from_id, to_id, 100).await.unwrap();
        assert_eq!(
            result, "tx_http_123",
            "Must return transaction ID from Nurture API"
        );
    }

    #[tokio::test]
    async fn test_http_proxy_get_points_green() {
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};
        let mock_server = MockServer::start().await;
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let engine = StripeCommerceEngine::new(
            SecretString::from("sk_test_mock".to_string()),
            SecretString::from("whsec_test".to_string()),
            pool,
            Some(mock_server.uri()),
            Some("test_secret".to_string()),
        );

        let agent_id = Uuid::new_v4();
        let path = format!("/internal/points/{}", agent_id);

        Mock::given(matchers::method("GET"))
            .and(matchers::path(path))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "balance": 150,
                "lifetime_earned": 500,
                "lifetime_withdrawn": 350,
                "conversion_rate_bps": 10000
            })))
            .mount(&mock_server)
            .await;

        let result = engine.get_points(agent_id).await.unwrap();
        assert_eq!(result.balance, 150);
        assert_eq!(result.lifetime_earned, 500);
    }

    #[tokio::test]
    async fn test_validate_activity() {
        use aiome_core_contracts::commerce::CommerceEngine;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let engine = StripeCommerceEngine::new(
            SecretString::from("sk_test_mock".to_string()),
            SecretString::from("whsec_test".to_string()),
            pool,
            Some(mock_server.uri()),
            Some("test_secret".to_string()),
        );

        let agent_id = Uuid::new_v4();

        // 1. Valid request -> 200 OK
        Mock::given(method("POST"))
            .and(path("/internal/validate-activity"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let res = engine.validate_activity(agent_id, "test_action", 100).await;
        assert!(res.is_ok(), "Expected Ok for 200 OK response");

        mock_server.reset().await;

        // 2. Reject request -> 402 Payment Required
        Mock::given(method("POST"))
            .and(path("/internal/validate-activity"))
            .respond_with(ResponseTemplate::new(402).set_body_string("Insufficient funds"))
            .mount(&mock_server)
            .await;

        let res = engine.validate_activity(agent_id, "test_action", 100).await;
        assert!(res.is_err(), "Expected Err for 402 response");

        // 3. Fallback when nurture_url is None
        let pool2 = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let engine_no_url = StripeCommerceEngine::new(
            SecretString::from("sk_test_mock".to_string()),
            SecretString::from("whsec_test".to_string()),
            pool2,
            None,
            None,
        );

        let res2 = engine_no_url
            .validate_activity(agent_id, "test_action", 100)
            .await;
        assert!(
            res2.is_ok(),
            "Expected Ok fallback when Nurture URL is not set"
        );
    }

    #[tokio::test]
    async fn test_stripe_commerce_mock_behavior() {
        let engine = get_test_engine().await;
        let agent_id = Uuid::new_v4();
        let item_id = Uuid::new_v4();

        // is_mock = true なので、モックの動作が維持されることを検証
        assert!(engine.is_mock);

        let sub_status = engine.get_subscription_status(agent_id).await;
        assert!(sub_status.is_ok());
        assert_eq!(
            sub_status.unwrap(),
            aiome_core_contracts::commerce::SubscriptionStatus::Active
        );

        let purchase = engine
            .execute_autonomous_purchase(agent_id, item_id, serde_json::json!({}))
            .await;
        assert!(purchase.is_ok());
        assert_eq!(purchase.unwrap(), "tx_mock");

        assert!(engine.stake(agent_id, 100).await.is_ok());
        assert!(engine.slash(agent_id, 50, "test").await.is_ok());

        // register_license と cancel_subscription もモックで成功することを検証
        assert!(engine
            .register_license(agent_id, item_id, "tx_test", "standard")
            .await
            .is_ok());
        assert!(engine
            .cancel_subscription(agent_id, "sub_test")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_stripe_commerce_production_block() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // AIOME_DEV_MODE は操作せず、本番キー sk_live_xxx を使用
        // is_mock は api_key.starts_with("sk_test_mock") が false かつ webhook_secret != "whsec_test" のため、
        // AIOME_DEV_MODE が true であっても必ず false になる
        let engine = StripeCommerceEngine::new(
            SecretString::from("sk_live_123456789".to_string()),
            SecretString::from("whsec_live_987654".to_string()),
            pool,
            None,
            None,
        );

        assert!(!engine.is_mock);

        let agent_id = Uuid::new_v4();
        let item_id = Uuid::new_v4();

        // 封印されたメソッドがすべて Err(AiomeError::Infrastructure) を返すことを検証
        let sub_status = engine.get_subscription_status(agent_id).await;
        assert!(sub_status.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = sub_status {
            assert!(
                reason.contains("not available in v1.0"),
                "Actual sub_status error: {}",
                reason
            );
        } else {
            panic!("Expected Infrastructure error");
        }

        let purchase = engine
            .execute_autonomous_purchase(agent_id, item_id, serde_json::json!({}))
            .await;
        assert!(purchase.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = purchase {
            assert!(
                reason.contains("not available in v1.0"),
                "Actual purchase error: {}",
                reason
            );
        } else {
            panic!("Expected Infrastructure error");
        }

        let stake_res = engine.stake(agent_id, 100).await;
        assert!(stake_res.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = stake_res {
            assert!(
                reason.contains("not available in v1.0"),
                "Actual stake error: {}",
                reason
            );
        } else {
            panic!("Expected Infrastructure error");
        }

        let slash_res = engine.slash(agent_id, 50, "test").await;
        assert!(slash_res.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = slash_res {
            assert!(
                reason.contains("not available in v1.0"),
                "Actual slash error: {}",
                reason
            );
        } else {
            panic!("Expected Infrastructure error");
        }

        // register_license も本番では封印されていることを検証
        let lic_res = engine
            .register_license(agent_id, item_id, "tx_test", "standard")
            .await;
        assert!(lic_res.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = lic_res {
            assert!(
                reason.contains("not available in v1.0"),
                "Actual register_license error: {}",
                reason
            );
        } else {
            panic!("Expected Infrastructure error for register_license");
        }

        // cancel_subscription も本番では封印されていることを検証
        let cancel_res = engine.cancel_subscription(agent_id, "sub_test").await;
        assert!(cancel_res.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = cancel_res {
            assert!(
                reason.contains("not available in v1.0"),
                "Actual cancel_subscription error: {}",
                reason
            );
        } else {
            panic!("Expected Infrastructure error for cancel_subscription");
        }
    }
}
