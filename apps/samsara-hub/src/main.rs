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

pub struct HubState {
    pool: shared::db::DatabasePool,
    secret: secrecy::SecretString,
    pub auth_manager: Arc<dyn shared::auth::AuthManager>,
    tx: broadcast::Sender<HubMessage>,
    active_connections: std::sync::atomic::AtomicUsize,
    pub agent_registry: mdns_listener::AgentRegistry,
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
struct FederatedKarmaRecord {
    id: String,
    karma_type: String,
    related_skill: String,
    lesson: String,
    weight: i64,
    soul_version_hash: Option<String>,
    lamport_clock: i64,
    node_id: String,
    signature: Option<String>,
    created_at: String,
    clone_origin_id: Option<String>,
    generation: Option<i64>,
    somatic_valence: Option<f64>,
}

mod hub_discovery_tests;
mod mdns_listener;

#[derive(sqlx::FromRow, Serialize, Deserialize)]
struct ImmuneRuleRecord {
    id: String,
    pattern: String,
    severity: i64,
    action: String,
    lamport_clock: i64,
    node_id: String,
    signature: Option<String>,
    created_at: String,
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
struct ArenaMatchRecord {
    id: String,
    skill_a: String,
    skill_b: String,
    topic: String,
    winner: String,
    reasoning: String,
    created_at: String,
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
struct TopicRecord {
    topic_id: String,
    peer_pubkey: String,
    summary: Option<String>,
    turn_count: i32,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
                .unwrap_or_else(|| panic!("Invalid DB Path"))
        )
    });
    let secret_val = std::env::var("FEDERATION_SECRET").unwrap_or_else(|_| {
        tracing::error!("🚨 [CRITICAL] FEDERATION_SECRET must be set for Samsara Hub security!");
        std::process::exit(1);
    });
    let secret = secrecy::SecretString::from(secret_val);
    std::env::remove_var("FEDERATION_SECRET");
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
    let _mdns_daemon = mdns_listener::start_mdns_listener(agent_registry.clone())
        .map_err(|e| anyhow::anyhow!("mDNS listener failed to start: {}", e))?;

