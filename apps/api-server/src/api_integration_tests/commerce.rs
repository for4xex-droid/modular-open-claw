/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::common::*;
use crate::app_state::Component;
use axum::http::StatusCode;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_stripe_webhook_idempotency_and_license_grant() {
    let (server, state, _tmp) = create_test_server().await;
    let registry = state.registry.clone();

    let agent_id = uuid::Uuid::new_v4();
    let asset_id = uuid::Uuid::new_v4();

    let mut metadata = std::collections::HashMap::new();
    metadata.insert(String::from("agent_id"), agent_id.to_string());
    metadata.insert(String::from("asset_id"), asset_id.to_string());

    // async-stripe 1.0.0-rc.5 の strict deserialization をバイパスするため
    // serde_json::Value で手動構築。必須フィールドのみ明示的に設定し、
    // 残りは CheckoutSession スキーマ充填（テスト安定性のため）。
    let session_val = serde_json::json!({
        // === テスト本質フィールド ===
        "id": "cs_test_123",
        "object": "checkout.session",
        "metadata": metadata,
        "amount_total": 1000,
        // === スキーマ充填（strict deserialization 対策） ===
        "automatic_tax": { "enabled": false, "status": null },
        "created": 1677628800,
        "currency": "usd",
        "livemode": false,
        "mode": "payment",
        "payment_status": "paid",
        "status": "complete",
        "amount_subtotal": 1000,
        "cancel_url": "http://example.com/cancel",
        "custom_fields": [],
        "custom_text": { "shipping_address": null, "submit": null, "terms_of_service_acceptance": null, "after_submit": null },
        "customer_creation": "always",
        "expires_at": 1677629800,
        "payment_method_types": ["card"],
        "phone_number_collection": { "enabled": false },
        "success_url": "http://example.com/success",
        "tax_id_collection": { "enabled": false }
    });

    let db_path = _tmp.path().join("test.db");
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&format!("sqlite:{}", db_path.to_str().unwrap()))
        .await
        .unwrap();

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS revenue_splits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tx_id TEXT NOT NULL,
            recipient_id TEXT NOT NULL,
            role TEXT NOT NULL,
            amount INTEGER NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    registry
        .register_asset(infrastructure::registry::AssetManifest {
            id: asset_id,
            creator_id: uuid::Uuid::new_v4(),
            asset_type: infrastructure::registry::AssetType::Plugin,
            name: "Test Asset".to_string(),
            description: "Test".to_string(),
            price_coins: 1000,
            safety_level: aiome_core_contracts::contracts::ToolSafetyLevel::Safe,
            metadata: None,
        })
        .await
        .unwrap();

    let payload_val = serde_json::json!({
        "id": "evt_test_123",
        "object": "event",
        "api_version": "2022-11-15",
        "created": 1677628800,
        "livemode": false,
        "pending_webhooks": 1,
        "request": {
            "id": null,
            "idempotency_key": null
        },
        "type": "checkout.session.completed",
        "data": {
            "object": session_val
        }
    });

    let payload = serde_json::to_string(&payload_val).unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let sig_header = format!("t={},v1=dummy_signature", now);

    let resp: axum_test::TestResponse = server
        .post("/api/v1/commerce/webhook")
        .add_header("stripe-signature", sig_header.clone())
        .add_header("content-type", "application/json")
        .text(payload.clone())
        .await;

    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);

    // 現在の api-server 実装では grant_license は呼ばれないため、ここは RED になるはずだったが、実装により GREEN になる
    let is_owned = registry.check_ownership(agent_id, asset_id).await.unwrap();
    assert!(
        is_owned,
        "Webhook must grant license in a single transaction (GREEN)"
    );
}
#[serial]
#[tokio::test]
async fn test_gift_policy_dynamic() {
    let (server, _state, _tmp) = create_test_server().await;

    // Auth token corresponds to agent_id 00000000-0000-0000-0000-000000000001
    let agent_id = "00000000-0000-0000-0000-000000000001";
    let resp = server
        .get(&format!("/api/v1/gift/policy/{}", agent_id))
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);
    let json: serde_json::Value = resp.json();
    assert_eq!(json["max_amount_usd"], 5.0);
    assert_eq!(json["daily_limit_reached"], false);
}
#[serial]
#[tokio::test]
async fn test_gift_send_success() {
    let (server, _state, _tmp) = create_test_server().await;

    let agent_id = "00000000-0000-0000-0000-000000000001";
    let payload = json!({
        "recipient_email": "test@example.com",
        "amount_usd": 2.5,
        "reason": "Test gift"
    });

    let verified_bearer = "Bearer mock_valid_token_ekyc_test_user".to_string();

    let resp = server
        .post(&format!("/api/v1/gift/send/{}", agent_id))
        .add_header(axum::http::header::AUTHORIZATION, verified_bearer)
        .json(&payload)
        .await;

    assert_eq!(resp.status_code(), axum::http::StatusCode::CREATED);
    let json: serde_json::Value = resp.json();
    assert_eq!(json["status"], "Sent");
    assert!(json.get("order_id").is_some());
}
#[serial]
#[tokio::test]
async fn test_gift_send_unverified_blocked() {
    let (server, _state, _tmp) = create_test_server().await;

    let agent_id = "00000000-0000-0000-0000-000000000001";
    let payload = json!({
        "recipient_email": "hacker@example.com",
        "amount_usd": 5.0,
        "reason": "Unverified gift"
    });

    let unverified_bearer = "Bearer mock_valid_token_unverified_user".to_string();

    let resp = server
        .post(&format!("/api/v1/gift/send/{}", agent_id))
        .add_header(axum::http::header::AUTHORIZATION, unverified_bearer)
        .json(&payload)
        .await;

    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::FORBIDDEN,
        "Unverified user should not be able to send gifts"
    );
}
#[serial]
#[tokio::test]
async fn test_commerce_purchase_unverified_blocked() {
    let (server, _state, _tmp) = create_test_server().await;

    let agent_id = "00000000-0000-0000-0000-000000000001";
    let payload = json!({
        "item_id": "00000000-0000-0000-0000-000000000002",
        "metadata": {}
    });

    let unverified_bearer = "Bearer mock_valid_token_unverified_user".to_string();

    let resp = server
        .post(&format!("/api/v1/commerce/purchase/{}", agent_id))
        .add_header(axum::http::header::AUTHORIZATION, unverified_bearer)
        .json(&payload)
        .await;

    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::FORBIDDEN,
        "Unverified user should not be able to execute purchases"
    );
}
#[serial]
#[tokio::test]
async fn test_subscription_lifecycle() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = "Bearer mock_valid_token_ekyctest_user".to_string();

    // 1. Create Subscription
    let payload = json!({
        "agent_id": "00000000-0000-0000-0000-000000000001",
        "plan_id": "price_gold_monthly"
    });

    let resp = server
        .post("/api/v1/commerce/subscription/create")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let json = resp.json::<serde_json::Value>();
    assert_eq!(json["subscription_id"], "sub_mock_123");

    // 2. Get Status
    let status_resp = server
        .get("/api/v1/commerce/subscription/00000000-0000-0000-0000-000000000001")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(status_resp.status_code(), StatusCode::OK);
    let status_json = status_resp.json::<aiome_core_contracts::commerce::SubscriptionStatus>();
    assert_eq!(
        status_json,
        aiome_core_contracts::commerce::SubscriptionStatus::Active
    );

    // 3. Cancel Subscription
    let cancel_resp = server
        .post("/api/v1/commerce/subscription/cancel")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&json!({"agent_id": "00000000-0000-0000-0000-000000000001", "subscription_id": "sub_mock_123"}))
        .await;

    assert_eq!(cancel_resp.status_code(), StatusCode::OK);
}
#[serial]
#[tokio::test]
async fn test_syndicate_guild_api_flow() {
    let (server, _state, _tmp_dir) = create_test_server().await;
    let bearer = test_bearer(); // sub-1

    // 1. Create Guild
    let create_req = serde_json::json!({
        "name": "Integration Syndicate",
        "description": "Formed by API"
    });
    let resp = server
        .post("/api/v1/syndicate/guilds")
        .add_header("Authorization", &bearer)
        .json(&create_req)
        .await;
    resp.assert_status(axum::http::StatusCode::CREATED);
    let guild_id: uuid::Uuid = resp.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    // 2. List Guilds
    let resp = server
        .get("/api/v1/syndicate/guilds")
        .add_header("Authorization", &bearer)
        .await;
    resp.assert_status(axum::http::StatusCode::OK);
    let guilds: Vec<aiome_core_contracts::syndicate::Guild> = resp.json();
    assert!(guilds.iter().any(|g| g.id == guild_id));

    // 3. Add Member
    let other_agent_id = uuid::Uuid::new_v4();
    let add_req = serde_json::json!({
        "agent_id": other_agent_id,
        "role": "contributor"
    });
    let resp = server
        .post(&format!("/api/v1/syndicate/guilds/{}/members", guild_id))
        .add_header("Authorization", &bearer)
        .json(&add_req)
        .await;
    resp.assert_status(axum::http::StatusCode::OK);

    // 4. List Members
    let resp = server
        .get(&format!("/api/v1/syndicate/guilds/{}/members", guild_id))
        .add_header("Authorization", &bearer)
        .await;
    resp.assert_status(axum::http::StatusCode::OK);
    let members: Vec<aiome_core_contracts::syndicate::GuildMember> = resp.json();
    assert_eq!(members.len(), 2); // Owner + New Member

    // 5. Delete Guild
    let resp = server
        .delete(&format!("/api/v1/syndicate/guilds/{}", guild_id))
        .add_header("Authorization", &bearer)
        .await;
    resp.assert_status(axum::http::StatusCode::OK);
}
#[serial]
#[tokio::test]
async fn test_syndicate_guild_sanitization() {
    let (server, _state, _tmp_dir) = create_test_server().await;
    let bearer = test_bearer();

    // 1. Create Guild with "dirty" input
    let create_req = serde_json::json!({
        "name": "<script>alert('xss')</script>Safe Guild",
        "description": "<b>Description</b> with <iframe src='malicious.com'></iframe>"
    });
    let resp = server
        .post("/api/v1/syndicate/guilds")
        .add_header("Authorization", &bearer)
        .json(&create_req)
        .await;
    resp.assert_status(axum::http::StatusCode::CREATED);
    let guild_id: uuid::Uuid = resp.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    // 2. Fetch Guilds and verify sanitization
    let resp = server
        .get("/api/v1/syndicate/guilds")
        .add_header("Authorization", &bearer)
        .await;
    resp.assert_status(axum::http::StatusCode::OK);
    let guilds: Vec<aiome_core_contracts::syndicate::Guild> = resp.json();
    let guild = guilds
        .iter()
        .find(|g| g.id == guild_id)
        .expect("Guild not found");

    // Expecting: "Safe Guild" and "Description with " (or similar depending on purge_entities)
    // purge_entities usually strips tags.
    assert_eq!(guild.name, "Safe Guild");
    assert_eq!(guild.description.as_ref().unwrap(), "Description with");
}
#[serial]
#[tokio::test]
async fn test_create_checkout_session() {
    let (server, _state, _tmp_dir) = create_test_server().await;

    let agent_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

    // 正常系 (mock実装は "cs_test_mock" を返す)
    let res = server
        .post("/api/v1/commerce/checkout-session/create")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&serde_json::json!({
            "agent_id": agent_id.to_string(),
            "price_id": "price_123",
            "success_url": "https://example.com/success",
            "cancel_url": "https://example.com/cancel"
        }))
        .await;

    assert_eq!(res.status_code(), 200);
    let json: serde_json::Value = res.json();
    assert_eq!(json["url"].as_str().unwrap(), "cs_test_mock");

    // 異常系: 他人の agent_id で作成しようとする
    let res_forbidden = server
        .post("/api/v1/commerce/checkout-session/create")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&serde_json::json!({
            "agent_id": uuid::Uuid::new_v4().to_string(), // another agent
            "price_id": "price_123",
            "success_url": "https://example.com/success",
            "cancel_url": "https://example.com/cancel"
        }))
        .await;

    assert_eq!(res_forbidden.status_code(), 403);

    // 異常系: 不正なURLスキーム (http://)
    let res_bad_url = server
        .post("/api/v1/commerce/checkout-session/create")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&serde_json::json!({
            "agent_id": agent_id.to_string(),
            "price_id": "price_123",
            "success_url": "http://example.com/success",
            "cancel_url": "https://example.com/cancel"
        }))
        .await;

    assert_eq!(res_bad_url.status_code(), 400);

    // 異常系: cancel_url 側が不正なスキーム
    let res_bad_cancel = server
        .post("/api/v1/commerce/checkout-session/create")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&serde_json::json!({
            "agent_id": agent_id.to_string(),
            "price_id": "price_123",
            "success_url": "https://example.com/success",
            "cancel_url": "http://example.com/cancel"
        }))
        .await;

    assert_eq!(res_bad_cancel.status_code(), 400);
}
#[serial]
#[tokio::test]
async fn test_commerce_release_escrow_idor() {
    let (server, _state, _tmp) = create_test_server().await;

    let bearer =
        "Bearer mock_valid_token_ekyc_test_user:00000000-0000-0000-0000-000000000001".to_string();

    // Attempt to release an escrow we OWN
    let valid_payload = serde_json::json!({
        "recipient_id": uuid::Uuid::new_v4().to_string()
    });

    let res_valid = server
        .post("/api/v1/commerce/escrow/valid_escrow_123/release")
        .add_header(axum::http::header::AUTHORIZATION, bearer.clone())
        .json(&valid_payload)
        .await;

    assert_eq!(res_valid.status_code(), reqwest::StatusCode::OK);

    // Attempt to release an escrow we DO NOT OWN
    let res_invalid = server
        .post("/api/v1/commerce/escrow/other_users_escrow_456/release")
        .add_header(axum::http::header::AUTHORIZATION, bearer)
        .json(&valid_payload)
        .await;

    // This should fail due to IDOR protection
    assert_eq!(res_invalid.status_code(), reqwest::StatusCode::FORBIDDEN);
}
#[serial]
#[tokio::test]
async fn test_stripe_webhook_invoice_paid_unlocks_account() {
    let (server, state, _tmp) = create_test_server().await;

    // Set up DB tables
    let sqlite_pool = state.db_pool.get_inner().get_sqlite_pool().unwrap();
    // sqlx::query(
    //     "CREATE TABLE IF NOT EXISTS stripe_customers (id TEXT PRIMARY KEY, customer_id TEXT UNIQUE NOT NULL, agent_id TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
    // ).execute(sqlite_pool).await.unwrap();
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS stripe_webhook_events (event_id TEXT PRIMARY KEY, event_type TEXT NOT NULL, metadata TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
    ).execute(sqlite_pool).await.unwrap();
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS system_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, category TEXT NOT NULL, is_secret BOOLEAN NOT NULL, updated_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
    ).execute(sqlite_pool).await.unwrap();

    let agent_id = "agent-123456";
    let customer_id = "cus_test123";
    sqlx::query("INSERT INTO stripe_customers (id, customer_id, agent_id) VALUES ('1', ?, ?)")
        .bind(customer_id)
        .bind(agent_id)
        .execute(sqlite_pool)
        .await
        .unwrap();

    let payload = serde_json::json!({
        "id": "evt_test123",
        "type": "invoice.paid",
        "data": {
            "object": {
                "customer": customer_id,
                "subscription": "sub_test123"
            }
        }
    });

    let resp = server
        .post("/api/v1/commerce/webhook")
        .add_header("stripe-signature", "dummy_sig")
        .json(&payload)
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);

    // Verify setting was changed to false
    use aiome_core::traits::SettingsOps;
    let setting_key = format!("agency.{}.mcp_suspended", agent_id);
    let setting = state
        .job_queue
        .get_inner()
        .get_setting_value(&setting_key)
        .await
        .unwrap();
    assert_eq!(setting.as_deref(), Some("false"));
}
#[serial]
#[tokio::test]
async fn test_stripe_webhook_payment_failed_suspends_account() {
    let (server, state, _tmp) = create_test_server().await;

    let sqlite_pool = state.db_pool.get_inner().get_sqlite_pool().unwrap();
    // sqlx::query(
    //     "CREATE TABLE IF NOT EXISTS stripe_customers (id TEXT PRIMARY KEY, customer_id TEXT UNIQUE NOT NULL, agent_id TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
    // ).execute(sqlite_pool).await.unwrap();
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS stripe_webhook_events (event_id TEXT PRIMARY KEY, event_type TEXT NOT NULL, metadata TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
    ).execute(sqlite_pool).await.unwrap();
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS system_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, category TEXT NOT NULL, is_secret BOOLEAN NOT NULL, updated_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
    ).execute(sqlite_pool).await.unwrap();

    let agent_id = "agent-123456";
    let customer_id = "cus_test123";
    sqlx::query("INSERT INTO stripe_customers (id, customer_id, agent_id) VALUES ('1', ?, ?)")
        .bind(customer_id)
        .bind(agent_id)
        .execute(sqlite_pool)
        .await
        .unwrap();

    let payload = serde_json::json!({
        "id": "evt_test999",
        "type": "invoice.payment_failed",
        "data": {
            "object": {
                "customer": customer_id,
                "subscription": "sub_test123"
            }
        }
    });

    let resp = server
        .post("/api/v1/commerce/webhook")
        .add_header("stripe-signature", "dummy_sig")
        .json(&payload)
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);

    // Verify setting was changed to true
    use aiome_core::traits::SettingsOps;
    let setting_key = format!("agency.{}.mcp_suspended", agent_id);
    let setting = state
        .job_queue
        .get_inner()
        .get_setting_value(&setting_key)
        .await
        .unwrap();
    assert_eq!(setting.as_deref(), Some("true"));
}
#[serial]
#[tokio::test]
async fn test_stripe_webhook_rejects_missing_signature() {
    let (server, _state, _tmp) = create_test_server().await;

    let payload = serde_json::json!({
        "id": "evt_test123",
        "type": "invoice.paid"
    });

    // Send POST without stripe-signature header
    let resp = server.post("/api/v1/commerce/webhook").json(&payload).await;

    // Positive check for Negative Test verification protocol (Should reject)
    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);

    let json = resp.json::<serde_json::Value>();
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("Missing stripe-signature header"));
}
#[serial]
#[tokio::test]
async fn test_stripe_webhook_checkout_session_completed_syncs_to_nurture_ledger() {
    // 1. Setup Mock Nurture API Server before creating the test server
    let sync_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter_clone = sync_counter.clone();

    let mock_nurture_app = axum::Router::new().route(
        "/internal/coin-charge",
        axum::routing::post(move |_req: axum::extract::Request| async move {
            counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            axum::response::Json(serde_json::json!({"status": "success"}))
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let nurture_url = format!("http://127.0.0.1:{}", port);

    tokio::spawn(async move {
        axum::serve(listener, mock_nurture_app).await.unwrap();
    });

    // Configure State via Environment Variables
    std::env::set_var("NURTURE_API_URL", &nurture_url);
    std::env::set_var("NURTURE_INTERNAL_SECRET", "mock_secret");

    let (server, state, _tmp) = create_test_server().await;

    // Create customers and events table for the sqlite DB
    let pool = state.db_pool.get_sqlite_pool().unwrap();
    // sqlx::query(
    //     "CREATE TABLE IF NOT EXISTS stripe_customers (id TEXT PRIMARY KEY, customer_id TEXT UNIQUE NOT NULL, agent_id TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
    // )
    // .execute(pool)
    // .await
    // .unwrap();
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS stripe_webhook_events (event_id TEXT PRIMARY KEY, event_type TEXT NOT NULL, metadata TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
    )
    .execute(pool)
    .await
    .unwrap();
    // Insert dummy asset into registry using proper API
    let asset_manifest = infrastructure::registry::AssetManifest {
        id: uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
        creator_id: uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
        asset_type: infrastructure::registry::AssetType::LoRA,
        name: "Mock Asset".to_string(),
        description: "desc".to_string(),
        price_coins: 5000,
        safety_level: aiome_core_contracts::contracts::ToolSafetyLevel::Safe,
        metadata: None,
    };

    // Attempting to create the table first just in case
    let pool = state.db_pool.get_sqlite_pool().unwrap();
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS asset_registry (id TEXT PRIMARY KEY, creator_id TEXT NOT NULL, asset_type TEXT NOT NULL, name TEXT NOT NULL, description TEXT, price_coins INTEGER NOT NULL DEFAULT 0, safety_level TEXT NOT NULL DEFAULT 'safe', metadata TEXT, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
    )
    .execute(pool)
    .await;

    state
        .registry
        .get_inner()
        .register_asset(asset_manifest)
        .await
        .unwrap();

    // Set Stripe Secret for signature generation
    std::env::set_var("STRIPE_WEBHOOK_SECRET", "whsec_mock_secret");

    // 3. Construct Payload
    let payload = json!({
        "id": "evt_mock123",
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "id": "cs_mock123",
                "customer": "cus_mock123",
                "amount_total": 5000,
                "currency": "jpy",
                "payment_status": "paid",
                "metadata": {
                    "agent_id": "00000000-0000-0000-0000-000000000001",
                    "asset_id": "00000000-0000-0000-0000-000000000002"
                }
            }
        }
    });

    let payload_str = payload.to_string();

    // Generate valid signature
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(b"whsec_mock_secret").unwrap();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let signed_payload = format!("{}.{}", timestamp, payload_str);
    mac.update(signed_payload.as_bytes());
    let sig_hash = hex::encode(mac.finalize().into_bytes());
    let sig_header = format!("t={},v1={}", timestamp, sig_hash);

    // Subscribe to event_sender BEFORE sending the webhook
    let mut rx = state.event_sender.get_inner().subscribe();

    // 4. Send Webhook Request
    let response = server
        .post("/api/v1/commerce/webhook")
        .add_header("stripe-signature", sig_header)
        .json(&payload)
        .await;

    response.assert_status(axum::http::StatusCode::OK);

    // 5. Verify Nurture Ledger Synchronization
    // Wait for the background spawn task for Nurture sync to complete (max 3 seconds)
    let mut sync_count = 0;
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        sync_count = sync_counter.load(std::sync::atomic::Ordering::SeqCst);
        if sync_count >= 1 {
            break;
        }
    }

    // [Verification Protocol: Negative Test -> Positive Result]
    // If commerce_webhook doesn't implement the sync, this will fail.
    assert_eq!(
        sync_count, 1,
        "Webhook should have synced to Nurture Ledger"
    );

    // 6. Verify SSE Broadcast via CoreEvent::CommerceEvent
    let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("Timeout waiting for CommerceEvent event")
        .expect("Failed to receive event");

    match event {
        aiome_core_contracts::events::CoreEvent::CommerceEvent {
            event_type,
            amount,
            currency,
            ..
        } => {
            assert_eq!(event_type, "checkout.session.completed");
            assert_eq!(amount, 5000);
            assert_eq!(currency, "jpy");
        }
        _ => panic!("Expected CommerceEvent event, got another event"),
    }
}

