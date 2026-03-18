/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

#![deny(unsafe_code)]

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::{Datelike, Utc};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

#[derive(Debug, Deserialize)]
struct ProxyRequest {
    caller_id: String,
    prompt: String,
    system: Option<String>,
    endpoint: String, // "gemini" etc (Hardcoded Enum-like check)
}

#[derive(Debug, Serialize)]
struct ProxyResponse {
    result: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct QuotaState {
    total_calls: u64,
    last_reset_day: u32, // Day of the year
    #[serde(default)]
    per_caller_calls: std::collections::HashMap<String, u64>,
}

impl Default for QuotaState {
    fn default() -> Self {
        Self {
            total_calls: 0,
            last_reset_day: Utc::now().ordinal(),
            per_caller_calls: std::collections::HashMap::new(),
        }
    }
}

use std::collections::HashMap;

#[derive(Clone)]
struct AppState {
    gemini_key: Arc<SecretString>,
    vault_secret: Arc<SecretString>,
    client: reqwest::Client,
    state: Arc<RwLock<QuotaState>>,
    persistence_path: PathBuf,
    caller_quotas: Arc<HashMap<String, u64>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
    let gemini_key = env::var("GEMINI_API_KEY").unwrap_or_else(|_| {
        error!("🚨 [CRITICAL] GEMINI_API_KEY must be set in key-proxy/.env");
        std::process::exit(1);
    });

    let vault_secret = env::var("VAULT_SECRET").unwrap_or_else(|_| {
        error!("🚨 [CRITICAL] VAULT_SECRET must be set for Abyss Vault access!");
        std::process::exit(1);
    });

    // Self-Wipe: Remove from environment immediately
    #[allow(unsafe_code)]
    fn wipe_env(key: &str) {
        // SAFETY: This is called during the single-threaded initialization phase.
        // Rust 1.66+ made env::set_var/remove_var unsafe due to potential thread-safety
        // issues if called concurrently with other threads reading the environment.
        // Since this is the main setup before any other threads are spawned, it is safe.
        unsafe {
            env::remove_var(key);
        }
    }
    wipe_env("GEMINI_API_KEY");
    wipe_env("VAULT_SECRET");
    info!("🧹 [KeyProxy] Environment wiped. Keys are now only in memory.");

    let mut quotas = HashMap::new();
    quotas.insert("daemon".to_string(), 1000);
    quotas.insert("watchtower".to_string(), 100);
    quotas.insert("api-server".to_string(), 1000);
    quotas.insert("aiome-agent".to_string(), 1000);

