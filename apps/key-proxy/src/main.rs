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
    content: String,
    stop_reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_time_ms: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct QuotaState {
    pub(crate) total_calls: u64,
    pub(crate) last_reset_day: u32, // Day of the year
    #[serde(default)]
    pub(crate) per_caller_calls: std::collections::HashMap<String, u64>,
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
pub(crate) struct AppState {
    pub(crate) gemini_key: Arc<SecretString>,
    pub(crate) vault_secret: Arc<SecretString>,
    pub(crate) client: reqwest::Client,
    pub(crate) state: Arc<RwLock<QuotaState>>,
    pub auth_manager: Arc<dyn infrastructure::auth::AuthManager>,
    pub(crate) persistence_path: PathBuf,
    pub(crate) caller_quotas: Arc<HashMap<String, u64>>,
    pub(crate) wp_api_url: Option<String>,
    pub(crate) wp_api_token: Option<Arc<SecretString>>,
}

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
    //    Initial attempt from CWD (essential for dev environments)
    dotenvy::dotenv().ok();

    let resolver = shared::app_data::AppDataResolver::new().map_err(|e| anyhow::anyhow!(e))?;

    //    Explicit attempt from application root (essential for Production)
    let app_env_path = resolver.root().join(".env");
    if app_env_path.exists() && dotenvy::from_path(&app_env_path).is_ok() {
        tracing::info!(
            "Loaded explicit environment from {}",
            app_env_path.display()
        );
    }
    let gemini_key = env::var("GEMINI_API_KEY").unwrap_or_else(|_| {
        error!("🚨 [CRITICAL] GEMINI_API_KEY must be set in key-proxy/.env");
        std::process::exit(1);
    });

    let vault_secret = env::var("VAULT_SECRET").unwrap_or_else(|_| {
        error!("🚨 [CRITICAL] VAULT_SECRET must be set for Abyss Vault access!");
        std::process::exit(1);
    });

    let wp_api_url = env::var("WP_API_URL").ok();
    let wp_api_token = env::var("WP_API_TOKEN").ok();

    // Self-Wipe: Remove from environment immediately
    // key-proxy では shared::security::scrub_env を使用して一元化
    shared::security::scrub_env("GEMINI_API_KEY");
    shared::security::scrub_env("VAULT_SECRET");
    shared::security::scrub_env("WP_API_TOKEN");
    info!("🧹 [KeyProxy] Environment wiped. Keys are now only in memory.");

    let mut quotas = HashMap::new();
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

    let state = AppState {
        gemini_key: Arc::new(SecretString::from(gemini_key)),
        vault_secret: Arc::new(SecretString::from(vault_secret)),
        client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
        state: Arc::new(RwLock::new(quota_state)),
        auth_manager: {
            match std::env::var("JWT_PRIVATE_KEY_B64") {
                Ok(key_b64) => {
                    info!("🔑 [KeyProxy] Loading JWT private key from environment");
                    shared::security::scrub_env("JWT_PRIVATE_KEY_B64");
                    Arc::new(
                        infrastructure::auth::JwtAuthManager::from_private_key_b64(&key_b64)
                            .map_err(|e| anyhow::anyhow!("Invalid JWT_PRIVATE_KEY_B64: {}", e))?,
                    )
                }
                #[cfg(debug_assertions)]
                Err(_) => {
                    warn!("⚠️ [KeyProxy] JWT key not set, using MockAuthManager (dev only)");
                    Arc::new(infrastructure::auth::MockAuthManager::new())
                }
                #[cfg(not(debug_assertions))]
                Err(_) => {
                    error!("🚨 [FATAL] JWT_PRIVATE_KEY_B64 must be set in production!");
                    std::process::exit(1);
                }
            }
        },
        persistence_path,
        caller_quotas: Arc::new(quotas),
        wp_api_url,
        wp_api_token: wp_api_token.map(|t| Arc::new(SecretString::from(t))),
    };

    let app = Router::new()
        .route("/api/v1/llm/complete", post(handle_llm_complete))
        .route("/api/v1/llm/stream", post(handle_llm_stream))
        .route("/api/v1/llm/embed", post(handle_llm_embed))
        .route("/api/v1/wp/publish", post(handle_wp_publish))
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

#[tracing::instrument(skip(state))]
pub(crate) async fn auth_middleware(
    State(state): State<AppState>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Strategy 1: JWT validation via AuthManager
    let mut authenticated = false;
    if auth_header.starts_with("Bearer ") {
        let token = auth_header.trim_start_matches("Bearer ");
        if let Ok(claims) = state.auth_manager.validate_token(token).await {
            if claims.agent_id != uuid::Uuid::nil() {
                authenticated = true;
            }
        }
    }

    // Strategy 2: Legacy Vault Secret fallback
    if !authenticated {
        let expected = format!("Bearer {}", state.vault_secret.expose_secret());
        if auth_header.len() == expected.len() {
            if bool::from(subtle::ConstantTimeEq::ct_eq(
                auth_header.as_bytes(),
                expected.as_bytes(),
            )) {
                authenticated = true;
            }
        }
    }

    // Strategy 3: Query parameter fallback (for SDKs that use key=... instead of headers)
    if !authenticated {
        if let Some(query) = req.uri().query() {
            for param in query.split('&') {
                if let Some(provided_key) = param.strip_prefix("key=") {
                    if bool::from(subtle::ConstantTimeEq::ct_eq(
                        provided_key.as_bytes(),
                        state.vault_secret.expose_secret().as_bytes(),
                    )) {
                        authenticated = true;
                        break;
                    }
                }
            }
        }
    }

    // Strategy 4: Custom header fallback (for Google GenAI SDK which uses x-goog-api-key)
    if !authenticated {
        if let Some(goog_key) = req
            .headers()
            .get("x-goog-api-key")
            .and_then(|h| h.to_str().ok())
        {
            if bool::from(subtle::ConstantTimeEq::ct_eq(
                goog_key.as_bytes(),
                state.vault_secret.expose_secret().as_bytes(),
            )) {
                authenticated = true;
            }
        }
    }

    if authenticated {
        Ok(next.run(req).await)
    } else {
        warn!("⛔ [KeyProxy] Unauthorized access attempt.");
        Err(StatusCode::UNAUTHORIZED)
    }
}

#[tracing::instrument(skip(state))]
pub(crate) async fn handle_llm_complete(
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

    let mut gemini_payload = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": payload.prompt
            }]
        }]
    });
    if let Some(s) = payload.system {
        if let Some(obj) = gemini_payload.as_object_mut() {
            obj.insert(
                "system_instruction".to_string(),
                serde_json::json!({ "parts": [{ "text": s }] }),
            );
        }
    }

    let start_time = tokio::time::Instant::now();

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

                        let total_tokens = body
                            .get("usageMetadata")
                            .and_then(|u| u.get("totalTokenCount"))
                            .and_then(|c| c.as_u64());

                        if let Some(tokens) = total_tokens {
                            let cost_usd = tokens as f64 * 0.00000015;
                            tracing::info!(
                                target: "key_proxy::metrics",
                                tokens = tokens,
                                cost_usd = cost_usd,
                                model = %gemini_model,
                                "💰 [KeyProxy] Cost metric recorded"
                            );
                        }

                        let response_time_ms = start_time.elapsed().as_millis() as u64;

                        Json(ProxyResponse {
                            content: text,
                            stop_reason: "end_turn".to_string(),
                            total_tokens,
                            response_time_ms: Some(response_time_ms),
                        })
                        .into_response()
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
    #[serde(skip_serializing_if = "Option::is_none")]
    response_time_ms: Option<u64>,
}

