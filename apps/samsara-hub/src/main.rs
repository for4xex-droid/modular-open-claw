#![cfg_attr(test, allow(clippy::unwrap_used))]
/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![forbid(unsafe_code)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::unnecessary_cast)]

use axum::{
    error_handling::HandleErrorLayer,
    extract::DefaultBodyLimit,
    http::StatusCode,
    routing::{get, post},
    Router,
};
// Standard imports
// use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};

mod auth;
mod handlers;
#[cfg(test)]
mod hub_auth_tests;
#[cfg(test)]
mod hub_discovery_tests;
mod mdns_listener;
mod models;
mod state;
mod workers;

use crate::handlers::federation::{push_handler, sync_handler};
use crate::handlers::ws::ws_handler;
use crate::state::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    shared::process_hardening::pre_main_hardening();

    if std::env::var("CELL_ID").unwrap_or_default().is_empty() {
        eprintln!("🚨 FATAL: CELL_ID is not set! The Sovereign Verifier architecture requires strict cellular isolation. No identity = No survival.");
        std::process::exit(1);
    }

    // Initialize tracing with JSON for easier aggregation in the hub
    tracing_subscriber::fmt().json().init();

    // 1. Initial attempt from CWD (essential for dev environments)
    dotenvy::dotenv().ok();
    dotenvy::from_path(".env.secret").ok();

    // Fetch and inject secrets from key-proxy if configured (§CISO-1)
    if let Err(e) = shared::security::fetch_and_inject_secrets().await {
        tracing::error!(
            "🚨 Failed to fetch and inject secrets from key-proxy: {:?}",
            e
        );
        return Err(e);
    }

    let resolver = shared::app_data::AppDataResolver::new()
        .map_err(|e| anyhow::anyhow!("Failed to initialize AppDataResolver: {}", e))?;

    // 2. Explicit attempt from application root (essential for Production)
    let app_env_path = resolver.root().join(".env");
    if app_env_path.exists() && dotenvy::from_path(&app_env_path).is_ok() {
        tracing::info!(
            "Loaded explicit environment from {}",
            app_env_path.display()
        );
    }
    let app_secret_path = resolver.root().join(".env.secret");
    if app_secret_path.exists() && dotenvy::from_path(&app_secret_path).is_ok() {
        tracing::info!(
            "Loaded explicit secret environment from {}",
            app_secret_path.display()
        );
    }

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        format!(
            "sqlite:{}?mode=rwc",
            resolver
                .root()
                .join("samsara_hub.db")
                .to_str()
                .unwrap_or_else(|| {
                    eprintln!("Invalid DB Path");
                    std::process::exit(1);
                })
        )
    });
    let secret_val = std::env::var("FEDERATION_SECRET").unwrap_or_else(|_| {
        tracing::error!("🚨 [CRITICAL] FEDERATION_SECRET must be set for Samsara Hub security!");
        std::process::exit(1);
    });
    let secret = secrecy::SecretString::from(secret_val);
    shared::security::scrub_env("FEDERATION_SECRET");
    let port = std::env::var("PORT").unwrap_or_else(|_| "3016".to_string());
    // Initialize Unified Database Pool
    let pool = if db_url.starts_with("postgres://") || db_url.starts_with("postgresql://") {
        let p = shared::db::DatabasePool::new_postgres(&db_url).await?;
        // DatabasePool::new_postgres already handles standard db initialization where appropriate.
        p
    } else {
        shared::db::DatabasePool::new_sqlite(&db_url).await?
    };

    init_hub_db(&pool).await?;

    // Create broadcast channel for real-time rule/karma notification
    let (tx, _) = broadcast::channel(100);
    let agent_registry = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let token = CancellationToken::new();
    let supervisor = infrastructure::supervisor::TaskSupervisor::new(10, 300);

    let _mdns_daemon =
        mdns_listener::start_mdns_listener(agent_registry.clone(), &supervisor, token.clone())
            .map_err(|e| anyhow::anyhow!("mDNS listener failed to start: {}", e))?;

    let state = Arc::new(HubState {
        pool: pool.clone(),
        secret,
        auth_manager: {
            match std::env::var("JWT_PRIVATE_KEY_B64") {
                Ok(key_b64) => {
                    shared::security::scrub_env("JWT_PRIVATE_KEY_B64");
                    Arc::new(
                        shared::auth::JwtAuthManager::from_private_key_b64(&key_b64)
                            .map_err(|e| anyhow::anyhow!("JWT initialize failed: {}", e))?,
                    )
                }
                #[cfg(debug_assertions)]
                Err(_) => {
                    warn!("⚠️ [SamsaraHub] JWT key not set, using MockAuthManager (dev only)");
                    Arc::new(shared::auth::MockAuthManager::new())
                        as Arc<dyn shared::auth::AuthManager>
                }
                #[cfg(not(debug_assertions))]
                Err(_) => {
                    error!("🚨 [FATAL] JWT_PRIVATE_KEY_B64 must be set in production!");
                    std::process::exit(1);
                }
            }
        },
        tx,
        active_connections: std::sync::atomic::AtomicUsize::new(0),
        agent_registry,
        config: shared::config::AiomeConfig::load().unwrap_or_else(|e| {
            warn!(
                "⚠️ [SamsaraHub] AiomeConfig::load() failed: {}. Using default config.",
                e
            );
            shared::config::AiomeConfig::default()
        }),
    });

    // Spawn the Approval Worker to process quarantine
    struct ApprovalWorkerTask {
        pool: shared::db::DatabasePool,
    }
    impl infrastructure::supervisor::SupervisedTask for ApprovalWorkerTask {
        fn name(&self) -> &'static str {
            "ApprovalWorker"
        }
        fn run(
            &self,
            ct: tokio_util::sync::CancellationToken,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
            let pool = self.pool.clone();
            Box::pin(async move {
                // If approval_worker fails, we panic and restart
                crate::workers::approval_worker(pool, ct).await;
            })
        }
    }
    supervisor.spawn_supervised(ApprovalWorkerTask { pool: pool.clone() }, token.clone());

    let state_bg = state.clone();
    struct MaintenanceTask {
        state_bg: Arc<HubState>,
    }
    impl infrastructure::supervisor::SupervisedTask for MaintenanceTask {
        fn name(&self) -> &'static str {
            "HubMaintenance"
        }
        fn run(
            &self,
            ct: tokio_util::sync::CancellationToken,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
            let state_bg = self.state_bg.clone();
            Box::pin(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Every hour
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            info!("♻️ [HubMaintenance] Running Maintenance...");
                            if let Some(sq) = state_bg.pool.get_sqlite_pool() {
                                 let has_error = match sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)").execute(sq).await {
                                     Err(e) => {
                                         tracing::error!("🚨 [HubMaintenance] SQLite checkpoint failed: {}", e);
                                         true
                                     }
                                     Ok(_) => false,
                                 };
                                 if has_error {
                                     tokio::time::sleep(Duration::from_secs(5)).await;
                                     continue;
                                 }
                            }
                        }
                        _ = ct.cancelled() => break,
                    }
                }
            })
        }
    }
    supervisor.spawn_supervised(MaintenanceTask { state_bg }, token.clone());

    let app = build_app(state.clone());

    let addr = format!("127.0.0.1:{}", port);
    info!("🏔️ Samsara Hub (The Validator) listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(token))
        .await?;

    info!("🛑 [samsara-hub] Closing database connections gracefully...");
    state.pool.close().await;
    info!("✅ [samsara-hub] Shutdown complete.");

    Ok(())
}

