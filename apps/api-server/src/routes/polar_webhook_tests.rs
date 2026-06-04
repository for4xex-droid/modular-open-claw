/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#[cfg(test)]
mod tests {
    use crate::api_integration_tests::create_test_server;
    use axum::http::StatusCode;
    use serde_json::json;
    use serial_test::serial;

    // Helper to generate a dummy signature for Polar (svix format)
    fn generate_polar_signature(
        webhook_id: &str,
        timestamp: &str,
        body: &str,
        secret: &str,
    ) -> String {
        use base64::Engine;
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let to_sign = format!("{}.{}.{}", webhook_id, timestamp, body);
        let decoded_secret = base64::prelude::BASE64_STANDARD
            .decode(secret)
            .unwrap_or_default();
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(&decoded_secret).unwrap();
        mac.update(to_sign.as_bytes());
        let b64_sig = base64::prelude::BASE64_STANDARD.encode(mac.finalize().into_bytes());
        format!("v1,{}", b64_sig)
    }

    #[serial]
    #[tokio::test]
    async fn test_polar_webhook_verification_fails_with_bad_sig() {
        std::env::set_var("POLAR_API_KEY", "test_key");
        std::env::set_var(
            "POLAR_WEBHOOK_SECRET",
            "whsec_dGVzdF9zZWNyZXRfZm9yX2htYWNfMTIzNDU2Nzg5",
        );
        let (server, _state, _tmp) = create_test_server().await;

        let payload = json!({
            "id": "evt_123",
            "type": "checkout.completed",
            "data": {
                "id": "chk_123"
            }
        });

        let response = server
            .post("/api/v1/commerce/webhook/polar")
            .add_header("webhook-id", "msg_123")
            .add_header("webhook-timestamp", "1614556800")
            .add_header("webhook-signature", "v1,invalid_sig")
            .json(&payload)
            .await;

        // signature verification should fail
        assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
    }

    #[serial]
    #[tokio::test]
    async fn test_polar_webhook_checkout_completed_success() {
        // Setup mock environment variables before server creation
        std::env::set_var("POLAR_API_KEY", "test_key");
        std::env::set_var(
            "POLAR_WEBHOOK_SECRET",
            "whsec_dGVzdF9zZWNyZXRfZm9yX2htYWNfMTIzNDU2Nzg5",
        ); // base64 encoded "test_secret_for_hmac_123456789"

        let (server, state, _tmp) = create_test_server().await;
        let registry = state.registry.clone();

        let agent_id = uuid::Uuid::new_v4();
        let asset_id = uuid::Uuid::new_v4();

        // 1. Register dummy asset in the registry
        let pool = state.db_pool.get_sqlite_pool().unwrap();
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS asset_registry (id TEXT PRIMARY KEY, creator_id TEXT NOT NULL, asset_type TEXT NOT NULL, name TEXT NOT NULL, description TEXT, price_coins INTEGER NOT NULL DEFAULT 0, safety_level TEXT NOT NULL DEFAULT 'safe', metadata TEXT, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
        )
        .execute(pool)
        .await;

        let asset_manifest = infrastructure::registry::AssetManifest {
            id: asset_id,
            creator_id: uuid::Uuid::new_v4(),
            asset_type: infrastructure::registry::AssetType::LoRA,
            name: "Polar Mock Asset".to_string(),
            description: "Polar Test".to_string(),
            price_coins: 1000,
            safety_level: aiome_core_contracts::contracts::ToolSafetyLevel::Safe,
            metadata: None,
        };
        registry
            .get_inner()
            .register_asset(asset_manifest)
            .await
            .unwrap();

        // 2. Setup polar_webhook_events table in test DB if not exists
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS polar_webhook_events (event_id TEXT PRIMARY KEY, event_type TEXT NOT NULL, metadata TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
        )
        .execute(pool)
        .await
        .unwrap();

        // 3. Prepare checkout.completed event
        let payload = json!({
            "id": "evt_polar_chk_123",
            "type": "checkout.completed",
            "data": {
                "id": "chk_123",
                "amount_total": 1000,
                "metadata": {
                    "agent_id": agent_id.to_string(),
                    "asset_id": asset_id.to_string()
                }
            }
        });

        let payload_str = payload.to_string();
        let timestamp = "1614556800";
        let sig = generate_polar_signature(
            "msg_polar_chk_123",
            timestamp,
            &payload_str,
            "dGVzdF9zZWNyZXRfZm9yX2htYWNfMTIzNDU2Nzg5",
        );

        let response = server
            .post("/api/v1/commerce/webhook/polar")
            .add_header("webhook-id", "msg_polar_chk_123")
            .add_header("webhook-timestamp", timestamp)
            .add_header("webhook-signature", &sig)
            .json(&payload)
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);

