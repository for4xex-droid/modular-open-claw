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

pub async fn init_database(preflight: &PreflightResult) -> anyhow::Result<DatabaseResult> {
    let config = &preflight.config;
    let resolver = &preflight.resolver;
    let plugin_registry = &preflight.plugin_registry;

    // === 🏗️ STAGE 2/7: Database ===
    let ts_pool = if config.db_path.starts_with("postgres://")
        || config.db_path.starts_with("postgresql://")
    {
        let pg = sqlx::PgPool::connect(&config.db_path)
            .await
            .map_err(|e| anyhow::anyhow!("🚨 Failed to connect TS pool: {}", e))?;
        infrastructure::db::DatabasePool::Postgres(pg)
    } else {
        use std::str::FromStr;
        let opts = sqlx::sqlite::SqliteConnectOptions::from_str(&config.db_path)
            .map_err(|e| anyhow::anyhow!("🚨 Invalid DB path for TS: {}", e))?
            .create_if_missing(true);
        let sq = sqlx::sqlite::SqlitePoolOptions::new()
            .connect_with(opts)
            .await
            .map_err(|e| anyhow::anyhow!("🚨 Failed to connect TS pool: {}", e))?;
        infrastructure::db::DatabasePool::Sqlite(sq)
    };

    let db_pool = ts_pool.clone();

    let trajectory_store: Arc<dyn aiome_core::trajectory::TrajectoryStore> = {
        Arc::new(infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(ts_pool))
    };

    // 🛡️ Pre-migration automatic backup (Sinking Ship Audit #19)
    backup_sqlite_db_before_migration(&config.db_path);

    let job_queue =
        infrastructure::job_queue::UniversalJobQueue::new(db_pool.clone(), None, trajectory_store)
            .await
            .unwrap_or_else(|e| {
                error!("🚨 Failed to init DB at {}: {}", config.db_path, e);
                std::process::exit(1);
            });
    let job_queue = Arc::new(job_queue);

    let eval_logger = Arc::new(
        infrastructure::llm::evaluation_logger::EvaluationLogger::new(Arc::new(
            infrastructure::llm::evaluation_logger::SqlEvalLogRepository::new(db_pool.clone()),
        )),
    );

    use infrastructure::audit_logger::AsyncAuditLogger;
    let audit_logger = Arc::new(AsyncAuditLogger::new(
        Arc::new(db_pool.clone()),
        10000, // Process up to 10k items in memory before blocking
    ));

    let system_agent_id = job_queue
        .get_system_agent_id()
        .await
        .unwrap_or_else(|_| uuid::Uuid::nil());

    let circuit_breaker = Arc::new(infrastructure::circuit_breaker::CircuitBreaker::new(
        "api-server",
        infrastructure::circuit_breaker::CircuitBreakerConfig {
            failure_threshold: 5,
            reset_timeout: std::time::Duration::from_secs(60),
        },
    ));

    // G-2: Per-Agent Rate Limiter (60 requests per minute)
    let rate_limiter = infrastructure::rate_limiter::AgentRateLimiter::new(60)?;

    let slo_engine = Arc::new(infrastructure::slo_engine::SloEngine::new(
        infrastructure::slo_engine::SloConfig {
            error_budget_max: 100,
            warning_threshold: 80,
        },
        chrono::Duration::hours(24),
    ));

    let http_client = aiome_core::http::get_http_client().clone();

    let sandbox = Arc::new(
        shared::sandbox::PathSandbox::new(resolver.root())
            .map_err(|e| anyhow::anyhow!("🚨 Failed to initialize PathSandbox: {}", e))?,
    );

    let mut hook_manager = infrastructure::security::hook_manager::HookManager::new();
    let behavior_monitor = infrastructure::security::behavior_monitor::BehaviorMonitor::new(
        job_queue.clone(),
        sandbox.clone(),
        None, // Global system limit
        100,
    );
    hook_manager.add_hook(Arc::new(behavior_monitor));

    // Register Hermes LoopDetectorHook
    let loop_detector = infrastructure::security::LoopDetectorHook::default();
    hook_manager.add_hook(Arc::new(loop_detector));
    // NOTE: Plugin agent hooks are registered here.
    // Plugins MUST be registered via `plugin_registry.register()` BEFORE this point
    // for their hooks to be included. Currently Nurture connects OOP via API,
    // so this will be empty until in-process plugin loading is implemented.
    for hook in plugin_registry.get_agent_hooks() {
        hook_manager.add_hook(hook);
    }
    let hook_manager = Arc::new(hook_manager);

    Ok(DatabaseResult {
        db_pool,
        job_queue,
        eval_logger,
        audit_logger,
        system_agent_id,
        circuit_breaker,
        rate_limiter,
        slo_engine,
        http_client,
        sandbox,
        hook_manager,
    })
}