    let state = Arc::new(HubState {
        pool: pool.clone(),
        secret,
        auth_manager: {
            match std::env::var("JWT_PRIVATE_KEY_B64") {
                Ok(key_b64) => Arc::new(
                    shared::auth::JwtAuthManager::from_private_key_b64(&key_b64)
                        .map_err(|e| anyhow::anyhow!("JWT initialize failed: {}", e))?,
                ),
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
    });

    let token = CancellationToken::new();

    // Spawn the Approval Worker to process quarantine
    tokio::spawn(approval_worker(pool, token.clone()));

    let state_bg = state.clone();
    let token_bg = token.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Every hour
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    info!("♻️ [HubMaintenance] Running Maintenance...");
                    if let Some(sq) = state_bg.pool.get_sqlite_pool() {
                         let _ = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)").execute(sq).await;
                    }
                }
                _ = token_bg.cancelled() => break,
            }
        }
    });

    let app = build_app(state);

    let addr = format!("127.0.0.1:{}", port);
    info!("🏔️ Samsara Hub (The Validator) listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(token))
        .await?;

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

async fn init_hub_db(pool: &shared::db::DatabasePool) -> anyhow::Result<()> {
    match pool {
        shared::db::DatabasePool::Sqlite(p) => {
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS approved_karma (
                    id TEXT PRIMARY KEY,
                    node_id TEXT NOT NULL,
                    karma_type TEXT NOT NULL,
                    related_skill TEXT NOT NULL,
                    lesson TEXT NOT NULL,
                    weight INTEGER NOT NULL,
                    soul_version_hash TEXT,
                    lamport_clock INTEGER NOT NULL DEFAULT 0,
                    signature TEXT,
                    approved_at TEXT,
                    created_at TEXT NOT NULL,
                    clone_origin_id TEXT,
                    generation INTEGER,
                    somatic_valence REAL
                );",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS quarantined_karma (
                    id TEXT PRIMARY KEY,
                    node_id TEXT NOT NULL,
                    karma_type TEXT NOT NULL,
                    related_skill TEXT NOT NULL,
                    lesson TEXT NOT NULL,
                    weight INTEGER NOT NULL,
                    soul_version_hash TEXT,
                    lamport_clock INTEGER NOT NULL DEFAULT 0,
                    signature TEXT,
                    received_at TEXT,
                    created_at TEXT NOT NULL,
                    clone_origin_id TEXT,
                    generation INTEGER,
                    somatic_valence REAL
                );",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS approved_rules (
                    id TEXT PRIMARY KEY,
                    pattern TEXT NOT NULL,
                    severity INTEGER NOT NULL,
                    action TEXT NOT NULL,
                    node_id TEXT NOT NULL,
                    lamport_clock INTEGER NOT NULL DEFAULT 0,
                    signature TEXT,
                    approved_at TEXT,
                    created_at TEXT NOT NULL
                );",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS quarantined_rules (
                    id TEXT PRIMARY KEY,
                    node_id TEXT NOT NULL,
                    pattern TEXT NOT NULL,
                    severity INTEGER NOT NULL,
                    action TEXT NOT NULL,
                    lamport_clock INTEGER NOT NULL DEFAULT 0,
                    signature TEXT,
                    received_at TEXT,
                    created_at TEXT NOT NULL
                );",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE INDEX IF NOT EXISTS idx_approved_karma_at ON approved_karma(approved_at);",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE INDEX IF NOT EXISTS idx_approved_rules_at ON approved_rules(approved_at);",
            )
            .execute(p)
            .await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_q_karma_node_clock ON quarantined_karma(node_id, lamport_clock);").execute(p).await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_q_rules_node_clock ON quarantined_rules(node_id, lamport_clock);").execute(p).await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS approved_arena_matches (
                id TEXT PRIMARY KEY,
                skill_a TEXT NOT NULL,
                skill_b TEXT NOT NULL,
                topic TEXT NOT NULL,
                winner TEXT,
                reasoning TEXT,
                approved_at TEXT,
                created_at TEXT NOT NULL
            );",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS quarantined_arena_matches (
                id TEXT PRIMARY KEY,
                skill_a TEXT NOT NULL,
                skill_b TEXT NOT NULL,
                topic TEXT NOT NULL,
                winner TEXT,
                reasoning TEXT,
                received_at TEXT,
                created_at TEXT NOT NULL
            );",
            )
            .execute(p)
            .await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_approved_arena_at ON approved_arena_matches(approved_at);").execute(p).await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_a_karma_node_clock ON approved_karma(node_id, lamport_clock);").execute(p).await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_a_rules_node_clock ON approved_rules(node_id, lamport_clock);").execute(p).await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS node_reputation (
                node_id TEXT PRIMARY KEY,
                reputation_score INTEGER NOT NULL DEFAULT 100,
                is_banned INTEGER NOT NULL DEFAULT 0,
                last_seen_at TEXT NOT NULL
            );",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS biome_topics (
                topic_id TEXT PRIMARY KEY,
                peer_pubkey TEXT NOT NULL,
                summary TEXT,
                status TEXT NOT NULL DEFAULT 'Active',
                turn_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS biome_relay_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                recipient_pubkey TEXT NOT NULL,
                payload TEXT NOT NULL,
                is_delivered INTEGER NOT NULL DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now'))
            );",
            )
            .execute(p)
            .await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_biome_relay_recipient ON biome_relay_queue(recipient_pubkey) WHERE is_delivered = 0;").execute(p).await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS hub_timeline (
                id TEXT PRIMARY KEY,
                automerge_blob BLOB NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            );",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS timeline_snapshots (
                node_id TEXT PRIMARY KEY,
                snapshot_blob BLOB NOT NULL,
                received_at TEXT NOT NULL
            );",
            )
            .execute(p)
            .await?;
        }
        shared::db::DatabasePool::Postgres(p) => {
            // Postgres schema is primarily handled by PostgresInitializer, but we ensure basic Hub tables exist.
            // Note: PostgresInitializer::init_db is called earlier in main for other tables.
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS approved_karma (
                    id TEXT PRIMARY KEY,
                    node_id TEXT NOT NULL,
                    karma_type TEXT NOT NULL,
                    related_skill TEXT NOT NULL,
                    lesson TEXT NOT NULL,
                    weight INTEGER NOT NULL,
                    soul_version_hash TEXT,
                    lamport_clock BIGINT NOT NULL DEFAULT 0,
                    signature TEXT,
                    approved_at TIMESTAMP,
                    created_at TIMESTAMP NOT NULL,
                    clone_origin_id TEXT,
                    generation INTEGER,
                    somatic_valence REAL
                );",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS quarantined_karma (
                    id TEXT PRIMARY KEY,
                    node_id TEXT NOT NULL,
                    karma_type TEXT NOT NULL,
                    related_skill TEXT NOT NULL,
                    lesson TEXT NOT NULL,
                    weight INTEGER NOT NULL,
                    soul_version_hash TEXT,
                    lamport_clock BIGINT NOT NULL DEFAULT 0,
                    signature TEXT,
                    received_at TIMESTAMP,
                    created_at TIMESTAMP NOT NULL,
                    clone_origin_id TEXT,
                    generation INTEGER,
                    somatic_valence REAL
                );",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS approved_rules (
                    id TEXT PRIMARY KEY,
                    pattern TEXT NOT NULL,
                    severity INTEGER NOT NULL,
                    action TEXT NOT NULL,
                    node_id TEXT NOT NULL,
                    lamport_clock BIGINT NOT NULL DEFAULT 0,
                    signature TEXT,
                    approved_at TIMESTAMP,
                    created_at TIMESTAMP NOT NULL
                );",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS quarantined_rules (
                    id TEXT PRIMARY KEY,
                    node_id TEXT NOT NULL,
                    pattern TEXT NOT NULL,
                    severity INTEGER NOT NULL,
                    action TEXT NOT NULL,
                    lamport_clock BIGINT NOT NULL DEFAULT 0,
                    signature TEXT,
                    received_at TIMESTAMP,
                    created_at TIMESTAMP NOT NULL
                );",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE INDEX IF NOT EXISTS idx_approved_karma_at ON approved_karma(approved_at);",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE INDEX IF NOT EXISTS idx_approved_rules_at ON approved_rules(approved_at);",
            )
            .execute(p)
            .await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_q_karma_node_clock ON quarantined_karma(node_id, lamport_clock);").execute(p).await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_q_rules_node_clock ON quarantined_rules(node_id, lamport_clock);").execute(p).await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS approved_arena_matches (
                id TEXT PRIMARY KEY,
                skill_a TEXT NOT NULL,
                skill_b TEXT NOT NULL,
                topic TEXT NOT NULL,
                winner TEXT,
                reasoning TEXT,
                approved_at TIMESTAMP,
                created_at TIMESTAMP NOT NULL
            );",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS quarantined_arena_matches (
                id TEXT PRIMARY KEY,
                skill_a TEXT NOT NULL,
                skill_b TEXT NOT NULL,
                topic TEXT NOT NULL,
                winner TEXT,
                reasoning TEXT,
                received_at TIMESTAMP,
                created_at TIMESTAMP NOT NULL
            );",
            )
            .execute(p)
            .await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_approved_arena_at ON approved_arena_matches(approved_at);").execute(p).await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_a_karma_node_clock ON approved_karma(node_id, lamport_clock);").execute(p).await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_a_rules_node_clock ON approved_rules(node_id, lamport_clock);").execute(p).await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS node_reputation (
                node_id TEXT PRIMARY KEY,
                reputation_score INTEGER NOT NULL DEFAULT 100,
                is_banned INTEGER NOT NULL DEFAULT 0,
                last_seen_at TIMESTAMP NOT NULL
            );",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS biome_topics (
                topic_id TEXT PRIMARY KEY,
                peer_pubkey TEXT NOT NULL,
                summary TEXT,
                status TEXT NOT NULL DEFAULT 'Active',
                turn_count INTEGER NOT NULL DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );",
            )
            .execute(p)
            .await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS biome_relay_queue (
                id SERIAL PRIMARY KEY,
                recipient_pubkey TEXT NOT NULL,
                payload TEXT NOT NULL,
                is_delivered INTEGER NOT NULL DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );",
            )
            .execute(p)
            .await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_biome_relay_recipient ON biome_relay_queue(recipient_pubkey) WHERE is_delivered = 0;").execute(p).await?;
            sqlx::query("CREATE TABLE IF NOT EXISTS hub_timeline (id TEXT PRIMARY KEY, automerge_blob BYTEA NOT NULL, updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)").execute(p).await?;
            sqlx::query("CREATE TABLE IF NOT EXISTS federated_metrics (node_id TEXT NOT NULL, metrics_json TEXT NOT NULL, received_at TEXT NOT NULL, PRIMARY KEY (node_id, received_at))").execute(p).await?;
            sqlx::query("CREATE TABLE IF NOT EXISTS timeline_snapshots (node_id TEXT PRIMARY KEY, snapshot_blob BYTEA NOT NULL, received_at TEXT NOT NULL)").execute(p).await?;
        }
    }

    info!("✅ Hub Database initialized (Approved & Quarantine layers + BFT/Reputation & Biome).");
    Ok(())
}

