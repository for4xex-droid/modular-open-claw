/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use crate::api_integration_tests::create_test_server;
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn test_stripe_webhook_checkout_completed() -> anyhow::Result<()> {
    let (server, _state, _tmp) = create_test_server().await;

    let agent_id = uuid::Uuid::new_v4().to_string();
    let asset_id = uuid::Uuid::new_v4().to_string();

    let pool = _state.db_pool.get_inner().get_sqlite_pool_or_err()?;
    sqlx::query("INSERT INTO asset_registry (id, creator_id, asset_type, name, description, price_coins, safety_level) VALUES ($1, $2, 'lora', 'Test Asset', 'Desc', 1000, 'safe')")
        .bind(&asset_id)
        .bind(&agent_id)
        .execute(pool).await?;

    let payload = json!({
        "id": "evt_test_123",
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "id": "cs_test_123",
                "client_reference_id": "test-user-id",
                "amount_total": 1000,
                "metadata": {
                    "agent_id": agent_id,
                    "asset_id": asset_id
                }
            }
        }
    });

    let response = server
        .post("/api/v1/commerce/webhook")
        .add_header("stripe-signature", "t=123,v1=test_sig")
        .json(&payload)
        .await;

    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "Webhook failed: {}",
        response.text()
    );

    // Verify idempotency
    let repeat_response = server
        .post("/api/v1/commerce/webhook")
        .add_header("stripe-signature", "t=123,v1=test_sig")
        .json(&payload)
        .await;

    assert_eq!(repeat_response.status_code(), StatusCode::OK);
    Ok(())
}

#[tokio::test]
async fn test_mcp_billing_guard_rejection() -> anyhow::Result<()> {
    let (server, _state, _tmp) = create_test_server().await;

    // The local MockCommerceEngine in api_integration_tests.rs is hardcoded to reject validate_activity
    // ONLY for the agent_id "00000000-0000-0000-0000-fa1100000000".
    let auth_token =
        "Bearer mock_valid_token_testuser:00000000-0000-0000-0000-fa1100000000".to_string();

    // Simulate an MCP invocation without sufficient funds
    // Use a whitelisted package to pass the Security Gate and hit the Billing Guard
    let spawn_payload = json!({
        "id": "test-client-id",
        "command": "uvx",
        "args": ["@modelcontextprotocol/server-sqlite", "--db-path", "/tmp/test.db"]
    });

    let response = server
        .post("/api/skills/mcp/spawn")
        .add_header(axum::http::header::AUTHORIZATION, auth_token)
        .json(&spawn_payload)
        .await;

    let status = response.status_code();
    let text = response.text();
    println!("Status: {}, Text: {}", status, text);

    // We expect a payment rejection due to insufficient balance.
    // The exact error message should reflect the Billing Guard logic.
    assert!(
        status.is_client_error(),
        "Expected client error, got {}",
        status
    );
    assert!(
        text.contains("MCP Billing Guard rejected spawn"),
        "Expected specific Billing Guard rejection message, got: {}",
        text
    );
    Ok(())
}
