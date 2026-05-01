#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![forbid(unsafe_code)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::unnecessary_cast)]

use aiome_core::contracts::{
    ApprovalState, FederatedKarma, FederationPushRequest, FederationPushResponse,
    FederationSyncRequest, FederationSyncResponse, HubMessage, ImmuneRule,
};
use axum::{
    error_handling::HandleErrorLayer,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        DefaultBodyLimit, Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
// Standard imports
use shared::sql_fetch_all;
// use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};

mod handlers;
mod hub_auth_tests;
mod hub_discovery_tests;
mod mdns_listener;
mod models;
mod state;

use crate::handlers::federation::{push_handler, sync_handler};
use crate::handlers::verify_bearer;
use crate::handlers::ws::ws_handler;
use crate::models::*;
use crate::state::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("CELL_ID").unwrap_or_default().is_empty() {
        eprintln!("🚨 FATAL: CELL_ID is not set! The Sovereign Verifier architecture requires strict cellular isolation. No identity = No survival.");
        std::process::exit(1);
    }

    // Initialize tracing with JSON for easier aggregation in the hub
    tracing_subscriber::fmt().json().init();

    // 1. Initial attempt from CWD (essential for dev environments)
    dotenvy::dotenv().ok();

    let resolver = shared::app_data::AppDataResolver::new();

    // 2. Explicit attempt from application root (essential for Production)
    let app_env_path = resolver.root().join(".env");
    if app_env_path.exists() && dotenvy::from_path(&app_env_path).is_ok() {
        tracing::info!(
            "Loaded explicit environment from {}",
            app_env_path.display()
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
                approval_worker(pool, ct).await;
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

async fn health_handler() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(serde_json::json!({"status": "healthy", "service": "samsara-hub"})),
    )
}

async fn list_topics_handler(
    State(state): State<Arc<HubState>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let query =
        "SELECT * FROM biome_topics WHERE status = 'Active' ORDER BY updated_at DESC LIMIT 50"
            .to_string();
    let rows: Vec<TopicRecord> =
        sql_fetch_all!(&state.pool, TopicRecord, &query).unwrap_or_default();

    let topics: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "topic_id": row.topic_id,
                "peer_pubkey": row.peer_pubkey,
                "summary": row.summary,
                "turn_count": row.turn_count,
                "created_at": row.created_at,
            })
        })
        .collect();

    (StatusCode::OK, Json(serde_json::json!(topics)))
}

async fn list_agents_handler(
    State(state): State<Arc<HubState>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let reg = state.agent_registry.read().await;
    let mut agents = Vec::new();
    for info in reg.values() {
        agents.push(serde_json::json!({
            "did": info.did,
            "ip": info.ip,
            "port": info.port,
            "last_seen_seconds_ago": info.last_seen.elapsed().as_secs()
        }));
    }
    (StatusCode::OK, Json(serde_json::json!(agents)))
}

