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

use aiome_core::expression::tts_worker::TtsWorker;
use aiome_core::traits::JobQueue;
use shared::health::HealthMonitor;

pub struct PreflightResult {
    pub resolver: shared::app_data::AppDataResolver,
    pub config: Arc<shared::config::AiomeConfig>,
    pub cancel_token: CancellationToken,
    pub metrics_handle: metrics_exporter_prometheus::PrometheusHandle,
    pub plugin_registry: crate::plugin_loader::PluginRegistry,
    pub secrets: BootSecrets,
    pub health_monitor: Arc<Mutex<HealthMonitor>>,
    pub live_manager: Option<Arc<dyn aiome_core_contracts::traits::LiveSessionManager>>,
    pub db_url: String,
}

pub struct BootSecrets {
    pub stripe_key: Option<String>,
    pub nurture_secret: Option<String>,
    pub search_key: Option<String>,
    pub x_token: Option<String>,
    pub tts_openai_key: Option<String>,
}

pub struct DatabaseResult {
    pub db_pool: infrastructure::db::DatabasePool,
    pub job_queue: Arc<infrastructure::job_queue::UniversalJobQueue>,
    pub eval_logger: Arc<infrastructure::llm::evaluation_logger::EvaluationLogger>,
    pub audit_logger: Arc<infrastructure::audit_logger::AsyncAuditLogger>,
    pub system_agent_id: uuid::Uuid,
    pub circuit_breaker: Arc<infrastructure::circuit_breaker::CircuitBreaker>,
    pub rate_limiter: infrastructure::rate_limiter::AgentRateLimiter,
    pub slo_engine: Arc<infrastructure::slo_engine::SloEngine>,
    pub http_client: reqwest::Client,
    pub sandbox: Arc<shared::sandbox::PathSandbox>,
    pub hook_manager: Arc<infrastructure::security::hook_manager::HookManager>,
}

pub struct ProviderResult {
    pub provider: Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>,
    pub bg_provider: Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>,
    pub embed_provider: Arc<dyn aiome_core::llm_provider::EmbeddingProvider>,
}

pub struct CoreServicesResult {
    pub artifact_store: Arc<dyn aiome_core::traits::ArtifactStore>,
    pub event_sender: tokio::sync::broadcast::Sender<aiome_core_contracts::events::CoreEvent>,
    pub llm_semaphore: Arc<tokio::sync::Semaphore>,
    pub forge_semaphore: Arc<tokio::sync::Semaphore>,
    pub compute_semaphore: Arc<tokio::sync::Semaphore>,
    pub formal_proof_gate: Arc<dyn aiome_contracts::proof::FormalProofGate>,
    pub wasm_skill_manager: Arc<infrastructure::skills::WasmSkillManager>,
    pub skill_forge: Arc<infrastructure::skills::forge::SkillForge>,
    pub commerce_engine: Option<Arc<dyn aiome_core_contracts::commerce::CommerceEngine>>,
    pub api_server_secret: Arc<secrecy::SecretString>,
    pub federation_secret: Option<Arc<secrecy::SecretString>>,
    pub gift_engine: Arc<dyn aiome_core_contracts::commerce::GiftEngine>,
    pub ekyc_engine: Arc<dyn aiome_core_contracts::ekyc::EkycEngine>,
    pub ekyc_session_store: Arc<dyn aiome_core_contracts::ekyc::EkycSessionStore>,
    pub quarantine_store: Arc<dyn infrastructure::compliance::quarantine::QuarantineStore>,
    pub auth_manager: Arc<dyn infrastructure::auth::AuthManager>,
    pub soul_store: Arc<infrastructure::soul_store::UniversalSoulStore>,
    pub soul_pipeline: Arc<
        soul::pipeline::SoulPipeline<
            infrastructure::soul_adapter::CoreDomainAdapter,
            infrastructure::samsara_engine::DefaultSamsaraEngine,
        >,
    >,
    pub soul_mutator: Arc<infrastructure::soul_mutator::SoulMutator>,
    pub intent_generator: Arc<infrastructure::intent::IntentGenerator>,
    pub intent_firewall: Arc<infrastructure::intent::IntentFirewall>,
    pub context_engine: Arc<infrastructure::context_engine::ContextEngine>,
    pub belief_gate: Arc<infrastructure::belief_consistency_gate::BeliefConsistencyGate>,
    pub router_provider: Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>,
    pub autonomous_running: Arc<std::sync::atomic::AtomicBool>,
    pub autonomous_config:
        Arc<tokio::sync::RwLock<Option<aiome_core::biome::autonomous::AutonomousConfig>>>,
    pub docker_failures: Arc<tokio::sync::RwLock<std::collections::HashMap<String, u32>>>,
    pub mcp_manager: Arc<mcp::client::McpProcessManager>,
    pub gig_engine: Arc<dyn aiome_core_contracts::gig::GigEngine>,
    pub voice_drm: Arc<infrastructure::security::VoiceCoreDrm>,
    pub registry: Arc<infrastructure::registry::RegistryManager>,
    pub transcription_engine: Arc<dyn aiome_core_contracts::traits::TranscriptionEngine>,
    pub task_dispatcher: Arc<infrastructure::task_orchestrator::TaskDispatcher>,
    pub rlm_client: Arc<dyn aiome_core_contracts::rlm::RlmProvider>,
    pub cortex_projector: Arc<infrastructure::cortex_file_projector::CortexFileProjector>,
    pub a2a_client: Arc<dyn aiome_core_contracts::a2a::A2aClient>,
    pub disk_quota_mgr: Arc<infrastructure::disk_quota::DiskQuotaManager>,
    pub quality_gate_store: Arc<dyn infrastructure::quality_gate_store::QualityGateStore>,
    pub publish_pipeline: Arc<infrastructure::publisher::PublishPipeline>,
    pub api_server_secret_raw: String,
    pub stripe_key_raw: Option<String>,
    pub tts_openai_api_key_raw: Option<String>,
    pub vault_backend: Arc<dyn aiome_core_contracts::vault_backend::VaultBackend>,
    pub prompt_registry: Arc<dyn infrastructure::prompt_registry::PromptRegistry>,
    pub spec_provider: Arc<dyn infrastructure::spec_provider::SpecProvider>,
    pub tokens_css: String,
}

pub struct BootContext {
    pub state: crate::app_state::AppState,
    pub plugin_registry: crate::plugin_loader::PluginRegistry,
    pub metrics_handle: metrics_exporter_prometheus::PrometheusHandle,
    pub cancel_token: tokio_util::sync::CancellationToken,
    pub cors_layer: tower_http::cors::CorsLayer,
}

pub async fn boot_sequence() -> anyhow::Result<BootContext> {
    let oxilean_power = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

    let preflight = init_env_and_preflight().await?;
    let db_result = init_database(&preflight).await?;
    let llm_result = init_llm_providers(
        &preflight.config,
        &db_result,
        preflight.live_manager.clone(),
    )
    .await?;
    let core_result =
        init_core_services(&preflight, &db_result, &llm_result, oxilean_power.clone()).await?;

    let state = assemble_app_state(
        &preflight,
        &db_result,
        &llm_result,
        &core_result,
        oxilean_power,
    )
    .await?;

    spawn_background_workers(
        &state,
        &core_result.belief_gate,
        preflight.cancel_token.clone(),
    )
    .await?;

    let cors_layer = init_cors()?;

    Ok(BootContext {
        state,
        plugin_registry: preflight.plugin_registry,
        metrics_handle: preflight.metrics_handle,
        cancel_token: preflight.cancel_token,
        cors_layer,
    })
}