async fn health_handler() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(serde_json::json!({"status": "healthy", "service": "samsara-hub"})),
    )
}

#[derive(serde::Deserialize)]
struct CreateTopicRequest {
    topic_id: String,
    peer_pubkey: String,
    summary: Option<String>,
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

fn verify_bearer(auth_header: &str, secret: &secrecy::SecretString) -> bool {
    use secrecy::ExposeSecret;
    use subtle::ConstantTimeEq;
    let expected = format!("Bearer {}", secret.expose_secret());
    // SEC: Always perform constant-time comparison regardless of length to prevent timing leaks
    let max_len = std::cmp::max(auth_header.len(), expected.len());
    let mut a = vec![0u8; max_len];
    let mut b = vec![0u8; max_len];
    a[..auth_header.len()].copy_from_slice(auth_header.as_bytes());
    b[..expected.len()].copy_from_slice(expected.as_bytes());
    auth_header.len() == expected.len() && bool::from(a.ct_eq(&b))
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
        "{}:{}:{}",
        msg.sender_pubkey, msg.topic_id, msg.lamport_clock
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

#[derive(serde::Deserialize)]
pub struct BiomeWsQuery {
    pub node_id: String,
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

async fn sync_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Json(payload): Json<FederationSyncRequest>,
) -> impl IntoResponse {
    // Auth Wall
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
        warn!(
            "🔒 Unauthorized sync attempt from node: {}",
            payload.node_id
        );
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized"})),
        )
            .into_response();
    }

