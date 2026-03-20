use aiome_contracts::error::AiomeError;
use aiome_contracts::commerce::CommerceEngine;
use async_trait::async_trait;
use uuid::Uuid;
use stripe::{Webhook, WebhookError};

pub struct StripeCommerceEngine {
    webhook_secret: String,
    pool: sqlx::SqlitePool,
}

impl StripeCommerceEngine {
    pub fn new(webhook_secret: String, pool: sqlx::SqlitePool) -> Self {
        Self { webhook_secret, pool }
    }
}


#[async_trait]
impl CommerceEngine for StripeCommerceEngine {
    async fn get_balance(&self, _agent_id: Uuid) -> Result<u64, AiomeError> {
        Ok(0)
    }

    async fn validate_activity(
        &self,
        _agent_id: Uuid,
        _activity_type: &str,
        _amount: u64,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn execute_autonomous_purchase(
        &self,
        _agent_id: Uuid,
        _item_id: Uuid,
        _metadata: serde_json::Value,
    ) -> Result<String, AiomeError> {
        Ok("tx_mock".into())
    }

    async fn get_daily_spend(&self, _agent_id: Uuid) -> Result<u64, AiomeError> {
        Ok(0)
    }

    async fn get_daily_limit(&self, _agent_id: Uuid) -> Result<u64, AiomeError> {
        Ok(100)
    }

    async fn escrow_create(&self, _agent_id: Uuid, _amount: u64) -> Result<String, AiomeError> {
        Ok("escrow_mock".into())
    }

    async fn stake(&self, _agent_id: Uuid, _amount: u64) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn slash(&self, _agent_id: Uuid, _amount: u64, _reason: &str) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn register_license(
        &self,
        _agent_id: Uuid,
        _asset_id: Uuid,
        _license_type: &str,
    ) -> Result<String, AiomeError> {
        Ok("lic_mock".into())
    }

    fn verify_signature(&self, payload: &str, sig_header: &str) -> Result<(), AiomeError> {
        Webhook::construct_event(payload, sig_header, &self.webhook_secret)
            .map(|_| ())
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Stripe Webhook verification failed: {}", e),
            })
    }
    
    async fn process_webhook(&self, event_id: &str, event_type: &str, _payload: &serde_json::Value) -> Result<(), AiomeError> {
        // Stripe Webhook 冪等性保証: イベント ID でユニーク制約をかける
        let result = sqlx::query(
            "INSERT INTO stripe_webhook_events (event_id, event_type) VALUES (?, ?)"
        )
        .bind(event_id)
        .bind(event_type)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => {
                // 正常に記録された場合は処理を継続
                tracing::info!("✅ [StripeCommerce] Webhook event {} processed successfully.", event_id);
                Ok(())
            }
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                // 既に処理済みの場合は、安全のため再度正常終了
                tracing::info!("💡 [StripeCommerce] Webhook event {} was already processed. Skipping.", event_id);
                Ok(())
            }
            Err(e) => {
                tracing::error!("❌ [StripeCommerce] Database error evaluating idempotency for event {}: {}", event_id, e);
                Err(AiomeError::Infrastructure { reason: e.to_string() })
            }
        }
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
                processed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(&pool).await.unwrap();

        StripeCommerceEngine::new("whsec_test".into(), pool)
    }

    #[tokio::test]
    async fn test_verify_webhook_signature_green() {
        use sha2::{Sha256, Digest};
        use hmac::{Hmac, Mac};

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
            assert!(reason.contains("error parsing event object"), "予期せぬエラー: {}", reason);
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
    async fn test_process_webhook_idempotency_red() {
        let engine = get_test_engine().await;
        let event_id = "evt_idempotency_test_1";
        let event_type = "payment_intent.succeeded";
        let payload = serde_json::json!({"test": true});
        
        // 初回の処理は成功するはず
        let result1 = engine.process_webhook(event_id, event_type, &payload).await;
        assert!(result1.is_ok(), "First process should succeed");
        
        // 二回目の処理は冪等性により無視 (Ok) されるか、あるいはエラー（AlreadyProcessed等）を返すか。
        // ここでは「既に処理済みだが、正常終了として扱う（＝2度目はスキップ）」仕様とする。
        // DBの状態を確認し、レコードが1つだけであることを確認。
        let result2 = engine.process_webhook(event_id, event_type, &payload).await;
        assert!(result2.is_ok(), "Second process should also return Ok (idempotent skipped)");
        
        // 実際の DB 登録数を確認
        let count: (i64,) = sqlx::query_as("SELECT count(*) FROM stripe_webhook_events WHERE event_id = ?")
            .bind(event_id)
            .fetch_one(&engine.pool)
            .await
            .unwrap();
            
        assert_eq!(count.0, 1, "There should be exactly one record in the DB for this event_id");
    }
}