#[serial]
#[tokio::test]
async fn test_stripe_webhook_subscription_deleted_suspends_account() {
    let (server, state, _tmp) = create_test_server().await;

    let sqlite_pool = state.db_pool.get_inner().get_sqlite_pool().unwrap();
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS stripe_webhook_events (event_id TEXT PRIMARY KEY, event_type TEXT NOT NULL, metadata TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
    ).execute(sqlite_pool).await.unwrap();

    let agent_id = "agent-deleted-123";
    let customer_id = "cus_deleted123";
    sqlx::query("INSERT INTO stripe_customers (id, customer_id, agent_id) VALUES ('2', ?, ?)")
        .bind(customer_id)
        .bind(agent_id)
        .execute(sqlite_pool)
        .await
        .unwrap();

    let payload = serde_json::json!({
        "id": "evt_sub_del_123",
        "type": "customer.subscription.deleted",
        "data": {
            "object": {
                "customer": customer_id,
                "subscription": "sub_del_123"
            }
        }
    });

    let resp = server
        .post("/api/v1/commerce/webhook")
        .add_header("stripe-signature", "dummy_sig")
        .json(&payload)
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);

    use aiome_core::traits::SettingsOps;
    let setting_key = format!("agency.{}.mcp_suspended", agent_id);
    let setting = state
        .job_queue
        .get_inner()
        .get_setting_value(&setting_key)
        .await
        .unwrap();
    assert_eq!(setting.as_deref(), Some("true"));
}

