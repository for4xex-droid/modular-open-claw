/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![cfg_attr(test, allow(clippy::unwrap_used))]
#![deny(unsafe_code)]
#![allow(clippy::collapsible_if)]

use axum::{
    Router,
    http::StatusCode,
    routing::{delete, get, post, put},
};
use secrecy::SecretString;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};

pub(crate) mod auth;
pub(crate) mod config;
pub(crate) mod handlers;
pub(crate) mod quota;

#[cfg(test)]
mod tests;

use crate::auth::auth_middleware;
use crate::config::{AppState, QuotaState};
use crate::handlers::{
    llm::{handle_llm_complete, handle_llm_embed, handle_llm_stream},
    passthrough::handle_gemini_passthrough,
    secrets::handle_get_secrets,
    wordpress::handle_wp_publish,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    shared::process_hardening::pre_main_hardening();

    tracing_subscriber::fmt::init();
    info!("🔐 [KeyProxy] Starting the Abyss Vault...");

    // 1. Extreme Security: Memory Lock (mlockall)
    #[cfg(target_os = "linux")]
    {
        use nix::sys::mman::{MlockAllFlags, mlockall};
        if let Err(e) = mlockall(MlockAllFlags::MCL_CURRENT | MlockAllFlags::MCL_FUTURE) {
            error!("❌ [KeyProxy] mlockall failed: {}. ABORTING for safety.", e);
            eprintln!("SECURITY VIOLATION: Could not lock memory to RAM.");
            std::process::exit(1);
        }
        info!("🧠 [KeyProxy] Memory locked to RAM (no swap).");
    }

    // 7. Security: Anti-Debugger (petersen's trick / ptrace)
    #[cfg(target_os = "macos")]
    {
        use nix::sys::ptrace;
        if ptrace::traceme().is_err() {
            error!("🚨 [KeyProxy] Debugger detected! Panic for safety.");
            eprintln!("SECURITY VIOLATION: Debugger attached.");
            std::process::exit(1);
        }
    }

    // 2. Load keys and SELF-WIPE ENV
    dotenvy::dotenv().ok();
    dotenvy::from_path(".env.secret").ok();

    let resolver = shared::app_data::AppDataResolver::new().map_err(|e| anyhow::anyhow!(e))?;

    // Initialize AbyssVault SQLite database for secrets storing (§CISO-1)
    let vault_db_path = env::var("ABYSS_VAULT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| resolver.root().join("abyss_vault.db"));

    if let Some(parent) = vault_db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let vault_db_url = format!("sqlite:{}?mode=rwc", vault_db_path.to_string_lossy());
    let vault_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&vault_db_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to AbyssVault DB: {}", e))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS vault_secrets (key TEXT PRIMARY KEY, encrypted_value BLOB NOT NULL)"
    )
    .execute(&vault_pool)
    .await?;

    let db_pool = infrastructure::db::DatabasePool::Sqlite(vault_pool);
    let vault_backend = Arc::new(
        infrastructure::security::sqlite_vault_backend::UniversalVaultBackend::new(db_pool),
    );

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
    let gemini_key = env::var("GEMINI_API_KEY")
        .ok()
        .or_else(|| shared::security::get_keychain_secret("com.aiome.gemini-api-key"))
        .unwrap_or_else(|| {
            error!("🚨 [CRITICAL] GEMINI_API_KEY must be set in macOS Keychain or environment");
            std::process::exit(1);
        });

    let vault_secret = env::var("VAULT_SECRET")
        .ok()
        .or_else(|| shared::security::get_keychain_secret("com.aiome.vault-secret"))
        .unwrap_or_else(|| {
            error!("🚨 [CRITICAL] VAULT_SECRET must be set in macOS Keychain or environment");
            std::process::exit(1);
        });

    let wp_api_url = env::var("WP_API_URL").ok();
    let wp_api_token = env::var("WP_API_TOKEN").ok();

    // Self-Wipe: Remove from environment immediately
    shared::security::scrub_env("GEMINI_API_KEY");
    shared::security::scrub_env("VAULT_SECRET");
    shared::security::scrub_env("WP_API_TOKEN");
    info!("🧹 [KeyProxy] Environment wiped. Keys are now only in memory.");

    let mut quotas = std::collections::HashMap::new();
    quotas.insert("daemon".to_string(), 1000);
    quotas.insert("watchtower".to_string(), 100);
    quotas.insert("api-server".to_string(), 50000);
    quotas.insert("aiome-agent".to_string(), 10000);

    let resolver = shared::app_data::AppDataResolver::new().map_err(|e| anyhow::anyhow!(e))?;
    let persistence_path = env::var("QUOTA_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| resolver.resolve("config/key_proxy_state.json"));

    if let Some(parent) = persistence_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let quota_state = if persistence_path.exists() {
        let data = std::fs::read_to_string(&persistence_path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        QuotaState::default()
    };

    let gemini_model = env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.0-flash".to_string());
    let gemini_embed_model =
        env::var("GEMINI_EMBED_MODEL").unwrap_or_else(|_| "text-embedding-004".to_string());
    info!(
        "🤖 [KeyProxy] Models loaded at boot: complete={}, embed={}",
        gemini_model, gemini_embed_model
    );

    let state = AppState {
        gemini_key: Arc::new(SecretString::from(gemini_key)),
        vault_secret: Arc::new(SecretString::from(vault_secret)),
        client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
        state: Arc::new(tokio::sync::RwLock::new(quota_state)),
        auth_manager: build_auth_manager(std::env::var("JWT_PRIVATE_KEY_B64").ok())?,
        persistence_path,
        caller_quotas: Arc::new(quotas),
        wp_api_url,
        wp_api_token: wp_api_token.map(|t| Arc::new(SecretString::from(t))),
        gemini_model,
        gemini_embed_model,
        vault_backend,
    };

    let app = Router::new()
        .route("/api/v1/llm/complete", post(handle_llm_complete))
        .route("/api/v1/llm/stream", post(handle_llm_stream))
        .route("/api/v1/llm/embed", post(handle_llm_embed))
        .route("/api/v1/wp/publish", post(handle_wp_publish))
        .route("/api/v1/secrets", post(handle_get_secrets))
        .route(
            "/api/v1/admin/status",
            get(crate::handlers::vault_admin::handle_vault_status),
        )
        .route(
            "/api/v1/admin/secrets",
            put(crate::handlers::vault_admin::handle_vault_store),
        )
        .route(
            "/api/v1/admin/secrets/:key",
            delete(crate::handlers::vault_admin::handle_vault_delete),
        )
        .route("/api/v1/health", get(|| async { StatusCode::OK }))
        .route("/proxy/gemini/*path", post(handle_gemini_passthrough))
        .route("/proxy/gemini/*path", get(handle_gemini_passthrough))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
        // --- Defense Layer 3: Security Headers ---
        .layer(
            tower_http::set_header::SetResponseHeaderLayer::if_not_present(
                axum::http::header::X_CONTENT_TYPE_OPTIONS,
                axum::http::HeaderValue::from_static("nosniff"),
            ),
        )
        .layer(
            tower_http::set_header::SetResponseHeaderLayer::if_not_present(
                axum::http::header::X_FRAME_OPTIONS,
                axum::http::HeaderValue::from_static("DENY"),
            ),
        )
        .layer(
            tower_http::set_header::SetResponseHeaderLayer::if_not_present(
                axum::http::header::STRICT_TRANSPORT_SECURITY,
                axum::http::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
            ),
        )
        // --- Defense Layer 2: Rate Limiting (30 req/min = 1 req per 2s) ---
        .layer(
            tower::ServiceBuilder::new()
                .layer(axum::error_handling::HandleErrorLayer::new(
                    |err: tower::BoxError| async move {
                        tracing::warn!("🛡️ [KeyProxy] Rate limit / buffer error: {}", err);
                        (
                            StatusCode::TOO_MANY_REQUESTS,
                            format!("Rate limit exceeded: {}", err),
                        )
                    },
                ))
                .buffer(256)
                .rate_limit(30, std::time::Duration::from_secs(60))
                .into_inner(),
        )
        // --- Defense Layer 1: Payload & Timeout Protection ---
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            10 * 1024 * 1024,
        )) // 10MB max (covers WP payload limits)
        .layer(tower_http::timeout::TimeoutLayer::new(
            std::time::Duration::from_secs(120),
        )); // 120s for LLM calls

    let port = env::var("KEY_PROXY_PORT").unwrap_or_else(|_| "3017".to_string());
    let bind_addr = if env::var("BIND_ALL").map(|v| v == "true").unwrap_or(false) {
        "0.0.0.0"
    } else {
        "127.0.0.1"
    };
    let addr = format!("{}:{}", bind_addr, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("🚀 [KeyProxy] Abyss Vault listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

pub(crate) fn build_auth_manager(
    key_b64_opt: Option<String>,
) -> anyhow::Result<Arc<dyn infrastructure::auth::AuthManager>> {
    match key_b64_opt {
        Some(key_b64) if !key_b64.is_empty() && !key_b64.starts_with('<') => {
            info!("🔑 [KeyProxy] Loading JWT private key from environment");
            shared::security::scrub_env("JWT_PRIVATE_KEY_B64");
            match infrastructure::auth::JwtAuthManager::from_private_key_b64(&key_b64) {
                Ok(manager) => Ok(Arc::new(manager)),
                Err(e) => {
                    #[cfg(debug_assertions)]
                    {
                        warn!(
                            "⚠️ [KeyProxy] Invalid JWT_PRIVATE_KEY_B64: {}. Falling back to MockAuthManager for development.",
                            e
                        );
                        Ok(Arc::new(infrastructure::auth::MockAuthManager::new()))
                    }
                    #[cfg(not(debug_assertions))]
                    {
                        Err(anyhow::anyhow!("Invalid JWT_PRIVATE_KEY_B64: {}", e))
                    }
                }
            }
        }
        _ => {
            #[cfg(debug_assertions)]
            {
                warn!(
                    "⚠️ [KeyProxy] JWT key not set or placeholder, using MockAuthManager (dev only)"
                );
                Ok(Arc::new(infrastructure::auth::MockAuthManager::new()))
            }
            #[cfg(not(debug_assertions))]
            {
                Err(anyhow::anyhow!(
                    "JWT_PRIVATE_KEY_B64 must be set in production!"
                ))
            }
        }
    }
}
