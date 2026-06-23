/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::common::*;
use axum::http::StatusCode;
use serial_test::serial;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[serial]
#[tokio::test]
async fn test_vault_routes_rbac() {
    // テスト用のダミー設定を環境変数に仕込む
    std::env::set_var("TEST_KEY_PROXY_URL", "http://localhost:9999");
    std::env::set_var("TEST_VAULT_SECRET", "test_vault_secret");

    let (server, _state, _tmp) = create_test_server().await;

    // 1. 認証なしの場合 -> 401 Unauthorized
    let resp = server.get("/api/v1/vault/status").await;
    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);

    // 2. 一般ユーザーの場合 -> 403 Forbidden
    let user_bearer = "Bearer mock_valid_token_user_123".to_string();
    let resp = server
        .get("/api/v1/vault/status")
        .add_header(axum::http::header::AUTHORIZATION, &user_bearer)
        .await;
    assert_eq!(resp.status_code(), StatusCode::FORBIDDEN);
}

#[serial]
#[tokio::test]
async fn test_vault_routes_admin_ok() {
    // 1. Mock Server の起動
    let mock_server = MockServer::start().await;

    // 2. Mock の登録 (GET /api/v1/admin/status)
    let mock_status_response = serde_json::json!({
        "secrets": [
            { "key": "GEMINI_API_KEY", "category": "ai", "is_set": true }
        ],
        "total": 1,
        "configured": 1
    });

    Mock::given(method("GET"))
        .and(path("/api/v1/admin/status"))
        .and(header("Authorization", "Bearer test_vault_secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_status_response))
        .mount(&mock_server)
        .await;

    // Mock の登録 (PUT /api/v1/admin/secrets)
    Mock::given(method("PUT"))
        .and(path("/api/v1/admin/secrets"))
        .and(header("Authorization", "Bearer test_vault_secret"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Mock の登録 (DELETE /api/v1/admin/secrets/GEMINI_API_KEY)
    Mock::given(method("DELETE"))
        .and(path("/api/v1/admin/secrets/GEMINI_API_KEY"))
        .and(header("Authorization", "Bearer test_vault_secret"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // 環境変数経由でモックサーバーの URL とシークレットを設定
    std::env::set_var("TEST_KEY_PROXY_URL", mock_server.uri());
    std::env::set_var("TEST_VAULT_SECRET", "test_vault_secret");

    let (server, _state, _tmp) = create_test_server().await;
    let admin_bearer = "Bearer mock_valid_token_admin".to_string();

    // 3. API 呼び出しのテスト (status)
    let resp = server
        .get("/api/v1/vault/status")
        .add_header(axum::http::header::AUTHORIZATION, &admin_bearer)
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["total"], 1);

    // 4. API 呼び出しのテスト (upsert)
    let resp_put = server
        .put("/api/v1/vault/secrets")
        .add_header(axum::http::header::AUTHORIZATION, &admin_bearer)
        .json(&serde_json::json!({
            "key": "GEMINI_API_KEY",
            "value": "secret_val"
        }))
        .await;
    assert_eq!(resp_put.status_code(), StatusCode::OK);

    // 5. API 呼び出しのテスト (delete)
    let resp_del = server
        .delete("/api/v1/vault/secrets/GEMINI_API_KEY")
        .add_header(axum::http::header::AUTHORIZATION, &admin_bearer)
        .await;
    assert_eq!(resp_del.status_code(), StatusCode::OK);
}
