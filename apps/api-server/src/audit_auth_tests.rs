/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::*;
use crate::api_integration_tests::create_test_server;
use axum::http::StatusCode;
use axum_test::TestServer;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_audit_logs_access_rbac() {
    let (server, _state, _tmp) = create_test_server().await;

    // 1. Missing Token should be REJECTED (UNAUTHORIZED)
    let resp = server.get("/api/v1/logs").await;
    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);

    // 2. Admin should be allowed
    let admin_bearer = "Bearer mock_valid_token_admin";
    let resp = server
        .get("/api/v1/logs")
        .add_header(axum::http::header::AUTHORIZATION, admin_bearer)
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    // 3. System should be allowed
    let system_bearer = "Bearer mock_valid_token_system";
    let resp = server
        .get("/api/v1/logs")
        .add_header(axum::http::header::AUTHORIZATION, system_bearer)
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    // 4. Agent (Standard User) should be REJECTED (FORBIDDEN)
    let agent_bearer = "Bearer mock_valid_token_agent_user";
    let resp = server
        .get("/api/v1/logs")
        .add_header(axum::http::header::AUTHORIZATION, agent_bearer)
        .await;
    assert_eq!(resp.status_code(), StatusCode::FORBIDDEN);
}

#[serial]
#[tokio::test]
async fn test_audit_ledger_access_rbac() {
    let (server, _state, _tmp) = create_test_server().await;

    // Admin allowed
    let resp = server
        .get("/api/v1/audit/ledger")
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer mock_valid_token_admin",
        )
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    // Agent REJECTED
    let resp = server
        .get("/api/v1/audit/ledger")
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer mock_valid_token_agent_user",
        )
        .await;
    assert_eq!(resp.status_code(), StatusCode::FORBIDDEN);
}

#[serial]
#[tokio::test]
async fn test_audit_quarantine_release_rbac() {
    let (server, _state, _tmp) = create_test_server().await;

    // Admin ALLOWED (MockQuarantineStore returns Ok(()), so expect 200 OK)
    let resp = server
        .post("/api/v1/audit/quarantine/asset-123/release")
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer mock_valid_token_admin",
        )
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    // Agent REJECTED from POST release
    let resp = server
        .post("/api/v1/audit/quarantine/asset-123/release")
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer mock_valid_token_agent_user",
        )
        .await;
    assert_eq!(resp.status_code(), StatusCode::FORBIDDEN);
}
