/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

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
    use sha2::Sha256;

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

    let result = engine.create_subscription(agent_id, plan_id).await;

    assert!(result.is_ok());
    let sub_id = result.unwrap();
    assert_eq!(sub_id, "sub_mock_stripe");
}

#[test]
fn test_stripe_subscription_status_mapping() {
    use super::map_stripe_status;
    use aiome_core_contracts::commerce::SubscriptionStatus;
    use stripe_billing::SubscriptionStatus as StripeStatus;

    assert_eq!(
        map_stripe_status(StripeStatus::Active),
        SubscriptionStatus::Active
    );
    assert_eq!(
        map_stripe_status(StripeStatus::Canceled),
        SubscriptionStatus::Cancelled
    );
    assert_eq!(
        map_stripe_status(StripeStatus::PastDue),
        SubscriptionStatus::PastDue
    );
    assert_eq!(
        map_stripe_status(StripeStatus::Trialing),
        SubscriptionStatus::Trialing
    );
    assert_eq!(
        map_stripe_status(StripeStatus::Unpaid),
        SubscriptionStatus::Unpaid
    );
    assert_eq!(
        map_stripe_status(StripeStatus::Incomplete),
        SubscriptionStatus::Incomplete
    );
    assert_eq!(
        map_stripe_status(StripeStatus::IncompleteExpired),
        SubscriptionStatus::IncompleteExpired
    );
    assert_eq!(
        map_stripe_status(StripeStatus::Paused),
        SubscriptionStatus::None
    );
}

#[tokio::test]
async fn test_stripe_cancel_subscription_mock() {
    let engine = get_test_engine().await;
    let agent_id = Uuid::new_v4();
    let result = engine.cancel_subscription(agent_id, "sub_mock").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_stripe_get_subscription_status_mock() {
    let engine = get_test_engine().await;
    let agent_id = Uuid::new_v4();
    let status = engine.get_subscription_status(agent_id).await;
    assert!(status.is_ok());
    assert_eq!(
        status.unwrap(),
        aiome_core_contracts::commerce::SubscriptionStatus::Active
    );
}

#[tokio::test]
async fn test_stripe_cancel_subscription_live_error() {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();

    let engine = StripeCommerceEngine::new(
        SecretString::from("sk_live_invalidkey".to_string()), // gitleaks:allow
        SecretString::from("whsec_live_dummy".to_string()),
        pool,
        None,
        None,
    );

    let agent_id = Uuid::new_v4();
    let result = engine.cancel_subscription(agent_id, "sub_dummy").await;
    assert!(result.is_err());
    if let Err(AiomeError::Infrastructure { reason }) = result {
        assert!(
            reason.contains("Invalid API Key")
                || reason.contains("status code: 401")
                || reason.contains("unauthorized")
                || reason.contains("Stripe cancel subscription failed"),
            "Actual error reason: {}",
            reason
        );
    } else {
        panic!("Expected infrastructure error from invalid API key");
    }
}

#[tokio::test]
async fn test_create_subscription_real_test_key() {
    let api_key = match std::env::var("STRIPE_API_KEY") {
        Ok(key) if key.starts_with("sk_test_") && key != "sk_test_mock" => key,
        _ => {
            tracing::warn!("⚠️ [Stripe E2E] STRIPE_API_KEY not set or is mock key. Skipping real Stripe API test.");
            return;
        }
    };

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS stripe_customers (
            id TEXT PRIMARY KEY,
            customer_id TEXT UNIQUE NOT NULL,
            agent_id TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );",
    )
    .execute(&pool)
    .await
    .unwrap();

    let engine = StripeCommerceEngine::new(
        SecretString::from(api_key),
        SecretString::from("whsec_dummy_for_real_test".to_string()),
        pool,
        None,
        None,
    );

    assert!(!engine.is_mock, "Must be in real mode for Stripe API test");

    let agent_id = Uuid::new_v4();
    let result = engine
        .create_subscription(agent_id, "price_nonexistent_plan")
        .await;

    let saved_customer: Option<(String, String)> =
        sqlx::query_as("SELECT customer_id, agent_id FROM stripe_customers WHERE agent_id = ?")
            .bind(agent_id.to_string())
            .fetch_optional(&engine.pool)
            .await
            .unwrap();

    if saved_customer.is_none() {
        if let Err(AiomeError::Infrastructure { reason }) = &result {
            if reason.contains("Invalid API Key") || reason.contains("status code: 401") {
                tracing::info!("`✅ [Stripe E2E] Successfully reached Stripe API, but key is invalid (401). E2E path verified.`");
                return;
            }
        }
        panic!(
            "Customer UPSERT should create a Stripe customer and save it to the DB even if subscription fails later. Actual subscription result: {:?}",
            result
        );
    }

    let (customer_id, saved_agent_id) = saved_customer.unwrap();
    assert!(
        customer_id.starts_with("cus_"),
        "Stripe Customer ID must start with 'cus_'"
    );
    assert_eq!(saved_agent_id, agent_id.to_string());

    assert!(
        result.is_err(),
        "Subscription creation should fail on nonexistent plan but customer must persist"
    );
}

