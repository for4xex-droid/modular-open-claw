/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#![allow(clippy::default_constructed_unit_structs)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::type_complexity)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::manual_inspect)]

use crate::logging;
use crate::plugin_loader;

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

use super::*;

pub async fn init_env_and_preflight() -> anyhow::Result<PreflightResult> {
    // 1. Initial attempt from CWD (essential for dev environments to catch AIOME_DEV_MODE)
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
        .map_err(|e| anyhow::anyhow!("🚨 [FATAL] Failed to resolve app data directory: {}", e))?;

    // 2. Explicit attempt from application root (essential for Production Tauri sidecars)
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
    let stripe_price_sub_monthly = std::env::var("STRIPE_PRICE_SUBSCRIPTION_MONTHLY")
        .ok()
        .filter(|v| !v.trim().is_empty());

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

    // DB Path Isolation Verification (Phase 6)
    let db_is_sqlite = db_url.starts_with("sqlite:")
        || db_url.starts_with("sqlite://")
        || (!db_url.starts_with("postgres://") && !db_url.starts_with("postgresql://"));
    if db_is_sqlite {
        use std::path::PathBuf;
        let db_file_path = if db_url.starts_with("sqlite://") {
            PathBuf::from(db_url.trim_start_matches("sqlite://"))
        } else if db_url.starts_with("sqlite:") {
            let clean_url = db_url.trim_start_matches("sqlite:");
            let path_part = clean_url.split('?').next().unwrap_or(clean_url);
            PathBuf::from(path_part)
        } else {
            PathBuf::from(&db_url)
        };

        let is_isolated = if let Ok(canonical_root) = resolver.root().canonicalize() {
            if let Ok(canonical_db) = db_file_path.parent().map(|p| p.canonicalize()).transpose() {
                if let Some(cdb) = canonical_db {
                    cdb.starts_with(&canonical_root)
                } else {
                    false
                }
            } else {
                db_file_path.starts_with(resolver.root())
            }
        } else {
            db_file_path.starts_with(resolver.root())
        };

        let is_test_binary = std::env::current_exe()
            .map(|p| p.to_string_lossy().contains("/deps/"))
            .unwrap_or(false);

        if !is_isolated && !is_test_binary {
            let err_msg = format!(
                "🚨 SECURITY VIOLATION: Database path '{}' is outside the isolated cell directory '{}'!",
                db_file_path.display(),
                resolver.root().display()
            );
            let is_dev = std::env::var("AIOME_DEV_MODE")
                .map(|v| v == "1")
                .unwrap_or(false);
            if is_dev {
                eprintln!("⚠️ WARNING: {}", err_msg);
                std::process::exit(1);
            } else {
                return Err(anyhow::anyhow!(err_msg));
            }
        }
    }

    let gig_artifacts = resolver.resolve("gig_artifacts");
    if !gig_artifacts.exists() {
        if let Err(e) = std::fs::create_dir_all(&gig_artifacts) {
            tracing::error!(
                "Failed to create gig_artifacts directory {}: {}",
                gig_artifacts.display(),
                e
            );
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    #[allow(deprecated, unsafe_code)]
    async fn test_dotenv_secret_loaded() {
        // Arrange
        unsafe {
            std::env::remove_var("TEST_DOTENV_SECRET_LOADED");
        }

        let backup_exists = std::path::Path::new(".env.secret").exists();
        if backup_exists {
            std::fs::rename(".env.secret", ".env.secret.bak").unwrap();
        }

        std::fs::write(".env.secret", "TEST_DOTENV_SECRET_LOADED=true_secret_value").unwrap();

        // Act
        let _ = init_env_and_preflight().await;

        // Clean up
        let _ = std::fs::remove_file(".env.secret");
        if backup_exists {
            let _ = std::fs::rename(".env.secret.bak", ".env.secret");
        }

        // Assert
        let loaded_val = std::env::var("TEST_DOTENV_SECRET_LOADED").unwrap_or_default();
        unsafe {
            std::env::remove_var("TEST_DOTENV_SECRET_LOADED");
        }

        assert_eq!(loaded_val, "true_secret_value");
    }

    #[tokio::test]
    #[serial]
    #[allow(deprecated, unsafe_code)]
    async fn test_fetch_and_inject_secrets() {
        use std::collections::HashMap;
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        unsafe {
            std::env::set_var("KEY_PROXY_URL", mock_server.uri());
            std::env::set_var("VAULT_SECRET", "test_vault_secret");
        }

        let mut expected_secrets = HashMap::new();
        expected_secrets.insert(
            "STRIPE_API_KEY".to_string(),
            "mock_stripe_key_val".to_string(),
        );
        expected_secrets.insert(
            "GEMINI_API_KEY".to_string(),
            "mock_gemini_key_val".to_string(),
        );

        Mock::given(method("POST"))
            .and(path("/api/v1/secrets"))
            .and(header("Authorization", "Bearer test_vault_secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(expected_secrets))
            .mount(&mock_server)
            .await;

        let result = shared::security::fetch_and_inject_secrets().await;
        assert!(result.is_ok());

        assert_eq!(
            std::env::var("STRIPE_API_KEY").unwrap_or_default(),
            "mock_stripe_key_val"
        );
        assert_eq!(
            std::env::var("GEMINI_API_KEY").unwrap_or_default(),
            "mock_gemini_key_val"
        );

        // --- Negative Test: Invalid VAULT_SECRET ---
        unsafe {
            std::env::set_var("VAULT_SECRET", "wrong_vault_secret");
        }
        Mock::given(method("POST"))
            .and(path("/api/v1/secrets"))
            .and(header("Authorization", "Bearer wrong_vault_secret"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let neg_result = shared::security::fetch_and_inject_secrets().await;
        assert!(
            neg_result.is_err(),
            "Expected failure with invalid vault secret"
        );

        // --- Revert ---
        unsafe {
            std::env::remove_var("KEY_PROXY_URL");
            std::env::remove_var("VAULT_SECRET");
            std::env::remove_var("STRIPE_API_KEY");
            std::env::remove_var("GEMINI_API_KEY");
        }
    }
}
