/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::common::{create_test_server, test_bearer};
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn test_compliance_ban_guard_normal() {
    let (server, _state, _tmp) = create_test_server().await;

    // 1. Positive Test: 通常のリクエストは正常に通過することを確認
    let response = server
        .get("/api/v1/settings")
        .add_header("Authorization", test_bearer())
        .await;

    // 通常設定エンドポイントは200 OK（未設定の場合は初期値が返るなど）
    assert!(
        response.status_code() == StatusCode::OK || response.status_code() == StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn test_compliance_ban_guard_blocked() {
    let (server, state, _tmp) = create_test_server().await;

    let agent_id = uuid::Uuid::new_v4();
    let token = format!("mock_valid_token_banned_user:{}", agent_id);
    let bearer = format!("Bearer {}", token);

    // 1. BAN前の状態確認 (正常に通過するはず)
    let response = server
        .get("/api/v1/settings")
        .add_header("Authorization", bearer.clone())
        .await;
    assert!(
        response.status_code() == StatusCode::OK || response.status_code() == StatusCode::NOT_FOUND
    );

    // 2. Negative Test: アカウントを違反としてBANする
    state
        .ban_store
        .ban(
            &agent_id,
            "CSAM Violation Detected",
            "CRITICAL",
            "automated_guardian",
        )
        .await
        .unwrap();

    // 3. BAN後の状態確認 (403 Forbidden が Fail-Closed で返却されるべき)
    let response = server
        .get("/api/v1/settings")
        .add_header("Authorization", bearer.clone())
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);

    let body_text = response.text();
    assert!(body_text.contains("suspended") || body_text.contains("compliance"));

    // 4. Revert: BANを解除する
    state.ban_store.unban(&agent_id).await.unwrap();

    // 5. 解除後の確認 (再び通過可能)
    let response = server
        .get("/api/v1/settings")
        .add_header("Authorization", bearer)
        .await;
    assert!(
        response.status_code() == StatusCode::OK || response.status_code() == StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn test_compliance_ban_guard_fail_closed() {
    let (server, state, _tmp) = create_test_server().await;

    let agent_id = uuid::Uuid::new_v4();
    let token = format!("mock_valid_token_unlucky_user:{}", agent_id);
    let bearer = format!("Bearer {}", token);

    // 1. Positive: エラーが発生していないときは正常通過
    let response = server
        .get("/api/v1/settings")
        .add_header("Authorization", bearer.clone())
        .await;
    assert!(
        response.status_code() == StatusCode::OK || response.status_code() == StatusCode::NOT_FOUND
    );

    // 2. Negative: MockBanStore で意図的にエラーフラグを立てて、接続失敗をシミュレート
    let ban_store_any = state.ban_store.as_any();
    let mock = ban_store_any
        .downcast_ref::<infrastructure::compliance::ban_store::MockBanStore>()
        .expect("Test server should use MockBanStore");

    mock.set_should_fail(true);

    // 3. 検証 (Fail-Closed原則により、データベース障害時は 503 Service Unavailable で安全にリクエストが拒否される)
    let response = server
        .get("/api/v1/settings")
        .add_header("Authorization", bearer.clone())
        .await;

    assert_eq!(response.status_code(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(response.text().contains("Compliance check failed"));

    // 4. Revert: エラー状態を戻す
    mock.set_should_fail(false);

    // 5. 復旧後、再び正常に通過することを確認
    let response = server
        .get("/api/v1/settings")
        .add_header("Authorization", bearer)
        .await;
    assert!(
        response.status_code() == StatusCode::OK || response.status_code() == StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn test_compliance_exempt_cancellation() {
    let (server, state, _tmp) = create_test_server().await;

    // ekyc から始まる sub にすることで MockAuthManager が ekyc_verified: true を返す
    let agent_id = uuid::Uuid::new_v4();
    let token = format!("mock_valid_token_ekyccustomer:{}", agent_id);
    let bearer = format!("Bearer {}", token);

    // 1. まず通常時、サブスク解約が叩けること（モック経由なので最終的に200 OK）
    let payload = json!({
        "agent_id": agent_id,
        "subscription_id": "sub_123456"
    });

    let response = server
        .post("/api/v1/commerce/subscription/cancel")
        .add_header("Authorization", bearer.clone())
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    // 2. このユーザーを違反行為でBANする
    state
        .ban_store
        .ban(&agent_id, "Abuse behavior", "HIGH", "admin")
        .await
        .unwrap();

    // 3. 通常の認証API（/settings）は BANガードにより 403 Forbidden で拒否されることを確認
    let response_settings = server
        .get("/api/v1/settings")
        .add_header("Authorization", bearer.clone())
        .await;
    assert_eq!(response_settings.status_code(), StatusCode::FORBIDDEN);

    // 4. 消費者保護例外 (Positive例外): サブスクキャンセルは、BANされているアクターでも例外的に通過することを確認！
    let response_cancel = server
        .post("/api/v1/commerce/subscription/cancel")
        .add_header("Authorization", bearer)
        .json(&payload)
        .await;

    assert_eq!(response_cancel.status_code(), StatusCode::OK);
}
