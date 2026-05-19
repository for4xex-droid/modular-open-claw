/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::common::*;
use axum::http::StatusCode;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_setup_init_flow() {
    let (server, _state, _tmp) = create_test_server().await;

    // Initially, no admin account
    let payload = json!({
        "admin_email": "admin@example.com",
        "admin_password": "supersecurepassword123",
        "ai_name": "MyTestAI",
        "view_mode": "expert",
        "language": "en",
        "tos_accepted": true
    });

    let resp = server.post("/api/v1/setup/init").json(&payload).await;

    // Should succeed and return JWT
    assert_eq!(resp.status_code(), StatusCode::OK);
    let json: serde_json::Value = resp.json();
    assert!(json.get("access_token").is_some());

    // Second call should fail (403 Forbidden)
    let resp2 = server.post("/api/v1/setup/init").json(&payload).await;
    assert_eq!(resp2.status_code(), StatusCode::FORBIDDEN);
}
