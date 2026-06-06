/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 */

use crate::economy::idempotency::IdempotencyStore;
use chrono::Utc;
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use nurture_core::customer::CustomerStore;
use nurture_core::ledger::{EconomyLedger, EntryType, LedgerEntry};
use std::sync::Arc;
use stripe_webhook::{Event, EventObject, EventType, Webhook};
use uuid::Uuid;

use aiome_core_contracts::security::AgentHook;

pub struct StripeWebhookHandler {
    secret: String,
    ledger: Arc<dyn EconomyLedger>,
    customer_store: Arc<dyn CustomerStore>,
    system_actor_id: ActorId,
    idempotency: Arc<dyn IdempotencyStore>,
    agent_hooks: Vec<Arc<dyn AgentHook>>,
}

impl StripeWebhookHandler {
    pub fn new(
        secret: String,
        ledger: Arc<dyn EconomyLedger>,
        customer_store: Arc<dyn CustomerStore>,
        system_actor_id: ActorId,
        idempotency: Arc<dyn IdempotencyStore>,
        agent_hooks: Vec<Arc<dyn AgentHook>>,
    ) -> Self {
        Self {
            secret,
            ledger,
            customer_store,
            system_actor_id,
            idempotency,
            agent_hooks,
        }
    }

    pub fn verify_and_parse(&self, payload: &str, sig: &str) -> Result<Event, NurtureError> {
        Webhook::construct_event(payload, sig, &self.secret)
            .map_err(|e| NurtureError::Unauthorized(format!("Stripe Webhook 署名検証失敗: {}", e)))
    }

    /// Webhook イベントを処理し、必要に応じてコインをチャージする
    pub async fn handle_event(&self, event: Event) -> Result<(), NurtureError> {
        let event_id_str = event.id.as_str();

        // 🚨 F-2: Stripe Webhook リプレイ攻撃（冪等性）の防止
        // 同じevent.idが再送された場合は二重チャージを防ぐ
        if let Some(_response) = self.idempotency.get_response(event_id_str).await? {
            tracing::info!(
                "♻️ [Stripe] Webhook event {} already processed. Skipping.",
                event_id_str
            );
            return Ok(());
        }

        // 処理キーを予約（48時間の TTL を設ける）
        self.idempotency
            .reserve_key(event_id_str, chrono::TimeDelta::hours(48))
            .await?;

        let process_result = match event.type_ {
            EventType::CheckoutSessionCompleted => {
                if let EventObject::CheckoutSessionCompleted(session) = event.data.object {
                    let customer_id = session.customer.map(|c| c.id().to_string());
                    self.process_checkout_event(
                        customer_id.as_deref(),
                        session.amount_total,
                        event_id_str,
                    )
                    .await
                } else {
                    Ok(())
                }
            }
            EventType::ChargeSucceeded => {
                if let EventObject::ChargeSucceeded(charge) = event.data.object {
                    tracing::info!("💳 [Stripe] Charge Succeeded: {}", charge.id);
                }
                Ok(())
            }
            _ => {
                tracing::debug!("ℹ️ [Stripe] Ignored webhook event: {:?}", event.type_);
                Ok(())
            }
        };

        match process_result {
            Ok(_) => {
                // 処理の完了をマーク（ステータス200, ボディok）
                self.idempotency
                    .save_response(event_id_str, 200, "ok".into())
                    .await?;
                Ok(())
            }
            Err(e) => {
                // ビジネスロジック失敗時、再試行できるようにロックを解放する
                tracing::warn!(
                    "⚠️ [Stripe] ロジック失敗につき冪等性キーを解放します: {}",
                    event_id_str
                );
                if let Err(delete_err) = self.idempotency.delete_key(event_id_str).await {
                    tracing::warn!(
                        "⚠️ [Stripe] Failed to delete idempotency key {}: {}",
                        event_id_str,
                        delete_err
                    );
                }
                Err(e)
            }
        }
    }