#[serial]
#[tokio::test]
async fn test_stripe_webhook_subscription_updated_past_due_suspends_account() {
    let (server, state, _tmp) = create_test_server().await;

    let sqlite_pool = state.db_pool.get_inner().get_sqlite_pool().unwrap();
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS stripe_webhook_events (event_id TEXT PRIMARY KEY, event_type TEXT NOT NULL, metadata TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
    ).execute(sqlite_pool).await.unwrap();

    let agent_id = "agent-updated-123";
    let customer_id = "cus_updated123";
    sqlx::query("INSERT INTO stripe_customers (id, customer_id, agent_id) VALUES ('3', ?, ?)")
        .bind(customer_id)
        .bind(agent_id)
        .execute(sqlite_pool)
        .await
        .unwrap();

    let payload = serde_json::json!({
        "id": "evt_sub_upd_123",
        "type": "customer.subscription.updated",
        "data": {
            "object": {
                "customer": customer_id,
                "subscription": "sub_upd_123",
                "status": "past_due"
            }
        }
    });

    let resp = server
        .post("/api/v1/commerce/webhook")
        .add_header("stripe-signature", "dummy_sig")
        .json(&payload)
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);

    use aiome_core::traits::SettingsOps;
    let setting_key = format!("agency.{}.mcp_suspended", agent_id);
    let setting = state
        .job_queue
        .get_inner()
        .get_setting_value(&setting_key)
        .await
        .unwrap();
    assert_eq!(setting.as_deref(), Some("true"));
}