    // BFT: BAN Check
    // BFT: BAN Check
    let ban_check_query = format!(
        "SELECT is_banned FROM node_reputation WHERE node_id = {}",
        state.pool.ph(0)
    );
    let is_banned =
        shared::sql_fetch_optional!(&state.pool, (bool,), &ban_check_query, &payload.node_id)
            .unwrap_or(Some((false,)))
            .unwrap_or((false,))
            .0;

    if is_banned {
        warn!(
            "🛡️ [BFT] Rejecting sync from BANNED node: {}",
            payload.node_id
        );
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Node is banned"})),
        )
            .into_response();
    }

    info!(
        "🌐 Node {} pulling approved updates since {:?}",
        payload.node_id, payload.since
    );

    let since = payload
        .since
        .unwrap_or_else(|| "1970-01-01T00:00:00".to_string());

    // Fetch ONLY approved data with Pagination (Flaw 2: OOM Defense)
    // Fetch approved AND quarantined data for synchronization (Phase 31 reliability)
    let karma_sync_query = format!(
        "SELECT id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, node_id, signature, clone_origin_id, generation, somatic_valence FROM (
            SELECT id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, node_id, signature, clone_origin_id, generation, somatic_valence, approved_at as ts FROM approved_karma
            UNION ALL
            SELECT id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, node_id, signature, clone_origin_id, generation, somatic_valence, received_at as ts FROM quarantined_karma
         ) WHERE ts > {} ORDER BY ts ASC LIMIT 500",
         state.pool.ph(0)
    );
    let karmas: Vec<FederatedKarmaRecord> =
        shared::sql_fetch_all!(&state.pool, FederatedKarmaRecord, &karma_sync_query, &since)
            .unwrap_or_default();
    let rule_sync_query = format!(
        "SELECT id, pattern, severity, action, created_at, lamport_clock, node_id, signature FROM approved_rules 
         WHERE approved_at > {} ORDER BY approved_at ASC LIMIT 500",
         state.pool.ph(0)
    );
    let rules: Vec<ImmuneRuleRecord> =
        shared::sql_fetch_all!(&state.pool, ImmuneRuleRecord, &rule_sync_query, &since)
            .unwrap_or_default();
    let has_more = karmas.len() == 500 || rules.len() == 500;

    let arena_sync_query = format!("SELECT id, skill_a, skill_b, topic, winner, reasoning, created_at FROM approved_arena_matches WHERE approved_at > {} ORDER BY approved_at ASC LIMIT 500", state.pool.ph(0));
    let arena_rows: Vec<ArenaMatchRecord> =
        shared::sql_fetch_all!(&state.pool, ArenaMatchRecord, &arena_sync_query, &since)
            .unwrap_or_default();

    // Fetch latest Automerge Snapshot for this node if it exists
    let snapshot_query = format!(
        "SELECT snapshot_blob FROM timeline_snapshots WHERE node_id = {}",
        state.pool.ph(0)
    );
    let snapshot_blob: Option<Vec<u8>> =
        shared::sql_fetch_optional!(&state.pool, (Vec<u8>,), &snapshot_query, &payload.node_id)
            .unwrap_or_else(|_| Some((Vec::new(),)))
            .map(|t| t.0);

    let _next_cursor: Option<String> = if has_more {
        // Find the latest approved_at for pagination (Keyset Pagination)
        // For simplicity, we just use the last item's timestamp if we hit the limit
        // In a real high-perf system, we'd query for the max timestamp in the results.
        None // Placeholder: will be refined if needed, but since is enough for now.
    } else {
        None
    };

    let response = FederationSyncResponse {
        new_karmas: karmas
            .into_iter()
            .map(|k| FederatedKarma {
                id: k.id,
                job_id: None,
                karma_type: k.karma_type,
                related_skill: k.related_skill,
                lesson: k.lesson,
                weight: k.weight as i32,
                last_applied_at: Some(k.created_at.clone()),
                created_at: k.created_at,
                soul_version_hash: k.soul_version_hash,
                lamport_clock: k.lamport_clock as u64,
                node_id: k.node_id,
                signature: k.signature,
                clone_origin_id: k.clone_origin_id,
                generation: k.generation.map(|g| g as u32),
                somatic_valence: k.somatic_valence,
                score: 0.0,
            })
            .collect(),
        new_immune_rules: rules
            .into_iter()
            .map(|r| ImmuneRule {
                id: r.id,
                pattern: r.pattern,
                severity: r.severity as u8,
                action: r.action,
                created_at: r.created_at,
                approval_status: ApprovalState::Approved,
                input_constraints: None,
                lamport_clock: r.lamport_clock as u64,
                node_id: r.node_id,
                signature: r.signature,
            })
            .collect(),
        new_arena_matches: arena_rows
            .into_iter()
            .map(|a| aiome_core::contracts::ArenaMatch {
                id: a.id,
                skill_a: a.skill_a,
                skill_b: a.skill_b,
                topic: a.topic,
                winner: Some(a.winner),
                reasoning: a.reasoning,
                created_at: a.created_at,
            })
            .collect(),
        server_time: chrono::Utc::now().to_rfc3339(),
        next_cursor: None,
        has_more,
        automerge_snapshot: snapshot_blob,
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn push_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Json(mut payload): Json<FederationPushRequest>,
) -> impl IntoResponse {
    // 🛡️ [GlassWorm Shield] Sanitize all inbound text fields to prevent Federation Worm Attack
    payload.node_id = shared::guardrails::strip_invisible_unicode(&payload.node_id).into_owned();
    for k in &mut payload.karmas {
        k.id = shared::guardrails::strip_invisible_unicode(&k.id).into_owned();
        k.job_id = k
            .job_id
            .take()
            .map(|s| shared::guardrails::strip_invisible_unicode(&s).into_owned());
        k.karma_type = shared::guardrails::strip_invisible_unicode(&k.karma_type).into_owned();
        k.lesson = shared::guardrails::strip_invisible_unicode(&k.lesson).into_owned();
        k.related_skill =
            shared::guardrails::strip_invisible_unicode(&k.related_skill).into_owned();
        k.node_id = shared::guardrails::strip_invisible_unicode(&k.node_id).into_owned();
        k.signature = k
            .signature
            .take()
            .map(|s| shared::guardrails::strip_invisible_unicode(&s).into_owned());
        k.clone_origin_id = k
            .clone_origin_id
            .take()
            .map(|s| shared::guardrails::strip_invisible_unicode(&s).into_owned());
    }
    for r in &mut payload.rules {
        r.id = shared::guardrails::strip_invisible_unicode(&r.id).into_owned();
        r.pattern = shared::guardrails::strip_invisible_unicode(&r.pattern).into_owned();
        r.action = shared::guardrails::strip_invisible_unicode(&r.action).into_owned();
        r.node_id = shared::guardrails::strip_invisible_unicode(&r.node_id).into_owned();
        r.signature = r
            .signature
            .take()
            .map(|s| shared::guardrails::strip_invisible_unicode(&s).into_owned());
    }
    for m in &mut payload.arena_matches {
        m.id = shared::guardrails::strip_invisible_unicode(&m.id).into_owned();
        m.skill_a = shared::guardrails::strip_invisible_unicode(&m.skill_a).into_owned();
        m.skill_b = shared::guardrails::strip_invisible_unicode(&m.skill_b).into_owned();
        m.topic = shared::guardrails::strip_invisible_unicode(&m.topic).into_owned();
        m.winner = m
            .winner
            .take()
            .map(|s| shared::guardrails::strip_invisible_unicode(&s).into_owned());
        m.reasoning = shared::guardrails::strip_invisible_unicode(&m.reasoning).into_owned();
    }

    // Auth Wall
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
        warn!(
            "🔒 Unauthorized push attempt from node: {}",
            payload.node_id
        );
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized"})),
        )
            .into_response();
    }

    // BFT: BAN Check
    let ban_check_query = format!(
        "SELECT is_banned FROM node_reputation WHERE node_id = {}",
        state.pool.ph(0)
    );
    let is_banned =
        shared::sql_fetch_optional!(&state.pool, (bool,), &ban_check_query, &payload.node_id)
            .unwrap_or(Some((false,)))
            .unwrap_or((false,))
            .0;

    if is_banned {
        warn!(
            "🛡️ [BFT] Rejecting push from BANNED node: {}",
            payload.node_id
        );
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Node is banned"})),
        )
            .into_response();
    }

    let karma_count = payload.karmas.len();
    let rule_count = payload.rules.len();
    info!(
        "📥 Received push from node {}: {} Karmas, {} Rules. Sending to Quarantine.",
        payload.node_id, karma_count, rule_count
    );

    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal error"})),
            )
                .into_response()
        }
    };

    let received_at_dt = chrono::Utc::now();
    for k in &payload.karmas {
        // BFT: Equivocation Check (Double-Signing)
        let equiv_check_query = format!(
            "SELECT COUNT(*) FROM (
                SELECT id FROM approved_karma WHERE node_id = {} AND lamport_clock = {} AND (lesson != {} OR weight != {})
                UNION ALL
                SELECT id FROM quarantined_karma WHERE node_id = {} AND lamport_clock = {} AND (lesson != {} OR weight != {})
             ) AS equiv LIMIT 1",
             state.pool.ph(0), state.pool.ph(1), state.pool.ph(2), state.pool.ph(3),
             state.pool.ph(4), state.pool.ph(5), state.pool.ph(6), state.pool.ph(7)
        );

        let equiv_exists = shared::sql_fetch_optional!(
            &state.pool,
            (i64,),
            &equiv_check_query,
            &k.node_id,
            &(k.lamport_clock as i64),
            &k.lesson,
            &(k.weight as i64),
            &k.node_id,
            &(k.lamport_clock as i64),
            &k.lesson,
            &(k.weight as i64)
        )
        .unwrap_or(Some((0,)))
        .unwrap_or((0,))
        .0 > 0;
        if equiv_exists {
            warn!(
                "🛡️ [BFT] EQUIVOCATION detected from node: {}. Slashing node.",
                k.node_id
            );
            let slash_query = format!("UPDATE node_reputation SET is_banned = 1, reputation_score = -1000 WHERE node_id = {}", state.pool.ph(0));
            let _ = shared::sql_exec!(&state.pool, &slash_query, &k.node_id);
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "Equivocation detected"})),
            )
                .into_response();
        }

        let quarantine_karma_query = format!(
            "INSERT INTO quarantined_karma (id, node_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, signature, received_at, clone_origin_id, generation, somatic_valence)
             VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})
             ON CONFLICT(id) DO NOTHING "
