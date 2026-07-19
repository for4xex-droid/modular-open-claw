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

pub mod core_services;
pub use core_services::*;

pub mod plugins;
pub use plugins::*;

use crate::mcp;

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

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
    /// Fast tier 用プロバイダー（ローカル LLM 優先、FallbackRouter で wrap 済み）
    pub fast_provider: Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>,
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
    /// OP-083-C: optional DI for x402 (None if env not configured — boot continues).
    pub x402_negotiator: Option<Arc<dyn aiome_core_contracts::X402Negotiator>>,
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
        Arc<tokio::sync::RwLock<Option<aiome_core::commune::autonomous::AutonomousConfig>>>,
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
    pub workflow_execution_tracker:
        Arc<crate::workflow_execution_tracker::WorkflowExecutionTracker>,
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
    let mut preflight = preflight;
    let db_result = init_database(&preflight).await?;
    let llm_result = init_llm_providers(
        &preflight.config,
        &db_result,
        preflight.live_manager.clone(),
    )
    .await?;
    let core_result =
        init_core_services(&preflight, &db_result, &llm_result, oxilean_power.clone()).await?;

    register_in_process_plugins(
        &mut preflight.plugin_registry,
        preflight.cancel_token.clone(),
        &preflight.secrets.nurture_secret,
        &db_result,
        &core_result,
    )
    .await?;
    attach_plugin_hooks(&preflight.plugin_registry, &db_result.hook_manager);

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