#[serial]
#[tokio::test]
async fn test_stripe_webhook_dispute_created_suspends_account() {
    let (server, state, _tmp) = create_test_server().await;

    let sqlite_pool = state.db_pool.get_inner().get_sqlite_pool().unwrap();
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS stripe_webhook_events (event_id TEXT PRIMARY KEY, event_type TEXT NOT NULL, metadata TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
    ).execute(sqlite_pool).await.unwrap();

    let agent_id = "00000000-0000-0000-0000-000000000999";
    let payload = serde_json::json!({
        "id": "evt_dispute_123",
        "type": "charge.dispute.created",
        "data": {
            "object": {
                "id": "dp_123",
                "metadata": {
                    "agent_id": agent_id
                }
            }
        }
    });

    let mut rx = state.event_sender.get_inner().subscribe();

    let resp = server
        .post("/api/v1/commerce/webhook")
        .add_header("stripe-signature", "dummy_sig")
        .json(&payload)
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);

    use aiome_core::traits::SettingsOps;
    let setting_key = format!("agency.{}.mcp_suspended", agent_id);
    let setting = state
        .job_queue
        .get_inner()
        .get_setting_value(&setting_key)
        .await
        .unwrap();
    assert_eq!(setting.as_deref(), Some("true"));

    // Verify SSE Broadcast for dispute
    let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("Timeout waiting for dispute CommerceEvent")
        .expect("Failed to receive event");

    match event {
        aiome_core_contracts::events::CoreEvent::CommerceEvent {
            event_type,
            agent_id: ev_agent_id,
            description,
            ..
        } => {
            assert_eq!(event_type, "dispute_received");
            assert_eq!(ev_agent_id.to_string(), agent_id);
            assert!(description.contains("evt_dispute_123"));
        }
        _ => panic!("Expected CommerceEvent for dispute, got another event"),
    }
}