pub fn init_cors() -> anyhow::Result<CorsLayer> {
    use axum::http::HeaderValue;

    let mut layer = CorsLayer::new()
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    match std::env::var("ALLOWED_ORIGINS") {
        Ok(origins) if !origins.is_empty() => {
            let list: Vec<HeaderValue> = origins
                .split(',')
                .map(|s| {
                    HeaderValue::from_str(s.trim()).map_err(|e| {
                        anyhow::anyhow!("🚨 Invalid origin in ALLOWED_ORIGINS '{}': {}", s, e)
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            layer = layer.allow_origin(AllowOrigin::list(list));
            info!("🌐 [CORS] Allowed origins: {}", origins);
        }
        _ => {
            #[cfg(debug_assertions)]
            {
                warn!("⚠️ [CORS] ALLOWED_ORIGINS not set. All origins allowed in dev mode.");
                layer = layer.allow_origin(AllowOrigin::any());
            }
            #[cfg(not(debug_assertions))]
            {
                error!("🚨 [FATAL SECURITY ERROR] ALLOWED_ORIGINS MUST be set in production!");
                std::process::exit(1);
            }
        }
    }

    Ok(layer)
}

/// 本番環境で API_SERVER_SECRET がセキュアな要件を満たしているか判定する
pub fn is_secure_production_secret(secret: &str) -> bool {
    let s = secret.trim();
    if s.len() < 16 {
        return false;
    }

    let blocklist = [
        "dev_secret_donotuseinprod",
        "dev_secret_change_me_immediately",
        "quickstart_secret_change_in_production",
        "mock_valid_token_tester",
    ];
    !blocklist.contains(&s)
}

pub async fn init_env_and_preflight() -> anyhow::Result<PreflightResult> {
    // 1. Initial attempt from CWD (essential for dev environments to catch AIOME_DEV_MODE)
    dotenvy::dotenv().ok();

    let resolver = shared::app_data::AppDataResolver::new();

    // 2. Explicit attempt from application root (essential for Production Tauri sidecars)
    let app_env_path = resolver.root().join(".env");
    if app_env_path.exists() && dotenvy::from_path(&app_env_path).is_ok() {
        tracing::info!(
            "Loaded explicit environment from {}",
            app_env_path.display()
        );
    }

    // 0. Initialize Metrics EXPORTER (Q-5)
    let metrics_handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()
        .map_err(|e| anyhow::anyhow!("Failed to install Prometheus recorder: {}", e))?;
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
pub async fn init_llm_providers(
    config: &Arc<shared::config::AiomeConfig>,
    db: &DatabaseResult,
    live_manager: Option<Arc<dyn aiome_core_contracts::traits::LiveSessionManager>>,
) -> anyhow::Result<ProviderResult> {
    // === 🏗️ STAGE 3/7: Engine ===
    let provider = Arc::new(infrastructure::llm::dynamic::DynamicLlmProvider {
        ops: db.job_queue.clone(),
        client: db.http_client.clone(),
        fallback_host: config.ollama_host.clone(),
        fallback_model: config.ollama_model.clone(),
        gemini_api_key: config.gemini_api_key.clone(),
        openai_api_key: config.openai_api_key.clone(),
        anthropic_api_key: config.anthropic_api_key.clone(),
        circuit_breaker: db.circuit_breaker.clone(),
        slo_engine: db.slo_engine.clone(),
        hook_manager: db.hook_manager.clone(),
        live_manager: live_manager.clone(),
        eval_logger: Some(db.eval_logger.clone()),
    });

    let bg_instance = Arc::new(infrastructure::llm::dynamic::BackgroundLlmProvider {
        ops: db.job_queue.clone(),
        client: db.http_client.clone(),
        fallback_model: config.ollama_model.clone(),
        fallback_host: config.ollama_host.clone(),
        gemini_api_key: config.gemini_api_key.clone(),
        openai_api_key: config.openai_api_key.clone(),
        anthropic_api_key: config.anthropic_api_key.clone(),
        hook_manager: db.hook_manager.clone(),
        live_manager: live_manager.clone(),
        eval_logger: Some(db.eval_logger.clone()),
    });

    let bg_provider: Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync> =
        bg_instance.clone();
    let embed_provider: Arc<dyn aiome_core::llm_provider::EmbeddingProvider> = bg_instance.clone();

    // Wire embedding provider back to job_queue (resolves circular dependency)
    db.job_queue
        .set_embedding_provider(embed_provider.clone())
        .await;

    let embed_type = std::env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "ruri".to_string());
    tracing::info!(
        "🧠 [LLM] Front-end: DynamicLlm (DB-configured), Background: {} ({}), Embedding: {}",
        std::env::var("BG_LLM_PROVIDER").unwrap_or_else(|_| "ollama".to_string()),
        std::env::var("BG_LLM_MODEL").unwrap_or_else(|_| "gemma4:26b".to_string()),
        embed_type,
    );

    if std::env::var("ENABLE_TOOL_REVIEWER").unwrap_or_else(|_| "true".to_string()) == "true" {
        let reviewer_hook = infrastructure::security::ToolCallReviewerHook::new(
            bg_provider.clone(),
            20, // max_reviews_per_session
            Some(db.eval_logger.clone()),
        );
        db.hook_manager.add_hook(Arc::new(reviewer_hook));
        tracing::info!("🛡️ [HookManager] ToolCallReviewerHook registered successfully");
    }

    Ok(ProviderResult {
        provider: provider as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>,
        bg_provider,
        embed_provider,
    })
}

pub async fn init_core_services(
    preflight: &PreflightResult,
    db: &DatabaseResult,
    llm: &ProviderResult,
    oxilean_power: std::sync::Arc<std::sync::atomic::AtomicU32>,
) -> anyhow::Result<CoreServicesResult> {
    let resolver = &preflight.resolver;
    let config = &preflight.config;
    let cancel_token = &preflight.cancel_token;
    let plugin_registry = &preflight.plugin_registry;
    let _tts_openai_key = &preflight.secrets.tts_openai_key;
    let stripe_key_raw = &preflight.secrets.stripe_key;
    let nurture_secret_raw = &preflight.secrets.nurture_secret;

    let db_pool = &db.db_pool;
    let job_queue = &db.job_queue;
    let audit_logger = &db.audit_logger;
    let system_agent_id = db.system_agent_id;
    let http_client = &db.http_client;
    let hook_manager = &db.hook_manager;

    let embed_provider = &llm.embed_provider;
    let bg_provider = &llm.bg_provider;
    let provider = &llm.provider;

    // === 🏗️ STAGE 4/7: Core Services ===
    let artifact_store = Arc::new(
        infrastructure::artifact_store::UniversalArtifactStore::new(
            db_pool.clone(),
            resolver.resolve("artifacts"),
        )
        .with_embeddings(embed_provider.clone())
        .with_audit_logger(audit_logger.clone())
        .with_job_queue(job_queue.clone()),
    );

    let (event_sender, _) = tokio::sync::broadcast::channel(100);

    let llm_semaphore = Arc::new(tokio::sync::Semaphore::new(10));
    let forge_semaphore = Arc::new(tokio::sync::Semaphore::new(2));
    let compute_semaphore = Arc::new(tokio::sync::Semaphore::new(1));

    let formal_proof_gate = {
        let host = config.shadow_clone_grpc_host.clone();
        let port = config.shadow_clone_grpc_port.clone();
        let addr = format!("http://{}:{}", host, port);
        let endpoint = tonic::transport::Endpoint::from_shared(addr)
            .map_err(|e| anyhow::anyhow!("Invalid gRPC endpoint: {}", e))?;
        let channel = endpoint.connect_lazy();
        let token = config
            .a2a_auth_token
            .clone()
            .map(|s| {
                use secrecy::ExposeSecret;
                s.expose_secret().to_string()
            })
            .unwrap_or_default();
        Arc::new(infrastructure::grpc_proof_gate::GrpcFormalProofGate::new(
            channel, token,
        )) as Arc<dyn aiome_contracts::proof::FormalProofGate>
    };

    let wasm_skill_manager = Arc::new(
        infrastructure::skills::WasmSkillManager::new(
            resolver.resolve("wasm_storage"),
            resolver.resolve("sandbox"),
        )
        .map_err(|e| anyhow::anyhow!("🚨 Failed to initialize WasmSkillManager: {}", e))?
        .with_db_pool(db_pool.clone()),
    );

    let skill_forge = Arc::new(infrastructure::skills::forge::SkillForge::new(
        resolver.resolve("forge_template"),
        resolver.resolve("wasm_storage"),
    ));

    let commerce_engine = {
        let stripe_key = stripe_key_raw.clone();
        let polar_key = std::env::var("POLAR_API_KEY").ok();
        shared::security::scrub_env("POLAR_API_KEY");
        let stripe_webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_default();
        let polar_webhook_secret = std::env::var("POLAR_WEBHOOK_SECRET").unwrap_or_default();
        shared::security::scrub_env("STRIPE_WEBHOOK_SECRET");
        shared::security::scrub_env("POLAR_WEBHOOK_SECRET");

        let sqlite_pool = db_pool.get_sqlite_pool_or_err()?.clone();

        let nurture_url = std::env::var("NURTURE_API_URL").ok();
        let nurture_secret = nurture_secret_raw.clone();

        let config_commerce = if let Some(key) = stripe_key {
            aiome_commerce::factory::CommerceConfig {
                provider: aiome_commerce::factory::ProviderType::Stripe,
                api_key: Some(secrecy::SecretString::from(key)),
                webhook_secret: secrecy::SecretString::from(stripe_webhook_secret),
                base_url: None,
            }
        } else if let Some(key) = polar_key {
            aiome_commerce::factory::CommerceConfig {
                provider: aiome_commerce::factory::ProviderType::Polar,
                api_key: Some(secrecy::SecretString::from(key)),
                webhook_secret: secrecy::SecretString::from(polar_webhook_secret),
                base_url: std::env::var("POLAR_BASE_URL").ok(),
            }
        } else {
            aiome_commerce::factory::CommerceConfig {
                provider: aiome_commerce::factory::ProviderType::Mock,
                api_key: None,
                webhook_secret: secrecy::SecretString::from("".to_string()),
                base_url: None,
            }
        };

        Some(
            aiome_commerce::CommerceEngineFactory::create(
                config_commerce,
                sqlite_pool,
                nurture_url,
                nurture_secret,
                Some(oxilean_power.clone()),
            )
            .await?,
        )
    };

    let api_server_secret_raw = match std::env::var("API_SERVER_SECRET") {
        Ok(s) => {
            #[cfg(not(debug_assertions))]
            if !is_secure_production_secret(&s) {
                tracing::error!(
                    "🚨 [FATAL SECURITY ERROR] API_SERVER_SECRET is either an insecure default or too short (< 16 chars). Please set a strong, random secret in production!"
                );
                std::process::exit(1);
            }
            s
        }
        Err(_) => {
            let diagnosis = shared::bootstrap_detector::BootstrapDetector::diagnose(
                resolver.root(),
                None,
                None,
            );
            if diagnosis.mode == shared::bootstrap_detector::BootMode::Setup {
                tracing::warn!("⚠️ [api-server] Entering Setup Mode. Using temporary secret for initialization. MUST be configured via WebUI.");
                "setup_mode_temporary_secret_do_not_use".to_string()
            } else {
                #[cfg(debug_assertions)]
                {
                    tracing::warn!("⚠️ [api-server] API_SERVER_SECRET not set. Using insecure default for development.");
                    "dev_secret_donotuseinprod".to_string()
                }
                #[cfg(not(debug_assertions))]
                {
                    tracing::error!(
                        "🚨 [FATAL SECURITY ERROR] API_SERVER_SECRET MUST be set in production!"
                    );
                    std::process::exit(1);
                }
            }
        }
    };
    let federation_secret_raw = std::env::var("FEDERATION_SECRET").ok();

    let api_server_secret = Arc::new(secrecy::SecretString::from(api_server_secret_raw.clone()));
    let federation_secret = federation_secret_raw.map(|s| Arc::new(secrecy::SecretString::from(s)));

    shared::security::scrub_env("API_SERVER_SECRET");
    shared::security::scrub_env("FEDERATION_SECRET");

    // Soul (Sense Foundation)
    let soul_store = Arc::new(infrastructure::soul_store::UniversalSoulStore::new(
        db_pool.clone(),
    ));

    // Phase 37a: Step 2 - SoulPipeline Initialization
    let soul_adapter = infrastructure::soul_adapter::CoreDomainAdapter::new(
        job_queue.clone(),
        Some(embed_provider.clone()),
    );
    let samsara_engine = infrastructure::samsara_engine::DefaultSamsaraEngine::new(
        bg_provider.clone(),
        "You are the Core Soul Engine. Process experiences and distill wisdom.".to_string(),
    );
    let mut soul_pipeline = soul::pipeline::SoulPipeline::new(soul_adapter, samsara_engine);

    // Register WhisperMiddleware (L2.5)
    soul_pipeline.add_middleware(Box::new(
        infrastructure::llm::whisper_middleware::WhisperMiddleware::new(),
    ));
    let soul_pipeline = Arc::new(soul_pipeline);

    // AgentSense (AS-1)
    let intent_firewall = Arc::new(infrastructure::intent::IntentFirewall::new()?);
    let context_engine = Arc::new(infrastructure::context_engine::ContextEngine::new(
        provider.clone(),
        job_queue.clone(),
        llm_semaphore.clone(),
    ));
    let intent_generator = Arc::new(infrastructure::intent::IntentGenerator::new(
        context_engine.clone(),
        provider.clone(),
        intent_firewall.clone(),
        soul_store.clone(),
    ));

    // Initialize BeliefConsistencyGate (Phase 49)
    let slm_bridge = if !config.ollama_host.is_empty() {
        Some(Arc::new(
            infrastructure::slm_bridge::SlmBridge::new_with_command("ollama"),
        ))
    } else {
        None
    };

    let soul_beliefs = match std::fs::read_to_string(resolver.resolve("SOUL.md")) {
        Ok(content) => {
            let beliefs: Vec<String> = content
                .lines()
                .filter(|l| l.trim().starts_with("- ") || l.trim().starts_with("**"))
                .map(|l| l.trim().to_string())
                .collect();
            // RT-5 Fix: 信念が少なすぎる場合は警告を出す
            if beliefs.len() < 3 {
                tracing::warn!("⚠️ [BeliefGate] SOUL.md contains fewer than 3 parseable beliefs ({}). Gate effectiveness may be degraded.", beliefs.len());
            }
            beliefs
        }
        Err(e) => {
            tracing::error!("🚨 [BeliefGate] Failed to read SOUL.md: {}. BeliefConsistencyGate will operate with minimal beliefs.", e);
            vec!["Be helpful and resourceful.".to_string()]
        }
    };

    let belief_gate = Arc::new(
        infrastructure::belief_consistency_gate::BeliefConsistencyGate::new(
            provider.clone(),
            slm_bridge.clone(),
            soul_beliefs,
            None,
        ),
    );

    // --- Phase HTML Report: Load Design Tokens ---
    let tokens_css = {
        let tokens_path = resolver.resolve("tokens_css");
        // Fallback path check (development mode)
        let path = if tokens_path.exists() {
            tokens_path
        } else {
            // Check apps/management-console/src/styles/tokens.css relative to workspace
            resolver
                .root()
                .join("../../apps/management-console/src/styles/tokens.css")
        };

        match std::fs::read_to_string(&path) {
            Ok(css) => {
                tracing::info!(
                    "💎 [Bootstrap] Successfully loaded design tokens from {}",
                    path.display()
                );
                css
            }
            Err(e) => {
                tracing::warn!("⚠️ [Bootstrap] Failed to load design tokens from {}: {}. HTML reports will have limited styling.", path.display(), e);
                "/* Fallback tokens missing */".to_string()
            }
        }
    };

    // Initialize MemoryCrystallizer Background Loop (Phase 49)
    let crystallizer = Arc::new(
        infrastructure::memory_crystallizer::MemoryCrystallizer::new(
            provider.clone(),
            job_queue.clone() as Arc<dyn infrastructure::job_queue::DistillationOps>,
            forge_semaphore.clone(),
            slm_bridge.clone(),
            Some(belief_gate.clone()),
        ),
    );

    let crystallizer_task = crystallizer.clone();
    let supervisor = infrastructure::supervisor::TaskSupervisor::new(10, 300);
    struct MemoryCrystallizerTask {
        crystallizer: Arc<infrastructure::memory_crystallizer::MemoryCrystallizer>,
    }
    impl infrastructure::supervisor::SupervisedTask for MemoryCrystallizerTask {
        fn name(&self) -> &'static str {
            "MemoryCrystallizer"
        }
        fn run(
            &self,
            ct: tokio_util::sync::CancellationToken,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
            let crystallizer = self.crystallizer.clone();
            Box::pin(async move {
                tracing::info!("💎 [MemoryCrystallizer] Starting periodic distillation loop...");
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // Every 5 minutes
                loop {
                    tokio::select! {
                        _ = ct.cancelled() => break,
                        _ = interval.tick() => {
                            let has_error = match crystallizer.run_distillation_cycle().await {
                                Err(e) => {
                                    tracing::error!("🚨 [MemoryCrystallizer] Distillation error: {}", e);
                                    true
                                }
                                Ok(_) => false,
                            };
                            if has_error {
                                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                continue;
                            }
                        }
                    }
                }
            })
        }
    }
    supervisor.spawn_supervised(
        MemoryCrystallizerTask {
            crystallizer: crystallizer_task,
        },
        cancel_token.clone(),
    );

    let soul_mutator = Arc::new(infrastructure::soul_mutator::SoulMutator::new(
        provider.clone(),
        std::path::PathBuf::from("."),
        Some(belief_gate.clone()),
    ));
    let mut primary_provider: Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync> =
        provider.clone();

    use secrecy::ExposeSecret;
    let proxy_provider = infrastructure::llm::proxy::ProxyLlmProvider::new(
        config.key_proxy_url.clone(),
        "gemini".to_string(),
        "api-server".to_string(),
        None,
        config
            .vault_secret
            .as_ref()
            .map(|s| s.expose_secret().to_string()),
    );

    match tokio::time::timeout(
        std::time::Duration::from_secs(3),
        aiome_core::llm_provider::LlmProvider::test_connection(&proxy_provider),
    )
    .await
    {
        Ok(Ok(_)) => {
            tracing::info!(
                "🔐 [KeyProxy] Connected successfully! Enabling Zero-Trust primary routing."
            );
            primary_provider = Arc::new(proxy_provider);
        }
        Ok(Err(e)) => {
            tracing::warn!(
                "⚠️ [KeyProxy] Unreachable (Error: {})! Falling back to Local DynamicLlmProvider.",
                e
            );
        }
        Err(_) => {
            tracing::warn!(
                "⚠️ [KeyProxy] Unreachable (Timeout)! Falling back to Local DynamicLlmProvider."
            );
        }
    }

    let fallback_provider: Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync> =
        bg_provider.clone();
    let base_router_provider = Arc::new(infrastructure::llm::fallback_router::FallbackRouter::new(
        primary_provider,
        fallback_provider,
        3, // failure threshold
    ))
        as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>;
    let router_provider = Arc::new(infrastructure::llm::humanizer_filter::HumanizerFilter::new(
        base_router_provider,
        infrastructure::llm::humanizer_rules::default_rules_ja(),
        infrastructure::llm::writing_context::WritingContext::Default,
    )) as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>;
    let autonomous_running = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let autonomous_config = Arc::new(tokio::sync::RwLock::new(None));
    let docker_failures = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let mcp_manager = Arc::new(mcp::client::McpProcessManager::new());

    let gift_engine = {
        let key = config
            .tremendous_api_key
            .clone()
            .unwrap_or_else(|| secrecy::SecretString::from("".to_string()));
        let sandbox = std::env::var("TREMENDOUS_SANDBOX")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(true); // Default to true (Sandbox First)

        Arc::new(aiome_commerce::gift::TremendousGiftEngine::new(
            key,
            sandbox,
            db_pool.clone(),
            audit_logger.clone(),
        )) as Arc<dyn aiome_core_contracts::commerce::GiftEngine>
    };
    let ekyc_session_store = {
        let pool = db_pool.clone();
        Arc::new(aiome_commerce::ekyc::store::UniversalEkycSessionStore::new(
            pool.clone(),
        )) as Arc<dyn aiome_core_contracts::ekyc::EkycSessionStore>
    };
    let ekyc_engine = {
        let stripe_key = stripe_key_raw.clone().map(secrecy::SecretString::from);

        if let Some(key) = stripe_key {
            Arc::new(aiome_commerce::ekyc::StripeEkycEngine::new(
                key,
                std::env::var("EKYC_CALLBACK_URL").unwrap_or_else(|_| {
                    "http://management-console:1420/verify-callback".to_string()
                }),
                http_client.clone(),
            )) as Arc<dyn aiome_core_contracts::ekyc::EkycEngine>
        } else {
            #[cfg(debug_assertions)]
            {
                tracing::warn!("⚠️ [api-server] STRIPE_API_KEY not set. Using MockEkycEngine (always verified) for development.");
                Arc::new(aiome_commerce::ekyc::MockEkycEngine)
                    as Arc<dyn aiome_core_contracts::ekyc::EkycEngine>
            }
            #[cfg(not(debug_assertions))]
            {
                tracing::error!("🚨 [FATAL SECURITY ERROR] STRIPE_API_KEY must be set in production for eKYC enforcement!");
                std::process::exit(1);
            }
        }
    };
    let quarantine_store = {
        let pool = db_pool.clone();
        let store = infrastructure::compliance::quarantine::UniversalQuarantineStore::new(pool);
        Arc::new(store) as Arc<dyn infrastructure::compliance::quarantine::QuarantineStore>
    };
    let auth_manager = {
        match std::env::var("JWT_PRIVATE_KEY_B64") {
            Ok(key_b64) => {
                shared::security::scrub_env("JWT_PRIVATE_KEY_B64");
                tracing::info!("🔑 [Auth] Loading JWT private key from environment");
                Arc::new(
                    infrastructure::auth::JwtAuthManager::from_private_key_b64(&key_b64)
                        .map_err(|e| anyhow::anyhow!("🚨 Invalid JWT_PRIVATE_KEY_B64: {}", e))?,
                ) as Arc<dyn infrastructure::auth::AuthManager>
            }
            #[cfg(debug_assertions)]
            Err(_) => {
                tracing::warn!("⚠️ [Auth] JWT key not set, using MockAuthManager (dev only)");
                Arc::new(infrastructure::auth::MockAuthManager::new())
                    as Arc<dyn infrastructure::auth::AuthManager>
            }
            #[cfg(not(debug_assertions))]
            Err(_) => {
                tracing::error!("🚨 [FATAL] JWT_PRIVATE_KEY_B64 must be set in production!");
                std::process::exit(1);
            }
        }
    };

    // Initialize UniversalVaultBackend for secret management
    let vault_backend = Arc::new(
        infrastructure::security::sqlite_vault_backend::UniversalVaultBackend::new(db_pool.clone()),
    ) as Arc<dyn aiome_core_contracts::vault_backend::VaultBackend>;
    // === 🏗️ STAGE 5/7: Registry & Core Orchestration ===
    let registry = Arc::new(infrastructure::registry::RegistryManager::new(
        db_pool.clone(),
    ));

    // [A-3] MCP Discovery: Automated server discovery and registration
    {
        let mcp_manager = mcp_manager.clone();
        let registry = registry.clone();
        let vault_backend = vault_backend.clone();
        let config_clone = config.clone();
        let supervisor = infrastructure::supervisor::TaskSupervisor::new(5, 60);
        // child_token(): サーバーの Graceful Shutdown (cancel_token.cancel()) は McpDiscovery にも伝播する。
        // 一方、supervisor の Fail-Closed (mcp_cancel.cancel()) は親の cancel_token には伝播しない。
        let mcp_cancel = cancel_token.child_token();
        struct McpDiscoveryTask {
            mcp_manager: Arc<mcp::client::McpProcessManager>,
            registry: Arc<infrastructure::registry::RegistryManager>,
            vault_backend: Arc<dyn aiome_core_contracts::vault_backend::VaultBackend>,
            config: Arc<shared::config::AiomeConfig>,
        }
        impl infrastructure::supervisor::SupervisedTask for McpDiscoveryTask {
            fn name(&self) -> &'static str {
                "McpDiscovery"
            }
            fn run(
                &self,
                ct: tokio_util::sync::CancellationToken,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
                let mcp_manager = self.mcp_manager.clone();
                let registry = self.registry.clone();
                let vault = self.vault_backend.clone();
                let config = self.config.clone();
                Box::pin(async move {
                    tracing::info!("🔍 [MCP Discovery] Starting periodic discovery loop...");
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
                    loop {
                        tokio::select! {
                            _ = ct.cancelled() => {
                                tracing::info!("🛑 [MCP Discovery] Shutdown requested");
                                break;
                            }
                            _ = interval.tick() => {
                                if let Err(e) = mcp::discovery::discover_and_connect(
                                    &mcp_manager,
                                    &registry,
                                    Some(vault.clone()),
                                    &config,
                                )
                                .await
                                {
                                    tracing::error!(
                                        "🚨 [MCP Discovery] Failed during discovery: {}",
                                        e
                                    );
                                }
                            }
                        }
                    }
                })
            }
        }
        supervisor.spawn_supervised(
            McpDiscoveryTask {
                mcp_manager,
                registry,
                vault_backend: vault_backend.clone(),
                config: config_clone,
            },
            mcp_cancel,
        );
    }

    let voice_drm = Arc::new(
        infrastructure::security::VoiceCoreDrm::new(
            config.abyss_vault_url.clone(),
            registry.clone(),
            db_pool.clone(),
        )
        .await,
    );
    let gig_engine = Arc::new(aiome_commerce::gig::UniversalGigEngine::new(
        db_pool.clone(),
        commerce_engine
            .clone()
            .ok_or_else(|| anyhow::anyhow!("🚨 [api-server] Commerce Engine must be initialized for Gig Engine (check STRIPE_API_KEY)"))?,
        provider.clone(),
        resolver.resolve("gig_artifacts"),
    )) as Arc<dyn aiome_core_contracts::gig::GigEngine>;

    // [Step 1.7] Initialize TranscriptionEngine
    let stt_enabled = std::env::var("AIOME_STT_ENABLED")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);

    let transcription_engine: Arc<dyn aiome_core_contracts::traits::TranscriptionEngine> = Arc::new(
        infrastructure::whisper_transcription::WhisperTranscriptionAdapter::new(
            Arc::new(infrastructure::security::BastionGuard::new_internal(
                aiome_core::security::PermissionManifest::default(),
            )),
            stt_enabled,
        ),
    );

    let validator = Arc::new(
        infrastructure::validator::DefaultConstitutionalValidator::new(
            bg_provider.clone(),
            slm_bridge.clone(),
        ),
    );
    // [Step 1.8] Initialize TaskDispatcher & DockerConductor (Phase 43)
    let tool_discovery = Arc::new(
        infrastructure::skills::discovery::DefaultToolDiscoveryEngine::new(
            wasm_skill_manager.clone(),
            bg_provider.clone(),
        ),
    );
    let strategic_planner = Arc::new(
        infrastructure::task_orchestrator::planner::DefaultStrategicPlanner::new(
            bg_provider.clone(),
        ),
    );

    let soul_path = resolver.resolve("SOUL.md");
    let soul_md = std::fs::read_to_string(&soul_path).unwrap_or_else(|_| String::new());
    let oracle = Arc::new(
        infrastructure::oracle::Oracle::new(bg_provider.clone(), soul_md.clone())
            .with_event_tx(event_sender.clone()),
    );

    let diagnostics_engine = Arc::new(infrastructure::diagnostics::AgentRxDiagnostics::new(
        bg_provider.clone(),
    ));

    // [Step 1.7.5] Bootstrap PublishPipeline for SEO / CMS integration
    let publish_pipeline = Arc::new(infrastructure::publisher::PublishPipeline::new({
        let mut publishers: Vec<Box<dyn aiome_core_contracts::traits::Publisher>> = Vec::new();

        let wp_enabled = config.wp_sdk_enabled || config.wp_api_url.is_some();

        if wp_enabled {
            if !config.key_proxy_url.is_empty() {
                use secrecy::ExposeSecret;
                let vault_secret = config
                    .vault_secret
                    .as_ref()
                    .map(|s| s.expose_secret().to_string())
                    .unwrap_or_default();
                publishers.push(Box::new(
                    infrastructure::publisher::wordpress::WordPressAdapter::new_vault(
                        config.key_proxy_url.clone(),
                        "api-server".to_string(),
                        vault_secret,
                    ),
                ));
                tracing::info!(
                    "✅ [PublishPipeline] WordPress publisher registered via Abyss Vault Proxy."
                );
            } else {
                tracing::warn!("⚠️ [PublishPipeline] WordPress enabled but key_proxy_url is empty. Publishing may fail.");
            }
        } else {
            tracing::warn!("⚠️ [PublishPipeline] WordPress publisher not configured.");
        }

        // Only inject Mock in pure unit tests or when explicitly forced for integration testing
        #[cfg(test)]
        {
            if publishers.is_empty() {
                publishers.push(Box::new(infrastructure::publisher::mock_x::MockXPublisher));
            }
        }

        #[cfg(debug_assertions)]
        if std::env::var("AIOME_FORCE_MOCK_PUBLISHER").is_ok() && publishers.is_empty() {
            tracing::info!("🧪 [PublishPipeline] Forcing MockXPublisher due to AIOME_FORCE_MOCK_PUBLISHER env var.");
            publishers.push(Box::new(infrastructure::publisher::mock_x::MockXPublisher));
        }

        if publishers.is_empty() {
            tracing::warn!("⚠️ [PublishPipeline] No publishers registered. SEO content will be generated but NOT published.");
        }

        publishers
    }));

    let quality_gate_store =
        Arc::new(infrastructure::quality_gate_store::SqliteQualityGateStore::new(db_pool.clone()))
            as Arc<dyn infrastructure::quality_gate_store::QualityGateStore>;

    let mut task_dispatcher = infrastructure::task_orchestrator::TaskDispatcher::new(
        job_queue.clone(),
        std::time::Duration::from_millis(100),
        Some(event_sender.clone()),
        Some(tool_discovery as Arc<dyn aiome_core_contracts::traits::ToolDiscoveryEngine>),
        Some(strategic_planner as Arc<dyn aiome_core_contracts::traits::StrategicPlanner>),
        Some(validator.clone()),
        Some(soul_path),
        Some(oracle),
        Some(gig_engine.clone()),
        Some(diagnostics_engine),
        Some(Arc::new(
            infrastructure::immune_system::AdaptiveImmuneSystem::new(bg_provider.clone()),
        )),
        Some(quality_gate_store.clone()),
        Some(hook_manager.clone()),
    );
    // Register DockerConductor
    let grpc_config = infrastructure::grpc::a2a_grpc_client::GrpcClientConfig {
        endpoint_url: config.a2a_node_url.clone(), // dynamically overwritten in conduct()
        connect_timeout: std::time::Duration::from_secs(5),
        auth_token: "".to_string(), // dynamically overwritten in conduct()
    };
    let docker_gemini_key = config
        .gemini_api_key
        .as_ref()
        .map(|k| secrecy::SecretString::from(secrecy::ExposeSecret::expose_secret(k).to_string()))
        .unwrap_or_else(|| secrecy::SecretString::from(String::new()));
    let docker_conductor = Arc::new(infrastructure::docker_conductor::DockerConductor::new(
        commerce_engine.clone(),
        grpc_config,
        docker_gemini_key,
    ));
    task_dispatcher.register_conductor(docker_conductor);

    // Register BrowserConductor
    let browser_gemini_key = config
        .gemini_api_key
        .as_ref()
        .map(|k| secrecy::SecretString::from(secrecy::ExposeSecret::expose_secret(k).to_string()))
        .unwrap_or_else(|| secrecy::SecretString::from(String::new()));
    let browser_conductor = Arc::new(infrastructure::browser_conductor::BrowserConductor::new(
        commerce_engine.clone(),
        browser_gemini_key,
    ));
    task_dispatcher.register_conductor(browser_conductor);

    // Register CsamScanConductor
    let csam_conductor = Arc::new(
        infrastructure::task_orchestrator::csam::CsamScanConductor::new(
            Some(artifact_store.clone() as Arc<dyn aiome_core::traits::ArtifactStore>),
            db_pool.clone(),
        ),
    );
    task_dispatcher.register_conductor(csam_conductor);

    // Register GenericLlmConductor for scientific_experiment and data_processing
    let generic_conductor = Arc::new(
        infrastructure::task_orchestrator::llm_conductor::GenericLlmConductor::new(
            bg_provider.clone(),
            vec!["scientific_experiment", "data_processing"],
        ),
    );
    task_dispatcher.register_conductor(generic_conductor);

    // Register GeoAuditConductor for standalone GEO audits
    let geo_url = config.geo_optimizer_url.clone();
    let geo_threshold = config.geo_citability_threshold;

    let geo_conductor = Arc::new(
        infrastructure::task_orchestrator::geo_audit::GeoAuditConductor::new(
            geo_url.clone(),
            geo_threshold,
        ),
    );
    task_dispatcher.register_conductor(geo_conductor);

    // Register dedicated SeoContentConductor for autonomous SEO lifecycle
    let seo_conductor = Arc::new(
        infrastructure::task_orchestrator::seo_content::SeoContentConductor::new(
            bg_provider.clone(),
            publish_pipeline.clone(),
            std::env::var("GEO_ENABLED").unwrap_or_else(|_| "true".to_string()) == "true",
            geo_url,
            geo_threshold,
        ),
    );
    task_dispatcher.register_conductor(seo_conductor);

    let task_dispatcher = Arc::new(task_dispatcher);

    // Spawn the loop
    let dispatcher_for_bg = task_dispatcher.clone();
    let cancel_for_dispatcher = cancel_token.clone();
    tokio::spawn(async move {
        tokio::select! {
            _ = dispatcher_for_bg.run_dispatch_loop() => {}
            _ = cancel_for_dispatcher.cancelled() => {
                tracing::info!("🛑 [TaskDispatcher] Shutting down cleanly...");
            }
        }
    });

    // [Phase 51] Initialize Node IPC Client (A2A gRPC)
    let a2a_client = {
        let endpoint_url = config.a2a_node_url.clone();
        let auth_token = config
            .a2a_node_token
            .clone()
            .map(|s| {
                use secrecy::ExposeSecret;
                s.expose_secret().to_string()
            })
            .unwrap_or_else(|| {
                tracing::warn!(
                    "⚠️ [api-server] A2A_NODE_TOKEN not set! Insecure A2A communication."
                );
                "placeholder_for_phase51".to_string()
            });
        let grpc_config = infrastructure::grpc::a2a_grpc_client::GrpcClientConfig {
            endpoint_url,
            connect_timeout: std::time::Duration::from_secs(5),
            auth_token,
        };
        Arc::new(infrastructure::grpc::a2a_grpc_client::A2aGrpcClient::new(
            grpc_config,
        )) as Arc<dyn aiome_core_contracts::a2a::A2aClient>
    };

    let disk_quota_mgr = infrastructure::disk_quota::DiskQuotaManager::new(
        db_pool.clone(),
        500 * 1024 * 1024, // 500MB per agent
    );
    if let Err(e) = disk_quota_mgr.init().await {
        tracing::error!("🚨 Failed to init disk_quota schema: {}", e);
        std::process::exit(1);
    }

    // Commerce engine reference — unwrap here for both AppState and LoRA Marketplace
    let commerce_engine_arc: Arc<dyn aiome_core_contracts::commerce::CommerceEngine> =
        commerce_engine.clone().ok_or_else(|| {
            anyhow::anyhow!(
                "🚨 [api-server] Commerce Engine must be initialized (check STRIPE_API_KEY config)"
            )
        })?;

    let cortex_projector_arc = Arc::new(
        infrastructure::cortex_file_projector::CortexFileProjector::new(
            db_pool.clone(),
            resolver.resolve("cortex_fs"),
        ),
    );

    // [Phase RLM] Initialize RlmClient
    let rlm_url = config.rlm_api_url.clone();
    let rlm_client_arc = Arc::new(infrastructure::llm::rlm_client::RlmClient::new(
        rlm_url,
        job_queue.clone(),
    )) as Arc<dyn aiome_core_contracts::rlm::RlmProvider>;

    Ok(CoreServicesResult {
        artifact_store,
        event_sender,
        llm_semaphore,
        forge_semaphore,
        compute_semaphore,
        formal_proof_gate,
        wasm_skill_manager,
        skill_forge,
        commerce_engine,
        api_server_secret,
        federation_secret,
        gift_engine,
        ekyc_engine,
        ekyc_session_store,
        quarantine_store,
        auth_manager,
        soul_store,
        soul_pipeline,
        soul_mutator,
        intent_generator,
        intent_firewall,
        context_engine,
        belief_gate,
        router_provider,
        autonomous_running,
        autonomous_config,
        docker_failures,
        mcp_manager,
        gig_engine,
        voice_drm,
        registry,
        transcription_engine,
        task_dispatcher,
        rlm_client: rlm_client_arc,
        cortex_projector: cortex_projector_arc,
        a2a_client,
        disk_quota_mgr: Arc::new(disk_quota_mgr),
        quality_gate_store,
        publish_pipeline,
        api_server_secret_raw,
        stripe_key_raw: stripe_key_raw.clone(),
        tts_openai_api_key_raw: preflight.secrets.tts_openai_key.clone(),
        vault_backend,
        tokens_css,
        prompt_registry: {
            let base_dir = resolver.resolve("prompts");
            std::fs::create_dir_all(&base_dir).ok();
            match infrastructure::prompt_registry::MinijinjaPromptRegistry::new(&base_dir) {
                Ok(pr) => Arc::new(pr) as Arc<dyn infrastructure::prompt_registry::PromptRegistry>,
                Err(e) => {
                    tracing::warn!("⚠️ [PromptRegistry] Failed to init Minijinja PromptRegistry: {}. Using Mock.", e);
                    Arc::new(infrastructure::prompt_registry::NoopPromptRegistry)
                        as Arc<dyn infrastructure::prompt_registry::PromptRegistry>
                }
            }
        },
        spec_provider: {
            let workflows_dir = resolver.resolve(".agent/workflows");
            Arc::new(infrastructure::spec_provider::FsSpecProvider::new(
                workflows_dir,
            )) as Arc<dyn infrastructure::spec_provider::SpecProvider>
        },
    })
}
pub async fn assemble_app_state(
    preflight: &PreflightResult,
    db: &DatabaseResult,
    llm: &ProviderResult,
    core: &CoreServicesResult,
    oxilean_power: std::sync::Arc<std::sync::atomic::AtomicU32>,
) -> anyhow::Result<crate::app_state::AppState> {
    let docs_path = "../../docs";

    let state = crate::app_state::AppState {
        oxilean_power,
        hook_chain: Default::default(),
        hook_manager: Component::new(db.hook_manager.clone()),
        db_pool: Component::new(std::sync::Arc::new(db.db_pool.clone())),
        health_monitor: Component::new(preflight.health_monitor.clone()),
        job_queue: Component::new(db.job_queue.clone()),
        wasm_skill_manager: Component::new(core.wasm_skill_manager.clone()),
        skill_forge: Component::new(core.skill_forge.clone()),
        docs_path: docs_path.to_string(),
        llm_semaphore: Component::new(core.llm_semaphore.clone()),
        forge_semaphore: Component::new(core.forge_semaphore.clone()),
        mcp_sessions: Component::new(std::sync::Arc::new(tokio::sync::RwLock::new(
            std::collections::HashMap::new(),
        ))),
        mcp_manager: Component::new(core.mcp_manager.clone()),
        artifact_store: Component::new(
            core.artifact_store.clone() as std::sync::Arc<dyn aiome_core::traits::ArtifactStore>
        ),
        event_sender: Component::new(core.event_sender.clone()),
        context_engine: Component::new(core.context_engine.clone()),
        soul_mutator: Component::new(core.soul_mutator.clone()),
        soul_store: Component::new(core.soul_store.clone()),
        provider: Component::new(core.router_provider.clone()),
        autonomous_running: Component::new(core.autonomous_running.clone()),
        autonomous_config: Component::new(core.autonomous_config.clone()),
        http_client: Component::new(db.http_client.clone()),
        docker_failures: Component::new(core.docker_failures.clone()),
        security_policy: {
            let mut policy = shared::security::SecurityPolicy::default();
            for tool in preflight.plugin_registry.registered_tools() {
                policy.register_tool(&tool);
            }
            policy
        },
        commerce_engine: Component::new(
            core.commerce_engine
                .clone()
                .ok_or_else(|| anyhow::anyhow!("commerce engine must be initialized"))?,
        ),
        gig_engine: Component::new(core.gig_engine.clone()),
        circuit_breaker: Component::new(db.circuit_breaker.clone()),
        rate_limiter: Component::new(db.rate_limiter.clone()),
        slo_engine: Component::new(db.slo_engine.clone()),
        skill_arena: Component::new(std::sync::Arc::new(
            infrastructure::skills::skill_arena::SkillArena::new().with_db_pool(db.db_pool.clone()),
        )),
        api_server_secret: Component::new(core.api_server_secret.clone()),
        federation_secret: Component::new(
            core.federation_secret
                .clone()
                .unwrap_or_else(|| Arc::new(secrecy::SecretString::from(String::new()))),
        ),
        config: Component::new(preflight.config.clone()),
        gift_engine: Component::new(core.gift_engine.clone()),
        ekyc_engine: Component::new(core.ekyc_engine.clone()),
        ekyc_session_store: Component::new(core.ekyc_session_store.clone()),
        quarantine_store: Component::new(core.quarantine_store.clone()),
        auth_manager: Component::new(core.auth_manager.clone()),
        system_agent_id: db.system_agent_id,
        voice_drm: Component::new(core.voice_drm.clone()),
        registry: Component::new(core.registry.clone()),
        intent_generator: Component::new(core.intent_generator.clone()),
        intent_firewall: Component::new(core.intent_firewall.clone()),
        audit_logger: Component::new(db.audit_logger.clone()),
        affiliate_adapter: Component::new({
            #[cfg(debug_assertions)]
            {
                std::sync::Arc::new(infrastructure::intent::MockAffiliateAdapter::new())
                    as std::sync::Arc<dyn aiome_core_contracts::traits::AffiliateAdapter>
            }
            #[cfg(not(debug_assertions))]
            {
                std::sync::Arc::new(infrastructure::intent::DisabledAffiliateAdapter::new())
                    as std::sync::Arc<dyn aiome_core_contracts::traits::AffiliateAdapter>
            }
        }),
        soul_pipeline: Component::new(core.soul_pipeline.clone()),
        transcription_engine: Component::new(core.transcription_engine.clone()),
        task_dispatcher: Component::new(core.task_dispatcher.clone()),
        lora_engine: {
            let core_engine = std::sync::Arc::new(aiome_core::lora::engine::LoraEngine::new());
            let engine =
                std::sync::Arc::new(infrastructure::lora_training::LoraTrainingService::new(
                    core_engine,
                    Some(core.soul_mutator.clone()),
                    Some(db.job_queue.clone()),
                    Some(core.event_sender.clone()),
                    Some(core.compute_semaphore.clone()),
                ));
            Component::new(engine as std::sync::Arc<dyn aiome_core_contracts::traits::LoraEngine>)
        },
        tts_provider: {
            let tts_type = std::env::var("TTS_PROVIDER").unwrap_or_else(|_| "mock".to_string());
            let provider: std::sync::Arc<dyn aiome_core_contracts::traits::TtsProvider> =
                match tts_type.as_str() {
                    "openai" => {
                        let key: secrecy::SecretString =
                            match &core.tts_openai_api_key_raw {
                                Some(raw) => secrecy::SecretString::from(raw.clone()),
                                None => {
                                    tracing::warn!(
                                        "⚠️ [TTS] TTS_OPENAI_API_KEY missing, OpenAI TTS will fail"
                                    );
                                    preflight.config.openai_api_key.clone().unwrap_or_else(|| {
                                        secrecy::SecretString::from(String::new())
                                    })
                                }
                            };
                        let model = std::env::var("TTS_OPENAI_MODEL")
                            .unwrap_or_else(|_| "tts-1".to_string());
                        std::sync::Arc::new(infrastructure::tts::OpenAiTtsProvider::new(
                            key,
                            model,
                            std::env::var("OPENAI_TTS_ENDPOINT").ok(),
                        ))
                    }
                    "xtts" => {
                        let endpoint = preflight
                            .config
                            .xtts_endpoint
                            .clone()
                            .unwrap_or_else(|| format!("http://127.0.0.1:{}", 18020));
                        std::sync::Arc::new(infrastructure::tts::XttsProvider::new(endpoint))
                    }
                    _ => {
                        #[cfg(debug_assertions)]
                        {
                            std::sync::Arc::new(infrastructure::tts::MockTtsProvider::default())
                        }
                        #[cfg(not(debug_assertions))]
                        {
                            std::sync::Arc::new(infrastructure::tts::DisabledTtsProvider::default())
                        }
                    }
                };
            Component::new(provider)
        },
        news_service: {
            let rss = std::sync::Arc::new(infrastructure::rss_collector::RssCollector::new(
                std::sync::Arc::new(infrastructure::rss_collector::SqlTrendCacheRepository::new(
                    db.db_pool.clone(),
                )),
            ));
            Component::new(rss as std::sync::Arc<dyn aiome_core_contracts::traits::NewsService>)
        },
        live_session_manager: Component(preflight.live_manager.clone()),
        syndicate_store: Component::new(std::sync::Arc::new(
            aiome_commerce::syndicate::UniversalSyndicateStore::new(db.db_pool.clone()),
        )),
        hierarchical_router: Component::new(std::sync::Arc::new(
            infrastructure::hierarchical_router::HierarchicalRouter::new(
                llm.provider.clone(),
                db.db_pool.get_sqlite_pool_or_err()?.clone(),
            ),
        )),
        rlm_client: Component::new(core.rlm_client.clone()),
        formal_proof_gate: Component::new(core.formal_proof_gate.clone()),
        a2a_client: Component::new(core.a2a_client.clone()),
        compute_semaphore: Component::new(core.compute_semaphore.clone()),
        disk_quota: Component::new(core.disk_quota_mgr.clone()),
        publish_pipeline: Component::new(core.publish_pipeline.clone()),
        cortex_projector: Component::new(core.cortex_projector.clone()),
        lora_marketplace: {
            let vault_root = preflight.config.resolver.resolve("vault");
            let commerce_for_marketplace = core.commerce_engine.clone().ok_or_else(|| {
                anyhow::anyhow!("commerce engine must be initialized for lora marketplace")
            })?;
            let marketplace = Arc::new(
                infrastructure::lora_marketplace::UniversalLoraMarketplace::new(
                    db.db_pool.clone(),
                    commerce_for_marketplace,
                    vault_root,
                ),
            );
            Component::new(
                marketplace
                    as std::sync::Arc<dyn aiome_core_contracts::lora_marketplace::LoraMarketplace>,
            )
        },
        quality_gate_store: Component::new(core.quality_gate_store.clone()),
        ws_active_connections: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        harness_cache: Component::new(std::sync::Arc::new(
            infrastructure::skills::harness::HarnessCache::new(),
        )),
        upload_semaphore: Component::new(std::sync::Arc::new(tokio::sync::Semaphore::new(2))),
        generative_engine: {
            let engine_type =
                std::env::var("GENERATIVE_ENGINE").unwrap_or_else(|_| "mock".to_string());
            let engine: std::sync::Arc<dyn aiome_core_contracts::traits::GenerativeEngine> =
                match engine_type.as_str() {
                    "comfyui" => {
                        let base_url = preflight.config.comfyui_url.clone();
                        std::sync::Arc::new(
                            infrastructure::generative_engine::ComfyUiGenerativeEngine::new(
                                base_url,
                                Some(core.compute_semaphore.clone()),
                            ),
                        )
                    }
                    "falai" => {
                        let api_key = std::env::var("FAL_KEY").unwrap_or_default();
                        shared::security::scrub_env("FAL_KEY");
                        std::sync::Arc::new(
                            infrastructure::generative_engine::FalAiGenerativeEngine::new(
                                secrecy::SecretString::from(api_key),
                                std::env::var("FAL_AI_ENDPOINT").ok(),
                            ),
                        )
                    }
                    _ => {
                        #[cfg(any(test, debug_assertions))]
                        {
                            tracing::warn!(
                                "⚠️ [GenerativeEngine] Using Mock engine for development."
                            );
                            std::sync::Arc::new(
                            infrastructure::generative_engine::mock::MockGenerativeEngine::default(
                            ),
                        )
                        }
                        #[cfg(not(any(test, debug_assertions)))]
                        {
                            tracing::error!("🚨 [FATAL] GenerativeEngine must be explicitly configured in production (GENERATIVE_ENGINE=comfyui|falai).");
                            std::process::exit(1);
                        }
                    }
                };
            Component::new(engine)
        },
        cortex_ingester: Component::new(std::sync::Arc::new(
            infrastructure::cortex_ingester::CortexIngester::new(
                core.router_provider.clone(),
                db.db_pool.clone(),
            ),
        )),
        project_rules_cache: Component::new(std::sync::Arc::new(
            moka::future::Cache::builder()
                .time_to_live(std::time::Duration::from_secs(30))
                .build(),
        )),
        cortex_query: Component::new(std::sync::Arc::new(
            infrastructure::cortex_query::CortexQueryEngine::new(
                core.router_provider.clone(),
                db.db_pool.clone(),
            )
            .with_rlm_provider(core.rlm_client.clone()),
        )),
        feature_flags_cache: Component::new(std::sync::Arc::new(
            moka::future::Cache::builder()
                .time_to_live(std::time::Duration::from_secs(60))
                .build(),
        )),
        eval_logger: Component::new(db.eval_logger.clone()),
        a2ui_catalog: {
            let mut catalog = infrastructure::a2ui::AiomeCatalog::default();
            catalog.register_component(
                "text",
                serde_json::json!({"content": "string", "markdown": "boolean"}),
            );
            catalog.register_component("button", serde_json::json!({"label": "string", "action": "string (whitelist: approve_job:<uuid>, run_skill:<name>, cancel_job:<uuid>)", "variant": "string (optional)"}));
            catalog.register_component("list", serde_json::json!({"ordered": "boolean"}));
            catalog.register_component(
                "form",
                serde_json::json!({"action": "string", "submitLabel": "string"}),
            );
            catalog.register_component(
                "input",
                serde_json::json!({"name": "string", "label": "string", "type": "string"}),
            );
            catalog.register_component("taskApproval", serde_json::json!({"title": "string", "description": "string", "riskLevel": "string"}));
            catalog.register_component(
                "taskResult",
                serde_json::json!({"success": "boolean", "message": "string"}),
            );
            catalog.register_component(
                "treasureItem",
                serde_json::json!({"name": "string", "value": "number", "currency": "string"}),
            );
            catalog.register_component("progressBar", serde_json::json!({"progress": "number", "label": "string", "status": "string (optional)"}));
            catalog.register_component("dataTable", serde_json::json!({"headers": "array of strings", "rows": "array of arrays of strings"}));
            catalog.register_component("chart", serde_json::json!({"type": "string (line, bar, pie)", "data": "object", "title": "string"}));
            catalog.register_component("alert", serde_json::json!({"type": "string (info, warning, error, success)", "message": "string"}));
            catalog.register_component(
                "cellStatus",
                serde_json::json!({"cellId": "string", "status": "string", "metrics": "object"}),
            );
            catalog.register_component(
                "timeline",
                serde_json::json!({"events": "array of objects {date, title, description}"}),
            );
            catalog.register_component(
                "codeBlock",
                serde_json::json!({"code": "string", "language": "string"}),
            );
            Component::new(std::sync::Arc::new(catalog))
        },
        nurture_url: std::env::var("NURTURE_API_URL").ok(),
        nurture_internal_secret: preflight.secrets.nurture_secret.clone(),
        gig_updater: Component::new(std::sync::Arc::new(
            infrastructure::gig_metadata_updater::DbGigUpdater::new(
                db.db_pool.get_sqlite_pool_or_err()?.clone(),
            ),
        )
            as std::sync::Arc<dyn aiome_contracts::gig_metadata::GigMetadataUpdater>),
        pkce_cache: Component::new(Arc::new(
            moka::future::Cache::builder()
                .time_to_live(std::time::Duration::from_secs(600))
                .max_capacity(10_000)
                .build(),
        )),
        mcp_oauth_secrets: {
            let mut secrets = std::collections::HashMap::new();
            for provider in crate::mcp::discovery::ALLOWED_OAUTH_PROVIDERS {
                let id_env = format!("{}_CLIENT_ID", provider.to_uppercase());
                let sec_env = format!("{}_CLIENT_SECRET", provider.to_uppercase());
                if let (Ok(client_id), Ok(client_secret)) =
                    (std::env::var(&id_env), std::env::var(&sec_env))
                {
                    shared::security::scrub_env(&id_env);
                    shared::security::scrub_env(&sec_env);
                    secrets.insert(
                        provider.to_string(),
                        crate::mcp::discovery::OAuthCredentials {
                            client_id,
                            client_secret: secrecy::SecretString::from(client_secret),
                        },
                    );
                }
            }
            secrets
        },
        vault_backend: Component::new(core.vault_backend.clone()),
        prompt_registry: Component::new(core.prompt_registry.clone()),
        spec_provider: Component::new(core.spec_provider.clone()),
        tokens_css: core.tokens_css.clone(),
    };

    Ok(state)
}
pub async fn spawn_background_workers(
    state: &crate::app_state::AppState,
    belief_gate: &std::sync::Arc<infrastructure::belief_consistency_gate::BeliefConsistencyGate>,
    cancel_token: tokio_util::sync::CancellationToken,
) -> anyhow::Result<()> {
    let resolver = &state.config.get_inner().resolver;
    let job_queue = state.job_queue.get_inner().clone();

    // [Step 1.8.5] Spawn Unified Internal Services (Watchtower & Heartbeat)
    internal_services::spawn_all(state.clone()).await;

    // [Step 1.8.6] Spawn BlobStorageAdapter (Event-Driven Physical Asset Purging)
    let blob_adapter = std::sync::Arc::new(infrastructure::blob_storage::BlobStorageAdapter::new(
        resolver.root().to_path_buf(),
    ));
    let blob_rx = job_queue.event_bus.subscribe();
    blob_adapter.start_event_listener(blob_rx).await;

    // [Step 1.9] Initialize and Spawn TtsWorker Background Loop
    let tts_worker_jq = state.job_queue.get_inner().clone();
    let tts_worker_provider = state.tts_provider.get_inner().clone();
    let tts_worker_speaker = state
        .config
        .get_inner()
        .xtts_speaker
        .clone()
        .unwrap_or_else(|| "p225".to_string());
    let tts_worker_artifacts = resolver.resolve("artifacts");

    let tts_cancel = cancel_token.clone();
    tokio::spawn(async move {
        tracing::info!("🎙️ [TtsWorker] Starting background synthesis loop...");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            tokio::select! {
                _ = tts_cancel.cancelled() => {
                    tracing::info!("🛑 [TtsWorker] Shutting down cleanly...");
                    break;
                }
                _ = interval.tick() => {
                    if let Err(e) = TtsWorker::process_pending_tts(
                        &*tts_worker_jq,
                        &*tts_worker_provider,
                        &tts_worker_speaker,
                        &tts_worker_artifacts,
                    )
                    .await
                    {
                        tracing::error!("🚨 [TtsWorker] Loop error: {}", e);
                    }
                }
            }
        }
    });

    // [Step 1.10] Initialize and Spawn CortexCompiler Background Loop
    let compiler_provider = state.provider.get_inner().clone();
    let compiler_pool = state.db_pool.get_inner().clone();
    let compiler_semaphore = state.compute_semaphore.get_inner().clone();
    let compiler_gate = Some(belief_gate.clone());
    let compiler_projector = state.cortex_projector.get_inner().clone();
    let cortex_cancel = cancel_token.clone();
    tokio::spawn(async move {
        tracing::info!("📚 [Cortex] Starting compilation loop...");
        let compiler = infrastructure::cortex_compiler::CortexCompiler::new(
            compiler_provider,
            (*compiler_pool).clone(),
            compiler_gate,
            compiler_semaphore,
        )
        .with_file_projector(compiler_projector);
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1800));
        loop {
            tokio::select! {
                _ = cortex_cancel.cancelled() => {
                    tracing::info!("🛑 [Cortex] Shutting down cleanly...");
                    break;
                }
                _ = interval.tick() => {
                    match compiler.run_compilation_cycle().await {
                        Ok(ref report) if report.new_articles > 0 || report.updated_articles > 0 => {
                            tracing::info!(
                                "📚 [Cortex] Compilation: {} new, {} updated",
                                report.new_articles,
                                report.updated_articles
                            );
                        }
                        Err(e) => tracing::error!("🚨 [Cortex] Compilation error: {}", e),
                        _ => {} // Ignore empty cycles
                    }
                }
            }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_cors() {
        std::env::set_var("ALLOWED_ORIGINS", "http://localhost:1420");
        let result = init_cors();
        assert!(result.is_ok(), "init_cors failed: {:?}", result.err());

        // Negative test (invalid origin)
        std::env::set_var("ALLOWED_ORIGINS", "invalid\nuri");
        let result = init_cors();
        assert!(result.is_err(), "init_cors should fail with invalid URI");
    }

    #[test]
    fn test_is_secure_production_secret() {
        assert!(!is_secure_production_secret("")); // Too short
        assert!(!is_secure_production_secret("short")); // Too short (5)
        assert!(!is_secure_production_secret("123456789012345")); // Too short (15)

        assert!(!is_secure_production_secret("dev_secret_donotuseinprod")); // Blocklist
        assert!(!is_secure_production_secret(
            "dev_secret_change_me_immediately"
        )); // Blocklist
        assert!(!is_secure_production_secret(
            "quickstart_secret_change_in_production"
        )); // Blocklist
        assert!(!is_secure_production_secret("mock_valid_token_tester")); // Blocklist

        assert!(is_secure_production_secret("my_super_secret_key_123!@#")); // Valid (26 chars)
        assert!(is_secure_production_secret("1234567890123456")); // Valid (16 chars)
    }
}
