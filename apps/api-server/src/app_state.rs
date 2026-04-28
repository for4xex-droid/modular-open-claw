/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::commerce::CommerceEngine;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::{ArtifactStore, JobQueue, TranscriptionEngine};
use aiome_core_contracts::commerce::GiftEngine;
use aiome_core_contracts::ekyc::EkycEngine;
use infrastructure::audit_logger::AsyncAuditLogger;
use infrastructure::auth::AuthManager;
use infrastructure::circuit_breaker::CircuitBreaker;
use infrastructure::compliance::quarantine::QuarantineStore;
use infrastructure::context_engine::ContextEngine;
use infrastructure::job_queue::UniversalJobQueue;
use infrastructure::rate_limiter::AgentRateLimiter;
use infrastructure::registry::RegistryManager;
use infrastructure::security::VoiceCoreDrm;
use infrastructure::skills::forge::SkillForge;
use infrastructure::skills::WasmSkillManager;
use infrastructure::slo_engine::SloEngine;
use infrastructure::soul_mutator::SoulMutator;
use infrastructure::soul_store::UniversalSoulStore;
use shared::config::AiomeConfig;
use shared::health::HealthMonitor;
use shared::security::SecurityPolicy;
use shared::watchtower::CoreEvent;
use std::sync::Arc;
use tokio::sync::Mutex;

use aiome_core_contracts::ekyc::EkycSessionStore;

#[derive(Clone, Debug)]
pub struct Component<T>(pub Option<T>);

impl<T> Default for Component<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T> Component<T> {
    pub fn new(val: T) -> Self {
        Self(Some(val))
    }
    pub fn as_opt(&self) -> Option<&T> {
        self.0.as_ref()
    }

    pub fn get_inner(&self) -> &T {
        use std::ops::Deref;
        self.deref()
    }
}

impl<T> std::ops::Deref for Component<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap_or_else(|| panic!("Component not initialized in AppState! Make sure to provide it in create_test_server or main. Type: {}", std::any::type_name::<T>()))
    }
}