    let persistence_path = env::var("QUOTA_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("workspace/config/key_proxy_state.json"));

    if let Some(parent) = persistence_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let quota_state = if persistence_path.exists() {
        let data = std::fs::read_to_string(&persistence_path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        QuotaState::default()
    };

    let state = AppState {
        gemini_key: Arc::new(SecretString::from(gemini_key)),
        vault_secret: Arc::new(SecretString::from(vault_secret)),
        client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
        state: Arc::new(RwLock::new(quota_state)),
        persistence_path,
        caller_quotas: Arc::new(quotas),
    };

    let app = Router::new()
        .route("/api/v1/llm/complete", post(handle_llm_complete))
        .route("/api/v1/llm/stream", post(handle_llm_stream))
        .route("/api/v1/llm/embed", post(handle_llm_embed))
        .route("/api/v1/health", get(|| async { StatusCode::OK }))
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
        .layer(tower_http::limit::RequestBodyLimitLayer::new(1024 * 1024)) // 1MB max
        .layer(tower_http::timeout::TimeoutLayer::new(
            std::time::Duration::from_secs(120),
        )); // 120s for LLM calls

    let port = env::var("KEY_PROXY_PORT").unwrap_or_else(|_| "3010".to_string());
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

async fn auth_middleware(
    State(state): State<AppState>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let expected = format!("Bearer {}", state.vault_secret.expose_secret());

    if auth_header
        .filter(|a| subtle::ConstantTimeEq::ct_eq(a.as_bytes(), expected.as_bytes()).into())
        .is_some()
    {
        return Ok(next.run(req).await);
    }

    warn!("⛔ [KeyProxy] Unauthorized access attempt.");
    Err(StatusCode::UNAUTHORIZED)
}

async fn handle_llm_complete(
    State(state): State<AppState>,
    Json(payload): Json<ProxyRequest>,
) -> impl IntoResponse {
    let safe_caller_id = payload.caller_id.replace(['\n', '\r'], "_");
    info!("📩 [KeyProxy] Request from caller: {}", safe_caller_id);

    if let Err(status) = check_and_increment_quota(&state, &payload.caller_id).await {
        return status.into_response();
    }

    let gemini_model = env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.0-flash".to_string());
    let url = match payload.endpoint.as_str() {
        "gemini" => format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            gemini_model
        ),
        _ => return (StatusCode::BAD_REQUEST, "Invalid endpoint").into_response(),
    };

    let gemini_payload = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": payload.prompt
            }]
        }],
        "system_instruction": payload.system.map(|s| {
            serde_json::json!({ "parts": [{ "text": s }] })
        })
    });

    let res = state
        .client
        .post(url)
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", state.gemini_key.expose_secret())
        .json(&gemini_payload)
        .send()
        .await;

    match res {
        Ok(resp) => {
            if resp.status().is_success() {
                let body_res: Result<serde_json::Value, _> = resp.json().await;
                match body_res {
                    Ok(body) => {
                        let text = body
                            .get("candidates")
                            .and_then(|c| c.get(0))
                            .and_then(|c| c.get("content"))
                            .and_then(|c| c.get("parts"))
                            .and_then(|p| p.get(0))
                            .and_then(|p| p.get("text"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string();

                        Json(ProxyResponse { result: text }).into_response()
                    }
                    Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error")
                        .into_response(),
                }
            } else {
                let status = resp.status();
                error!("❌ [KeyProxy] Upstream error: {}", status);
                (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
            }
        }
        Err(e) => {
            error!("❌ [KeyProxy] Request failed: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
        }
    }
}
#[derive(Debug, Serialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
}

async fn handle_llm_embed(
    State(state): State<AppState>,
    Json(payload): Json<ProxyRequest>,
) -> impl IntoResponse {
    let safe_caller_id = payload.caller_id.replace(['\n', '\r'], "_");
    info!(
        "🧬 [KeyProxy] Embedding request from caller: {}",
        safe_caller_id
    );

    if let Err(status) = check_and_increment_quota(&state, &payload.caller_id).await {
        return status.into_response();
    }

    let embed_model =
        env::var("GEMINI_EMBED_MODEL").unwrap_or_else(|_| "text-embedding-004".to_string());
    let url = match payload.endpoint.as_str() {
        "gemini-embed" => format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent",
            embed_model
        ),
        _ => return (StatusCode::BAD_REQUEST, "Invalid endpoint").into_response(),
    };

    let gemini_payload = serde_json::json!({
        "content": {
            "parts": [{ "text": payload.prompt }]
        }
    });

    let res = state
        .client
        .post(url)
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", state.gemini_key.expose_secret())
        .json(&gemini_payload)
        .send()
        .await;

    match res {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                let emb = body["embedding"]["values"].as_array();
                if let Some(values) = emb {
                    let vec: Vec<f32> = values
                        .iter()
                        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                        .collect();
                    Json(EmbedResponse { embedding: vec }).into_response()
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
                }
            } else {
                error!("❌ [KeyProxy] Upstream error: {}", resp.status());
                (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
            }
        }
        Err(e) => {
            error!("❌ [KeyProxy] Request failed: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
        }
    }
}

async fn handle_llm_stream(
    State(state): State<AppState>,
    Json(payload): Json<ProxyRequest>,
) -> impl IntoResponse {
    let safe_caller_id = payload.caller_id.replace(['\n', '\r'], "_");
    info!(
        "🌊 [KeyProxy] Streaming request from caller: {}",
        safe_caller_id
    );

    if let Err(status) = check_and_increment_quota(&state, &payload.caller_id).await {
        return status.into_response();
    }

    let gemini_model = env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.0-flash".to_string());
    let url = match payload.endpoint.as_str() {
        "gemini" => format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse",
            gemini_model
        ),
        _ => return (StatusCode::BAD_REQUEST, "Invalid endpoint").into_response(),
    };

    let gemini_payload = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": payload.prompt
            }]
        }],
        "system_instruction": payload.system.map(|s| {
            serde_json::json!({ "parts": [{ "text": s }] })
        })
    });

    let res = state
        .client
        .post(url)
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", state.gemini_key.expose_secret())
        .json(&gemini_payload)
        .send()
        .await;

    match res {
        Ok(resp) => {
            if resp.status().is_success() {
                use futures::StreamExt;
                let stream = resp.bytes_stream().map(|chunk_res| match chunk_res {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes).to_string();
                        Ok::<axum::response::sse::Event, std::convert::Infallible>(
                            axum::response::sse::Event::default().data(text),
                        )
                    }
                    Err(e) => {
                        let error_json = serde_json::json!({ "error": e.to_string() });
                        Ok::<axum::response::sse::Event, std::convert::Infallible>(
                            axum::response::sse::Event::default().data(
                                serde_json::to_string(&error_json)
                                    .unwrap_or_else(|_| "{}".to_string()),
                            ),
                        )
                    }
                });
                axum::response::sse::Sse::new(stream).into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
            }
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response(),
    }
}

async fn check_and_increment_quota(
    state: &AppState,
    caller_id: &str,
) -> Result<(u64, u32), StatusCode> {
    if !state.caller_quotas.contains_key(caller_id) {
        warn!("🚫 [KeyProxy] Unknown caller: {}", caller_id);
        return Err(StatusCode::FORBIDDEN);
    }

    let mut q = state.state.write().await;
    let today = Utc::now().ordinal();
    if q.last_reset_day != today {
        info!("🗓️ [KeyProxy] New day detected. Resetting global quota.");
        q.total_calls = 0;
        q.per_caller_calls.clear();
        q.last_reset_day = today;
    }

    q.total_calls += 1;
    let total = q.total_calls;

    let caller_total = {
        let count = q.per_caller_calls.entry(caller_id.to_string()).or_insert(0);
        *count += 1;
        *count
    };

    if let Some(&limit) = state.caller_quotas.get(caller_id)
        && caller_total > limit {
            warn!(
                "🛑 [KeyProxy] Caller {} exceeded quota ({})",
                caller_id, limit
            );
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

    if total > 5000 {
        error!(
            "🛑 [KeyProxy] Global quota exceeded! (Day: {})",
            q.last_reset_day
        );
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    if total % 10 == 0 {
        let path = state.persistence_path.clone();
        let state_clone = q.clone();
        tokio::spawn(async move {
            if let Ok(data) = serde_json::to_string(&state_clone) {
                let _ = tokio::fs::write(path, data).await;
            }
        });
    }

    Ok((total, q.last_reset_day))
}
