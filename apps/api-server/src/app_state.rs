/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use std::sync::Arc;
use tokio::sync::Mutex;
use aiome_core::traits::{ArtifactStore, JobQueue};
use aiome_core::llm_provider::LlmProvider;
use aiome_core::commerce::CommerceEngine;
use infrastructure::job_queue::SqliteJobQueue;
use infrastructure::skills::WasmSkillManager;
use infrastructure::skills::forge::SkillForge;
use infrastructure::context_engine::ContextEngine;
use infrastructure::soul_mutator::SoulMutator;
use infrastructure::soul_store::SqliteSoulStore;
use infrastructure::circuit_breaker::CircuitBreaker;
use infrastructure::slo_engine::SloEngine;
use infrastructure::compliance::ekyc::EkycEngine;
use infrastructure::compliance::quarantine::QuarantineStore;
use infrastructure::auth::AuthManager;
use shared::health::HealthMonitor;
use shared::config::AiomeConfig;
use shared::security::SecurityPolicy;
use infrastructure::security::VoiceCoreDrm;
use infrastructure::registry::RegistryManager;
use shared::watchtower::CoreEvent;
use aiome_contracts::commerce::GiftEngine;

use infrastructure::compliance::ekyc_store::EkycSessionStore;

#[derive(Clone)]
pub struct AppState {
    pub health_monitor: Arc<Mutex<HealthMonitor>>,
    pub job_queue: Arc<SqliteJobQueue>,
    pub wasm_skill_manager: Arc<WasmSkillManager>,
    pub skill_forge: Arc<SkillForge>,
    pub docs_path: String,
    pub llm_semaphore: Arc<tokio::sync::Semaphore>,
    pub forge_semaphore: Arc<tokio::sync::Semaphore>,
    pub mcp_sessions: Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, tokio::sync::mpsc::Sender<String>>>,
    >,
    pub mcp_manager: Arc<crate::mcp::client::McpProcessManager>,
    pub artifact_store: Arc<dyn ArtifactStore>,
    pub event_sender: tokio::sync::broadcast::Sender<CoreEvent>,
    pub context_engine: Arc<ContextEngine>,
    pub soul_mutator: Arc<SoulMutator>,
    pub soul_store: Arc<SqliteSoulStore>,
    pub provider: Arc<dyn LlmProvider + Send + Sync>,
    pub autonomous_running: Arc<std::sync::atomic::AtomicBool>,
    pub autonomous_config: Arc<tokio::sync::RwLock<Option<aiome_core::biome::AutonomousConfig>>>,
    pub http_client: reqwest::Client,
    pub docker_failures: Arc<tokio::sync::RwLock<std::collections::HashMap<String, u32>>>,
    pub security_policy: SecurityPolicy,
    pub commerce_engine: Option<Arc<dyn CommerceEngine>>,
    pub circuit_breaker: Arc<CircuitBreaker>,
    pub slo_engine: Arc<SloEngine>,
    pub api_server_secret: Arc<secrecy::SecretString>,
    pub federation_secret: Option<Arc<secrecy::SecretString>>,
    pub config: Arc<AiomeConfig>,
    pub gift_engine: Arc<dyn GiftEngine>,
    pub ekyc_engine: Arc<dyn EkycEngine>,
    pub ekyc_session_store: Arc<dyn EkycSessionStore>,
    pub quarantine_store: Arc<dyn QuarantineStore>,
    pub auth_manager: Arc<dyn AuthManager>,
    pub system_agent_id: uuid::Uuid,
    pub voice_drm: Arc<VoiceCoreDrm>,
    pub registry: Arc<RegistryManager>,
}