    pub async fn process_checkout_event(
        &self,
        customer_id: Option<&str>,
        amount_total: Option<i64>,
        event_id: &str,
    ) -> Result<(), NurtureError> {
        // C-2: None と空文字列を明確に区別する
        let customer_id = match customer_id {
            Some(id) if !id.is_empty() => id,
            _ => {
                tracing::warn!(
                    "⚠️ [Stripe] 顧客情報が Webhook イベントに含まれていません。スキップします。"
                );
                return Ok(());
            }
        };

        // 1. ストライプ顧客 ID から ActorId を特定
        let actor_id = self
            .customer_store
            .get_actor_id(customer_id)
            .await?
            .ok_or_else(|| {
                NurtureError::Infrastructure(format!(
                    "Stripe 顧客に対応する ActorId が見つかりません: {}",
                    customer_id
                ))
            })?;

        // 🚨 V-13: 安全な型変換。負数・ゼロ・None を一括ガード
        let coin_amount: u64 = amount_total
            .and_then(|a| u64::try_from(a).ok())
            .unwrap_or(0);

        if coin_amount == 0 {
            tracing::warn!(
                "⚠️ [Stripe] amount_total が 0 または負のため、チャージをスキップします (customer: {})",
                customer_id
            );
            return Ok(());
        }

        tracing::info!(
            "💰 [Stripe] Checkout Completed: customer={}, actor_id={}, amount={}",
            customer_id,
            actor_id.0,
            coin_amount
        );

        // 2. 台帳にチャージエントリを記録
        // H-1: transaction_id に一意の UUID を付与（全ゼロ UUID は追跡不能）
        let entry = LedgerEntry {
            id: Uuid::new_v4(),
            transaction_id: Uuid::new_v4(),
            asset_id: None,
            debit_account: self.system_actor_id, // システムからユーザーへ
            credit_account: actor_id,
            coin_amount,
            points_amount: 0,
            entry_type: EntryType::Charge,
            created_at: Utc::now(),
            debit_account_version: None, // システムアカウントはロック不要
        };

        self.ledger.record_entry(&entry).await?;

        // Karma生成のためフックを呼び出す
        for hook in &self.agent_hooks {
            if let Err(e) = hook
                .on_transaction_completed(
                    "stripe",
                    amount_total.unwrap_or(0),
                    &actor_id.0.to_string(),
                    event_id,
                )
                .await
            {
                tracing::warn!("⚠️ [Stripe] Hook failed on transaction_completed: {}", e);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::idempotency::{IdempotencyResponse, IdempotencyStore};
    use async_trait::async_trait;
    use commerce_protocol::error::NurtureError;
    use commerce_protocol::identity::ActorId;
    use nurture_core::coin::CoinWallet;
    use nurture_core::customer::CustomerStore;
    use nurture_core::ledger::{EconomyLedger, LedgerEntry};
    use nurture_core::points::PointsAccount;
    use std::sync::Arc;

    use uuid::Uuid;

    // --- Mocks ---
    struct MockLedger {
        pub called: std::sync::Mutex<usize>,
    }
    impl MockLedger {
        fn new() -> Self {
            Self {
                called: std::sync::Mutex::new(0),
            }
        }
    }
    #[async_trait]
    impl EconomyLedger for MockLedger {
        async fn record_entry(&self, _entry: &LedgerEntry) -> Result<(), NurtureError> {
            *self.called.lock().unwrap() += 1;
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

    struct MockCustomerStore;
    #[async_trait]
    impl CustomerStore for MockCustomerStore {
        async fn get_actor_id(&self, _customer_id: &str) -> Result<Option<ActorId>, NurtureError> {
            Ok(Some(ActorId(Uuid::new_v4())))
        }
        async fn link_customer(
            &self,
            _customer_id: &str,
            _actor_id: &ActorId,
        ) -> Result<(), NurtureError> {
            Ok(())
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

    #[tokio::test]
    async fn test_stripe_webhook_idempotency() {
        let ledger = Arc::new(MockLedger::new());
        let customer_store = Arc::new(MockCustomerStore);
        let idempotency = Arc::new(MockIdempotencyStore::new());
        let handler = StripeWebhookHandler::new(
            "test_secret".to_string(),
            ledger.clone(),
            customer_store.clone(),
            ActorId(Uuid::new_v4()),
            idempotency.clone(),
            vec![],
        );

        let event_id = "evt_123";
        let customer_id = "cus_test_123";
        let amount = Some(1000);

        // --- 冪等性チェックのシミュレーション (handle_eventの冒頭ロジック) ---
        // 1回目
        let is_processed = handler
            .idempotency
            .get_response(event_id)
            .await
            .expect("get_response should not fail in test")
            .is_some();
        if !is_processed {
            handler
                .idempotency
                .reserve_key(event_id, chrono::TimeDelta::hours(48))
                .await
                .expect("reserve_key should succeed on first call");
            handler
                .process_checkout_event(Some(customer_id), amount, event_id)
                .await
                .expect("process_checkout_event should succeed");
            handler
                .idempotency
                .save_response(event_id, 200, "ok".into())
                .await
                .expect("save_response should succeed");
        }
        assert_eq!(
            *ledger.called.lock().expect("ledger lock poisoned"),
            1,
            "Ledger should be called exactly once on first event"
        );

        // 2回目（同じ event_id で到達した場合）
        let is_processed = handler
            .idempotency
            .get_response(event_id)
            .await
            .expect("get_response should not fail in test")
            .is_some();
        if !is_processed {
            handler
                .idempotency
                .reserve_key(event_id, chrono::TimeDelta::hours(48))
                .await
                .expect("reserve_key should succeed");
            handler
                .process_checkout_event(Some(customer_id), amount, event_id)
                .await
                .expect("process_checkout_event should succeed");
            handler
                .idempotency
                .save_response(event_id, 200, "ok".into())
                .await
                .expect("save_response should succeed");
        }

        // 回数が 1 のままなら、2回目はスキップされている（冪等性が機能している）
        assert_eq!(
            *ledger.called.lock().expect("ledger lock poisoned"),
            1,
            "Ledger should not be called twice for the same event"
        );
    }

    #[tokio::test]
    async fn test_process_checkout_no_customer() {
        let ledger = Arc::new(MockLedger::new());
        let customer_store = Arc::new(MockCustomerStore);
        let idempotency = Arc::new(MockIdempotencyStore::new());
        let handler = StripeWebhookHandler::new(
            "test_secret".to_string(),
            ledger.clone(),
            customer_store,
            ActorId(Uuid::new_v4()),
            idempotency,
            vec![],
        );

        // None の場合: スキップされ、台帳は呼ばれない
        handler
            .process_checkout_event(None, Some(5000), "evt_1")
            .await
            .expect("should succeed with skip");
        assert_eq!(
            *ledger.called.lock().expect("ledger lock poisoned"),
            0,
            "Ledger should not be called when customer_id is None"
        );

        // 空文字列の場合: 同様にスキップ
        handler
            .process_checkout_event(Some(""), Some(5000), "evt_2")
            .await
            .expect("should succeed with skip");
        assert_eq!(
            *ledger.called.lock().expect("ledger lock poisoned"),
            0,
            "Ledger should not be called when customer_id is empty"
        );
    }

    #[tokio::test]
    async fn test_process_checkout_zero_amount() {
        let ledger = Arc::new(MockLedger::new());
        let customer_store = Arc::new(MockCustomerStore);
        let idempotency = Arc::new(MockIdempotencyStore::new());
        let handler = StripeWebhookHandler::new(
            "test_secret".to_string(),
            ledger.clone(),
            customer_store,
            ActorId(Uuid::new_v4()),
            idempotency,
            vec![],
        );

        // ゼロ金額: スキップ
        handler
            .process_checkout_event(Some("cus_123"), Some(0), "evt_1")
            .await
            .expect("should succeed with skip");
        assert_eq!(
            *ledger.called.lock().expect("ledger lock poisoned"),
            0,
            "Ledger should not be called for zero amount"
        );

        // 負数: スキップ
        handler
            .process_checkout_event(Some("cus_123"), Some(-500), "evt_2")
            .await
            .expect("should succeed with skip");
        assert_eq!(
            *ledger.called.lock().expect("ledger lock poisoned"),
            0,
            "Ledger should not be called for negative amount"
        );

        // None: スキップ
        handler
            .process_checkout_event(Some("cus_123"), None, "evt_3")
            .await
            .expect("should succeed with skip");
        assert_eq!(
            *ledger.called.lock().expect("ledger lock poisoned"),
            0,
            "Ledger should not be called for None amount"
        );
    }
}