#[serial]
#[tokio::test]
async fn test_preflight_production_mode_rejects_missing_price_id() {
    std::env::set_var("STRIPE_TEST_MODE", "false");
    std::env::remove_var("STRIPE_PRICE_SUBSCRIPTION_MONTHLY");

    let result = crate::bootstrap::preflight::init_env_and_preflight().await;

    std::env::set_var("STRIPE_TEST_MODE", "true");

    assert!(
        result.is_err(),
        "Preflight must reject startup in production mode if STRIPE_PRICE_SUBSCRIPTION_MONTHLY is not set"
    );

    let err_str = result.err().unwrap().to_string();
    assert!(
        err_str.contains("STRIPE_PRICE_SUBSCRIPTION_MONTHLY must be set in production mode")
            || err_str.contains("must be set in production"),
        "Actual preflight error: {}",
        err_str
    );
}

#[serial]
#[tokio::test]
async fn test_commerce_price_id_env_loading() {
    std::env::set_var(
        "STRIPE_PRICE_SUBSCRIPTION_MONTHLY",
        "price_test_overwrite_12345",
    );

    let (_server, state, _tmp) = create_test_server().await;

    std::env::remove_var("STRIPE_PRICE_SUBSCRIPTION_MONTHLY");

    assert_eq!(
        state.stripe_price_subscription_monthly.as_deref(),
        Some("price_test_overwrite_12345")
    );
}