,
             state.pool.ph(0), state.pool.ph(1), state.pool.ph(2), state.pool.ph(3), state.pool.ph(4),
             state.pool.ph(5), state.pool.ph(6), state.pool.ph(7), state.pool.ph(8), state.pool.ph(9),
             state.pool.ph(10), state.pool.ph(11), state.pool.ph(12), state.pool.ph(13)
        );
        let res = match &mut tx {
            shared::db::DatabaseTransaction::Sqlite(ref mut t) => {
                sqlx::query::<sqlx::Sqlite>(&quarantine_karma_query)
                    .bind(&k.id)
                    .bind(&k.node_id)
                    .bind(&k.karma_type)
                    .bind(&k.related_skill)
                    .bind(&k.lesson)
                    .bind(k.weight as i64)
                    .bind(&k.soul_version_hash)
                    .bind(&k.created_at)
                    .bind(k.lamport_clock as i64)
                    .bind(&k.signature)
                    .bind(&received_at_dt)
                    .bind(&k.clone_origin_id)
                    .bind(k.generation.map(|v| v as i64))
                    .bind(k.somatic_valence)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
            shared::db::DatabaseTransaction::Postgres(ref mut t) => {
                sqlx::query::<sqlx::Postgres>(&quarantine_karma_query)
                    .bind(&k.id)
                    .bind(&k.node_id)
                    .bind(&k.karma_type)
                    .bind(&k.related_skill)
                    .bind(&k.lesson)
                    .bind(k.weight as i64)
                    .bind(&k.soul_version_hash)
                    .bind(&k.created_at)
                    .bind(k.lamport_clock as i64)
                    .bind(&k.signature)
                    .bind(&received_at_dt)
                    .bind(&k.clone_origin_id)
                    .bind(k.generation.map(|v| v as i64))
                    .bind(k.somatic_valence)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
        };
        if let Err(e) = res {
            warn!("🛡️ [Push] Failed to quarantine karma {}: {}", k.id, e);
        }
    }

    for r in &payload.rules {
        // BFT: Equivocation Check (Double-Signing) for Rules
        let equiv_check_rule_query = format!(
            "SELECT COUNT(*) FROM (
                SELECT id FROM approved_rules WHERE node_id = {} AND lamport_clock = {} AND (pattern != {} OR severity != {} OR action != {})
                UNION ALL
                SELECT id FROM quarantined_rules WHERE node_id = {} AND lamport_clock = {} AND (pattern != {} OR severity != {} OR action != {})
             ) AS equiv LIMIT 1",
             state.pool.ph(0), state.pool.ph(1), state.pool.ph(2), state.pool.ph(3), state.pool.ph(4),
             state.pool.ph(5), state.pool.ph(6), state.pool.ph(7), state.pool.ph(8), state.pool.ph(9)
        );
        let exists = shared::sql_fetch_optional!(
            &state.pool,
            (i64,),
            &equiv_check_rule_query,
            &r.node_id,
            &(r.lamport_clock as i64),
            &r.pattern,
            &(r.severity as i64),
            &r.action,
            &r.node_id,
            &(r.lamport_clock as i64),
            &r.pattern,
            &(r.severity as i64),
            &r.action
        )
        .unwrap_or(Some((0,)))
        .unwrap_or((0,))
        .0;
        if exists > 0 {
            warn!(
                "🛡️ [BFT] EQUIVOCATION detected in RULE from node: {}. Slashing node.",
                r.node_id
            );
            let ban_query = format!("UPDATE node_reputation SET is_banned = 1, reputation_score = -1000 WHERE node_id = {}", state.pool.ph(0));
            let _ = shared::sql_exec!(&state.pool, &ban_query, &r.node_id);
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "Equivocation detected"})),
            )
                .into_response();
        }

        let quarantine_rule_query = format!(
            "INSERT INTO quarantined_rules (id, node_id, pattern, severity, action, created_at, lamport_clock, signature, received_at)
             VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {})
             ON CONFLICT(id) DO NOTHING ",
             state.pool.ph(0), state.pool.ph(1), state.pool.ph(2), state.pool.ph(3), state.pool.ph(4),
             state.pool.ph(5), state.pool.ph(6), state.pool.ph(7), state.pool.ph(8)
        );
        let res = match &mut tx {
            shared::db::DatabaseTransaction::Sqlite(t) => {
                sqlx::query::<sqlx::Sqlite>(&quarantine_rule_query)
                    .bind(&r.id)
                    .bind(&r.node_id)
                    .bind(&r.pattern)
                    .bind(r.severity as i64)
                    .bind(&r.action)
                    .bind(&r.created_at)
                    .bind(r.lamport_clock as i64)
                    .bind(&r.signature)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
            shared::db::DatabaseTransaction::Postgres(t) => {
                sqlx::query::<sqlx::Postgres>(&quarantine_rule_query)
                    .bind(&r.id)
                    .bind(&r.node_id)
                    .bind(&r.pattern)
                    .bind(r.severity as i64)
                    .bind(&r.action)
                    .bind(&r.created_at)
                    .bind(r.lamport_clock as i64)
                    .bind(&r.signature)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
        };
        if let Err(e) = res {
            warn!("🛡️ [Push] Failed to quarantine rule {}: {}", r.id, e);
        }
    }

    for a in &payload.arena_matches {
        let quarantine_arena_query = format!(
            "INSERT INTO quarantined_arena_matches (id, skill_a, skill_b, topic, winner, reasoning, created_at, received_at)
             VALUES ({}, {}, {}, {}, {}, {}, {}, {})
             ON CONFLICT(id) DO NOTHING ",
             state.pool.ph(0), state.pool.ph(1), state.pool.ph(2), state.pool.ph(3), state.pool.ph(4),
             state.pool.ph(5), state.pool.ph(6), state.pool.ph(7)
        );
        let res = match &mut tx {
            shared::db::DatabaseTransaction::Sqlite(t) => {
                sqlx::query::<sqlx::Sqlite>(&quarantine_arena_query)
                    .bind(&a.id)
                    .bind(&a.skill_a)
                    .bind(&a.skill_b)
                    .bind(&a.topic)
                    .bind(&a.winner)
                    .bind(&a.reasoning)
                    .bind(&a.created_at)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
            shared::db::DatabaseTransaction::Postgres(t) => {
                sqlx::query::<sqlx::Postgres>(&quarantine_arena_query)
                    .bind(&a.id)
                    .bind(&a.skill_a)
                    .bind(&a.skill_b)
                    .bind(&a.topic)
                    .bind(&a.winner)
                    .bind(&a.reasoning)
                    .bind(&a.created_at)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
        };
        if let Err(e) = res {
            warn!("🛡️ [Push] Failed to quarantine arena match {}: {}", a.id, e);
        }
    }

    // Store Automerge Snapshot (Binary Timeline)
    if let Some(snapshot) = &payload.automerge_snapshot {
        let snapshot_query = format!(
            "INSERT INTO timeline_snapshots (node_id, snapshot_blob, received_at) VALUES ({}, {}, {})
             ON CONFLICT(node_id) DO UPDATE SET snapshot_blob = excluded.snapshot_blob, received_at = excluded.received_at",
            state.pool.ph(0), state.pool.ph(1), state.pool.ph(2)
        );
        let res = match &mut tx {
            shared::db::DatabaseTransaction::Sqlite(t) => {
                sqlx::query::<sqlx::Sqlite>(&snapshot_query)
                    .bind(&payload.node_id)
                    .bind(snapshot)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
            shared::db::DatabaseTransaction::Postgres(t) => {
                sqlx::query::<sqlx::Postgres>(&snapshot_query)
                    .bind(&payload.node_id)
                    .bind(snapshot)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
        };
        if let Err(e) = res {
            warn!(
                "🛡️ [Push] Failed to store timeline snapshot from {}: {}",
                payload.node_id, e
            );
        }
    }

    if let Some(metrics) = &payload.metrics {
        let metrics_json = serde_json::to_string(metrics).unwrap_or_default();
        let metrics_query = format!(
            "INSERT INTO federated_metrics (node_id, metrics_json, received_at) VALUES ({}, {}, {})",
            state.pool.ph(0), state.pool.ph(1), state.pool.ph(2)
        );
        let res = match &mut tx {
            shared::db::DatabaseTransaction::Sqlite(t) => {
                sqlx::query::<sqlx::Sqlite>(&metrics_query)
                    .bind(&payload.node_id)
                    .bind(&metrics_json)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
            shared::db::DatabaseTransaction::Postgres(t) => {
                sqlx::query::<sqlx::Postgres>(&metrics_query)
                    .bind(&payload.node_id)
                    .bind(&metrics_json)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
        };
        if let Err(e) = res {
            warn!(
                "🛡️ [Push] Failed to store federated metrics from {}: {}",
                payload.node_id, e
            );
        }
    }

    if let Err(e) = tx.commit().await {
        error!("❌ Push commit failed: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Internal error"})),
        )
            .into_response();
    }

    let arenas_count = payload.arena_matches.len();

    // BFT: Update reputation / last_seen
    let reputation_query = format!(
        "INSERT INTO node_reputation (node_id, last_seen_at) VALUES ({}, {})
         ON CONFLICT(node_id) DO UPDATE SET last_seen_at = excluded.last_seen_at, reputation_score = node_reputation.reputation_score + 1",
         state.pool.ph(0), state.pool.ph(1)
    );
    let res = shared::sql_exec!(
        &state.pool,
        &reputation_query,
        &payload.node_id,
        &received_at_dt
    );
    if let Err(e) = res {
        warn!(
            "🛡️ [Push] Failed to update node reputation for {}: {}",
            payload.node_id, e
        );
    }

    // 📣 Real-time Broadcast to all connected nodes (Relay Sync)
    for r in &payload.rules {
        let _ = state.tx.send(HubMessage::NewImmuneRule(r.clone()));
    }
    for k in &payload.karmas {
        let _ = state.tx.send(HubMessage::NewKarma(k.clone()));
    }

    (
        StatusCode::OK,
        Json(FederationPushResponse {
            accepted_count: karma_count + rule_count + arenas_count,
            message: "Data received and placed in quarantine for validation. ".to_string(),
        }),
    )
        .into_response()
}

async fn ws_handler(
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    State(state): State<Arc<HubState>>,
) -> impl IntoResponse {
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
        warn!("🔒 Unauthorized WS upgrade attempt ");
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<HubState>) {
    use aiome_core::contracts::HubMessage;

    // TCP Exhaustion Defense (Max Connections)
    let current_conn = state
        .active_connections
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    if current_conn >= 1000 {
        warn!("🛡️ [BFT] Hub reached max WebSocket connections (1000). Rejecting new node.");
        state
            .active_connections
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        let _ = socket.send(Message::Close(None)).await;
        return;
    }

    info!(
        "🔌 Authorized node connected via WebSocket (Total: {})",
        current_conn + 1
    );

    let mut rx = state.tx.subscribe();
    let mut keepalive_timer = tokio::time::interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = keepalive_timer.tick() => {
                // Ping-Pong keepalive (Flaw 9)
                if socket.send(Message::Ping(Vec::new())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        info!("🔌 Node disconnected ");
                        break;
                    }
                    Some(Ok(Message::Text(text))) => {
                        // Handle Ping from client (Flaw 9)
                        if let Ok(HubMessage::Ping { client_time: _ }) = serde_json::from_str::<HubMessage>(&text) {
                            let pong = HubMessage::Pong { server_time: chrono::Utc::now().to_rfc3339() };
                            if let Ok(pong_text) = serde_json::to_string(&pong) {
                                let _ = socket.send(Message::Text(pong_text)).await;
                            }
                        }
                    }
                    _ => {}
                }
            }
            res = rx.recv() => {
                match res {
                    Ok(hub_msg) => {
                        if let Ok(text) = serde_json::to_string(&hub_msg) {
                            if socket.send(Message::Text(text)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("⚠️ WS Client lagged by {} messages. Triggering Catch-up Sync.", n);
                        let hub_msg = HubMessage::LaggedForceSync {
                            server_time: chrono::Utc::now().to_rfc3339()
                        };
                        if let Ok(text) = serde_json::to_string(&hub_msg) {
                            let _ = socket.send(Message::Text(text)).await;
                        }
                        // Continue loop, client will sync via REST
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
    state
        .active_connections
        .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
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

#[derive(serde::Deserialize)]
pub struct TimelineSyncRequest {
    pub hub_id: String,
    pub automerge_blob: Vec<u8>,
}

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
            if claims.agent_id != uuid::Uuid::nil() {
                authenticated = true;
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
    let origins_env = std::env::var("ALLOWED_ORIGINS").unwrap_or_default();
    let mut allowed_origins = vec![];

    // Add defaults
    let defaults = [
        "http://localhost:3000", // allow-anti-pattern
        "http://127.0.0.1:3000", // allow-anti-pattern
        "http://localhost:3015", // allow-anti-pattern
        "http://localhost:3016", // allow-anti-pattern
        "http://localhost:1420", // allow-anti-pattern
    ];
    for d in defaults {
        if let Ok(parsed) = d.parse() {
            allowed_origins.push(parsed);
        }
    }

    if !origins_env.is_empty() {
        for extra in origins_env.split(',') {
            if let Ok(parsed) = extra.trim().parse() {
                allowed_origins.push(parsed);
            }
        }
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
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled internal error: {}", err),
                    )
                }))
                .layer(BufferLayer::new(2048))
                .layer(RateLimitLayer::new(600, Duration::from_secs(60))), // High frequency for Biome
        )
        .with_state(state)
}

mod hub_reliability_tests;
#[cfg(test)]
mod hub_ws_tests;
