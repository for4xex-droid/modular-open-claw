use crate::error::AppError;
use infrastructure::db::DatabasePool;
use tracing::{error, info, Instrument};
use uuid::Uuid;

// auth-exempt: Helper function (Not an endpoint)
pub async fn enqueue_coin_charge_to_nurture(
    http_client: reqwest::Client,
    dlq_pool: std::sync::Arc<DatabasePool>,
    nurture_url: Option<String>,
    nurture_secret: Option<String>,
    agent_uuid: Uuid,
    amount: u64,
    ev_id: String,
) {
    if let (Some(url), Some(secret)) = (nurture_url, nurture_secret) {
        let coin_charge_span =
            tracing::info_span!("coin_charge_relay", agent = %agent_uuid, event = %ev_id);
        tokio::spawn(
            async move {
                let req_url = format!("{}/internal/coin-charge", url);
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
                    match http_client
                        .post(&req_url)
                        .header("Authorization", format!("Bearer {}", secret))
                        .json(&payload)
                        .send()
                        .await
                    {
                        Ok(res) if res.status().is_success() => {
                            info!(
                                "🪙 [StripeWebhook] Coin charge succeeded for {}",
                                agent_uuid
                            );
                            break;
                        }
                        Ok(res) => {
                            error!(
                                "❌ [StripeWebhook] Coin charge HTTP failed: {}",
                                res.status()
                            );
                        }
                        Err(e) => {
                            error!("❌ [StripeWebhook] Coin charge network error: {}", e);
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