        // Verify license was granted
        let is_owned = registry
            .get_inner()
            .check_ownership(agent_id, asset_id)
            .await
            .unwrap();
        assert!(
            is_owned,
            "License must be granted on Polar checkout completion"
        );
    }

    #[serial]
    #[tokio::test]
    async fn test_polar_webhook_subscription_lifecycle() {
        std::env::set_var("POLAR_API_KEY", "test_key");
        std::env::set_var(
            "POLAR_WEBHOOK_SECRET",
            "whsec_dGVzdF9zZWNyZXRfZm9yX2htYWNfMTIzNDU2Nzg5",
        );
        let (server, state, _tmp) = create_test_server().await;
        let pool = state.db_pool.get_sqlite_pool().unwrap();

        // Setup polar_webhook_events & settings table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS polar_webhook_events (event_id TEXT PRIMARY KEY, event_type TEXT NOT NULL, metadata TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS system_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, category TEXT NOT NULL, is_secret BOOLEAN NOT NULL, updated_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
        )
        .execute(pool)
        .await
        .unwrap();

        let agent_id = uuid::Uuid::new_v4();

        // 1. Test subscription.created (Unlock MCP)
        let payload_created = json!({
            "id": "evt_polar_sub_created_123",
            "type": "subscription.created",
            "data": {
                "id": "sub_123",
                "status": "active",
                "metadata": {
                    "actor_id": agent_id.to_string()
                }
            }
        });

        let payload_created_str = payload_created.to_string();
        let timestamp = "1614556800";
        let sig_created = generate_polar_signature(
            "msg_polar_sub_created_123",
            timestamp,
            &payload_created_str,
            "dGVzdF9zZWNyZXRfZm9yX2htYWNfMTIzNDU2Nzg5",
        );

        let response = server
            .post("/api/v1/commerce/webhook/polar")
            .add_header("webhook-id", "msg_polar_sub_created_123")
            .add_header("webhook-timestamp", timestamp)
            .add_header("webhook-signature", &sig_created)
            .json(&payload_created)
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);

        use aiome_core::traits::SettingsOps;
        let setting_key = format!("agency.{}.mcp_suspended", agent_id);
        let setting = state
            .job_queue
            .get_inner()
            .get_setting_value(&setting_key)
            .await
            .unwrap();
        assert_eq!(setting.as_deref(), Some("false"));

        // 2. Test subscription.deleted (Suspend MCP)
        let payload_deleted = json!({
            "id": "evt_polar_sub_deleted_123",
            "type": "subscription.deleted",
            "data": {
                "id": "sub_123",
                "status": "canceled",
                "metadata": {
                    "actor_id": agent_id.to_string()
                }
            }
        });

        let payload_deleted_str = payload_deleted.to_string();
        let sig_deleted = generate_polar_signature(
            "msg_polar_sub_deleted_123",
            timestamp,
            &payload_deleted_str,
            "dGVzdF9zZWNyZXRfZm9yX2htYWNfMTIzNDU2Nzg5",
        );

        let response_del = server
            .post("/api/v1/commerce/webhook/polar")
            .add_header("webhook-id", "msg_polar_sub_deleted_123")
            .add_header("webhook-timestamp", timestamp)
            .add_header("webhook-signature", &sig_deleted)
            .json(&payload_deleted)
            .await;

        assert_eq!(response_del.status_code(), StatusCode::OK);

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
    async fn test_polar_webhook_idempotency() {
        std::env::set_var("POLAR_API_KEY", "test_key");
        std::env::set_var(
            "POLAR_WEBHOOK_SECRET",
            "whsec_dGVzdF9zZWNyZXRfZm9yX2htYWNfMTIzNDU2Nzg5",
        );
        let (server, state, _tmp) = create_test_server().await;
        let pool = state.db_pool.get_sqlite_pool().unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS polar_webhook_events (event_id TEXT PRIMARY KEY, event_type TEXT NOT NULL, metadata TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);"
        )
        .execute(pool)
        .await
        .unwrap();

        let agent_id = uuid::Uuid::new_v4();

        let payload = json!({
            "id": "evt_polar_dup_123",
            "type": "subscription.created",
            "data": {
                "id": "sub_123",
                "status": "active",
                "metadata": {
                    "actor_id": agent_id.to_string()
                }
            }
        });

        let payload_str = payload.to_string();
        let timestamp = "1614556800";
        let sig = generate_polar_signature(
            "msg_polar_dup_123",
            timestamp,
            &payload_str,
            "dGVzdF9zZWNyZXRfZm9yX2htYWNfMTIzNDU2Nzg5",
        );

        // Send first time
        let response1 = server
            .post("/api/v1/commerce/webhook/polar")
            .add_header("webhook-id", "msg_polar_dup_123")
            .add_header("webhook-timestamp", timestamp)
            .add_header("webhook-signature", &sig)
            .json(&payload)
            .await;

        assert_eq!(response1.status_code(), StatusCode::OK);

        // Send second time with same event id
        let response2 = server
            .post("/api/v1/commerce/webhook/polar")
            .add_header("webhook-id", "msg_polar_dup_123")
            .add_header("webhook-timestamp", timestamp)
            .add_header("webhook-signature", &sig)
            .json(&payload)
            .await;

        // Second time should return OK but skip actual processing (idempotency check)
        assert_eq!(response2.status_code(), StatusCode::OK);
    }
}
