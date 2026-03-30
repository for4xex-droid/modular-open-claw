/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

#![forbid(unsafe_code)]
#![allow(unused_imports, unused_variables, dead_code, unused_mut)]

use crate::app_state::Component;
use aiome_contracts::commerce::CommerceEngine;
use aiome_contracts::commerce::GiftEngine;
use aiome_contracts::commerce::GiftPolicyContext;
use aiome_contracts::ekyc::EkycEngine;
use aiome_contracts::ekyc::EkycSessionStore;
use aiome_core::llm_provider::{EmbeddingProvider, LlmProvider};
use aiome_core::traits::TranscriptionEngine;
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

mod api;
#[cfg(test)]
mod api_integration_tests;
mod app_state;
mod auth;
mod autonomous_demo;
mod docker;
mod error;
#[cfg(test)]
mod job_management_tests;
mod logging;
mod mcp;
mod plugin_loader;
mod router;
mod routes;
mod skill_handler;
mod stream;

pub use app_state::AppState;
pub use router::build_app;

#[cfg(feature = "nurture")]
use commerce_protocol;
#[cfg(feature = "nurture")]
use nurture_api;

use aiome_core::expression::tts_worker::TtsWorker;
use aiome_core::traits::JobQueue;
use shared::health::HealthMonitor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // 0. Initialize Metrics EXPORTER (Q-5)
    let metrics_handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()
        .expect("Failed to install Prometheus recorder");
    tracing::info!("📊 Prometheus Metrics initialized at /api/v1/metrics");

    let static_path = "apps/api-server/static";
    let docs_path = "../../docs";

    let health_monitor = shared::health::HealthMonitor::new();
    let health_monitor = Arc::new(Mutex::new(health_monitor));

    let db_url = std::env::var("AIOME_DB_PATH")
        .unwrap_or_else(|_| "sqlite://workspace/aiome.db".to_string());
    if !std::path::Path::new("workspace").exists() {
        std::fs::create_dir_all("workspace").unwrap_or_else(|e| {
            error!("🚨 [CRITICAL] Failed to create workspace directory: {}", e);
            std::process::exit(1);
        });
    }

    if !std::path::Path::new("workspace/gig_artifacts").exists() {
        let _ = std::fs::create_dir_all("workspace/gig_artifacts");
    }

    let cancel_token = tokio_util::sync::CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();
    tokio::spawn(async move {
        if let Ok(_) = tokio::signal::ctrl_c().await {
            tracing::info!("🛑 [api-server] Received Ctrl-C, triggering shutdown...");
            cancel_token_clone.cancel();
        }
    });
    let plugin_registry = plugin_loader::PluginRegistry::new();

    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&db_url.replace("sqlite://", "sqlite:"))
        .await
        .unwrap_or_else(|e| {
            eprintln!("🚨 Failed to connect to SQLite for logging: {}", e);
            std::process::exit(1);
        });
    let logger_layer = logging::DbLoggerLayer::new(pool);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(logger_layer)
        .with(tracing_subscriber::filter::LevelFilter::INFO)
        .init();

    let config = shared::config::AiomeConfig::load().unwrap_or_else(|e| {
        error!("🚨 Failed to load config: {}", e);
        std::process::exit(1);
    });
    let config = Arc::new(config);

    let live_manager: Option<Arc<dyn aiome_contracts::traits::LiveSessionManager>> = {
        use secrecy::ExposeSecret;
        config.gemini_api_key.as_ref().map(|key| {
            Arc::new(
                aiome_core::llm_provider::live_session::LiveSessionProvider::new(
                    key.expose_secret().to_string(),
                    "gemini-2.0-flash-exp".to_string(),
                ),
            ) as Arc<dyn aiome_contracts::traits::LiveSessionManager>
        })
    };

    let trajectory_store: Arc<dyn aiome_core::trajectory::TrajectoryStore> = {
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
        Arc::new(infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(ts_pool))
    };

    let job_queue =
        infrastructure::job_queue::UniversalJobQueue::new(&config.db_path, None, trajectory_store)
            .await
            .unwrap_or_else(|e| {
                error!("🚨 Failed to init DB at {}: {}", config.db_path, e);
                std::process::exit(1);
            });
    let job_queue = Arc::new(job_queue);

    let audit_logger = Arc::new(AsyncAuditLogger::new(
        Arc::new(job_queue.get_pool().clone()),
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
    let rate_limiter = infrastructure::rate_limiter::AgentRateLimiter::new(60);

    let slo_engine = Arc::new(infrastructure::slo_engine::SloEngine::new(
        infrastructure::slo_engine::SloConfig {
            error_budget_max: 100,
            warning_threshold: 80,
        },
        chrono::Duration::hours(24),
    ));

    let http_client = aiome_core::http::get_http_client().clone();

    let sandbox = Arc::new(
        shared::sandbox::PathSandbox::new("workspace")
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
    let hook_manager = Arc::new(hook_manager);

    let provider = Arc::new(infrastructure::llm::dynamic::DynamicLlmProvider {
        jq: job_queue.clone(),
        client: http_client.clone(),
        fallback_host: config.ollama_host.clone(),
        fallback_model: config.ollama_model.clone(),
        gemini_api_key: config.gemini_api_key.clone(),
        openai_api_key: config.openai_api_key.clone(),
        anthropic_api_key: config.anthropic_api_key.clone(),
        circuit_breaker: circuit_breaker.clone(),
        slo_engine: slo_engine.clone(),
        hook_manager: hook_manager.clone(),
        live_manager: live_manager.clone(),
    });

    let bg_instance = Arc::new(infrastructure::llm::dynamic::BackgroundLlmProvider {
        jq: job_queue.clone(),
        client: http_client.clone(),
        fallback_model: config.ollama_model.clone(),
        fallback_host: config.ollama_host.clone(),
        gemini_api_key: config.gemini_api_key.clone(),
        openai_api_key: config.openai_api_key.clone(),
        anthropic_api_key: config.anthropic_api_key.clone(),
        hook_manager: hook_manager.clone(),
        live_manager: live_manager.clone(),
    });

    let bg_provider: Arc<dyn aiome_core::llm_provider::LlmProvider> = bg_instance.clone();
    let embed_provider: Arc<dyn aiome_core::llm_provider::EmbeddingProvider> = bg_instance.clone();

    // Wire embedding provider back to job_queue (resolves circular dependency)
    job_queue
        .set_embedding_provider(embed_provider.clone())
        .await;

    let embed_type = std::env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "ruri".to_string());
    info!(
        "🧠 [LLM] Front-end: DynamicLlm (DB-configured), Background: {} ({}), Embedding: {}",
        std::env::var("BG_LLM_PROVIDER").unwrap_or_else(|_| "ollama".to_string()),
        std::env::var("BG_LLM_MODEL").unwrap_or_else(|_| "qwen3.5:9b".to_string()),
        embed_type,
    );

    let artifact_store = infrastructure::artifact_store::UniversalArtifactStore::new(
        job_queue.get_pool().clone(),
        std::path::PathBuf::from("workspace/artifacts"),
    )
    .with_embeddings(embed_provider.clone())
    .with_audit_logger(audit_logger.clone());

    let (event_sender, _) = tokio::sync::broadcast::channel(100);

    let llm_semaphore = Arc::new(tokio::sync::Semaphore::new(10));
    let forge_semaphore = Arc::new(tokio::sync::Semaphore::new(2));

    let wasm_skill_manager = Arc::new(
        infrastructure::skills::WasmSkillManager::new(
            "workspace/wasm_storage",
            "workspace/sandbox",
        )
        .map_err(|e| anyhow::anyhow!("🚨 Failed to initialize WasmSkillManager: {}", e))?,
    );

    let skill_forge = Arc::new(infrastructure::skills::forge::SkillForge::new(
        "workspace/forge_template",
        "workspace/wasm_storage",
    ));

    let commerce_engine = {
        use secrecy::ExposeSecret;
        let stripe_key = std::env::var("STRIPE_API_KEY").ok().map(|key| {
            std::env::remove_var("STRIPE_API_KEY");
            secrecy::SecretString::from(key)
        });

        if let Some(key) = stripe_key {
            use secrecy::ExposeSecret;
            let webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_default();
            Some(Arc::new(aiome_commerce::stripe::StripeCommerceEngine::new(
                key.expose_secret().to_string(),
                webhook_secret,
                job_queue.get_pool().get_sqlite_pool_or_err()?.clone(),
            ))
                as Arc<dyn aiome_core::commerce::CommerceEngine>)
        } else {
            #[cfg(debug_assertions)]
            {
                warn!("⚠️ [api-server] STRIPE_API_KEY not set. Using MockCommerceEngine for development.");
                Some(Arc::new(aiome_commerce::mock::MockCommerceEngine::new())
                    as Arc<dyn aiome_core::commerce::CommerceEngine>)
            }
            #[cfg(not(debug_assertions))]
            {
                error!("🚨 [FATAL SECURITY ERROR] STRIPE_API_KEY must be set in production!");
                std::process::exit(1);
            }
        }
    };

    let api_server_secret_raw = match std::env::var("API_SERVER_SECRET") {
        Ok(s) => s,
        Err(_) => {
            #[cfg(debug_assertions)]
            {
                warn!("⚠️ [api-server] API_SERVER_SECRET not set. Using insecure default for development.");
                "dev_secret_donotuseinprod".to_string()
            }
            #[cfg(not(debug_assertions))]
            {
                error!("🚨 [FATAL SECURITY ERROR] API_SERVER_SECRET MUST be set in production!");
                std::process::exit(1);
            }
        }
    };
    let federation_secret_raw = std::env::var("FEDERATION_SECRET").ok();

    let api_server_secret = Arc::new(secrecy::SecretString::from(api_server_secret_raw.clone()));
    let federation_secret = federation_secret_raw.map(|s| Arc::new(secrecy::SecretString::from(s)));

    // Soul (Sense Foundation)
    let soul_store = Arc::new(infrastructure::soul_store::UniversalSoulStore::new(
        job_queue.get_pool().clone(),
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
    let intent_firewall = Arc::new(infrastructure::intent::IntentFirewall::new());
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

    let soul_beliefs = match std::fs::read_to_string("SOUL.md") {
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

    let belief_gate = Arc::new(BeliefConsistencyGate::new(
        provider.clone(),
        slm_bridge.clone(),
        soul_beliefs,
        None,
    ));

    // Initialize MemoryCrystallizer Background Loop (Phase 49)
    let crystallizer = Arc::new(MemoryCrystallizer::new(
        provider.clone(),
        job_queue.clone(),
        forge_semaphore.clone(),
        slm_bridge.clone(),
        Some(belief_gate.clone()),
    ));

    let crystallizer_task = crystallizer.clone();
    tokio::spawn(async move {
        info!("💎 [MemoryCrystallizer] Starting periodic distillation loop...");
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes
        loop {
            interval.tick().await;
            if let Err(e) = crystallizer_task.run_distillation_cycle().await {
                error!("🚨 [MemoryCrystallizer] Distillation error: {}", e);
            }
        }
    });

    let soul_mutator = Arc::new(infrastructure::soul_mutator::SoulMutator::new(
        provider.clone(),
        std::path::PathBuf::from("."),
        Some(belief_gate.clone()),
    ));
    let primary_provider: Arc<dyn LlmProvider + Send + Sync> = provider.clone();
    let fallback_provider: Arc<dyn LlmProvider + Send + Sync> = bg_provider.clone();
    let base_router_provider = Arc::new(infrastructure::llm::fallback_router::FallbackRouter::new(
        primary_provider,
        fallback_provider,
        3, // failure threshold
    )) as Arc<dyn LlmProvider + Send + Sync>;
    let router_provider = Arc::new(infrastructure::llm::humanizer_filter::HumanizerFilter::new(
        base_router_provider,
        infrastructure::llm::humanizer_rules::default_rules_ja(),
        infrastructure::llm::writing_context::WritingContext::Default,
    )) as Arc<dyn LlmProvider + Send + Sync>;
    let autonomous_running = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let autonomous_config = Arc::new(tokio::sync::RwLock::new(None));
    let docker_failures = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let mcp_manager = Arc::new(mcp::client::McpProcessManager::new());

    let gift_engine = {
        use secrecy::ExposeSecret;
        let key = config
            .tremendous_api_key
            .as_ref()
            .map(|s| s.expose_secret().to_string())
            .unwrap_or_default();
        let sandbox = std::env::var("TREMENDOUS_SANDBOX")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(true); // Default to true (Sandbox First)

        Arc::new(aiome_commerce::gift::TremendousGiftEngine::new(
            key,
            sandbox,
            job_queue.get_pool().clone(),
            audit_logger.clone(),
        )) as Arc<dyn GiftEngine>
    };
    let ekyc_session_store = {
        let pool = job_queue.get_pool().clone();
        Arc::new(aiome_commerce::ekyc::store::UniversalEkycSessionStore::new(
            pool.clone(),
        )) as Arc<dyn EkycSessionStore>
    };
    let ekyc_engine = {
        use secrecy::ExposeSecret;
        let stripe_key = std::env::var("STRIPE_API_KEY").ok().map(|key| {
            std::env::remove_var("STRIPE_API_KEY");
            secrecy::SecretString::from(key)
        });

        if let Some(key) = stripe_key {
            Arc::new(aiome_commerce::ekyc::StripeEkycEngine::new(
                key,
                "http://localhost:1420/verify-callback".to_string(),
                http_client.clone(),
            )) as Arc<dyn EkycEngine>
        } else {
            #[cfg(debug_assertions)]
            {
                warn!("⚠️ [api-server] STRIPE_API_KEY not set. Using MockEkycEngine (always verified) for development.");
                Arc::new(aiome_commerce::ekyc::MockEkycEngine) as Arc<dyn EkycEngine>
            }
            #[cfg(not(debug_assertions))]
            {
                error!("🚨 [FATAL SECURITY ERROR] STRIPE_API_KEY must be set in production for eKYC enforcement!");
                std::process::exit(1);
            }
        }
    };
    let quarantine_store = {
        let pool = job_queue.get_pool().clone();
        let store = infrastructure::compliance::quarantine::UniversalQuarantineStore::new(pool);
        Arc::new(store) as Arc<dyn QuarantineStore>
    };
    let auth_manager = {
        match std::env::var("JWT_PRIVATE_KEY_B64") {
            Ok(key_b64) => {
                std::env::remove_var("JWT_PRIVATE_KEY_B64");
                info!("🔑 [Auth] Loading JWT private key from environment");
                Arc::new(
                    infrastructure::auth::JwtAuthManager::from_private_key_b64(&key_b64)
                        .map_err(|e| anyhow::anyhow!("🚨 Invalid JWT_PRIVATE_KEY_B64: {}", e))?,
                ) as Arc<dyn AuthManager>
            }
            #[cfg(debug_assertions)]
            Err(_) => {
                warn!("⚠️ [Auth] JWT key not set, using MockAuthManager (dev only)");
                Arc::new(infrastructure::auth::MockAuthManager::new()) as Arc<dyn AuthManager>
            }
            #[cfg(not(debug_assertions))]
            Err(_) => {
                error!("🚨 [FATAL] JWT_PRIVATE_KEY_B64 must be set in production!");
                std::process::exit(1);
            }
        }
    };
    let registry = Arc::new(infrastructure::registry::RegistryManager::new(
        job_queue.get_pool().clone(),
    ));

    // [A-3] MCP Discovery: Automated server discovery and registration
    {
        let mcp_manager = mcp_manager.clone();
        let registry = registry.clone();
        tokio::spawn(async move {
            info!("🔍 [MCP Discovery] Starting automated server discovery...");
            if let Err(e) = mcp::discovery::discover_and_connect(&mcp_manager, &registry).await {
                error!("🚨 [MCP Discovery] Failed during initial discovery: {}", e);
            }
        });
    }

    let voice_drm = Arc::new(
        infrastructure::security::VoiceCoreDrm::new(
            std::env::var("ABYSS_VAULT_URL")
                .unwrap_or_else(|_| "http://localhost:3016".to_string()),
            registry.clone(),
            job_queue.get_pool().clone(),
        )
        .await,
    );
    let gig_engine = Arc::new(aiome_commerce::gig::UniversalGigEngine::new(
        job_queue.get_pool().clone(),
        commerce_engine
            .clone()
            .ok_or_else(|| anyhow::anyhow!("🚨 [api-server] Commerce Engine must be initialized for Gig Engine (check STRIPE_API_KEY)"))?,
        provider.clone(),
        std::path::PathBuf::from("workspace/gig_artifacts"),
    )) as Arc<dyn aiome_contracts::gig::GigEngine>;

    // [Step 1.7] Initialize TranscriptionEngine
    let stt_enabled = std::env::var("AIOME_STT_ENABLED")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);

    let transcription_engine: Arc<dyn TranscriptionEngine> =
        Arc::new(WhisperTranscriptionAdapter::new(
            Arc::new(infrastructure::security::BastionGuard::new_internal(
                aiome_core::security::PermissionManifest::default(),
            )),
            stt_enabled,
        ));

    let validator = Arc::new(
        infrastructure::validator::DefaultConstitutionalValidator::new(
            bg_provider.clone(),
            None, // TODO: 将来的にメインプロセスでも SLM を使用する場合は注入
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

    let soul_md = std::fs::read_to_string("workspace/SOUL.md").unwrap_or_else(|_| String::new());
    let oracle = Arc::new(infrastructure::oracle::Oracle::new(
        bg_provider.clone(),
        soul_md.clone(),
    ));

    let mut task_dispatcher = infrastructure::task_orchestrator::TaskDispatcher::new(
        job_queue.clone(),
        std::time::Duration::from_millis(100),
        Some(event_sender.clone()),
        Some(tool_discovery as Arc<dyn aiome_contracts::traits::ToolDiscoveryEngine>),
        Some(strategic_planner as Arc<dyn aiome_contracts::traits::StrategicPlanner>),
        Some(validator.clone()),
        Some(std::path::PathBuf::from("workspace/SOUL.md")),
        Some(oracle),
        Some(gig_engine.clone()),
    );
    // Register DockerConductor
    let grpc_config = infrastructure::grpc::a2a_grpc_client::GrpcClientConfig {
        endpoint_url: "http://127.0.0.1:50051".to_string(), // dynamically overwritten in conduct()
        connect_timeout: std::time::Duration::from_secs(5),
        auth_token: "".to_string(), // dynamically overwritten in conduct()
    };
    let docker_conductor = Arc::new(infrastructure::docker_conductor::DockerConductor::new(
        commerce_engine.clone(),
        grpc_config,
    ));
    task_dispatcher.register_conductor(docker_conductor);

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
        let endpoint_url =
            std::env::var("A2A_NODE_URL").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
        let auth_token = std::env::var("A2A_NODE_TOKEN")
            .unwrap_or_else(|_| "placeholder_for_phase51".to_string());
        let grpc_config = infrastructure::grpc::a2a_grpc_client::GrpcClientConfig {
            endpoint_url,
            connect_timeout: std::time::Duration::from_secs(5),
            auth_token,
        };
        Arc::new(infrastructure::grpc::a2a_grpc_client::A2aGrpcClient::new(
            grpc_config,
        )) as Arc<dyn aiome_contracts::a2a::A2aClient>
    };

    let state = AppState {
        health_monitor: Component::new(health_monitor),
        job_queue: Component::new(job_queue.clone()),
        wasm_skill_manager: Component::new(wasm_skill_manager),
        skill_forge: Component::new(skill_forge),
        docs_path: docs_path.to_string(),
        llm_semaphore: Component::new(llm_semaphore),
        forge_semaphore: Component::new(forge_semaphore),
        mcp_sessions: Component::new(Arc::new(tokio::sync::RwLock::new(
            std::collections::HashMap::new(),
        ))),
        mcp_manager: Component::new(mcp_manager),
        artifact_store: Component::new(
            Arc::new(artifact_store) as Arc<dyn aiome_core::traits::ArtifactStore>
        ),
        event_sender: Component::new(event_sender),
        context_engine: Component::new(context_engine),
        soul_mutator: Component::new(soul_mutator),
        soul_store: Component::new(soul_store),
        provider: Component::new(router_provider),
        autonomous_running: Component::new(autonomous_running),
        autonomous_config: Component::new(autonomous_config),
        http_client: Component::new(http_client.clone()),
        docker_failures: Component::new(docker_failures),
        security_policy: {
            let mut policy = shared::security::SecurityPolicy::default();
            for tool in plugin_registry.registered_tools() {
                policy.register_tool(&tool);
            }
            policy
        },
        commerce_engine: Component::new(commerce_engine.ok_or_else(|| {
            anyhow::anyhow!(
                "🚨 [api-server] Commerce Engine must be initialized (check STRIPE_API_KEY config)"
            )
        })?),
        gig_engine: Component::new(gig_engine),
        circuit_breaker: Component::new(circuit_breaker),
        rate_limiter: Component::new(rate_limiter),
        slo_engine: Component::new(slo_engine),
        api_server_secret: Component::new(api_server_secret),
        federation_secret: Component(federation_secret),
        config: Component::new(config.clone()),
        gift_engine: Component::new(gift_engine),
        ekyc_engine: Component::new(ekyc_engine),
        ekyc_session_store: Component::new(ekyc_session_store),
        quarantine_store: Component::new(quarantine_store),
        auth_manager: Component::new(auth_manager),
        system_agent_id,
        voice_drm: Component::new(voice_drm),
        registry: Component::new(registry),
        intent_generator: Component::new(intent_generator),
        intent_firewall: Component::new(intent_firewall),
        audit_logger: Component::new(audit_logger),
        affiliate_adapter: Component::new(
            Arc::new(infrastructure::intent::AffiliateAdapter::new()),
        ),
        soul_pipeline: Component::new(soul_pipeline),
        transcription_engine: Component::new(transcription_engine),
        task_dispatcher: Component::new(task_dispatcher),
        lora_engine: {
            let engine = Arc::new(aiome_core::lora::engine::LoraEngine::new());
            Component::new(engine as Arc<dyn aiome_contracts::traits::LoraEngine>)
        },
        tts_provider: {
            let tts_type = std::env::var("TTS_PROVIDER").unwrap_or_else(|_| "mock".to_string());
            let provider: Arc<dyn aiome_contracts::traits::TtsProvider> = match tts_type.as_str() {
                "openai" => {
                    use secrecy::ExposeSecret;
                    let key = std::env::var("TTS_OPENAI_API_KEY").unwrap_or_else(|_| {
                        config
                            .openai_api_key
                            .as_ref()
                            .map(|s| s.expose_secret().to_string())
                            .unwrap_or_default()
                    });
                    let model =
                        std::env::var("TTS_OPENAI_MODEL").unwrap_or_else(|_| "tts-1".to_string());
                    Arc::new(infrastructure::tts::OpenAiTtsProvider::new(key, model))
                }
                "xtts" => {
                    let endpoint = std::env::var("XTTS_ENDPOINT")
                        .unwrap_or_else(|_| "http://localhost:18020".to_string());
                    Arc::new(infrastructure::tts::XttsProvider::new(endpoint))
                }
                _ => Arc::new(infrastructure::tts::MockTtsProvider::default()),
            };
            Component::new(provider)
        },
        news_service: {
            let rss = Arc::new(infrastructure::rss_collector::RssCollector::new(
                job_queue.clone(),
            ));
            Component::new(rss as Arc<dyn aiome_contracts::traits::NewsService>)
        },
        live_session_manager: Component(live_manager),
        syndicate_store: Component::new(Arc::new(
            aiome_commerce::syndicate::SqliteSyndicateStore::new(
                job_queue
                    .get_pool()
                    .get_sqlite_pool()
                    .cloned()
                    .expect("SQLite pool required for SyndicateStore"),
            ),
        )),
        hierarchical_router: Component::new(Arc::new(
            infrastructure::hierarchical_router::HierarchicalRouter::new(
                bg_provider.clone(),
                job_queue
                    .get_pool()
                    .get_sqlite_pool()
                    .cloned()
                    .expect("SQLite pool required for HierarchicalRouter"),
            ),
        )),
        a2a_client: Component::new(a2a_client),
        ws_active_connections: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };

    // [Step 1.9] Initialize and Spawn TtsWorker Background Loop (Phase 13.3)
    let tts_worker_jq = state.job_queue.get_inner().clone();
    let tts_worker_provider = state.tts_provider.get_inner().clone();
    let tts_worker_speaker = state
        .config
        .get_inner()
        .xtts_speaker
        .clone()
        .unwrap_or_else(|| "p225".to_string());
    let tts_worker_artifacts = std::path::PathBuf::from("workspace/artifacts");

    tokio::spawn(async move {
        info!("🎙️ [TtsWorker] Starting background synthesis loop...");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Err(e) = TtsWorker::process_pending_tts(
                &*tts_worker_jq,
                &*tts_worker_provider,
                &tts_worker_speaker,
                &tts_worker_artifacts,
            )
            .await
            {
                error!("🚨 [TtsWorker] Loop error: {}", e);
            }
        }
    });

    let cors_layer = {
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
        layer
    };

    #[cfg(feature = "nurture")]
    let nurture_state = commerce_protocol::CommerceState::new();

    let app = build_app(
        state.clone(),
        cors_layer,
        static_path,
        #[cfg(feature = "nurture")]
        nurture_state,
        plugin_registry,
        metrics_handle,
    );

    // G-23: Periodic Federated Metrics Push (Background Maintenance Loop)
    let jq_for_bg = job_queue.clone();
    tokio::spawn(async move {
        use infrastructure::job_queue::federation::FederationOps;
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Every hour
        loop {
            interval.tick().await;
            info!("♻️ [Maintenance] Running periodic federated metrics push...");
            if let Err(e) = jq_for_bg.do_push_federated_metrics().await {
                error!("🚨 [Maintenance] Failed to push federated metrics: {}", e);
            }
        }
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], 3015));
    info!("🚀 [api-server] Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| anyhow::anyhow!(
        "🚨 [api-server] Failed to bind to address http://{}. Check if the port is already in use. Error: {}",
        addr, e
    ))?;

    let cancel_serve = cancel_token.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            cancel_serve.cancelled().await;
            tracing::info!("🛑 [api-server] Graceful shutdown triggered.");
        })
        .await
        .map_err(|e| anyhow::anyhow!("🚨 [api-server] Failed to start Axum server: {}", e))?;

    Ok(())
}