#[tracing::instrument(skip(state))]
pub(crate) async fn handle_llm_embed(
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

    let start_time = tokio::time::Instant::now();

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
                    let response_time_ms = start_time.elapsed().as_millis() as u64;
                    Json(EmbedResponse {
                        embedding: vec,
                        response_time_ms: Some(response_time_ms),
                    })
                    .into_response()
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

#[tracing::instrument(skip(state))]
pub(crate) async fn handle_llm_stream(
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

    let mut gemini_payload = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": payload.prompt
            }]
        }]
    });
    if let Some(s) = payload.system {
        if let Some(obj) = gemini_payload.as_object_mut() {
            obj.insert(
                "system_instruction".to_string(),
                serde_json::json!({ "parts": [{ "text": s }] }),
            );
        }
    }

    let start_time = tokio::time::Instant::now();

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
                let response_time_ms = start_time.elapsed().as_millis() as u64;
                tracing::info!(
                    target: "key_proxy::metrics",
                    response_time_ms = response_time_ms,
                    "🌊 [KeyProxy] Streaming response started"
                );

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

#[tracing::instrument(skip(state))]
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

    if let Some(&limit) = state.caller_quotas.get(caller_id) {
        tracing::info!(
            target: "key_proxy::metrics",
            caller_id = %caller_id,
            caller_calls = caller_total,
            caller_limit = limit,
            usage_ratio = (caller_total as f64 / limit as f64),
            "📈 [KeyProxy] Rate limit usage statistics"
        );

        if caller_total > limit {
            warn!(
                "🛑 [KeyProxy] Caller {} exceeded quota ({})",
                caller_id, limit
            );
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
    }

    if total > 150000 {
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

pub(crate) fn build_gemini_passthrough_url(path: &str, query: Option<&str>) -> String {
    let base = format!(
        "https://generativelanguage.googleapis.com/{}",
        path.trim_start_matches('/')
    );
    if let Some(q) = query {
        if !q.is_empty() {
            return format!("{}?{}", base, q);
        }
    }
    base
}

#[tracing::instrument(skip(state))]
pub(crate) async fn handle_gemini_passthrough(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<String>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    let query_string = req.uri().query().unwrap_or("").to_string();
    let method = req.method().clone();

    // Convert Request body to bytes
    let (parts, body) = req.into_parts();
    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(e) => {
            error!("❌ [KeyProxy] Failed to read passthrough body: {}", e);
            return (StatusCode::BAD_REQUEST, "Invalid body").into_response();
        }
    };

    let mut target_url = build_gemini_passthrough_url(&path, Some(&query_string));

    // Inject API key if not present
    if !target_url.contains("key=") {
        let separator = if target_url.contains('?') { "&" } else { "?" };
        target_url = format!(
            "{}{}key={}",
            target_url,
            separator,
            state.gemini_key.expose_secret()
        );
    } else {
        // Replace fake key with real key if it was passed
        let fake_key_start = match target_url.find("key=") {
            Some(idx) => idx + 4,
            None => {
                error!("❌ [KeyProxy] Logically unreachable: key= not found in else branch");
                return (StatusCode::INTERNAL_SERVER_ERROR, "Proxy internal error").into_response();
            }
        };
        let fake_key_end = target_url[fake_key_start..]
            .find('&')
            .map(|i| i + fake_key_start)
            .unwrap_or(target_url.len());
        target_url.replace_range(
            fake_key_start..fake_key_end,
            state.gemini_key.expose_secret(),
        );
    }

    info!("🌐 [KeyProxy] Passthrough to Gemini API: {}", path);

    let mut request_builder = state.client.request(method, &target_url);
    for (name, value) in parts.headers.iter() {
        let name_lower = name.as_str().to_lowercase();
        // Drop dangerous or overwritten headers
        if name_lower == "host" || name_lower == "authorization" || name_lower == "x-goog-api-key" {
            continue;
        }
        request_builder = request_builder.header(name, value);
    }

    // Ensure Content-Type is present
    if !parts.headers.contains_key("Content-Type") {
        request_builder = request_builder.header("Content-Type", "application/json");
    }

    let request_builder = request_builder.body(body_bytes);

    let res = match request_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            error!("❌ [KeyProxy] Gemini upstream error: {}", e);
            return (StatusCode::BAD_GATEWAY, "Proxy error").into_response();
        }
    };

    let status = res.status();
    let headers = res.headers().clone();
    let content_type = headers
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    let res_bytes = match res.bytes().await {
        Ok(b) => b,
        Err(e) => {
            error!("❌ [KeyProxy] Failed to read Gemini response: {}", e);
            return (StatusCode::BAD_GATEWAY, "Proxy read error").into_response();
        }
    };

    (
        status,
        [(axum::http::header::CONTENT_TYPE, content_type)],
        res_bytes,
    )
        .into_response()
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_gemini_payload_serialization_without_system_prompt() {
        let payload_prompt = "Hello";
        let payload_system: Option<String> = None;

        let mut gemini_payload = serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": payload_prompt
                }]
            }]
        });

        if let Some(s) = payload_system {
            if let Some(obj) = gemini_payload.as_object_mut() {
                obj.insert(
                    "system_instruction".to_string(),
                    serde_json::json!({ "parts": [{ "text": s }] }),
                );
            }
        }

        assert_eq!(
            gemini_payload.get("system_instruction"),
            None,
            "Should omit system_instruction when system prompt is absent"
        );
    }

    #[test]
    fn test_gemini_passthrough_url_construction() {
        let path = "v1beta/models/gemini-2.0-flash:generateContent".to_string();
        let query_string = Some("key=TEST_DUMMY_KEY".to_string());
        let expected = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key=TEST_DUMMY_KEY";

        let constructed = crate::build_gemini_passthrough_url(&path, query_string.as_deref());
        assert_eq!(constructed, expected);
    }

    #[test]
    fn test_gemini_payload_serialization_with_system_prompt() {
        let payload_prompt = "Hello";
        let payload_system: Option<String> = Some("You are a helpful assistant".to_string());

        let mut gemini_payload = serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": payload_prompt
                }]
            }]
        });

        if let Some(s) = payload_system {
            if let Some(obj) = gemini_payload.as_object_mut() {
                obj.insert(
                    "system_instruction".to_string(),
                    serde_json::json!({ "parts": [{ "text": s }] }),
                );
            }
        }

        assert!(
            gemini_payload.get("system_instruction").is_some(),
            "Should include system_instruction when system prompt is present"
        );
    }

    #[test]
    fn test_proxy_response_includes_telemetry() {
        use crate::ProxyResponse;
        let resp = ProxyResponse {
            content: "test".into(),
            stop_reason: "end_turn".into(),
            total_tokens: Some(42),
            response_time_ms: Some(150),
        };
        assert_eq!(resp.total_tokens, Some(42));
        assert_eq!(resp.response_time_ms, Some(150));
    }

    #[tokio::test]
    async fn test_proxy_embed_response_contains_telemetry() {
        let resp = super::EmbedResponse {
            embedding: vec![1.0, 2.0],
            response_time_ms: Some(123),
        };
        assert_eq!(resp.response_time_ms, Some(123));
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct WpProxyRequest {
    pub caller_id: String,
    pub title: String,
    pub content: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct WpProxyResponse {
    pub link: String,
}

#[tracing::instrument(skip(state))]
pub(crate) async fn handle_wp_publish(
    State(state): State<AppState>,
    Json(payload): Json<WpProxyRequest>,
) -> impl IntoResponse {
    let safe_caller_id = payload.caller_id.replace(['\n', '\r'], "_");
    info!("📩 [KeyProxy] WP Publish Request from: {}", safe_caller_id);

    if let Err(status) = check_and_increment_quota(&state, &payload.caller_id).await {
        return status.into_response();
    }

    let url = match &state.wp_api_url {
        Some(u) => format!("{}/wp-json/wp/v2/posts", u.trim_end_matches('/')),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "WP Integration not configured",
            )
                .into_response();
        }
    };

    let token = match &state.wp_api_token {
        Some(t) => t.expose_secret(),
        None => {
            return (StatusCode::SERVICE_UNAVAILABLE, "WP Token not configured").into_response();
        }
    };

    // §SEC: Validate WP status to prevent unauthorized state transitions (e.g. "trash")
    const ALLOWED_WP_STATUSES: &[&str] = &["draft", "publish", "pending", "private", "future"];
    if !ALLOWED_WP_STATUSES.contains(&payload.status.as_str()) {
        tracing::warn!(
            "🚫 [KeyProxy] Rejected invalid WP status: {}",
            payload.status
        );
        return (StatusCode::BAD_REQUEST, "Invalid WordPress post status").into_response();
    }

    let body = serde_json::json!({
        "title": payload.title,
        "content": payload.content,
        "status": payload.status,
    });

    let res = state
        .client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await;

    match res {
        Ok(resp) => {
            if resp.status().is_success() {
                if let Ok(wp_res) = resp.json::<serde_json::Value>().await {
                    let link = wp_res
                        .get("link")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    return Json(WpProxyResponse { link }).into_response();
                }
            } else {
                let status = resp.status();
                let err_text = resp.text().await.unwrap_or_default();
                tracing::error!("❌ [KeyProxy] WP Upstream error [{}]: {}", status, err_text);
            }
            (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
        }
        Err(e) => {
            tracing::error!("❌ [KeyProxy] WP Request failed: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
        }
    }
}
