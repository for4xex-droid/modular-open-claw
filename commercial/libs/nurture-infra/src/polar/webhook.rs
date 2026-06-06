/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use crate::economy::idempotency::IdempotencyStore;
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use nurture_core::ledger::EconomyLedger;
use std::sync::Arc;

use aiome_core_contracts::security::AgentHook;

pub struct PolarWebhookHandler {
    secret: String,
    ledger: Arc<dyn EconomyLedger>,
    system_actor_id: ActorId,
    idempotency: Arc<dyn IdempotencyStore>,
    agent_hooks: Vec<Arc<dyn AgentHook>>,
}

impl PolarWebhookHandler {
    pub fn new(
        secret: String,
        ledger: Arc<dyn EconomyLedger>,
        system_actor_id: ActorId,
        idempotency: Arc<dyn IdempotencyStore>,
        agent_hooks: Vec<Arc<dyn AgentHook>>,
    ) -> Self {
        Self {
            secret,
            ledger,
            system_actor_id,
            idempotency,
            agent_hooks,
        }
    }

    /// Webhook イベントを検証・処理し、メタデータから直接ActorIdを抽出してコインをチャージする
    pub async fn handle_event(
        &self,
        payload: &[u8],
        headers: &http::HeaderMap,
    ) -> Result<(), NurtureError> {
        // 1. Signature Verification
        let webhook = svix::webhooks::Webhook::new(&self.secret)
            .map_err(|e| NurtureError::Unauthorized(format!("Invalid Polar Secret: {}", e)))?;

        webhook.verify(payload, headers).map_err(|e| {
            NurtureError::Unauthorized(format!("Polar Webhook Verification Failed: {}", e))
        })?;

        // 2. Parse JSON
        let event: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| NurtureError::Infrastructure(format!("Invalid JSON: {}", e)))?;

        let event_type = event["type"].as_str().unwrap_or("");
        let event_id = event["data"]["id"].as_str().unwrap_or("unknown");

        // 3. Idempotency Check
        let idempotency_key = format!("polar_evt_{}", event_id);
        if let Some(_response) = self.idempotency.get_response(&idempotency_key).await? {
            tracing::info!(
                "♻️ [Polar] Webhook event {} already processed. Skipping.",
                event_id
            );
            return Ok(());
        }

        self.idempotency
            .reserve_key(&idempotency_key, chrono::Duration::hours(48))
            .await?;

        // 4. Process event
        let process_result = match event_type {
            "order.created" | "subscription.created" => {
                let amount_cents = event["data"]["amount"].as_i64().unwrap_or(0);
                let actor_id_str = event["data"]["metadata"]["actor_id"].as_str();

                if let Some(actor_str) = actor_id_str {
                    if let Ok(actor_id) = uuid::Uuid::parse_str(actor_str) {
                        let coin_amount: u64 = amount_cents.try_into().unwrap_or(0);
                        if coin_amount > 0 {
                            let entry = nurture_core::ledger::LedgerEntry {
                                id: uuid::Uuid::new_v4(),
                                transaction_id: uuid::Uuid::new_v4(),
                                asset_id: None,
                                debit_account: self.system_actor_id,
                                credit_account: ActorId(actor_id),
                                coin_amount,
                                points_amount: 0,
                                entry_type: nurture_core::ledger::EntryType::Charge,
                                created_at: chrono::Utc::now(),
                                debit_account_version: None,
                            };
                            match self.ledger.record_entry(&entry).await {
                                Ok(_) => {
                                    // Karma生成のためフックを呼び出す
                                    for hook in &self.agent_hooks {
                                        if let Err(e) = hook
                                            .on_transaction_completed(
                                                "polar",
                                                amount_cents,
                                                actor_str,
                                                event_id,
                                            )
                                            .await
                                        {
                                            tracing::warn!("⚠️ [Polar] Hook failed on transaction_completed: {}", e);
                                        }
                                    }
                                    Ok(())
                                }
                                Err(e) => Err(e),
                            }
                        } else {
                            Ok(())
                        }
                    } else {
                        tracing::warn!(
                            "⚠️ [Polar] Invalid actor_id format in metadata: {}",
                            actor_str
                        );
                        Ok(())
                    }
                } else {
                    tracing::warn!("⚠️ [Polar] No actor_id found in Polar webhook metadata");
                    Ok(())
                }
            }
            _ => Ok(()), // Ignore other events
        };

