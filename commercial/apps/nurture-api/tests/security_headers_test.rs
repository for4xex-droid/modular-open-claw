/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 */

use axum::http::{header, StatusCode};
use axum::routing::get;
use axum::Router;
use axum_test::TestServer;
use nurture_api::middleware::apply_security_middlewares;
use serial_test::serial;

/// テスト用の安全なサーバーを構築する。
fn build_test_app() -> Router {
    let app = Router::new().route("/dummy", get(|| async { "Hello" }));
    apply_security_middlewares(app)
}

// =========================================================================
// 1. セキュリティヘッダーの正常系テスト
// =========================================================================

#[tokio::test]
#[serial]
async fn test_security_headers_present() {
    // Arrange: CORS を全許可に設定
    std::env::set_var("NURTURE_CORS_ORIGIN", "*");
    let server = TestServer::new(build_test_app()).unwrap();

    // Act
    let res = server.get("/dummy").await;

    // Assert: ステータスコード
    assert_eq!(res.status_code(), StatusCode::OK);

    let headers = res.headers();

    // X-Content-Type-Options
    assert_eq!(
        headers
            .get(header::X_CONTENT_TYPE_OPTIONS)
            .expect("X-Content-Type-Options missing"),
        "nosniff"
    );

    // X-Frame-Options
    assert_eq!(
        headers
            .get(header::X_FRAME_OPTIONS)
            .expect("X-Frame-Options missing"),
        "DENY"
    );

    // Strict-Transport-Security (HSTS)
    assert_eq!(
        headers
            .get(header::STRICT_TRANSPORT_SECURITY)
            .expect("HSTS missing"),
        "max-age=31536000; includeSubDomains"
    );

    // Content-Security-Policy
    assert_eq!(
        headers.get("Content-Security-Policy").expect("CSP missing"),
        "default-src 'none'; frame-ancestors 'none'"
    );

    std::env::remove_var("NURTURE_CORS_ORIGIN");
}

// =========================================================================
// 2. CORS: 全許可モード (NURTURE_CORS_ORIGIN=*)
// =========================================================================

#[tokio::test]
#[serial]
async fn test_cors_permissive_mode() {
    // Arrange
    std::env::set_var("NURTURE_CORS_ORIGIN", "*");
    let server = TestServer::new(build_test_app()).unwrap();

    // Act: Origin ヘッダー付きの通常リクエスト
    let res = server
        .get("/dummy")
        .add_header(header::ORIGIN, "https://aiome.app")
        .await;

    // Assert: Access-Control-Allow-Origin: * が返却される
    let cors_headers = res.headers();
    assert_eq!(
        cors_headers
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .expect("CORS Allow-Origin missing in permissive mode"),
        "*"
    );

    std::env::remove_var("NURTURE_CORS_ORIGIN");
}

// =========================================================================
// 3. CORS: Preflight (OPTIONS) リクエストの検証
// =========================================================================

#[tokio::test]
#[serial]
async fn test_cors_preflight_options() {
    // Arrange
    std::env::set_var("NURTURE_CORS_ORIGIN", "*");
    let server = TestServer::new(build_test_app()).unwrap();

    // Act: Preflight OPTIONS リクエスト
    let res = server
        .method(axum::http::Method::OPTIONS, "/dummy")
        .add_header(header::ORIGIN, "https://aiome.app")
        .add_header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .await;

    // Assert: 200 OK で Allow-Origin が返却される
    let status = res.status_code();
    assert!(
        status == StatusCode::OK || status == StatusCode::NO_CONTENT,
        "Preflight should return 200 or 204, got {}",
        status
    );
    assert!(
        res.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_some(),
        "Preflight must include Access-Control-Allow-Origin"
    );

    std::env::remove_var("NURTURE_CORS_ORIGIN");
}

// =========================================================================
// 4. CORS: S2S 制限モード (NURTURE_CORS_ORIGIN 未設定)
// =========================================================================

#[tokio::test]
#[serial]
async fn test_cors_restrictive_mode_no_env() {
    // Arrange: 環境変数を明示的に削除
    std::env::remove_var("NURTURE_CORS_ORIGIN");
    let server = TestServer::new(build_test_app()).unwrap();

    // Act: Origin 付きリクエスト
    let res = server
        .get("/dummy")
        .add_header(header::ORIGIN, "https://evil.example.com")
        .await;

    // Assert: CORS ヘッダーなし (S2S モード)
    assert_eq!(res.status_code(), StatusCode::OK);
    assert!(
        res.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none(),
        "S2S mode should NOT include Access-Control-Allow-Origin"
    );
}

// =========================================================================
// 5. 非推奨ヘッダーが出力されないことの確認
// =========================================================================

#[tokio::test]
#[serial]
async fn test_deprecated_headers_absent() {
    std::env::remove_var("NURTURE_CORS_ORIGIN");
    let server = TestServer::new(build_test_app()).unwrap();

    let res = server.get("/dummy").await;
    let headers = res.headers();

    // X-XSS-Protection は非推奨のため、設定されていないことを確認
    assert!(
        headers.get("X-XSS-Protection").is_none(),
        "X-XSS-Protection is deprecated and should NOT be set"
    );
}
