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

#[cfg(test)]
mod tests;

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

fn map_stripe_status(
    status: stripe_billing::SubscriptionStatus,
) -> aiome_core_contracts::commerce::SubscriptionStatus {
    use aiome_core_contracts::commerce::SubscriptionStatus as TargetStatus;
    use stripe_billing::SubscriptionStatus as StripeStatus;

    match status {
        StripeStatus::Active => TargetStatus::Active,
        StripeStatus::Canceled => TargetStatus::Cancelled,
        StripeStatus::PastDue => TargetStatus::PastDue,
        StripeStatus::Trialing => TargetStatus::Trialing,
        StripeStatus::Unpaid => TargetStatus::Unpaid,
        StripeStatus::Incomplete => TargetStatus::Incomplete,
        StripeStatus::IncompleteExpired => TargetStatus::IncompleteExpired,
        _ => TargetStatus::None,
    }
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

trait IntoInfraError<T> {
    fn map_infra_err(self) -> Result<T, AiomeError>;
    fn map_infra_err_context(self, context: &str) -> Result<T, AiomeError>;
}

impl<T, E: std::fmt::Display> IntoInfraError<T> for Result<T, E> {
    fn map_infra_err(self) -> Result<T, AiomeError> {
        self.map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })
    }

    fn map_infra_err_context(self, context: &str) -> Result<T, AiomeError> {
        self.map_err(|e| AiomeError::Infrastructure {
            reason: format!("{}: {}", context, e),
        })
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

            let res = req.send().await.map_infra_err()?;

            if res.status().is_success() {
                let body: BalanceRes = res.json().await.map_infra_err()?;
                return Ok(body.balance);
            }
        }
        tracing::warn!("⚠️ [Billing] Nurture URL not configured or non-success response. Returning zero balance for agent.");
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
                    let status = res.status();
                    tracing::error!(
                        "🚨 [Billing] validate_activity got unexpected status {} for agent {}. Blocking (fail-closed).",
                        status,
                        agent_id
                    );
                    return Err(AiomeError::Infrastructure {
                        reason: format!(
                            "Billing validation failed with unexpected status: {}",
                            status
                        ),
                    });
                }
                Err(e) => {
                    tracing::error!(
                        "❌ [Billing] validate_activity network error for agent {}: {}. Blocking (fail-closed).",
                        agent_id,
                        e
                    );
                    return Err(AiomeError::Infrastructure {
                        reason: format!("Billing validation network error: {}", e),
                    });
                }
            }
        }
        Ok(())
    }

    async fn execute_autonomous_purchase(
        &self,
        agent_id: Uuid,
        item_id: Uuid,
        _metadata: serde_json::Value,
    ) -> Result<String, AiomeError> {
        if self.is_mock {
            return Ok("tx_mock".into());
        }

        if let (Some(url), Some(secret), Some(client)) = (
            &self.nurture_url,
            &self.nurture_secret,
            &self.nurture_client,
        ) {
            let req_url = format!("{}/internal/purchase", url);
            let payload = serde_json::json!({
                "buyer": agent_id,
                "item_id": item_id,
                "idempotency_key": format!("auto_{}_{}", agent_id, item_id),
            });

            let mut req = client
                .post(&req_url)
                .header("Authorization", format!("Bearer {}", secret))
                .json(&payload)
                .timeout(std::time::Duration::from_secs(10));

            if let Some(cert_header) = self.generate_oxp_header() {
                req = req.header("x-oxilean-proof-certificate", cert_header);
            }

            let res = req.send().await;
            match res {
                Ok(resp) => {
                    if resp.status().is_success() {
                        #[derive(serde::Deserialize)]
                        struct LocalPurchaseResponse {
                            transaction_id: String,
                        }

                        match resp.json::<LocalPurchaseResponse>().await {
                            Ok(body) => return Ok(body.transaction_id),
                            Err(e) => {
                                return Err(AiomeError::Infrastructure {
                                    reason: format!(
                                        "Failed to deserialize Nurture purchase response: {:?}",
                                        e
                                    ),
                                })
                            }
                        }
                    } else {
                        let status = resp.status();
                        let text = match resp.text().await {
                            Ok(t) => t,
                            Err(e) => {
                                tracing::warn!(
                                    "⚠️ [Billing] Failed to read error response body: {:?}",
                                    e
                                );
                                String::new()
                            }
                        };
                        return Err(AiomeError::Infrastructure {
                            reason: format!(
                                "Nurture purchase S2S failed with status [{}]: {}",
                                status, text
                            ),
                        });
                    }
                }
                Err(e) => {
                    return Err(AiomeError::Infrastructure {
                        reason: format!("Nurture purchase S2S request failed: {:?}", e),
                    });
                }
            }
        }

        Err(AiomeError::Infrastructure {
            reason: "Nurture S2S URL not configured".into(),
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

            let res = req.send().await.map_infra_err()?;

            if res.status().is_success() {
                let body: DailyStatsRes = res.json().await.map_infra_err()?;
                return Ok(body.spent_today);
            }
        }
        tracing::warn!("⚠️ [Billing] Nurture URL not configured or non-success response. Returning zero daily spend.");
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

            let res = req.send().await.map_infra_err()?;

            if res.status().is_success() {
                let body: DailyStatsRes = res.json().await.map_infra_err()?;
                return Ok(body.daily_limit);
            }
        }
        tracing::warn!("⚠️ [Billing] Nurture URL not configured or non-success response. Returning default daily limit (100).");
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

            let res = req.send().await.map_infra_err()?;

            if res.status().is_success() {
                #[derive(serde::Deserialize)]
                struct EscrowRes {
                    escrow_id: String,
                }
                let body: EscrowRes = res.json().await.map_infra_err()?;
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
                    Err(e).map_infra_err_context("DB insertion failed")
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

            let res = req.send().await.map_infra_err()?;

            if res.status().is_success() {
                let records: Vec<EscrowRecord> = res
                    .json()
                    .await
                    .map_infra_err_context("Failed to parse escrow list")?;
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
        .map_infra_err_context("Failed to fetch escrows")?;

        let records = rows
            .into_iter()
            .map(|row| -> Result<EscrowRecord, AiomeError> {
                Ok(EscrowRecord {
                    id: row
                        .try_get("id")
                        .map_infra_err_context("Failed to read escrow.id")?,
                    payer_id: row
                        .try_get("payer_id")
                        .map_infra_err_context("Failed to read escrow.payer_id")?,
                    order_id: row
                        .try_get("order_id")
                        .map_infra_err_context("Failed to read escrow.order_id")?,
                    amount: row
                        .try_get("amount")
                        .map_infra_err_context("Failed to read escrow.amount")?,
                    status: row
                        .try_get("status")
                        .map_infra_err_context("Failed to read escrow.status")?,
                    created_at: row
                        .try_get("created_at")
                        .map_infra_err_context("Failed to read escrow.created_at")?,
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

            let res = req.send().await.map_infra_err()?;

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
                Err(e).map_infra_err_context("Failed to release escrow")
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

            let res = req.send().await.map_infra_err()?;

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
                Err(e).map_infra_err_context("Failed to refund escrow")
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

            let customer = create_customer
                .send(&self.client)
                .await
                .map_infra_err_context("Failed to create Stripe customer")?;

            sqlx::query("INSERT INTO stripe_customers (agent_id, customer_id) VALUES (?, ?)")
                .bind(agent_id.to_string())
                .bind(customer.id.as_str())
                .execute(&self.pool)
                .await
                .map_infra_err()?;

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

        let session = create_session
            .send(&self.client)
            .await
            .map_infra_err_context("Failed to create Stripe checkout session")?;

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

        // Retrieve existing customer from registry table stripe_customers
        let existing_customer: Option<(String,)> =
            sqlx::query_as("SELECT customer_id FROM stripe_customers WHERE agent_id = ?")
                .bind(agent_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_infra_err()?;

        let customer_id = if let Some(row) = existing_customer {
            tracing::info!("✅ [Stripe] Found existing customer: {}", row.0);
            row.0.parse::<stripe_core::CustomerId>().map_infra_err()?
        } else {
            let desc = format!("Agent Soul: {}", agent_id);
            let create_customer = stripe_core::customer::CreateCustomer::new()
                .description(desc)
                .metadata(std::collections::HashMap::from([(
                    "agent_id".to_string(),
                    agent_id.to_string(),
                )]));

            let customer = create_customer
                .send(&self.client)
                .await
                .map_infra_err_context("Stripe Customer creation failed")?;

            // Save to DB
            sqlx::query("INSERT INTO stripe_customers (agent_id, customer_id) VALUES (?, ?)")
                .bind(agent_id.to_string())
                .bind(customer.id.to_string())
                .execute(&self.pool)
                .await
                .map_infra_err_context("Failed to save customer")?;

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

        create_sub
            .send(&self.client)
            .await
            .map_infra_err_context("Stripe Subscription creation failed")
            .map(|sub| {
                tracing::info!("✅ [Stripe] Subscription created: {}", sub.id);
                sub.id.to_string()
            })
    }
    async fn cancel_subscription(
        &self,
        _agent_id: Uuid,
        subscription_id: &str,
    ) -> Result<(), AiomeError> {
        if self.is_mock {
            return Ok(());
        }

        let sub_id = subscription_id
            .parse::<stripe_billing::SubscriptionId>()
            .map_infra_err_context("Invalid subscription ID format")?;

        stripe_billing::subscription::CancelSubscription::new(sub_id)
            .send(&self.client)
            .await
            .map_infra_err_context("Stripe cancel subscription failed")?;

        tracing::info!("✅ [Stripe] Subscription cancelled: {}", subscription_id);
        Ok(())
    }

    async fn get_subscription_status(
        &self,
        agent_id: Uuid,
    ) -> Result<aiome_core_contracts::commerce::SubscriptionStatus, AiomeError> {
        if self.is_mock {
            return Ok(aiome_core_contracts::commerce::SubscriptionStatus::Active);
        }

        let customer_id: Option<(String,)> =
            sqlx::query_as("SELECT customer_id FROM stripe_customers WHERE agent_id = ?")
                .bind(agent_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_infra_err_context("DB lookup failed for customer")?;

        let Some((customer_id_str,)) = customer_id else {
            return Ok(aiome_core_contracts::commerce::SubscriptionStatus::None);
        };

        let cust_id = customer_id_str
            .parse::<stripe_core::CustomerId>()
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Invalid customer ID format: {}", e),
            })?;
        let list_params = stripe_billing::subscription::ListSubscription::new().customer(cust_id);

        let list_res = list_params
            .send(&self.client)
            .await
            .map_infra_err_context("Stripe list subscriptions failed")?;

        if let Some(sub) = list_res.data.first() {
            Ok(map_stripe_status(sub.status.clone()))
        } else {
            Ok(aiome_core_contracts::commerce::SubscriptionStatus::None)
        }
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

            let res = req.send().await.map_infra_err_context("HTTP error")?;

            if res.status().is_success() {
                #[derive(serde::Deserialize)]
                struct TransferRes {
                    transaction_id: String,
                }
                let body: TransferRes = res.json().await.map_infra_err()?;
                return Ok(body.transaction_id);
            } else {
                let status = res.status();
                let text = match res.text().await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("⚠️ [Billing] Failed to read transfer error body: {:?}", e);
                        String::new()
                    }
                };
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
                    let text = match res.text().await {
                        Ok(t) => t,
                        Err(e) => {
                            tracing::warn!(
                                "⚠️ [Billing] Failed to read error response body: {:?}",
                                e
                            );
                            String::new()
                        }
                    };
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

            let res = req.send().await.map_infra_err_context("HTTP error")?;

            if res.status().is_success() {
                return Ok(());
            } else {
                let status = res.status();
                let text = match res.text().await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("⚠️ [Billing] Failed to read error response body: {:?}", e);
                        String::new()
                    }
                };
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

            let res = req.send().await.map_infra_err_context("HTTP error")?;

            if res.status().is_success() {
                return Ok(());
            } else {
                let status = res.status();
                let text = match res.text().await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("⚠️ [Billing] Failed to read error response body: {:?}", e);
                        String::new()
                    }
                };
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

            let res = req.send().await.map_infra_err_context("HTTP error")?;

            if res.status().is_success() {
                let body: aiome_core_contracts::commerce::PointsBalance =
                    res.json().await.map_infra_err()?;
                return Ok(body);
            } else {
                let status = res.status();
                let text = match res.text().await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("⚠️ [Billing] Failed to read error response body: {:?}", e);
                        String::new()
                    }
                };
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

            let res = req.send().await.map_infra_err_context("HTTP error")?;

            if res.status().is_success() {
                let body: Vec<aiome_core_contracts::commerce::TransactionRecord> =
                    res.json().await.map_infra_err()?;
                return Ok(body);
            } else {
                let status = res.status();
                let text = match res.text().await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("⚠️ [Billing] Failed to read error response body: {:?}", e);
                        String::new()
                    }
                };
                return Err(AiomeError::Infrastructure {
                    reason: format!("Get history failed ({}): {}", status, text),
                });
            }
        }

        Ok(vec![])
    }

    async fn create_portal_session(
        &self,
        agent_id: Uuid,
        return_url: &str,
    ) -> Result<String, AiomeError> {
        if self.is_mock {
            return Ok("https://example.com/portal-session-mock".to_string());
        }

        let customer_id: Option<(String,)> =
            sqlx::query_as("SELECT customer_id FROM stripe_customers WHERE agent_id = ?")
                .bind(agent_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_infra_err_context("DB lookup failed for customer")?;

        let Some((customer_id_str,)) = customer_id else {
            return Err(AiomeError::NotFound {
                reason: format!("Stripe customer not found for agent: {}", agent_id),
            });
        };

        let cust_id = customer_id_str
            .parse::<stripe_core::CustomerId>()
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Invalid customer ID format: {}", e),
            })?;

        let session = stripe_billing::billing_portal_session::CreateBillingPortalSession::new()
            .customer(cust_id)
            .return_url(return_url)
            .locale(stripe_billing::BillingPortalSessionLocale::Ja)
            .send(&self.client)
            .await
            .map_infra_err_context("Stripe billing portal session creation failed")?;

        Ok(session.url)
    }
}