        // 5. Finalize Idempotency
        match process_result {
            Ok(_) => {
                self.idempotency
                    .save_response(&idempotency_key, 200, "ok".into())
                    .await?;
                Ok(())
            }
            Err(e) => {
                tracing::warn!(
                    "⚠️ [Polar] ロジック失敗につき冪等性キーを解放します: {}",
                    idempotency_key
                );
                if let Err(delete_err) = self.idempotency.delete_key(&idempotency_key).await {
                    tracing::warn!(
                        "⚠️ [Polar] Failed to delete idempotency key {}: {}",
                        idempotency_key,
                        delete_err
                    );
                }
                Err(e)
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::clone_on_copy)]
mod tests {
    use super::*;
    use crate::economy::idempotency::IdempotencyResponse;
    use async_trait::async_trait;
    use nurture_core::coin::CoinWallet;
    use nurture_core::ledger::LedgerEntry;
    use nurture_core::points::PointsAccount;
    use uuid::Uuid;

    // --- Mocks ---
    struct MockLedger {
        pub called: std::sync::Mutex<usize>,
        pub last_amount: std::sync::Mutex<u64>,
        pub last_recipient: std::sync::Mutex<Option<ActorId>>,
    }
    impl MockLedger {
        fn new() -> Self {
            Self {
                called: std::sync::Mutex::new(0),
                last_amount: std::sync::Mutex::new(0),
                last_recipient: std::sync::Mutex::new(None),
            }
        }
    }
    #[async_trait]
    impl EconomyLedger for MockLedger {
        async fn record_entry(&self, entry: &LedgerEntry) -> Result<(), NurtureError> {
            *self.called.lock().unwrap() += 1;
            *self.last_amount.lock().unwrap() = entry.coin_amount;
            *self.last_recipient.lock().unwrap() = Some(entry.credit_account);
            Ok(())
        }
        async fn record_batch(&self, _entries: &[LedgerEntry]) -> Result<(), NurtureError> {
            Err(NurtureError::Infrastructure(
                "Not implemented in mock".into(),
            ))
        }
        async fn get_balance(&self, _actor: &ActorId) -> Result<CoinWallet, NurtureError> {
            Err(NurtureError::Infrastructure(
                "Not implemented in mock".into(),
            ))
        }
        async fn get_points(&self, _actor: &ActorId) -> Result<PointsAccount, NurtureError> {
            Err(NurtureError::Infrastructure(
                "Not implemented in mock".into(),
            ))
        }
        async fn get_history(
            &self,
            _actor: &ActorId,
            _limit: u32,
        ) -> Result<Vec<LedgerEntry>, NurtureError> {
            Err(NurtureError::Infrastructure(
                "Not implemented in mock".into(),
            ))
        }
        async fn get_entries_by_transaction(
            &self,
            _tx_id: &Uuid,
        ) -> Result<Vec<LedgerEntry>, NurtureError> {
            Err(NurtureError::Infrastructure(
                "Not implemented in mock".into(),
            ))
        }
    }

