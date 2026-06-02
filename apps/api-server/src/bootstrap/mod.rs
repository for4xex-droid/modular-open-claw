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

pub mod preflight;
pub use preflight::*;

pub mod database;
pub use database::*;

pub mod llm_providers;
pub use llm_providers::*;

pub mod state_assembly;
pub use state_assembly::*;

pub mod workers;
pub use workers::*;

pub mod helpers;
pub use helpers::*;

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
use infrastructure::compliance::ban_store::BanStore;
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
    pub stripe_price_subscription_monthly: Option<String>,
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
    pub alert_manager: Arc<infrastructure::alerts::AlertManager>,
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
    pub ban_store: Arc<dyn infrastructure::compliance::ban_store::BanStore>,
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
            Some(Arc::new(infrastructure::cortex_synth::LlmSynthJudge::new(
                provider.clone(),
            ))),
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

    let entropy_gate_provider = Arc::new(infrastructure::llm::entropy_gate::EntropyGate::new(
        base_router_provider,
        2.0, // entropy threshold
        3,   // max re-ask
    ))
        as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>;

    let router_provider = Arc::new(infrastructure::llm::humanizer_filter::HumanizerFilter::new(
        entropy_gate_provider,
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
        )?) as Arc<dyn aiome_core_contracts::commerce::GiftEngine>
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
    let ban_store = {
        let pool = db_pool.clone();
        let store = infrastructure::compliance::ban_store::UniversalBanStore::new(pool);
        store.init().await?;
        Arc::new(store) as Arc<dyn infrastructure::compliance::ban_store::BanStore>
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
    let docker_conductor = Arc::new(infrastructure::docker_conductor::DockerConductor::new(
        commerce_engine.clone(),
        grpc_config,
        Some(config.key_proxy_url.clone()),
        config.vault_secret.clone(),
    ));
    task_dispatcher.register_conductor(docker_conductor);

    // Register BrowserConductor
    let browser_conductor = Arc::new(infrastructure::browser_conductor::BrowserConductor::new(
        commerce_engine.clone(),
        Some(config.key_proxy_url.clone()),
        config.vault_secret.clone(),
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
        ban_store,
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