#[derive(Clone, Default)]
pub struct AppState {
    pub health_monitor: Component<Arc<Mutex<HealthMonitor>>>,
    pub db_pool: Component<Arc<infrastructure::db::DatabasePool>>,
    pub job_queue: Component<Arc<UniversalJobQueue>>,
    pub wasm_skill_manager: Component<Arc<WasmSkillManager>>,
    pub skill_forge: Component<Arc<SkillForge>>,
    pub docs_path: String,
    pub llm_semaphore: Component<Arc<tokio::sync::Semaphore>>,
    pub forge_semaphore: Component<Arc<tokio::sync::Semaphore>>,
    pub mcp_sessions: Component<
        Arc<
            tokio::sync::RwLock<
                std::collections::HashMap<String, tokio::sync::mpsc::Sender<String>>,
            >,
        >,
    >,
    pub mcp_manager: Component<Arc<crate::mcp::client::McpProcessManager>>,
    pub artifact_store: Component<Arc<dyn ArtifactStore>>,
    pub event_sender: Component<tokio::sync::broadcast::Sender<CoreEvent>>,
    pub context_engine: Component<Arc<ContextEngine>>,
    pub soul_mutator: Component<Arc<SoulMutator>>,
    pub soul_store: Component<Arc<UniversalSoulStore>>,
    pub provider: Component<Arc<dyn LlmProvider + Send + Sync>>,
    pub autonomous_running: Component<Arc<std::sync::atomic::AtomicBool>>,
    pub autonomous_config:
        Component<Arc<tokio::sync::RwLock<Option<aiome_core::biome::AutonomousConfig>>>>,
    pub http_client: Component<reqwest::Client>,
    pub docker_failures:
        Component<Arc<tokio::sync::RwLock<std::collections::HashMap<String, u32>>>>,
    pub security_policy: SecurityPolicy,
    pub commerce_engine: Component<Arc<dyn CommerceEngine>>,
    pub circuit_breaker: Component<Arc<CircuitBreaker>>,
    pub rate_limiter: Component<AgentRateLimiter>,
    pub slo_engine: Component<Arc<SloEngine>>,
    pub skill_arena: Component<Arc<infrastructure::skills::skill_arena::SkillArena>>,
    pub api_server_secret: Component<Arc<secrecy::SecretString>>,
    pub nurture_url: Option<String>,
    pub nurture_internal_secret: Option<String>,
    pub federation_secret: Component<Arc<secrecy::SecretString>>,
    pub config: Component<Arc<AiomeConfig>>,
    pub gift_engine: Component<Arc<dyn GiftEngine>>,
    pub ekyc_engine: Component<Arc<dyn EkycEngine>>,
    pub ekyc_session_store: Component<Arc<dyn EkycSessionStore>>,
    pub quarantine_store: Component<Arc<dyn QuarantineStore>>,
    pub auth_manager: Component<Arc<dyn AuthManager>>,
    pub system_agent_id: uuid::Uuid,
    pub voice_drm: Component<Arc<VoiceCoreDrm>>,
    pub registry: Component<Arc<RegistryManager>>,
    pub gig_engine: Component<Arc<dyn aiome_core_contracts::gig::GigEngine>>,
    pub intent_generator: Component<Arc<infrastructure::intent::IntentGenerator>>,
    pub intent_firewall: Component<Arc<infrastructure::intent::IntentFirewall>>,
    pub audit_logger: Component<Arc<AsyncAuditLogger>>,
    pub affiliate_adapter: Component<Arc<dyn aiome_core_contracts::traits::AffiliateAdapter>>,
    pub soul_pipeline: Component<
        Arc<
            soul::pipeline::SoulPipeline<
                infrastructure::soul_adapter::CoreDomainAdapter,
                infrastructure::samsara_engine::DefaultSamsaraEngine,
            >,
        >,
    >,
    pub transcription_engine: Component<Arc<dyn TranscriptionEngine>>,
    pub task_dispatcher: Component<Arc<infrastructure::task_orchestrator::TaskDispatcher>>,
    // --- Phase 0-4 Expansion ---
    pub lora_engine: Component<Arc<dyn aiome_core_contracts::traits::LoraEngine>>,
    pub tts_provider: Component<Arc<dyn aiome_core_contracts::traits::TtsProvider>>,
    pub news_service: Component<Arc<dyn aiome_core_contracts::traits::NewsService>>,
    pub live_session_manager: Component<Arc<dyn aiome_core_contracts::traits::LiveSessionManager>>,
    pub syndicate_store: Component<Arc<aiome_commerce::syndicate::UniversalSyndicateStore>>,
    pub hierarchical_router:
        Component<Arc<infrastructure::hierarchical_router::HierarchicalRouter>>,
    // --- Phase 51 Expansion ---
    pub a2a_client: Component<Arc<dyn aiome_core_contracts::a2a::A2aClient>>,
    pub ws_active_connections: Arc<std::sync::atomic::AtomicUsize>,
    pub harness_cache: Component<Arc<infrastructure::skills::harness::HarnessCache>>,
    pub upload_semaphore: Component<Arc<tokio::sync::Semaphore>>,
    pub compute_semaphore: Component<Arc<tokio::sync::Semaphore>>,
    pub disk_quota: Component<Arc<infrastructure::disk_quota::DiskQuotaManager>>,
    pub generative_engine: Component<Arc<dyn aiome_core_contracts::traits::GenerativeEngine>>,
    pub hook_chain: Component<Arc<infrastructure::skills::hooks::HookChain>>,
    pub project_rules_cache: Component<Arc<moka::future::Cache<std::path::PathBuf, String>>>,
    pub cortex_ingester: Component<Arc<infrastructure::cortex_ingester::CortexIngester>>,
    pub cortex_query: Component<Arc<infrastructure::cortex_query::CortexQueryEngine>>,
    pub lora_marketplace:
        Component<Arc<dyn aiome_core_contracts::lora_marketplace::LoraMarketplace>>,
    pub publish_pipeline: Component<Arc<infrastructure::publisher::PublishPipeline>>,
    pub cortex_projector:
        Component<Arc<infrastructure::cortex_file_projector::CortexFileProjector>>,
    // --- Phase 2-D ---
    pub feature_flags_cache: Component<Arc<moka::future::Cache<String, bool>>>,
    // --- Phase 3-D ---
    pub eval_logger: Component<Arc<infrastructure::llm::evaluation_logger::EvaluationLogger>>,
    // --- Phase A2UI ---
    pub a2ui_catalog: Component<Arc<infrastructure::a2ui::AiomeCatalog>>,
    pub quality_gate_store:
        Component<Arc<dyn infrastructure::quality_gate_store::QualityGateStore>>,
    pub hook_manager: Component<Arc<infrastructure::security::hook_manager::HookManager>>,
    pub oxilean_power: Arc<std::sync::atomic::AtomicU32>,
    // --- Phase RLM ---
    pub rlm_client: Component<Arc<dyn aiome_core_contracts::rlm::RlmProvider>>,
    // --- Phase 1 Verification ---
    pub formal_proof_gate: Component<Arc<dyn aiome_contracts::proof::FormalProofGate>>,
    pub gig_updater: Component<Arc<dyn aiome_contracts::gig_metadata::GigMetadataUpdater>>,
}

impl AppState {
    /// 特徴フラグの状態を取得します。mokaキャッシュを優先し、DBヒットを抑えます。
    pub async fn is_feature_enabled(&self, flag: &str) -> bool {
        if let Some(cache) = self.feature_flags_cache.as_opt() {
            if let Some(val) = cache.get(flag).await {
                return val;
            }
        }

        use aiome_core::traits::SettingsOps;
        let val = self.job_queue.get_inner().is_feature_enabled(flag).await;

        if let Some(cache) = self.feature_flags_cache.as_opt() {
            cache.insert(flag.to_string(), val).await;
        }

        val
    }

    /// システム共通のSoulハッシュを計算し取得します。
    pub async fn get_system_soul_hash(&self) -> String {
        use std::hash::{Hash, Hasher};
        let resolver = &self.config.get_inner().resolver;
        let soul = crate::system_instructions::read_app_data_file(resolver, "SOUL.md").await;
        let evolving_soul =
            crate::system_instructions::read_app_data_file(resolver, "EVOLVING_SOUL.md").await;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        format!("{}{}", soul, evolving_soul).hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}