async fn create_topic_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Json(mut req): Json<CreateTopicRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // 🛡️ [GlassWorm Shield] Sanitize text fields
    req.summary = req
        .summary
        .map(|s| shared::guardrails::strip_invisible_unicode(&s).into_owned());

    // Auth Check
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Try JWT (AuthManager) first
    let mut authenticated = false;
    if auth_header.starts_with("Bearer ") {
        let token = auth_header.trim_start_matches("Bearer ");
        if let Ok(claims) = state.auth_manager.validate_token(token).await {
            if claims.agent_id != uuid::Uuid::nil() {
                authenticated = true;
            }
        }
    }

    // Fallback to Legacy Shared Secret
    if !authenticated && verify_bearer(auth_header, &state.secret) {
        authenticated = true;
    }

    if !authenticated {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized"})),
        );
    }

    // 2. Proof of Karma (PoK) Verification
    // Requirement: Technical Karma weight sum >= 500
    // 2. Proof of Karma (PoK) Verification
    // Requirement: Technical Karma weight sum >= 500
    let karma_query = format!(
        "SELECT COALESCE(SUM(weight), 0) FROM approved_karma WHERE node_id = {} AND karma_type = 'Technical'",
        state.pool.ph(0)
    );
    let karma_sum =
        shared::sql_fetch_optional!(&state.pool, (i64,), &karma_query, &req.peer_pubkey)
            .unwrap_or(Some((0,)))
            .unwrap_or((0,))
            .0;

    info!(
        "🛡️ [Hub] PoK Check for {}: Technical Karma = {}",
        req.peer_pubkey, karma_sum
    );

    if karma_sum < 500 {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Insufficient Technical Karma to create a topic",
                "required": 500,
                "actual": karma_sum
            })),
        );
    }

    // 3. Insert Topic
    // 3. Insert Topic
    let insert_query = format!(
        "INSERT INTO biome_topics (topic_id, peer_pubkey, summary) VALUES ({}, {}, {})",
        state.pool.ph(0),
        state.pool.ph(1),
        state.pool.ph(2)
    );
    let res = shared::sql_exec!(
        &state.pool,
        &insert_query,
        &req.topic_id,
        &req.peer_pubkey,
        &req.summary
    );

    match res {
        Ok(_) => {
            info!(
                "🌟 [Hub] New Biome Topic created: {} by {}",
                req.topic_id, req.peer_pubkey
            );
            (
                StatusCode::CREATED,
                Json(serde_json::json!({"status": "created", "topic_id": req.topic_id})),
            )
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                serde_json::json!({"error": "Failed to create topic due to internal server error"}),
            ),
        ),
    }
}

async fn biome_relay_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Json(mut msg): Json<aiome_core::biome::BiomeMessage>,
) -> (StatusCode, Json<serde_json::Value>) {
    // 🛡️ [GlassWorm Shield] Sanitize text fields
    msg.content = shared::guardrails::strip_invisible_unicode(&msg.content).into_owned();

    // 1. Auth Check
    if !verify_bearer(
        headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .unwrap_or(""),
        &state.secret,
    ) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized"})),
        );
    }

    // 1.5 Topic Existence / Status Check
    // 1.5 Topic Existence / Status Check
    let topic_check_query = format!(
        "SELECT COUNT(*) FROM biome_topics WHERE topic_id = {} AND status = 'Active'",
        state.pool.ph(0)
    );
    let topic_exists =
        shared::sql_fetch_optional!(&state.pool, (i64,), &topic_check_query, &msg.topic_id)
            .unwrap_or(Some((0,)))
            .unwrap_or((0,))
            .0
            > 0;

    if !topic_exists {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Topic not found or inactive"})),
        );
    }

    // 2. Verification (Signature)
    use base64::prelude::*;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let mut valid = false;
    let payload = format!(
        "{}:{}:{}:{}",
        msg.sender_pubkey, msg.topic_id, msg.lamport_clock, msg.content
    );
    if let (Ok(pubkey_bytes), Ok(sig_bytes)) = (
        BASE64_STANDARD.decode(&msg.sender_pubkey),
        BASE64_STANDARD.decode(&msg.signature),
    ) {
        if let (Ok(pubkey_arr), Ok(sig)) = (
            pubkey_bytes.try_into() as Result<[u8; 32], _>,
            Signature::from_slice(&sig_bytes),
        ) {
            if let Ok(pubkey) = VerifyingKey::from_bytes(&pubkey_arr) {
                if pubkey.verify(payload.as_bytes(), &sig).is_ok() {
                    valid = true;
                }
            }
        }
    }

    if !valid {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"status": "error", "message": "Invalid Signature"})),
        );
    }

    // 3. CSAM Binary Filter (Plan D: Protocol-Level Enforcement)
    // 決してバイナリやカスタム画像をHubに流さない
    if msg.content.contains("data:image/")
        || msg.content.contains("data:video/")
        || msg.content.contains(";base64,")
    {
        warn!(
            "🚨 [CSAM Filter] Blocked Biome relay containing binary/base64 data from {}",
            msg.sender_pubkey
        );
        return (
            StatusCode::FORBIDDEN,
            Json(
                serde_json::json!({"status": "blocked", "message": "Binary data and inline assets are strictly prohibited by protocol"}),
            ),
        );
    }

    // 4. Relay Logic
    info!(
        "📫 [Hub] Relaying Biome Message from {} to topic {}",
        msg.sender_pubkey, msg.topic_id
    );

    // Buffer in DB
    let payload_json = serde_json::to_string(&msg).unwrap_or_default();
    let relay_insert_query = format!(
        "INSERT INTO biome_relay_queue (recipient_pubkey, payload) VALUES ({}, {})",
        state.pool.ph(0),
        state.pool.ph(1)
    );
    if let Err(e) = shared::sql_exec!(
        &state.pool,
        &relay_insert_query,
        &msg.recipient_pubkey,
        &payload_json
    ) {
        error!(
            "🛡️ [Relay] Failed to queue biome message for {}: {}",
            msg.recipient_pubkey, e
        );
    }

    // Update Turn Count in Topic (State Channel)
    let turn_update_query = format!(
        "UPDATE biome_topics SET turn_count = turn_count + 1, updated_at = {} WHERE topic_id = {}",
        state.pool.now_fn(),
        state.pool.ph(0)
    );
    if let Err(e) = shared::sql_exec!(&state.pool, &turn_update_query, &msg.topic_id) {
        warn!(
            "🛡️ [Relay] Failed to increment turn_count for {}: {}",
            msg.topic_id, e
        );
    }

    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"status": "accepted"})),
    )
}

