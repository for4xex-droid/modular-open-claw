/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 */

use anyhow::Context;
use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use sqlx::sqlite::SqlitePool;
use std::net::SocketAddr;
use subtle::ConstantTimeEq;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use commerce_protocol::identity::ActorId;
use nurture_api::routes::nurture_routes;
use nurture_api::state::AppState;
use nurture_core::policy::EconomyPolicy;
use secrecy::{ExposeSecret, SecretString};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // .env の読み込み
    dotenvy::dotenv().ok();

    // ロギングの初期化
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    // DB 接続
    let db_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("NURTURE_DB_PATH"))
        .unwrap_or_else(|_| "sqlite:nurture.db".to_string());

    let pool = SqlitePool::connect(&db_url).await.context("DB 接続失敗")?;

    // 自動マイグレーション実行 (🔴 B2 解決)
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .context("マイグレーション失敗")?;

    // コンポーネントの初期化
    let policy = match sqlx::query_scalar::<_, String>(
        "SELECT payload FROM nurture_settings WHERE setting_key = 'economy_policy'",
    )
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(payload)) => {
            match serde_json::from_str::<EconomyPolicy>(&payload) {
                Ok(p) => {
                    if let Err(ve) = p.validate() {
                        tracing::error!("🚨 [Nurture] Loaded EconomyPolicy is invalid: {:?}. Falling back to default.", ve);
                        EconomyPolicy::default()
                    } else {
                        tracing::info!("✅ [Nurture] EconomyPolicy was successfully loaded from DB and validated.");
                        p
                    }
                }
                Err(e) => {
                    tracing::error!("🚨 [Nurture] Failed to parse EconomyPolicy from DB: {}. Falling back to default.", e);
                    EconomyPolicy::default()
                }
            }
        }
        Ok(None) => {
            tracing::warn!("⚠️ [Nurture] EconomyPolicy not found in DB. Initializing with default and saving to DB.");
            let default_policy = EconomyPolicy::default();
            if let Ok(payload) = serde_json::to_string(&default_policy) {
                if let Err(insert_err) = sqlx::query(
                    "INSERT INTO nurture_settings (setting_key, payload, updated_at) VALUES ('economy_policy', ?, CURRENT_TIMESTAMP) ON CONFLICT(setting_key) DO NOTHING"
                )
                .bind(payload)
                .execute(&pool)
                .await {
                    tracing::warn!("⚠️ Failed to insert default economy policy: {}", insert_err);
                }
            }
            default_policy
        }
        Err(e) => {
            tracing::error!(
                "🚨 [Nurture] DB error while fetching EconomyPolicy: {}. Falling back to default.",
                e
            );
            EconomyPolicy::default()
        }
    };
    let system_actor_id = ActorId(Uuid::parse_str("00000000-0000-0000-0000-000000000001")?); // システムアカウント (nil との衝突回避)

    let cancel_token = tokio_util::sync::CancellationToken::new();
    let store = std::sync::Arc::new(
        nurture_bridge::job_queue::trajectory_store::SqliteTrajectoryStore::from_db_path(&db_url)
            .await?,
    ) as std::sync::Arc<dyn nurture_bridge::trajectory::TrajectoryStore>;
    let job_queue: std::sync::Arc<dyn nurture_bridge::traits::JobQueue> = std::sync::Arc::new(
        nurture_bridge::job_queue::UniversalJobQueue::new(
            nurture_bridge::db::DatabasePool::Sqlite(pool.clone()),
            None,
            store,
        )
        .await?,
    );

    let nurture_secret = std::env::var("NURTURE_INTERNAL_SECRET").unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            "dev_nurture_secret".to_string()
        } else {
            tracing::error!(
                "🚨 [Nurture-Auth] FATAL: NURTURE_INTERNAL_SECRET must be set in release builds!"
            );
            std::process::exit(1);
        }
    });
    nurture_bridge::security::scrub_env("NURTURE_INTERNAL_SECRET");

    let stripe_webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET").ok();
    nurture_bridge::security::scrub_env("STRIPE_WEBHOOK_SECRET");

    let polar_webhook_secret = std::env::var("POLAR_WEBHOOK_SECRET").ok();
    nurture_bridge::security::scrub_env("POLAR_WEBHOOK_SECRET");

    const MOCK_JWT_PRIVATE_KEY: &str = "dev_jwt_key_mock_base64_donotuseinprod";

    let jwt_private_key_b64 = std::env::var("JWT_PRIVATE_KEY_B64").unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            MOCK_JWT_PRIVATE_KEY.to_string()
        } else {
            tracing::error!(
                "🚨 [Nurture-Auth] FATAL: JWT_PRIVATE_KEY_B64 must be set in release builds!"
            );
            std::process::exit(1);
        }
    });
    let auth_manager: std::sync::Arc<dyn nurture_bridge::auth::AuthManager> = {
        #[cfg(any(test, debug_assertions))]
        {
            if jwt_private_key_b64 == MOCK_JWT_PRIVATE_KEY {
                std::sync::Arc::new(nurture_bridge::auth::MockAuthManager::new())
            } else {
                std::sync::Arc::new(
                    nurture_bridge::auth::JwtAuthManager::from_private_key_b64(
                        &jwt_private_key_b64,
                    )
                    .unwrap_or_else(|e| {
                        tracing::error!(
                            "🚨 [Nurture-Auth] Failed to initialize JwtAuthManager: {}",
                            e
                        );
                        std::process::exit(1);
                    }),
                )
            }
        }

        #[cfg(not(any(test, debug_assertions)))]
        {
            std::sync::Arc::new(
                nurture_bridge::auth::JwtAuthManager::from_private_key_b64(&jwt_private_key_b64)
                    .unwrap_or_else(|e| {
                        tracing::error!(
                            "🚨 [Nurture-Auth] Failed to initialize JwtAuthManager: {}",
                            e
                        );
                        std::process::exit(1);
                    }),
            )
        }
    };
    nurture_bridge::security::scrub_env("JWT_PRIVATE_KEY_B64");

    let drm_master_key = std::env::var("NURTURE_DRM_MASTER_KEY").unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            "dev_drm_master_key_1234567890".to_string()
        } else {
            tracing::error!(
                "🚨 [Nurture-DRM] FATAL: NURTURE_DRM_MASTER_KEY must be set in release builds for encryption!"
            );
            std::process::exit(1);
        }
    });
    nurture_bridge::security::scrub_env("NURTURE_DRM_MASTER_KEY");

    let a2a_auth_token = std::env::var("A2A_AUTH_TOKEN").ok().map(|token| {
        nurture_bridge::security::scrub_env("A2A_AUTH_TOKEN");
        SecretString::from(token)
    });

    let shadow_clone_grpc_host =
        std::env::var("SHADOW_CLONE_GRPC_HOST").unwrap_or_else(|_| "localhost".to_string());

    let shadow_clone_grpc_port =
        std::env::var("SHADOW_CLONE_GRPC_PORT").unwrap_or_else(|_| "50051".to_string());

    let state = AppState::init(
        pool,
        job_queue,
        policy,
        system_actor_id,
        cancel_token.clone(),
        SecretString::from(nurture_secret),
        stripe_webhook_secret.map(SecretString::from),
        polar_webhook_secret.map(SecretString::from),
        auth_manager,
        SecretString::from(drm_master_key),
        {
            #[cfg(feature = "cloud-storage")]
            {
                if let Ok(bucket) = std::env::var("S3_BUCKET_NAME") {
                    let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
                    let s3_client = aws_sdk_s3::Client::new(&aws_config);
                    std::sync::Arc::new(nurture_infra::storage::S3AssetStorage::new(s3_client, bucket))
                } else {
                    tracing::warn!("⚠️ [Nurture-Storage] S3_BUCKET_NAME not set. Falling back to MockAssetStorage.");
                    std::sync::Arc::new(nurture_infra::storage::MockAssetStorage::new())
                }
            }
            #[cfg(not(feature = "cloud-storage"))]
            {
                tracing::info!("📦 [Desktop] cloud-storage feature disabled. Using MockAssetStorage.");
                std::sync::Arc::new(nurture_infra::storage::MockAssetStorage::new())
            }
        },
        a2a_auth_token,
        shadow_clone_grpc_host,
        shadow_clone_grpc_port,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to init AppState: {}", e))?;

    // 内部ルート (Rate Limit 除外)
    let internal_routes = Router::new()
        .nest(
            "/internal",
            nurture_api::routes::internal::internal_routes(),
        )
        .layer(middleware::from_fn(internal_auth_middleware))
        .layer(axum::Extension(state.clone()));

    // パブリックルート (Rate Limit 適用)
    let public_routes = Router::new()
        // Stripe / Polar webhook は自身が署名検証を行うため、内部 S2S 認証からは除外する
        .nest(
            "/api/v1/stripe",
            nurture_api::routes::stripe::stripe_routes().layer(axum::Extension(state.clone())),
        )
        .nest(
            "/api/v1/polar",
            nurture_api::routes::polar::polar_routes().layer(axum::Extension(state.clone())),
        )
        .nest(
            "/api/v1",
            nurture_routes(state.clone())
                // --- Defense Layer 5: Zero-Trust Internal Authentication ---
                // 決済・管理ルートに対して Server-to-Server 認証を要求
                .layer(middleware::from_fn(internal_auth_middleware)),
        )
        // --- Defense Layer 3: Rate Limiting (5 req/min) ---
        // AIの暴走購入ストッパー: 1分間に5回以上の決済は物理的に不可能
        .layer(
            tower::ServiceBuilder::new()
                .layer(axum::error_handling::HandleErrorLayer::new(
                    |err: tower::BoxError| async move {
                        tracing::warn!(
                            "🛡️ [Nurture] Rate limit exceeded (runaway purchase prevention): {}",
                            err
                        );
                        (
                            StatusCode::TOO_MANY_REQUESTS,
                            "Rate limit exceeded. Please try again later.".to_string(),
                        )
                    },
                ))
                .buffer(128)
                .rate_limit(5, std::time::Duration::from_secs(60))
                .into_inner(),
        );

    // ルーターマージと共通防御レイヤーの設定
    let app = Router::new()
        .route("/health", get(health_check))
        .merge(internal_routes)
        .merge(public_routes);

    // --- Defense Layer 4: Security Headers & CORS ---
    let app = nurture_api::middleware::apply_security_middlewares(app)
        // --- Defense Layer 2: Payload Size Limit (5MB) ---
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            5 * 1024 * 1024,
        ))
        // --- Defense Layer 1: Request Timeout (30s) ---
        .layer(tower_http::timeout::TimeoutLayer::new(
            std::time::Duration::from_secs(30),
        ));

    // 🔒 127.0.0.1 強制バインド (デフォルト)
    // Docker Compose 環境時は NURTURE_BIND_ADDR=0.0.0.0 で上書きされる
    let bind_addr = std::env::var("NURTURE_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
    let addr: SocketAddr = format!("{}:3020", bind_addr)
        .parse()
        .context("Invalid bind address")?;

    tracing::info!(
        "🏦 [Nurture] Economy API サーバーを起動中 (内部専用): {}",
        addr
    );

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("TcpListener バインド失敗")?;

    // 🚨 P0-1: SIGTERM / Ctrl+C handler
    let cancel_for_signal = cancel_token.clone();
    tokio::spawn(async move {
        let ctrl_c = async {
            if let Err(e) = tokio::signal::ctrl_c().await {
                tracing::error!("Failed to install Ctrl+C handler: {}", e);
                std::future::pending::<()>().await;
            }
        };

        #[cfg(unix)]
        let terminate = async {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(mut s) => {
                    s.recv().await;
                }
                Err(e) => {
                    tracing::error!("Failed to install signal handler: {}", e);
                    std::future::pending::<()>().await;
                }
            }
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        tracing::info!("🛑 [Nurture] Termination signal received. Initiating graceful shutdown...");
        cancel_for_signal.cancel();
    });

    // 🚨 P0-1: Graceful Shutdown hook
    let cancel_serve = cancel_token.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            cancel_serve.cancelled().await;
            tracing::info!(
                "🛑 [Nurture] HTTP server is shutting down. Waiting for in-flight requests..."
            );
        })
        .await
        .context("サーバー停止")?;

    Ok(())
}

/// 🔒 Zero-Trust Internal Authentication Middleware
///
/// api-server (OpenClaw) → nurture-api の Server-to-Server 通信において、
/// NURTURE_INTERNAL_SECRET を用いた認証を必須とする。
/// SSRF経由の不正アクセスを防止するためのゼロトラスト・ゲート。
async fn internal_auth_middleware(
    axum::Extension(state): axum::Extension<nurture_api::state::SharedState>,
    req: Request,
    next: Next,
) -> Response {
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();

    let expected_secret = state.internal_secret.expose_secret();
    let expected_bearer = format!("Bearer {}", expected_secret);

    let is_valid = if auth_header.len() == expected_bearer.len() {
        bool::from(auth_header.as_bytes().ct_eq(expected_bearer.as_bytes()))
    } else {
        false
    };

    if is_valid {
        next.run(req).await
    } else {
        tracing::warn!(
            "🚨 [Nurture-Auth] Unauthorized access attempt. Header present: {}",
            !auth_header.is_empty()
        );
        (
            StatusCode::UNAUTHORIZED,
            "Unauthorized: Invalid internal credentials",
        )
            .into_response()
    }
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "version": "0.1.0" }))
}
