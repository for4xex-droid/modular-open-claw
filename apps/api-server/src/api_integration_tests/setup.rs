/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::common::*;
use aiome_core_contracts::SettingsOps;
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

/// 同意証跡（docs/legal/CONSENT_SPEC.md §2）: tos_version が settings に記録されること
#[serial]
#[tokio::test]
async fn test_setup_init_records_tos_consent_trail() {
    let (server, state, _tmp) = create_test_server().await;

    let payload = json!({
        "admin_email": "admin@example.com",
        "admin_password": "supersecurepassword123",
        "ai_name": "MyTestAI",
        "view_mode": "simple",
        "language": "ja",
        "tos_accepted": true,
        "tos_version": "v2.1"
    });

    let resp = server.post("/api/v1/setup/init").json(&payload).await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let version = state
        .job_queue
        .get_inner()
        .get_setting_value("tos_accepted_version")
        .await
        .unwrap();
    assert_eq!(version.as_deref(), Some("v2.1"));

    let accepted_at = state
        .job_queue
        .get_inner()
        .get_setting_value("tos_accepted_at")
        .await
        .unwrap();
    assert!(
        accepted_at.is_some_and(|v| !v.is_empty()),
        "tos_accepted_at must be recorded"
    );
}

/// Negative: 規約非同意（tos_accepted=false）は 400 拒否
#[serial]
#[tokio::test]
async fn test_setup_init_rejects_without_tos_acceptance() {
    let (server, _state, _tmp) = create_test_server().await;

    let payload = json!({
        "admin_email": "admin@example.com",
        "admin_password": "supersecurepassword123",
        "ai_name": "MyTestAI",
        "view_mode": "simple",
        "language": "en",
        "tos_accepted": false,
        "tos_version": "v2.0"
    });

    let resp = server.post("/api/v1/setup/init").json(&payload).await;
    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
}