#[serial]
#[tokio::test]
async fn test_commerce_price_id_dynamic_replacement() {
    std::env::set_var(
        "STRIPE_PRICE_SUBSCRIPTION_MONTHLY",
        "price_test_overwrite_99999",
    );
    let (server, _state, _tmp) = create_test_server().await;
    std::env::remove_var("STRIPE_PRICE_SUBSCRIPTION_MONTHLY");

    let bearer = "Bearer mock_valid_token_ekyctest_user".to_string();

    // 1. Create Subscription: plan_id "price_gold_monthly" should be dynamically replaced
    let payload = serde_json::json!({
        "agent_id": "00000000-0000-0000-0000-000000000001",
        "plan_id": "price_gold_monthly"
    });

    let resp = server
        .post("/api/v1/commerce/subscription/create")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let json = resp.json::<serde_json::Value>();
    assert_eq!(json["subscription_id"], "sub_mock_overwritten");

    // 2. Create Checkout Session: price_id "price_gold_monthly" should be dynamically replaced
    let payload_cs = serde_json::json!({
        "agent_id": "00000000-0000-0000-0000-000000000001",
        "price_id": "price_gold_monthly",
        "success_url": "https://localhost/success",
        "cancel_url": "https://localhost/cancel"
    });

    let resp_cs = server
        .post("/api/v1/commerce/checkout-session/create")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload_cs)
        .await;

    assert_eq!(resp_cs.status_code(), StatusCode::OK);
    let json_cs = resp_cs.json::<serde_json::Value>();
    assert_eq!(json_cs["url"], "cs_test_overwritten");
}
