/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use infrastructure::db::DatabasePool;
use tracing::{error, info, Instrument};
use uuid::Uuid;

/// Nurture `/internal/coin-charge` へ 1 回だけ POST する（DLQ 書込なし）。
pub async fn attempt_coin_charge_once(
    http_client: &reqwest::Client,
    nurture_url: &str,
    secret: &str,
    oxilean_power: u32,
    payload: &serde_json::Value,
) -> Result<(), String> {
    let req_url = format!("{}/internal/coin-charge", nurture_url.trim_end_matches('/'));
    let mut req = http_client
        .post(&req_url)
        .header("Authorization", format!("Bearer {secret}"))
        .timeout(std::time::Duration::from_secs(30))
        .json(payload);
    let Some(cert) = aiome_core_contracts::oxilean::OxiLeanProofCertificate::generate_header(
        "aiome-edge-node",
        oxilean_power,
        secret,
    ) else {
        return Err(format!(
            "oxp_header_generation_failed for {req_url}; request denied (fail-closed)"
        ));
    };
    req = req.header("X-OxiLean-Proof-Certificate", cert);
    let res = req.send().await.map_err(|e| format!("network: {e}"))?;
    if res.status().is_success() {
        Ok(())
    } else {
        Err(format!("http {}", res.status()))
    }
}

// auth-exempt: Helper function (Not an endpoint)
#[allow(clippy::too_many_arguments)]
pub async fn enqueue_coin_charge_to_nurture(
    http_client: reqwest::Client,
    dlq_pool: std::sync::Arc<DatabasePool>,
    nurture_url: Option<String>,
    nurture_secret: Option<String>,
    agent_uuid: Uuid,
    amount: u64,
    ev_id: String,
    oxilean_power: std::sync::Arc<std::sync::atomic::AtomicU32>,
) {
    if let (Some(url), Some(secret)) = (nurture_url, nurture_secret) {
        let coin_charge_span =
            tracing::info_span!("coin_charge_relay", agent = %agent_uuid, event = %ev_id);
        tokio::spawn(
            async move {
                let payload = serde_json::json!({
                    "actor_id": agent_uuid,
                    "amount": amount,
                    "currency": "coin",
                    "stripe_event_id": ev_id,
                    "idempotency_key": ev_id
                });

                let mut retry_count = 0;
                let mut delay = std::time::Duration::from_secs(1);
                loop {
                    let oxp = oxilean_power.load(std::sync::atomic::Ordering::Relaxed);
                    match attempt_coin_charge_once(
                        &http_client,
                        &url,
                        &secret,
                        oxp,
                        &payload,
                    )
                    .await
                    {
                        Ok(()) => {
                            info!(
                                "🪙 [StripeWebhook] Coin charge succeeded for {}",
                                agent_uuid
                            );
                            break;
                        }
                        Err(e) => {
                            error!("❌ [StripeWebhook] Coin charge attempt failed: {}", e);
                        }
                    }

                    retry_count += 1;
                    if retry_count >= 3 {
                        error!("🚨 [StripeWebhook] Webhook DLQ fallback: Failed to charge {} coins for {}. Event: {}", amount, agent_uuid, ev_id);

                        let dlq_payload = serde_json::to_string(&payload).unwrap_or_default();
                        error!("🔒 [StripeWebhook] DLQ payload backup: {}", dlq_payload);

                        const Q_DLQ_SQLITE: &str = "INSERT INTO outbox_dead_letters (id, event_type, payload, error_reason) VALUES (?, ?, ?, ?)";
                        const Q_DLQ_PG: &str = "INSERT INTO outbox_dead_letters (id, event_type, payload, error_reason) VALUES ($1, $2, $3, $4)";

                        let dlq_id = Uuid::new_v4().to_string();

                        let result = infrastructure::sql_exec!(
                            &*dlq_pool,
                            sqlite: Q_DLQ_SQLITE,
                            pg: Q_DLQ_PG,
                            &dlq_id,
                            "coin_charge_failed",
                            &dlq_payload,
                            "Max retries exceeded"
                        );

                        if let Err(e) = result {
                            error!("🔥 [StripeWebhook] CRITICAL: Failed to write to dead letters queue: {:?}", e);
                        } else {
                            info!("📦 [StripeWebhook] Saved failed coin charge to outbox_dead_letters.");
                        }
                        break;
                    }
                    tokio::time::sleep(delay).await;
                    delay *= 5;
                }
            }
            .instrument(coin_charge_span),
        );
    } else {
        error!(
            "🚨 [StripeWebhook] NURTURE_API_URL or NURTURE_INTERNAL_SECRET not set! Coin charge for {} ({} coins, event={}) will NOT be delivered.",
            agent_uuid, amount, ev_id
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn attempt_coin_charge_once_succeeds_with_valid_oxp() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/internal/coin-charge"))
            .and(header("authorization", "Bearer mock_secret"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock)
            .await;

        let payload = serde_json::json!({
            "actor_id": uuid::Uuid::new_v4(),
            "amount": 100,
            "currency": "coin",
            "stripe_event_id": "ev_test",
            "idempotency_key": "ev_test"
        });

        let client = reqwest::Client::new();
        let result =
            attempt_coin_charge_once(&client, &mock.uri(), "mock_secret", 950, &payload).await;

        assert!(result.is_ok(), "expected success, got {:?}", result);

        let received = mock
            .received_requests()
            .await
            .expect("mock should have received requests");
        assert_eq!(received.len(), 1);
        let oxp_b64 = received[0]
            .headers
            .get("x-oxilean-proof-certificate")
            .expect("OXP header must be present")
            .to_str()
            .unwrap();
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(oxp_b64)
            .expect("OXP header must be base64");
        let cert: aiome_core_contracts::oxilean::OxiLeanProofCertificate =
            serde_json::from_slice(&decoded).expect("OXP header must be JSON cert");
        assert_eq!(cert.subject_id, "aiome-edge-node");
        assert_eq!(cert.oxp_score, 950);
        assert!(cert.verify("mock_secret"));
    }

    #[tokio::test]
    async fn attempt_coin_charge_once_fails_on_http_error() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/internal/coin-charge"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock)
            .await;

        let payload = serde_json::json!({"actor_id": uuid::Uuid::new_v4()});
        let client = reqwest::Client::new();
        let result =
            attempt_coin_charge_once(&client, &mock.uri(), "mock_secret", 950, &payload).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("http"));
    }
}