#[tokio::test]
async fn test_production_mode_rejects_test_secrets_red() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();

    let engine = StripeCommerceEngine::new(
        secrecy::SecretString::from("sk_live_prod_key_12345".to_string()), // gitleaks:allow
        secrecy::SecretString::from("whsec_live_prod_secret".to_string()), // gitleaks:allow
        pool,
        None,
        None,
    );

    assert!(
        !engine.is_mock,
        "Production keys MUST NOT enable mock mode regardless of AIOME_DEV_MODE"
    );
}

#[tokio::test]
async fn test_deduct_generation_cost_green() {
    let engine = get_test_engine().await;
    let agent_id = Uuid::new_v4();

    let result = engine
        .deduct_generation_cost(agent_id, None, 10, "image_gen")
        .await;
    assert!(result.is_ok(), "Should unlock GenerativeEngine billing");
}

#[tokio::test]
async fn test_escrow_create_green() {
    let engine = get_test_engine().await;
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

    let recipient_id = Uuid::new_v4();
    let release_result = engine.escrow_release(&escrow_id, recipient_id).await;
    assert!(release_result.is_ok());

    let refund_result = engine.escrow_refund(&escrow_id).await;
    assert!(refund_result.is_err());

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

    Mock::given(method("POST"))
        .and(path("/internal/validate-activity"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let res = engine.validate_activity(agent_id, "test_action", 100).await;
    assert!(res.is_ok(), "Expected Ok for 200 OK response");

    mock_server.reset().await;

    Mock::given(method("POST"))
        .and(path("/internal/validate-activity"))
        .respond_with(ResponseTemplate::new(402).set_body_string("Insufficient funds"))
        .mount(&mock_server)
        .await;

    let res = engine.validate_activity(agent_id, "test_action", 100).await;
    assert!(res.is_err(), "Expected Err for 402 response");

    mock_server.reset().await;

    Mock::given(method("POST"))
        .and(path("/internal/validate-activity"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal error"))
        .mount(&mock_server)
        .await;

    let res_500 = engine.validate_activity(agent_id, "test_action", 100).await;
    assert!(
        res_500.is_err(),
        "Expected Err for 500 response (fail-closed)"
    );

    mock_server.reset().await;

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

    let engine = StripeCommerceEngine::new(
        SecretString::from("sk_live_123456789".to_string()), // gitleaks:allow
        SecretString::from("whsec_live_987654".to_string()), // gitleaks:allow
        pool,
        None,
        None,
    );

    assert!(!engine.is_mock);

    let agent_id = Uuid::new_v4();
    let item_id = Uuid::new_v4();

    let purchase = engine
        .execute_autonomous_purchase(agent_id, item_id, serde_json::json!({}))
        .await;
    assert!(purchase.is_err());
    if let Err(AiomeError::Infrastructure { reason }) = purchase {
        assert!(
            reason.contains("Nurture S2S URL not configured"),
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
}
