/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 */

use axum::http::header;
use axum::{http::Method, Router};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    set_header::SetResponseHeaderLayer,
};

/// ルートレベルの HTTP セキュリティミドルウェアを適用する。
///
/// 以下のレイヤーをまとめて付与する:
/// - **CORS**: `NURTURE_CORS_ORIGIN` 環境変数に基づくオリジン制御
///   (未設定時は S2S 前提で同一オリジンのみ許可)
/// - **X-Content-Type-Options: nosniff** — MIME スニッフィング防止
/// - **X-Frame-Options: DENY** — クリックジャッキング防止
/// - **Strict-Transport-Security** — HSTS (1年, includeSubDomains)
/// - **Content-Security-Policy** — API 専用の最小 CSP
///
/// # Example
/// ```ignore
/// let app = axum::Router::new();
/// let secured = nurture_api::middleware::apply_security_middlewares(app);
/// ```
pub fn apply_security_middlewares(app: Router) -> Router {
    // CORS: 環境変数で明示的に指定されたオリジンのみ許可。
    // 未設定時は AllowOrigin なし (ブラウザからのクロスオリジンは拒否) = S2S 安全デフォルト。
    let cors = build_cors_layer();

    app.layer(SetResponseHeaderLayer::if_not_present(
        header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    ))
    .layer(SetResponseHeaderLayer::if_not_present(
        header::X_FRAME_OPTIONS,
        axum::http::HeaderValue::from_static("DENY"),
    ))
    .layer(SetResponseHeaderLayer::if_not_present(
        header::STRICT_TRANSPORT_SECURITY,
        axum::http::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    ))
    .layer(SetResponseHeaderLayer::if_not_present(
        header::HeaderName::from_static("content-security-policy"),
        axum::http::HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
    ))
    .layer(cors)
}

/// CORS レイヤーを構築する。
///
/// - `NURTURE_CORS_ORIGIN=*` → 全オリジン許可 (開発・テスト用)
/// - `NURTURE_CORS_ORIGIN=https://app.example.com` → 指定オリジンのみ許可
/// - 未設定 → S2S 前提でオリジン制限なし (ブラウザアクセスを想定しない)
fn build_cors_layer() -> CorsLayer {
    let methods = [
        Method::GET,
        Method::POST,
        Method::OPTIONS,
        Method::PUT,
        Method::DELETE,
    ];

    match std::env::var("NURTURE_CORS_ORIGIN").ok() {
        Some(origin) if origin == "*" => CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(methods)
            .allow_headers(Any),
        Some(origin) => {
            let parsed: axum::http::HeaderValue = origin
                .parse()
                .unwrap_or_else(|_| {
                    tracing::error!(
                        "🚨 [Nurture-CORS] Invalid NURTURE_CORS_ORIGIN value: '{}'. Falling back to restrictive.",
                        origin
                    );
                    axum::http::HeaderValue::from_static("https://localhost")
                });
            CorsLayer::new()
                .allow_origin(AllowOrigin::exact(parsed))
                .allow_methods(methods)
                .allow_headers(Any)
        }
        None => {
            // S2S API: ブラウザからの直接アクセスを想定しないため
            // CORS ヘッダーを付与しない（最も制限的）
            CorsLayer::new().allow_methods(methods).allow_headers(Any)
        }
    }
}