    struct MockIdempotencyStore {
        pub responses:
            std::sync::Mutex<std::collections::HashMap<String, Option<IdempotencyResponse>>>,
    }
    impl MockIdempotencyStore {
        fn new() -> Self {
            Self {
                responses: std::sync::Mutex::new(std::collections::HashMap::new()),
            }
        }
    }
    #[async_trait]
    impl IdempotencyStore for MockIdempotencyStore {
        async fn get_response(
            &self,
            key: &str,
        ) -> Result<Option<Option<IdempotencyResponse>>, NurtureError> {
            let map = self.responses.lock().unwrap();
            Ok(map.get(key).cloned())
        }
        async fn reserve_key(
            &self,
            key: &str,
            _expires_in: chrono::Duration,
        ) -> Result<(), NurtureError> {
            let mut map = self.responses.lock().unwrap();
            if map.contains_key(key) {
                return Err(NurtureError::IdempotencyConflict {
                    key: key.to_string(),
                });
            }
            map.insert(key.to_string(), None);
            Ok(())
        }
        async fn save_response(
            &self,
            key: &str,
            status: u16,
            body: String,
        ) -> Result<(), NurtureError> {
            let mut map = self.responses.lock().unwrap();
            map.insert(
                key.to_string(),
                Some(IdempotencyResponse {
                    status_code: status,
                    body,
                }),
            );
            Ok(())
        }
        async fn delete_key(&self, key: &str) -> Result<(), NurtureError> {
            let mut map = self.responses.lock().unwrap();
            map.remove(key);
            Ok(())
        }
    }

    fn generate_svix_signature(
        secret: &str,
        msg_id: &str,
        timestamp: &str,
        payload: &str,
    ) -> String {
        use base64::Engine;
        let to_sign = format!("{}.{}.{}", msg_id, timestamp, payload);
        let secret_bytes = base64::engine::general_purpose::STANDARD
            .decode(secret.strip_prefix("whsec_").unwrap())
            .unwrap();
        let result = hmac_sha256::HMAC::mac(to_sign.as_bytes(), &secret_bytes);
        let encoded = base64::engine::general_purpose::STANDARD.encode(result);
        format!("v1,{}", encoded)
    }

    #[tokio::test]
    async fn test_polar_webhook_mint_coins_from_metadata() {
        let secret = "whsec_MfKOhlX1zKj7z4G6dNQy1yC2YF+r91jZ";
        let ledger = Arc::new(MockLedger::new());
        let idempotency = Arc::new(MockIdempotencyStore::new());
        let system_id = ActorId(Uuid::new_v4());
        let handler = PolarWebhookHandler::new(
            secret.to_string(),
            ledger.clone(),
            system_id,
            idempotency.clone(),
            vec![],
        );

        let target_actor_id = Uuid::new_v4();
        let event_id = "evt_polar_123";
        let timestamp = chrono::Utc::now().timestamp().to_string();

        // Polar `order.created` payload with metadata containing actor_id
        let payload = serde_json::json!({
            "type": "order.created",
            "data": {
                "id": "order_xyz",
                "amount": 1500,
                "metadata": {
                    "actor_id": target_actor_id.to_string()
                }
            }
        })
        .to_string();

        let signature = generate_svix_signature(secret, event_id, &timestamp, &payload);

        let mut headers = http::HeaderMap::new();
        headers.insert("svix-id", event_id.parse().unwrap());
        headers.insert("svix-timestamp", timestamp.parse().unwrap());
        headers.insert("svix-signature", signature.parse().unwrap());

        let result = handler.handle_event(payload.as_bytes(), &headers).await;

        // Assertions for RED state
        assert!(
            result.is_ok(),
            "Webhook processing failed: {:?}",
            result.err()
        );

        // Check if Ledger was called and credited to the CORRECT actor_id
        assert_eq!(
            *ledger.called.lock().unwrap(),
            1,
            "Ledger should be called once"
        );
        assert_eq!(
            *ledger.last_amount.lock().unwrap(),
            1500,
            "Should charge 1500 coins"
        );
        assert_eq!(
            (*ledger.last_recipient.lock().unwrap()).unwrap(),
            ActorId(target_actor_id),
            "Should charge the actor_id found in metadata"
        );
    }
}