async fn biome_ws_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Query(query): Query<BiomeWsQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    // Auth Check
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let mut authenticated = false;
    if auth_header.starts_with("Bearer ") {
        let token = auth_header.trim_start_matches("Bearer ");
        if let Ok(claims) = state.auth_manager.validate_token(token).await {
            if claims.agent_id != uuid::Uuid::nil() {
                authenticated = true;
            }
        }
    }

    if !authenticated && verify_bearer(auth_header, &state.secret) {
        authenticated = true;
    }

    if !authenticated {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    ws.on_upgrade(move |socket| handle_biome_ws(socket, state, query.node_id))
}

async fn handle_biome_ws(mut socket: WebSocket, state: Arc<HubState>, node_id: String) {
    let mut rx = state.tx.subscribe();

    info!(
        "📪 [BiomeWS] Node {} connected for real-time relay.",
        node_id
    );

    loop {
        tokio::select! {
            Ok(msg) = rx.recv() => {
                if let HubMessage::BiomeRelay(biome_msg) = msg {
                    // SEC: Only send if it's for this recipient
                    if biome_msg.recipient_pubkey != node_id {
                        continue;
                    }
                    let text = serde_json::to_string(&HubMessage::BiomeRelay(biome_msg)).unwrap_or_default();
                    if socket.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                if socket.send(Message::Ping(vec![])).await.is_err() {
                    break;
                }
            }
        }
    }
}

async fn approval_worker(pool: shared::db::DatabasePool, token: CancellationToken) {
    use base64::{prelude::BASE64_STANDARD, Engine};
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    info!("⚙️ [ApprovalWorker] Starting quarantine validation thread.");

    loop {
        if token.is_cancelled() {
            break;
        }

        // 1. Process Quarantined Karma
        let karma_fetch_query = "SELECT * FROM quarantined_karma LIMIT 50";
        let karmas: Vec<FederatedKarmaRecord> =
            shared::sql_fetch_all!(&pool, FederatedKarmaRecord, karma_fetch_query)
                .unwrap_or_default();

        for k in &karmas {
            let mut valid = false;
            if let Some(ref sig_b64) = k.signature {
                let payload = format!("{}:{}:{}", k.id, k.lesson, k.lamport_clock);
                if let (Ok(pubkey_bytes), Ok(sig_bytes)) = (
                    BASE64_STANDARD.decode(&k.node_id),
                    BASE64_STANDARD.decode(sig_b64),
                ) {
                    if let (Ok(pubkey_arr), Ok(sig)) = (
                        pubkey_bytes.try_into() as Result<[u8; 32], _>,
                        Signature::from_slice(&sig_bytes),
                    ) {
                        if let Ok(pubkey) = VerifyingKey::from_bytes(&pubkey_arr) {
                            if pubkey.verify(payload.as_bytes(), &sig).is_ok() {
                                valid = true;
                            }
                        }
                    }
                }
            }

            if valid {
                match pool.begin().await {
                    Ok(mut tx) => {
                        let approved_at_dt = chrono::Utc::now();
                        let approve_karma_query = format!(
                            "INSERT INTO approved_karma (id, node_id, karma_type, related_skill, lesson, weight, soul_version_hash, lamport_clock, signature, created_at, approved_at, clone_origin_id, generation, somatic_valence)
                             VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}) ON CONFLICT(id) DO NOTHING ",
                             pool.ph(0), pool.ph(1), pool.ph(2), pool.ph(3), pool.ph(4), pool.ph(5), pool.ph(6), pool.ph(7), pool.ph(8), pool.ph(9), pool.ph(10), pool.ph(11), pool.ph(12), pool.ph(13)
                        );
                        let res = match &mut tx {
                            shared::db::DatabaseTransaction::Sqlite(t) => {
                                sqlx::query::<sqlx::Sqlite>(&approve_karma_query)
                                    .bind(&k.id)
                                    .bind(&k.node_id)
                                    .bind(&k.karma_type)
                                    .bind(&k.related_skill)
                                    .bind(&k.lesson)
                                    .bind(k.weight as i64)
                                    .bind(&k.soul_version_hash)
                                    .bind(k.lamport_clock as i64)
                                    .bind(&k.signature)
                                    .bind(&k.created_at)
                                    .bind(&approved_at_dt)
                                    .bind(&k.clone_origin_id)
                                    .bind(k.generation.map(|v| v as i64))
                                    .bind(k.somatic_valence)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                            shared::db::DatabaseTransaction::Postgres(t) => {
                                sqlx::query::<sqlx::Postgres>(&approve_karma_query)
                                    .bind(&k.id)
                                    .bind(&k.node_id)
                                    .bind(&k.karma_type)
                                    .bind(&k.related_skill)
                                    .bind(&k.lesson)
                                    .bind(k.weight as i64)
                                    .bind(&k.soul_version_hash)
                                    .bind(k.lamport_clock as i64)
                                    .bind(&k.signature)
                                    .bind(&k.created_at)
                                    .bind(&approved_at_dt)
                                    .bind(&k.clone_origin_id)
                                    .bind(k.generation.map(|v| v as i64))
                                    .bind(k.somatic_valence)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                        };
                        if let Err(e) = res {
                            warn!(
                                "🛡️ [ApprovalWorker] Failed to insert approved karma {}: {}",
                                k.id, e
                            );
                        }

                        let delete_quarantine_query =
                            format!("DELETE FROM quarantined_karma WHERE id = {}", pool.ph(0));
                        let res_del = match &mut tx {
                            shared::db::DatabaseTransaction::Sqlite(t) => {
                                sqlx::query::<sqlx::Sqlite>(&delete_quarantine_query)
                                    .bind(&k.id)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                            shared::db::DatabaseTransaction::Postgres(t) => {
                                sqlx::query::<sqlx::Postgres>(&delete_quarantine_query)
                                    .bind(&k.id)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                        };
                        if let Err(e) = res_del {
                            warn!(
                                "🛡️ [ApprovalWorker] Failed to delete quarantined karma {}: {}",
                                k.id, e
                            );
                        }
                        if let Err(e) = tx.commit().await {
                            error!(
                                "❌ [ApprovalWorker] Failed to commit karma approval for {}: {}",
                                k.id, e
                            );
                        } else {
                            info!("✅ [ApprovalWorker] Approved Karma: {}", k.id);
                        }
                    }
                    Err(e) => error!("❌ [ApprovalWorker] Failed to start transaction: {:?}", e),
                }
            } else {
                warn!(
                    "🛡️ [ApprovalWorker] Rejecting invalid Karma (Signature Mismatch): {}",
                    k.id
                );
                // BFT Slashing: Penalize node reputation for invalid signatures
                // BFT Slashing
                let slash_query = format!("UPDATE node_reputation SET reputation_score = reputation_score - 10 WHERE node_id = {}", pool.ph(0));
                let _ = shared::sql_exec!(&pool, &slash_query, &k.node_id);
                let delete_malformed_query =
                    format!("DELETE FROM quarantined_karma WHERE id = {}", pool.ph(0));
                let _ = shared::sql_exec!(&pool, &delete_malformed_query, &k.id);
            }
        }

        // 2. Process Quarantined Rules
        let rule_fetch_query = "SELECT * FROM quarantined_rules LIMIT 50";
        let rules: Vec<ImmuneRuleRecord> =
            shared::sql_fetch_all!(&pool, ImmuneRuleRecord, rule_fetch_query).unwrap_or_default();

        for r in &rules {
            let mut valid = false;
            if let Some(ref sig_b64) = r.signature {
                let payload = format!("{}:{}:{}", r.id, r.pattern, r.lamport_clock);
                if let (Ok(pubkey_bytes), Ok(sig_bytes)) = (
                    BASE64_STANDARD.decode(&r.node_id),
                    BASE64_STANDARD.decode(sig_b64),
                ) {
                    if let (Ok(pubkey_arr), Ok(sig)) = (
                        pubkey_bytes.try_into() as Result<[u8; 32], _>,
                        Signature::from_slice(&sig_bytes),
                    ) {
                        if let Ok(pubkey) = VerifyingKey::from_bytes(&pubkey_arr) {
                            if pubkey.verify(payload.as_bytes(), &sig).is_ok() {
                                valid = true;
                            }
                        }
                    }
                }
            }

            if valid {
                match pool.begin().await {
                    Ok(mut tx) => {
                        let approved_at_dt = chrono::Utc::now();
                        let approve_rule_query = format!(
                            "INSERT INTO approved_rules (id, pattern, severity, action, node_id, lamport_clock, signature, created_at, approved_at)
                             VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}) ON CONFLICT(id) DO NOTHING ",
                             pool.ph(0), pool.ph(1), pool.ph(2), pool.ph(3), pool.ph(4), pool.ph(5), pool.ph(6), pool.ph(7), pool.ph(8)
                        );
                        let res = match &mut tx {
                            shared::db::DatabaseTransaction::Sqlite(t) => {
                                sqlx::query::<sqlx::Sqlite>(&approve_rule_query)
                                    .bind(&r.id)
                                    .bind(&r.pattern)
                                    .bind(r.severity)
                                    .bind(&r.action)
                                    .bind(&r.node_id)
                                    .bind(r.lamport_clock)
                                    .bind(&r.signature)
                                    .bind(&r.created_at)
                                    .bind(&approved_at_dt)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                            shared::db::DatabaseTransaction::Postgres(t) => {
                                sqlx::query::<sqlx::Postgres>(&approve_rule_query)
                                    .bind(&r.id)
                                    .bind(&r.pattern)
                                    .bind(r.severity)
                                    .bind(&r.action)
                                    .bind(&r.node_id)
                                    .bind(r.lamport_clock)
                                    .bind(&r.signature)
                                    .bind(&r.created_at)
                                    .bind(&approved_at_dt)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                        };
                        if let Err(e) = res {
                            warn!(
                                "🛡️ [ApprovalWorker] Failed to insert approved rule {}: {}",
                                r.id, e
                            );
                        }

                        let delete_quarantine_rule_query =
                            format!("DELETE FROM quarantined_rules WHERE id = {}", pool.ph(0));
                        let res_del = match &mut tx {
                            shared::db::DatabaseTransaction::Sqlite(t) => {
                                sqlx::query::<sqlx::Sqlite>(&delete_quarantine_rule_query)
                                    .bind(&r.id)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                            shared::db::DatabaseTransaction::Postgres(t) => {
                                sqlx::query::<sqlx::Postgres>(&delete_quarantine_rule_query)
                                    .bind(&r.id)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                        };
                        if let Err(e) = res_del {
                            warn!(
                                "🛡️ [ApprovalWorker] Failed to delete quarantined rule {}: {}",
                                r.id, e
                            );
                        }
                        if let Err(e) = tx.commit().await {
                            error!(
                                "❌ [ApprovalWorker] Failed to commit rule approval for {}: {}",
                                r.id, e
                            );
                        } else {
                            info!("✅ [ApprovalWorker] Approved Rule: {}", r.id);
                        }
                    }
                    Err(e) => error!("❌ [ApprovalWorker] Failed to start transaction: {:?}", e),
                }
            } else {
                warn!(
                    "🛡️ [ApprovalWorker] Rejecting invalid Rule (Signature Mismatch): {}",
                    r.id
                );
                // BFT Slashing: Penalize node reputation for invalid signatures
                // BFT Slashing
                let slash_rule_query = format!("UPDATE node_reputation SET reputation_score = reputation_score - 10 WHERE node_id = {}", pool.ph(0));
                let _ = shared::sql_exec!(&pool, &slash_rule_query, &r.node_id);
                let delete_malformed_rule_query =
                    format!("DELETE FROM quarantined_rules WHERE id = {}", pool.ph(0));
                let _ = shared::sql_exec!(&pool, &delete_malformed_rule_query, &r.id);
            }
        }

        // 3. Data Eviction (Flaw 3: Disk Exhaustion Defense)
        // Keep ONLY the last 1,000,000 Records
        let karma_evict_query = "DELETE FROM approved_karma WHERE id NOT IN (SELECT id FROM approved_karma ORDER BY approved_at DESC LIMIT 1000000)";
        let rule_evict_query = "DELETE FROM approved_rules WHERE id NOT IN (SELECT id FROM approved_rules ORDER BY approved_at DESC LIMIT 1000000)";

        let res_k = shared::sql_exec!(&pool, karma_evict_query);
        if let Err(e) = res_k {
            warn!("⚠️ [SamsaraHub] Karma eviction failed: {}", e);
        }

        let res_r = shared::sql_exec!(&pool, rule_evict_query);
        if let Err(e) = res_r {
            warn!("⚠️ [SamsaraHub] Rule eviction failed: {}", e);
        }

        // Dynamic Polling (Component 2: Backpressure Tuning)
        let total_processed = karmas.len() + rules.len();
        if total_processed >= 100 {
            // High load: Don't sleep, keep processing quarantine
            tokio::task::yield_now().await;
        } else {
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}

// Custom extractor for authenticated user claims
#[derive(Clone, Debug)]
pub struct AuthenticatedUser(pub shared::auth::AiomeCustomClaims);

use automerge::AutoCommit;

async fn auth_middleware(
    State(state): State<Arc<HubState>>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let mut authenticated = false;
    if auth_header.starts_with("Bearer ") {
        let token = auth_header.trim_start_matches("Bearer ");
        if let Ok(claims) = state.auth_manager.validate_token(token).await {
            // RBAC Enforcement: Hub operations require System, Admin, or Federated roles.
            if claims.roles.iter().any(|r| {
                matches!(
                    r,
                    shared::auth::Role::Admin
                        | shared::auth::Role::System
                        | shared::auth::Role::Federated
                )
            }) {
                authenticated = true;
            } else {
                warn!("⛔ [Hub] Access denied for roles: {:?}", claims.roles);
            }
        }
    }

    if !authenticated && verify_bearer(auth_header, &state.secret) {
        authenticated = true;
    }

    if authenticated {
        Ok(next.run(req).await)
    } else {
        warn!("⛔ [Hub] Unauthorized access attempt.");
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn timeline_sync_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Json(payload): Json<TimelineSyncRequest>,
) -> impl IntoResponse {
    use subtle::ConstantTimeEq;

    let auth = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let expected = format!("Bearer {}", state.secret.expose_secret());
    let is_auth_valid = if auth.len() == expected.len() {
        bool::from(auth.as_bytes().ct_eq(expected.as_bytes()))
    } else {
        false
    };

    if !is_auth_valid {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    // Load or Init Hub Master Doc
    let timeline_fetch_query = format!(
        "SELECT automerge_blob FROM hub_timeline WHERE id = {}",
        state.pool.ph(0)
    );
    let blob_opt = match &state.pool {
        shared::db::DatabasePool::Sqlite(p) => {
            sqlx::query_scalar::<_, Vec<u8>>(&timeline_fetch_query)
                .bind(&payload.hub_id)
                .fetch_optional(p)
                .await
                .unwrap_or(None)
        }
        shared::db::DatabasePool::Postgres(p) => {
            sqlx::query_scalar::<_, Vec<u8>>(&timeline_fetch_query)
                .bind(&payload.hub_id)
                .fetch_optional(p)
                .await
                .unwrap_or(None)
        }
    };

    let mut hub_doc = match blob_opt {
        Some(blob) => AutoCommit::load(&blob).unwrap_or_else(|_| AutoCommit::new()),
        None => AutoCommit::new(),
    };

    // CSAM Binary Filter: Decline oversized CRDT syncs which implies binary embedding
    if payload.automerge_blob.len() > 1024 * 1024 {
        // 1MB Hard Limit
        warn!(
            "🚨 [CSAM Filter] Blocked oversized CRDT timeline sync ({} bytes) from hub {}",
            payload.automerge_blob.len(),
            payload.hub_id
        );
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            "CRDT Timeline Sync exceeds maximum allowed size (binary embedding suspected)",
        )
            .into_response();
    }

    // Load and Merge Node's Doc
    if let Ok(mut node_doc) = AutoCommit::load(&payload.automerge_blob) {
        let _ = hub_doc.merge(&mut node_doc);
    }

    let finalized_blob = hub_doc.save();

    // Persist Hub Master Doc
    let timeline_persist_query = format!(
        "INSERT INTO hub_timeline (id, automerge_blob) VALUES ({}, {}) 
         ON CONFLICT(id) DO UPDATE SET automerge_blob = EXCLUDED.automerge_blob, updated_at = {}",
        state.pool.ph(0),
        state.pool.ph(1),
        state.pool.now_fn()
    );
    let res = match &state.pool {
        shared::db::DatabasePool::Sqlite(p) => sqlx::query(&timeline_persist_query)
            .bind(&payload.hub_id)
            .bind(&finalized_blob)
            .execute(p)
            .await
            .map(|_| ()),
        shared::db::DatabasePool::Postgres(p) => sqlx::query(&timeline_persist_query)
            .bind(&payload.hub_id)
            .bind(&finalized_blob)
            .execute(p)
            .await
            .map(|_| ()),
    };
    if let Err(e) = res {
        error!("🛡️ [Timeline] Failed to persist hub timeline: {}", e);
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "synchronized",
            "automerge_blob": finalized_blob
        })),
    )
        .into_response()
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

    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);

    Router::new()
        .route("/api/v1/federation/sync", post(sync_handler))
        .route("/api/v1/federation/push", post(push_handler))
        .route("/api/v1/health", get(health_handler))
        .route("/api/v1/registry/agents", get(list_agents_handler))
        .route(
            "/api/v1/biome/topics",
            get(list_topics_handler).post(create_topic_handler),
        )
        .route("/api/v1/biome/relay", post(biome_relay_handler))
        .route("/api/v1/biome/ws", get(biome_ws_handler))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        // WS and Sync handled inside their handlers for manual auth/handshake
        .route("/api/v1/federation/ws", get(ws_handler))
        .route("/api/v1/relay/timeline/sync", post(timeline_sync_handler))
        .layer(cors)
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
                .layer(RateLimitLayer::new(600, Duration::from_secs(60))), // High frequency for Biome
        )
        .with_state(state)
}

mod hub_reliability_tests;
#[cfg(test)]
mod hub_ws_tests;
