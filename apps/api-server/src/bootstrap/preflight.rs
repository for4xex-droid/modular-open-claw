/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
#![allow(clippy::default_constructed_unit_structs)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::type_complexity)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::manual_inspect)]

use crate::app_state::Component;
use crate::internal_services;
use crate::logging;
use crate::mcp;
use crate::plugin_loader;
use crate::AppState;
use aiome_core::llm_provider::{EmbeddingProvider, LlmProvider};
use aiome_core::traits::TranscriptionEngine;
use aiome_core_contracts::commerce::CommerceEngine;
use aiome_core_contracts::commerce::GiftEngine;
use aiome_core_contracts::commerce::GiftPolicyContext;
use aiome_core_contracts::ekyc::EkycEngine;
use aiome_core_contracts::ekyc::EkycSessionStore;
use infrastructure::audit_logger::AsyncAuditLogger;
use infrastructure::auth::AuthManager;
use infrastructure::belief_consistency_gate::BeliefConsistencyGate;
use infrastructure::circuit_breaker::CircuitBreaker;
use infrastructure::compliance::quarantine::QuarantineStore;
use infrastructure::memory_crystallizer::MemoryCrystallizer;
use infrastructure::slo_engine::SloEngine;
use infrastructure::whisper_transcription::WhisperTranscriptionAdapter;
use shared::config::AiomeConfig;

use async_trait::async_trait;
use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, STRICT_TRANSPORT_SECURITY, X_CONTENT_TYPE_OPTIONS,
    X_FRAME_OPTIONS,
};
use axum::http::HeaderValue;
use axum::{http::StatusCode, response::IntoResponse, response::Json, routing::get, Router};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use tracing::{debug, error, info, warn};
use utoipa::OpenApi;

use super::*;
use aiome_core::expression::tts_worker::TtsWorker;
use aiome_core::traits::JobQueue;
use shared::health::HealthMonitor;