async fn shutdown_signal(token: CancellationToken) {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!("🚨 [Hub] Failed to install Ctrl+C handler: {}", e);
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
                error!("🚨 [Hub] Failed to install signal handler: {}", e);
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("🔴 [samsara-hub] Received Ctrl+C signal. Initiating graceful shutdown...");
        },
        _ = terminate => {
            info!("🔴 [samsara-hub] Received Terminate signal. Initiating graceful shutdown...");
        },
    }

    token.cancel();
}

pub fn build_app(state: Arc<HubState>) -> Router {
    let mut allowed_origins = vec![];

    for origin in &state.config.allowed_origins {
        match origin.parse() {
            Ok(parsed) => allowed_origins.push(parsed),
            Err(e) => {
                tracing::warn!(
                    origin = %origin,
                    error = %e,
                    "⚠️ [SamsaraHub] Failed to parse CORS origin, skipping"
                );
            }
        }
    }

    if allowed_origins.is_empty() {
        tracing::warn!("🚨 [SamsaraHub] No valid ALLOWED_ORIGINS found in config. CORS will block all cross-origin requests.");
    } else {
        tracing::info!(
            count = allowed_origins.len(),
            "🌐 [SamsaraHub] CORS configured with {} allowed origin(s)",
            allowed_origins.len()
        );
    }

    let is_any = state.config.allowed_origins.contains(&"*".to_string());

    let cors_base = CorsLayer::new()
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);

    use crate::handlers::commune::{
        commune_relay_handler, commune_ws_handler, create_topic_handler, list_topics_handler,
    };
    use crate::handlers::middleware::auth_middleware;
    use crate::handlers::system::{health_handler, list_agents_handler};
    use crate::handlers::timeline::timeline_sync_handler;

    let router = Router::new()
        .route("/api/v1/federation/sync", post(sync_handler))
        .route("/api/v1/federation/push", post(push_handler))
        .route("/api/v1/registry/agents", get(list_agents_handler))
        .route(
            "/api/v1/commune/topics",
            get(list_topics_handler).post(create_topic_handler),
        )
        .route("/api/v1/commune/relay", post(commune_relay_handler))
        .route("/api/v1/commune/ws", get(commune_ws_handler))
        .route("/api/v1/relay/timeline/sync", post(timeline_sync_handler))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        // WS and Health handled outside middleware
        .route("/api/v1/federation/ws", get(ws_handler))
        .route("/api/v1/health", get(health_handler));

    let router = if is_any {
        router.layer(cors_base.allow_origin(tower_http::cors::Any))
    } else {
        router.layer(cors_base.allow_origin(allowed_origins))
    };

    router
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024)) // 5MB limit
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|err| async move {
                    let error_id = uuid::Uuid::new_v4().to_string();
                    tracing::error!("Samsara Hub Error [Error ID: {}]: {}", error_id, err);

                    let msg = if cfg!(not(debug_assertions)) {
                        format!("An internal service error occurred. Error ID: {}", error_id)
                    } else {
                        format!("Unhandled internal error: {}", err)
                    };

                    (StatusCode::INTERNAL_SERVER_ERROR, msg)
                }))
                .layer(BufferLayer::new(2048))
                .layer(RateLimitLayer::new(600, Duration::from_secs(60))), // High frequency for Commune
        )
        .with_state(state)
}

#[cfg(test)]
mod hub_reliability_tests;
#[cfg(test)]
mod hub_ws_tests;