pub async fn init_env_and_preflight() -> anyhow::Result<PreflightResult> {
    // 1. Initial attempt from CWD (essential for dev environments to catch AIOME_DEV_MODE)
    dotenvy::dotenv().ok();

    let resolver = shared::app_data::AppDataResolver::new()
        .map_err(|e| anyhow::anyhow!("🚨 [FATAL] Failed to resolve app data directory: {}", e))?;

    // 2. Explicit attempt from application root (essential for Production Tauri sidecars)
    let app_env_path = resolver.root().join(".env");
    if app_env_path.exists() && dotenvy::from_path(&app_env_path).is_ok() {
        tracing::info!(
            "Loaded explicit environment from {}",
            app_env_path.display()
        );
    }

    // 0. Initialize Metrics EXPORTER (Q-5)
    let metrics_handle = match metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()
    {
        Ok(handle) => handle,
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("already initialized")
                || err_str.contains("attempted to set a recorder")
            {
                tracing::warn!("📊 Prometheus Metrics already initialized. Reusing via build_recorder fallback.");
                let recorder =
                    metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder();
                recorder.handle()
            } else {
                return Err(anyhow::anyhow!(
                    "Failed to install Prometheus recorder: {}",
                    e
                ));
            }
        }
    };
    tracing::info!("📊 Prometheus Metrics initialized at /api/v1/metrics");

    let health_monitor = shared::health::HealthMonitor::new();
    let health_monitor = Arc::new(Mutex::new(health_monitor));

    // === 🏗️ STAGE 1/7: Pre-flight ===
    let root = resolver.root();
    if !root.exists() {
        std::fs::create_dir_all(root).unwrap_or_else(|e| {
            error!(
                "🚨 [CRITICAL] Failed to create app data directory at {}: {}",
                root.display(),
                e
            );
            std::process::exit(1);
        });
    }

    // === Secret Pre-load (Step 0 / Step 1-0 / Step 1-1) ===
    let stripe_key_raw = std::env::var("STRIPE_API_KEY").ok();
    shared::security::scrub_env("STRIPE_API_KEY");

    let is_test_mode = std::env::var("STRIPE_TEST_MODE")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(true);
    let stripe_price_sub_monthly = std::env::var("STRIPE_PRICE_SUBSCRIPTION_MONTHLY").ok();

    if !is_test_mode && stripe_key_raw.is_some() && stripe_price_sub_monthly.is_none() {
        return Err(anyhow::anyhow!(
            "STRIPE_PRICE_SUBSCRIPTION_MONTHLY must be set in production mode"
        ));
    }

    let nurture_secret_raw = std::env::var("NURTURE_INTERNAL_SECRET").ok();
    shared::security::scrub_env("NURTURE_INTERNAL_SECRET");

    // trend_sonar fallback vars
    let env_search_key = std::env::var("SEARCH_API_KEY").ok();
    shared::security::scrub_env("SEARCH_API_KEY");

    let env_x_token = std::env::var("X_BEARER_TOKEN").ok();
    shared::security::scrub_env("X_BEARER_TOKEN");

    let tts_openai_api_key_raw = std::env::var("TTS_OPENAI_API_KEY").ok();
    shared::security::scrub_env("TTS_OPENAI_API_KEY");

    let db_url = std::env::var("AIOME_DB_PATH").unwrap_or_else(|_| resolver.db_url());

    let gig_artifacts = resolver.resolve("gig_artifacts");
    if !gig_artifacts.exists() {
        let _ = std::fs::create_dir_all(&gig_artifacts);
    }

    let cancel_token = tokio_util::sync::CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();
    tokio::spawn(async move {
        let ctrl_c = async {
            if let Err(e) = tokio::signal::ctrl_c().await {
                tracing::error!("🚨 [api-server] Failed to install Ctrl+C handler: {}", e);
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
                    tracing::error!("🚨 [api-server] Failed to install SIGTERM handler: {}", e);
                    std::future::pending::<()>().await;
                }
            }
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                tracing::info!("🛑 [api-server] Received Ctrl-C, triggering shutdown...");
            },
            _ = terminate => {
                tracing::info!("🛑 [api-server] Received SIGTERM, triggering graceful shutdown...");
            },
        }

        cancel_token_clone.cancel();
    });
    let plugin_registry = plugin_loader::PluginRegistry::new();

    use std::str::FromStr;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    let opts =
        sqlx::sqlite::SqliteConnectOptions::from_str(&db_url.replace("sqlite://", "sqlite:"))
            .unwrap_or_else(|e| {
                eprintln!("🚨 [CRITICAL] Invalid AIOME_DB_PATH URL: {}", e);
                std::process::exit(1);
            })
            .create_if_missing(true);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(opts)
        .await
        .unwrap_or_else(|e| {
            eprintln!("🚨 Failed to connect to SQLite for logging: {}", e);
            std::process::exit(1);
        });
    let logger_layer = logging::DbLoggerLayer::new(pool);

    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(logger_layer)
        .with(tracing_subscriber::filter::LevelFilter::INFO)
        .try_init();

    let config = shared::config::AiomeConfig::load().unwrap_or_else(|e| {
        error!("🚨 Failed to load config: {}", e);
        std::process::exit(1);
    });
    let config = Arc::new(config);

    // [Self-Diagnosis Phase A-3]
    crate::self_diagnosis::run_startup_diagnosis(&config)
        .await
        .unwrap_or_else(|_| {
            std::process::exit(1);
        });

    let live_manager: Option<Arc<dyn aiome_core_contracts::traits::LiveSessionManager>> = {
        use secrecy::ExposeSecret;
        config.gemini_api_key.as_ref().map(|key| {
            Arc::new(
                aiome_core::llm_provider::live_session::LiveSessionProvider::new(
                    key.clone(),
                    "gemini-2.0-flash-exp".to_string(),
                ),
            ) as Arc<dyn aiome_core_contracts::traits::LiveSessionManager>
        })
    };

    let secrets = BootSecrets {
        stripe_key: stripe_key_raw,
        nurture_secret: nurture_secret_raw,
        search_key: env_search_key,
        x_token: env_x_token,
        tts_openai_key: tts_openai_api_key_raw,
        stripe_price_subscription_monthly: stripe_price_sub_monthly,
    };

    Ok(PreflightResult {
        resolver,
        config,
        cancel_token,
        metrics_handle,
        plugin_registry,
        secrets,
        health_monitor,
        live_manager,
        db_url,
    })
}
