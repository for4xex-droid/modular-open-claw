/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

#![forbid(unsafe_code)]

use aiome_core::llm_provider::EmbeddingProvider;

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
use tracing::{error, info, warn};
use utoipa::OpenApi;

mod api;
mod auth;
mod docker;
mod error;
mod logging;
mod mcp;
mod plugin_loader;
mod routes;
mod skill_handler;
mod stream;

#[cfg(feature = "nurture")]
use commerce_protocol;
#[cfg(feature = "nurture")]
use nurture_api;

use aiome_core::traits::JobQueue;
use shared::health::HealthMonitor;

#[derive(Clone)]
pub struct AppState {
    pub health_monitor: Arc<Mutex<HealthMonitor>>,
    pub job_queue: Arc<infrastructure::job_queue::SqliteJobQueue>,
    pub wasm_skill_manager: Arc<infrastructure::skills::WasmSkillManager>,
    pub skill_forge: Arc<infrastructure::skills::forge::SkillForge>,
    pub docs_path: String,
    pub llm_semaphore: Arc<tokio::sync::Semaphore>,
    pub forge_semaphore: Arc<tokio::sync::Semaphore>,
    pub mcp_sessions: Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, tokio::sync::mpsc::Sender<String>>>,
    >,
    pub mcp_manager: Arc<mcp::client::McpProcessManager>,
    pub artifact_store: Arc<dyn aiome_core::traits::ArtifactStore>,
    pub event_sender: tokio::sync::broadcast::Sender<shared::watchtower::CoreEvent>,
    pub context_engine: Arc<infrastructure::context_engine::ContextEngine>,
    pub soul_mutator: Arc<infrastructure::soul_mutator::SoulMutator>,
    pub soul_store: Arc<infrastructure::soul_store::SqliteSoulStore>,
    pub provider: Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>,
    pub autonomous_running: Arc<std::sync::atomic::AtomicBool>,
    pub autonomous_config: Arc<tokio::sync::RwLock<Option<aiome_core::biome::AutonomousConfig>>>,
    pub http_client: reqwest::Client,
    pub docker_failures: Arc<tokio::sync::RwLock<std::collections::HashMap<String, u32>>>,
    pub security_policy: shared::security::SecurityPolicy,
    pub commerce_engine: Option<Arc<dyn aiome_core::commerce::CommerceEngine>>,
    pub circuit_breaker: Arc<infrastructure::circuit_breaker::CircuitBreaker>,
    pub slo_engine: Arc<infrastructure::slo_engine::SloEngine>,
    pub api_server_secret: Arc<secrecy::SecretString>,
    pub federation_secret: Option<Arc<secrecy::SecretString>>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
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

    let cancel_token = tokio_util::sync::CancellationToken::new();
    let mut plugin_registry = plugin_loader::PluginRegistry::new();

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

    let job_queue = infrastructure::job_queue::SqliteJobQueue::new(&config.db_path)
        .await
        .unwrap_or_else(|e| {
            error!("🚨 Failed to init DB at {}: {}", config.db_path, e);
            std::process::exit(1);
        });
    let job_queue = Arc::new(job_queue);

    let circuit_breaker = Arc::new(infrastructure::circuit_breaker::CircuitBreaker::new(
        infrastructure::circuit_breaker::CircuitBreakerConfig {
            failure_threshold: 5,
            reset_timeout: std::time::Duration::from_secs(60),
        },
    ));

    let slo_engine = Arc::new(infrastructure::slo_engine::SloEngine::new(
        infrastructure::slo_engine::SloConfig {
            error_budget_max: 100,
            warning_threshold: 80,
        },
        chrono::Duration::hours(24),
    ));

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .redirect(reqwest::redirect::Policy::none()) // C5: Harden SSRF by disabling redirects
        .build()
        .unwrap_or_default();

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
    });

    let bg_instance = Arc::new(infrastructure::llm::dynamic::BackgroundLlmProvider {
        jq: job_queue.clone(),
        client: http_client.clone(),
        fallback_host: config.ollama_host.clone(),
        fallback_model: config.ollama_model.clone(),
        gemini_api_key: config.gemini_api_key.clone(),
        openai_api_key: config.openai_api_key.clone(),
        anthropic_api_key: config.anthropic_api_key.clone(),
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

    let artifact_store = infrastructure::artifact_store::SqliteArtifactStore::new(
        job_queue.get_pool().clone(),
        std::path::PathBuf::from("workspace/artifacts"),
    )
    .with_embeddings(embed_provider.clone());

    let artifact_store = Arc::new(artifact_store);

    let wasm_skill_manager = Arc::new(
        infrastructure::skills::WasmSkillManager::new("workspace/skills", "workspace")
            .unwrap_or_else(|e| {
                error!("🚨 Failed to initialize WasmSkillManager: {}", e);
                std::process::exit(1);
            }),
    );
    let skill_forge = Arc::new(infrastructure::skills::forge::SkillForge::new(
        "workspace/forge",
        "workspace/skills/custom",
    ));

    let llm_semaphore = Arc::new(tokio::sync::Semaphore::new(1)); // Ollama handles 1 request at a time
    let forge_semaphore = Arc::new(tokio::sync::Semaphore::new(1));
    let event_sender = tokio::sync::broadcast::channel::<shared::watchtower::CoreEvent>(100).0;

    skill_forge.ensure_forge_workspace().unwrap_or_else(|e| {
        error!("🚨 Failed to initialize skill_forge workspace: {}", e);
        std::process::exit(1);
    });

    let allowed_origins: Vec<HeaderValue> = config
        .allowed_origins
        .iter()
        .filter_map(|s| s.parse::<HeaderValue>().ok())
        .collect();
    info!("🌐 [CORS] Active origins: {:?}", config.allowed_origins);

    info!(
        "🌐 [CORS] Effective Allowed Origins: {:?}",
        config.allowed_origins
    );
    let cors_layer = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);

    #[cfg(feature = "nurture")]
    let (nurture_state, commerce_engine) = {
        info!("💰 [NURTURE] Initializing Economy Engine via Plugin Architecture...");
        let system_id = job_queue.get_system_agent_id().await.unwrap_or_else(|e| {
            error!("🚨 [NURTURE] Failed to get system agent ID: {}", e);
            uuid::Uuid::nil()
        });
        let nurture_plugin = nurture_api::plugin::create_plugin(
            job_queue.get_pool().clone(),
            system_id,
            event_sender.clone(),
            job_queue.clone(),
            cancel_token.clone(),
        )
        .await
        .unwrap_or_else(|e| {
            error!("💰 [NURTURE] Failed to create plugin: {}", e);
            std::process::exit(1);
        });

        plugin_registry.register(nurture_plugin.clone());
        let ce = nurture_plugin.commerce_engine();

        let ns = nurture_api::state::AppState::init(
            job_queue.get_pool().clone(),
            job_queue.clone(),
            nurture_api::state::EconomyPolicy::default(),
            commerce_protocol::identity::ActorId(system_id),
            cancel_token.clone(),
        )
        .await
        .unwrap_or_else(|e| {
            error!("🚨 [NURTURE] Failed to initialize state: {}", e);
            std::process::exit(1);
        });

        (ns, ce)
    };

    #[cfg(not(feature = "nurture"))]
    let commerce_engine = Some(Arc::new(infrastructure::commerce_mock::MockCommerceEngine)
        as Arc<dyn aiome_core::commerce::CommerceEngine>);

    let api_server_secret_raw = std::env::var("API_SERVER_SECRET").unwrap_or_else(|_| {
        warn!("⚠️ API_SERVER_SECRET not set, using insecure default!");
        "dev_secret".to_string()
    });
    let federation_secret_raw = std::env::var("FEDERATION_SECRET").ok();

    // SEC: Wipe sensitive environment variables immediately after loading
    std::env::remove_var("API_SERVER_SECRET");
    std::env::remove_var("FEDERATION_SECRET");

    let api_server_secret = Arc::new(secrecy::SecretString::from(api_server_secret_raw.clone()));
    let federation_secret = federation_secret_raw.map(|s| Arc::new(secrecy::SecretString::from(s)));

    let state = AppState {
        health_monitor,
        job_queue: job_queue.clone(),
        wasm_skill_manager,
        skill_forge,
        docs_path: docs_path.to_string(),
        llm_semaphore: llm_semaphore.clone(),
        forge_semaphore: forge_semaphore.clone(),
        mcp_sessions: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        mcp_manager: Arc::new(mcp::client::McpProcessManager::new()),
        artifact_store: artifact_store.clone(),
        event_sender: event_sender.clone(),
        context_engine: Arc::new(infrastructure::context_engine::ContextEngine::new(
            provider.clone(),
            job_queue.clone(),
            llm_semaphore.clone(),
        )),
        soul_mutator: Arc::new(infrastructure::soul_mutator::SoulMutator::new(
            provider.clone(),
            std::path::PathBuf::from("workspace"),
        )),
        soul_store: Arc::new(infrastructure::soul_store::SqliteSoulStore::new(Arc::new(
            job_queue.get_pool().clone(),
        ))),
        provider: provider.clone(),
        autonomous_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        autonomous_config: Arc::new(tokio::sync::RwLock::new(None)),
        http_client,
        docker_failures: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        security_policy: {
            let mut policy = shared::security::SecurityPolicy::default();
            for tool in plugin_registry.registered_tools() {
                policy.register_tool(&tool);
            }
            policy
        },
        commerce_engine,
        circuit_breaker: circuit_breaker.clone(),
        slo_engine: slo_engine.clone(),
        api_server_secret,
        federation_secret: federation_secret.clone(),
    };

    // RS-1: Pre-load soul cache to avoid DB I/O on hot paths
    let store = state.soul_store.clone();
    tokio::spawn(async move {
        match store.load_into_cache("system-soul").await {
            Ok(true) => tracing::info!("🛡️ Soul Memory Connection established (Cache Pre-loaded)"),
            Ok(false) => tracing::info!("🛡️ Soul Memory: No existing soul found (Genesis State)"),
            Err(e) => tracing::error!("🚨 Failed to pre-load soul cache: {:?}", e),
        }
    });

    let app = build_app(
        state.clone(),
        cors_layer,
        static_path,
        #[cfg(feature = "nurture")]
        nurture_state,
        plugin_registry,
    );

    // Initial Security Check (C1)
    if api_server_secret_raw == "dev_secret" || api_server_secret_raw.is_empty() {
        if cfg!(debug_assertions) {
            warn!("🚨 [SECURITY CRITICAL] API_SERVER_SECRET is set to fallback value or empty.");
            warn!("🚨 Please set a strong random secret in your .env file immediately.");
        } else {
            error!("🚨 [FATAL SECURITY ERROR] API_SERVER_SECRET IS INSECURE OR MISSING!");
            error!("🚨 Aiome will NOT start in release mode without a strong secret.");
            panic!("Insecure API_SERVER_SECRET in release build");
        }
    }

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3015".to_string())
        .parse()
        .unwrap_or_else(|_| {
            error!("🚨 [CRITICAL] Invalid PORT format in environment");
            std::process::exit(1);
        });
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("🌌 Aiome Management Console listening on {}", addr);

    let token = cancel_token;
    let jq_clone = job_queue.clone();
    let token_bg = token.clone();
    let federation_secret_bg = federation_secret.clone();
    let http_client_bg = state.http_client.clone();
    tokio::spawn(async move {
        let token = token_bg;
        let token_ws = token.clone();
        let fed_secret = federation_secret_bg;
        // Initialize LLM for background tasks (using bg_provider to avoid Ollama competition)
        let immune_system =
            infrastructure::immune_system::AdaptiveImmuneSystem::new(bg_provider.clone());

        // Heartbeat Wakeup Setup (Phase 1)
        let wakeup_provider = bg_provider.clone();
        let llm_semaphore = llm_semaphore.clone();
        let event_sender = event_sender.clone();
        let heartbeat_service = infrastructure::heartbeat_wakeup::HeartbeatWakeupService::new(
            wakeup_provider.clone(),
            llm_semaphore.clone(),
            std::path::PathBuf::from(
                std::env::var("WORKSPACE_DIR").unwrap_or_else(|_| ".".to_string()),
            ),
        );
        let crystallizer = infrastructure::memory_crystallizer::MemoryCrystallizer::new(
            wakeup_provider.clone(),
            jq_clone.clone(),
            llm_semaphore.clone(),
        );
        let learner =
            infrastructure::user_learner::UserLearner::new(wakeup_provider, llm_semaphore.clone());
        let mut wakeup_counter = 0;

        // 🌐 2. Federation Sync: Connect to Samsara Hub WebSocket for real-time updates
        let hub_ws_url = std::env::var("SAMSARA_HUB_WS").unwrap_or_else(|_| {
            config
                .samsara_hub_url
                .replace("http://", "ws://")
                .replace("https://", "wss://")
                + "/api/v1/federation/ws"
        });

        use secrecy::ExposeSecret;
        let hub_secret_val = fed_secret
            .as_ref()
            .map(|s| s.expose_secret().to_string())
            .unwrap_or_default();
        if hub_secret_val.is_empty() {
            warn!("⚠️ [BackgroundWorker] FEDERATION_SECRET is empty. Federation might fail.");
        }
        let jq_ws = jq_clone.clone();
        let provider_ws = provider.clone();

        tokio::spawn(async move {
            let token = token_ws;
            use aiome_core::contracts::HubMessage;
            use futures_util::{SinkExt, StreamExt};
            use tokio_tungstenite::tungstenite::client::IntoClientRequest;

            let self_node_id = jq_ws.get_node_id().await.unwrap_or_default();
            info!(
                "⚙️ [FederationWorker] Starting with Node ID: {}",
                self_node_id
            );
            let immune_system =
                infrastructure::immune_system::AdaptiveImmuneSystem::new(provider_ws);

            loop {
                if token.is_cancelled() {
                    info!("🛑 [FederationWorker] Shutdown requested. Exiting loop.");
                    break;
                }

                let request_res = hub_ws_url.clone().into_client_request();
                let mut request = match request_res {
                    Ok(req) => req,
                    Err(e) => {
                        error!(
                            "🛑 [FederationWorker] Invalid WS URL: {}. Retrying in 30s...",
                            e
                        );
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        continue;
                    }
                };

                let auth_val = format!("Bearer {}", hub_secret_val).parse();
                match auth_val {
                    Ok(val) => {
                        request.headers_mut().insert("Authorization", val);
                    }
                    Err(e) => {
                        error!("🛑 [FederationWorker] Failed to parse Authorization header: {}. Retrying in 30s...", e);
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        continue;
                    }
                }

                match tokio_tungstenite::connect_async(request).await {
                    Ok((mut ws_stream, _)) => {
                        info!("🌐 [FederationWorker] Connected to Samsara Hub.");
                        while let Some(msg) = ws_stream.next().await {
                            match msg {
                                Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                                    if let Ok(hub_msg) = serde_json::from_str::<HubMessage>(&text) {
                                        match hub_msg {
                                            HubMessage::NewImmuneRule(rule) => {
                                                // Gap 3 Mitigation: Echo Loop Prevention
                                                if rule.node_id == self_node_id {
                                                    continue;
                                                }
                                                info!("🛡️ [FederationWorker] Received remote rule: {}", rule.pattern);
                                                let _ = jq_ws.store_immune_rule(&rule).await;
                                            }
                                            HubMessage::NewKarma(karma) => {
                                                if karma.node_id == self_node_id {
                                                    continue;
                                                }
                                                info!("🧬 [FederationWorker] Received remote karma: {}", karma.id);
                                                // Normally handled by REST sync, but real-time push is also possible
                                            }
                                            HubMessage::LaggedForceSync { .. } => {
                                                warn!("⚠️ [FederationWorker] Hub reported lag. Forcing full sync in next maintenance cycle...");
                                                // Trigger via system state or a channel if needed, for now just wait for BG worker sync
                                            }
                                            HubMessage::Ping { client_time: _ } => {
                                                let _now_rfc = chrono::Utc::now().to_rfc3339();
                                                let pong = HubMessage::Pong {
                                                    server_time: chrono::Utc::now().to_rfc3339(),
                                                };
                                                if let Ok(text) = serde_json::to_string(&pong) {
                                                    let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(text.into())).await;
                                                }
                                            }
                                            HubMessage::BiomeRelay(msg) => {
                                                if msg.recipient_pubkey != self_node_id {
                                                    continue;
                                                }
                                                info!("📫 [FederationWorker] Incoming Biome Message from {}", msg.sender_pubkey);

                                                // 1. Signature Check
                                                let mut valid = false;
                                                let payload = format!(
                                                    "{}:{}:{}",
                                                    msg.sender_pubkey,
                                                    msg.topic_id,
                                                    msg.lamport_clock
                                                );
                                                if let (Ok(pubkey_bytes), Ok(sig_bytes)) = (
                                                    base64::engine::general_purpose::STANDARD
                                                        .decode(&msg.sender_pubkey),
                                                    base64::engine::general_purpose::STANDARD
                                                        .decode(&msg.signature),
                                                ) {
                                                    let pubkey_res: Result<[u8; 32], _> =
                                                        pubkey_bytes.as_slice().try_into();
                                                    if let Ok(pubkey_arr) = pubkey_res {
                                                        if let (Ok(pubkey), Ok(sig)) = (
                                                            ed25519_dalek::VerifyingKey::from_bytes(
                                                                &pubkey_arr,
                                                            ),
                                                            ed25519_dalek::Signature::from_slice(
                                                                &sig_bytes,
                                                            ),
                                                        ) {
                                                            use ed25519_dalek::Verifier;
                                                            if pubkey
                                                                .verify(payload.as_bytes(), &sig)
                                                                .is_ok()
                                                            {
                                                                valid = true;
                                                            }
                                                        }
                                                    }
                                                }

                                                if !valid {
                                                    warn!("🛡️ [FederationWorker] Invalid Biome Signature from {}", msg.sender_pubkey);
                                                    continue;
                                                }

                                                // 2. Immune system Check (Intent analysis)
                                                if let Ok(Some(rule)) = immune_system
                                                    .verify_intent(&msg.content, jq_ws.as_ref())
                                                    .await
                                                {
                                                    warn!("🛡️ [FederationWorker] Biome Message blocked by Immune System! Pattern: {}", rule.pattern);
                                                    continue;
                                                }

                                                // 3. Store
                                                let _ = sqlx::query("INSERT INTO biome_messages (sender_pubkey, recipient_pubkey, topic_id, content, karma_root_cid, signature, lamport_clock, encryption) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                                                    .bind(&msg.sender_pubkey).bind(&msg.recipient_pubkey).bind(&msg.topic_id).bind(&msg.content).bind(&msg.karma_root_cid).bind(&msg.signature).bind(msg.lamport_clock as i64).bind(&msg.encryption)
                                                    .execute(jq_ws.get_pool()).await;

                                                let _ = sqlx::query("INSERT INTO biome_peers (pubkey) VALUES (?) ON CONFLICT(pubkey) DO UPDATE SET last_seen_at = datetime('now')")
                                                    .bind(&msg.sender_pubkey).execute(jq_ws.get_pool()).await;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("⚠️ [FederationWorker] WS Stream Error: {:?}", e);
                                    break;
                                }
                                _ => {}
                            }
                        }
                        warn!("🔌 [FederationWorker] WebSocket disconnected. Recalibrating...");
                    }
                    Err(e) => {
                        warn!(
                            "⚠️ [FederationWorker] Connection failed: {:?}. Retrying...",
                            e
                        );
                    }
                }
                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(10)) => {},
                    _ = token.cancelled() => {
                        info!("🛑 [FederationWorker] Cancellation received during wait. Exiting.");
                        break;
                    }
                }
            }
        });

        let soul_store_bg = state.soul_store.clone();
        let embed_provider: Arc<dyn aiome_core::llm_provider::EmbeddingProvider> =
            bg_instance.clone();
        let adapter = infrastructure::soul_adapter::CoreDomainAdapter::new(
            jq_clone.clone(),
            Some(embed_provider),
        );
        use soul::adapter::SoulDomainAdapter;
        let distillation_prompt = adapter.distillation_system_prompt().to_string();

        let pipeline = soul::pipeline::SoulPipeline::new(
            adapter,
            infrastructure::samsara_engine::DefaultSamsaraEngine::new(
                bg_provider.clone(),
                distillation_prompt,
            ),
        );

        loop {
            if token.is_cancelled() {
                info!("🛑 [BackgroundWorker] Shutdown requested. Cleaning up...");
                break;
            }

            // 🌟 0. Evolution: Sync Samsara Level and handle Behavioral Shift
            let stats = jq_clone.get_agent_stats().await.unwrap_or_default();
            let current_level = stats.level;
            let mut collected_experiences: Vec<soul::model::Experience> = Vec::new();

            match jq_clone.sync_samsara_level().await {
                Ok(Some(aiome_core::contracts::SamsaraEvent::LevelUp {
                    old_level,
                    new_level,
                })) => {
                    info!(
                        "🌟 [Evolution] Level Up Detected: {} -> {}",
                        old_level, new_level
                    );
                    let mutator = infrastructure::soul_mutator::SoulMutator::new(
                        provider.clone(),
                        std::path::PathBuf::from("workspace"),
                    )
                    .with_prosecutor(provider.clone()); // Self-prosecution for MVP

                    if let Err(e) = mutator
                        .evolve_tactics(jq_clone.as_ref(), old_level, new_level)
                        .await
                    {
                        warn!("⚠️ [Evolution] Behavioral Shift failed: {:?}", e);
                    }

                    collected_experiences.push(soul::model::Experience {
                        id: uuid::Uuid::new_v4().to_string(),
                        domain: "evolution.level_up".to_string(),
                        content: format!(
                            "Agent evolved from level {} to {}. I must utilize this new power.",
                            old_level, new_level
                        ),
                        outcome_valence: 0.8,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        original_prediction: 0.0,
                    });
                }
                Ok(Some(other_event)) => {
                    info!("🌟 [Evolution] Unhandled Samsara event: {:?}", other_event);
                }
                Ok(None) => {}
                Err(e) => warn!("⚠️ [Evolution] Level sync failed: {:?}", e),
            }

            // 💤 0.5 Contemplation: Dream State (when idle)
            let pending_jobs = jq_clone.get_pending_job_count().await.unwrap_or(0);
            if pending_jobs == 0 {
                let dream_state = infrastructure::dream_state::DreamState::new();
                let search_api_key =
                    std::env::var("SEARCH_API_KEY").unwrap_or_else(|_| "none".to_string());
                let trend_sonar =
                    infrastructure::trend_sonar::ExternalTrendSonar::new(search_api_key);

                match dream_state
                    .dream(jq_clone.as_ref(), &trend_sonar, current_level)
                    .await
                {
                    Ok(Some(insight)) => {
                        collected_experiences.push(soul::model::Experience {
                            id: uuid::Uuid::new_v4().to_string(),
                            domain: "state.dream".to_string(),
                            content: insight,
                            outcome_valence: 0.5,
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            original_prediction: 0.0,
                        });
                    }
                    Ok(None) => {}
                    Err(e) => warn!("⚠️ [DreamState] Contemplation failed: {:?}", e),
                }
            }

            // 🛡️ 1. Auto-Healing: Analyze threats and generate new immune rules
            // Use try_acquire + short timeout to avoid blocking front-end requests
            if let Ok(_bg_permit) = llm_semaphore.try_acquire() {
                info!(
                    "⚙️ [BackgroundWorker] Starting autonomous threat analysis (Auto-Healing)..."
                );
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(30),
                    immune_system.analyze_threats(jq_clone.as_ref()),
                )
                .await
                {
                    Ok(Ok(n)) if n > 0 => {
                        info!("🛡️ [BackgroundWorker] {} new immune rules generated.", n);
                        collected_experiences.push(soul::model::Experience {
                            id: uuid::Uuid::new_v4().to_string(),
                            domain: "security.immune_response".to_string(),
                            content: format!("{} threats detected and neutralized via immune system feedback loop.", n),
                            outcome_valence: -0.6,
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            original_prediction: 0.0,
                        });
                    }
                    Ok(Ok(_)) => info!("🛡️ [BackgroundWorker] No new threats identified."),
                    Ok(Err(e)) => warn!("⚠️ [BackgroundWorker] Threat analysis failed: {:?}", e),
                    Err(_) => {
                        warn!("⏭️ [BackgroundWorker] Threat analysis timed out (30s), skipping.")
                    }
                }
            } else {
                info!("⏭️ [BackgroundWorker] LLM busy, skipping threat analysis.");
            }

            // 🧬 1.5 Soul Mutation: Attempt autonomous evolution
            let mutator = infrastructure::soul_mutator::SoulMutator::new(
                bg_provider.clone(),
                std::path::PathBuf::from("workspace"),
            )
            .with_prosecutor(bg_provider.clone());
            if let Ok(_bg_permit) = llm_semaphore.try_acquire() {
                info!("⚙️ [BackgroundWorker] Checking for Soul Mutation (Autonomous Evolution)...");
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(30),
                    mutator.transmute(jq_clone.as_ref()),
                )
                .await
                {
                    Ok(Ok(true)) => info!("🧬 [BackgroundWorker] Soul mutated successfully."),
                    Ok(Ok(false)) => info!("🧬 [BackgroundWorker] No soul mutation triggered."),
                    Ok(Err(e)) => warn!("⚠️ [BackgroundWorker] Soul mutation failed: {:?}", e),
                    Err(_) => {
                        warn!("⏭️ [BackgroundWorker] Soul mutation timed out (30s), skipping.")
                    }
                }
            } else {
                info!("⏭️ [BackgroundWorker] LLM busy, skipping soul mutation.");
            }
            // 🧬 1.6 Soul Engine (v3): Process queued experience & Rebirth
            // Pipeline and Store are initialized outside the loop (DS-2 fixed)

            // For now, assume a single target AgentSoul (id: "system-soul")
            let system_soul_id = "system-soul";
            match soul_store_bg.load_soul(system_soul_id).await {
                Ok(Some(mut agent_soul)) => {
                    // RS-5: Initial Prompt Generation Fallback
                    if agent_soul.generation == 1
                        && agent_soul.instinct.prompt_fragment.is_empty()
                        && agent_soul.experience_buffer.len() >= 100
                    {
                        use soul::engine::SamsaraEngine;
                        tracing::info!("🌟 [SoulEngine] Triggering initial instinct distillation (Fallback)...");
                        if let Ok(new_instinct) = pipeline.engine.distill(&agent_soul).await {
                            agent_soul.instinct = new_instinct;
                            let _ = soul_store_bg.save_soul(&agent_soul).await;
                        }
                    }

                    // Extract recent unhandled experiences (P-2 / P-3 integrations)
                    if !collected_experiences.is_empty() {
                        let mut trigger_save = false;
                        for exp in collected_experiences {
                            match pipeline.process_experience(&mut agent_soul, exp).await {
                                Ok(Some(new_soul)) => {
                                    info!("🌟 [SoulEngine] Samsara Triggered! Soul reborn to generation {}", new_soul.generation);
                                    let _ = soul_store_bg.save_soul(&new_soul).await;
                                    agent_soul = new_soul; // Update active soul object
                                }
                                Ok(None) => trigger_save = true,
                                Err(e) => warn!("⚠️ [SoulEngine] Pipeline error: {:?}", e),
                            }
                        }
                        if trigger_save {
                            if let Err(e) = soul_store_bg.save_soul(&agent_soul).await {
                                warn!("⚠️ [SoulEngine] Failed to save updated soul: {}", e);
                            }
                        }
                    } else if wakeup_counter % 12 == 0 {
                        // Periodic passive save (e.g. 1 hour = 12 * 5min)
                        let _ = soul_store_bg.save_soul(&agent_soul).await;
                    }
                }
                Ok(None) => {
                    // Initialize if missing
                    let mut fresh_soul = soul::model::AgentSoul::new(system_soul_id.to_string());
                    // Apply genesis hash logic
                    fresh_soul.compute_hash();
                    if let Err(e) = soul_store_bg.save_soul(&fresh_soul).await {
                        warn!("⚠️ [SoulEngine] Failed to initialize system soul: {}", e);
                    } else {
                        info!(
                            "🧬 [SoulEngine] Initialized new AgentSoul for {}",
                            system_soul_id
                        );
                    }
                }
                Err(e) => warn!("⚠️ [SoulEngine] Failed to load system soul: {:?}", e),
            }

            // 🎭 1.7 Autonomous Expression (Phase 4): Self-Expression based on Karma
            if wakeup_counter % 5 == 0 {
                if let Ok(true) = jq_clone.get_auto_expression_enabled().await {
                    if let Ok(_bg_permit) = llm_semaphore.try_acquire() {
                        info!("⚙️ [BackgroundWorker] Auto-Expression is enabled. Generating...");
                        let karma = jq_clone.fetch_all_karma(5).await.unwrap_or_default();
                        if !karma.is_empty() {
                            let soul_prompt = mutator.get_active_prompt().await.unwrap_or_default();
                            match tokio::time::timeout(tokio::time::Duration::from_secs(30), aiome_core::expression::engine::ExpressionEngine::generate(&karma, &soul_prompt, bg_provider.as_ref())).await {
                                Ok(Ok(expr)) => {
                                    let _ = jq_clone.store_expression(&expr).await;
                                    info!("🎭 [BackgroundWorker] Autonomous Expression generated: {}", expr.emotion);
                                },
                                Ok(Err(e)) => warn!("⚠️ [BackgroundWorker] Expression generation failed: {:?}", e),
                                Err(_) => warn!("⏭️ [BackgroundWorker] Expression generation timed out (30s), skipping."),
                            }
                        }
                    } else {
                        info!("⏭️ [BackgroundWorker] LLM busy, skipping auto-expression.");
                    }
                }
            }

            // 🌐 2. Swarm Sync: Push local data and Sync remote data via REST API
            info!("🌐 [BackgroundWorker] Starting Swarm Sync cycle...");
            let hub_base = config.samsara_hub_url.clone();
            let hub_secret_val = match fed_secret.as_ref() {
                Some(s) => s.expose_secret().to_string(),
                None => {
                    error!("🛑 [BackgroundWorker] FEDERATION_SECRET missing. Skipping Swarm Sync.");
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    continue;
                }
            };
            let hub_secret = hub_secret_val;
            let client = http_client_bg.clone();

            use aiome_core::contracts::{
                FederationPushRequest, FederationSyncRequest, FederationSyncResponse,
            };

            // 2-A. Push local unfederated data
            if let Ok((karmas, rules)) = jq_clone.fetch_unfederated_data().await {
                let karmas: Vec<aiome_core::contracts::FederatedKarma> = karmas;
                let rules: Vec<aiome_core::contracts::ImmuneRule> = rules;
                if !karmas.is_empty() || !rules.is_empty() {
                    let self_node_id = jq_clone.get_node_id().await.unwrap_or_default();
                    info!(
                        "📤 [BackgroundWorker] Pushing {} Karmas and {} Rules to Hub.",
                        karmas.len(),
                        rules.len()
                    );
                    let push_req = FederationPushRequest {
                        node_id: self_node_id,
                        karmas,
                        rules,
                        arena_matches: vec![],
                    };

                    let res = client
                        .post(format!("{}/api/v1/federation/push", hub_base))
                        .header("Authorization", format!("Bearer {}", hub_secret))
                        .json(&push_req)
                        .send()
                        .await;

                    if let Ok(r) = res {
                        if r.status().is_success() {
                            let k_ids = push_req.karmas.into_iter().map(|k| k.id).collect();
                            let r_ids = push_req.rules.into_iter().map(|r| r.id).collect();
                            let _ = jq_clone.mark_as_federated(k_ids, r_ids).await;
                            info!("✅ [BackgroundWorker] Cloud Push successful.");
                        } else {
                            warn!("⚠️ [BackgroundWorker] Hub rejected Push: {:?}", r.status());
                        }
                    }
                }
            }

            // 2-B. Sync remote approved data with Stateless Pagination (Flaw 2 Defense)
            info!("📥 [BackgroundWorker] Syncing from Hub: {}", hub_base);
            loop {
                let last_sync = jq_clone
                    .get_peer_sync_time("samsara-hub")
                    .await
                    .unwrap_or(None);
                let sync_req = FederationSyncRequest {
                    node_id: jq_clone.get_node_id().await.unwrap_or_default(),
                    since: last_sync,
                    protocol_version: "1.0".to_string(),
                };

                let res = client
                    .post(format!("{}/api/v1/federation/sync", hub_base))
                    .header("Authorization", format!("Bearer {}", hub_secret))
                    .json(&sync_req)
                    .send()
                    .await;

                if let Ok(resp) = res {
                    if resp.status().is_success() {
                        if let Ok(sync_res) = resp.json::<FederationSyncResponse>().await {
                            let karma_len = sync_res.new_karmas.len();
                            let rule_len = sync_res.new_immune_rules.len();
                            let has_more = sync_res.has_more;
                            let server_time = sync_res.server_time.clone();

                            if karma_len > 0 || rule_len > 0 {
                                info!("📥 [BackgroundWorker] Syncing {} new items from Hub (has_more: {}).", karma_len + rule_len, has_more);
                                let _ = jq_clone
                                    .import_federated_data(
                                        sync_res.new_karmas,
                                        sync_res.new_immune_rules,
                                        sync_res.new_arena_matches,
                                    )
                                    .await;
                            }

                            // Update last sync time to the server's processed timestamp for this batch
                            let _ = jq_clone
                                .update_peer_sync_time("samsara-hub", &server_time)
                                .await;

                            if !has_more {
                                break; // Batch complete
                            }
                            // Continue loop for next page
                        } else {
                            break;
                        }
                    } else {
                        warn!(
                            "⚠️ [BackgroundWorker] Hub rejected Sync: {:?}",
                            resp.status()
                        );
                        break;
                    }
                } else {
                    break;
                }
            }

            // 3. Content Publishing: Pick up 'publication' jobs
            if let Ok(Some(job)) = jq_clone.dequeue(&["publication"]).await {
                use infrastructure::publisher::{mock_x::MockXPublisher, PublishPipeline};
                let pipeline = PublishPipeline::new(vec![Box::new(MockXPublisher)]);

                let metadata =
                    serde_json::from_str(job.karma_directives.as_deref().unwrap_or("{}"))
                        .unwrap_or(serde_json::json!({}));
                let platform = metadata["platform"].as_str().unwrap_or("X");

                // For 'publication' jobs, the 'topic' field contains the content string
                let content = job.topic.clone();
                let artifacts_res: Result<Vec<String>, _> =
                    serde_json::from_str(job.output_artifacts.as_deref().unwrap_or("[]"));
                let artifacts: Vec<std::path::PathBuf> = artifacts_res
                    .unwrap_or_default()
                    .into_iter()
                    .map(std::path::PathBuf::from)
                    .collect();

                match pipeline
                    .run_job(platform, &content, &artifacts, &metadata)
                    .await
                {
                    Ok(cid) => {
                        let _ = jq_clone.complete_job(&job.id, None).await;
                        let _ = jq_clone.link_sns_data(&job.id, platform, &cid).await;
                        info!(
                            "✅ [BackgroundWorker] Publication successful (ID: {}).",
                            cid
                        );
                    }
                    Err(e) => {
                        let _ = jq_clone.fail_job(&job.id, &e.to_string()).await;
                        warn!("⚠️ [BackgroundWorker] Publication failed: {:?}", e);
                    }
                }
            }

            // 5. Memory Evolution: Procedural Forgetting Sweep
            if let Ok(archived) = jq_clone.karma_decay_sweep().await {
                if archived > 0 {
                    info!("♻️ [BackgroundWorker] Memory Evolution: Archived {} faint memories via decay sweep.", archived);
                }
            }

            // Adaptive Intelligence v1.0: Karma Tier Maintenance (6-hour cycle)
            if wakeup_counter % 72 == 0 {
                if let Err(e) = jq_clone.run_karma_tier_maintenance().await {
                    warn!(
                        "⚠️ [BackgroundWorker] Karma tier maintenance failed: {:?}",
                        e
                    );
                }
            }

            // 4. Storage GC: Maintain clean environment (Threshold: 10GB)
            if let Ok(purged) = jq_clone.storage_gc(10.0).await {
                if purged > 0 {
                    info!(
                        "♻️ [BackgroundWorker] Storage GC: Purged {} old artifacts.",
                        purged
                    );
                }
            }

            // 6. Heartbeat Wakeup Ping (Phase 1) - Every 30 maintenance cycles (~30 mins)
            if wakeup_counter % 30 == 0 {
                if let Some(msg) = heartbeat_service.run_wakeup_ping().await {
                    let _ = event_sender.send(shared::watchtower::CoreEvent::ProactiveTalk {
                        message: msg,
                        channel_id: 0,
                    });
                    info!("💓 [BackgroundWorker] Heartbeat: Proactive talk dispatched.");
                }
            }
            wakeup_counter = (wakeup_counter + 1) % 1440; // Prevent overflow, reset dailyish

            // 7. Memory Crystallization (Phase 2) - Daily maintenance
            if wakeup_counter == 0 {
                info!("💎 [BackgroundWorker] Memory Evolution: Starting Crystallization cycle...");
                let _ = crystallizer.run_distillation_cycle().await;
            }

            // 8. User Learning (Phase 2) - Hourly preference updates
            if wakeup_counter % 60 == 0 {
                if let Ok(channels) = jq_clone.fetch_undistilled_chats_by_channel().await {
                    for (channel_id, messages) in channels {
                        let summary = messages
                            .iter()
                            .map(|(_, role, content)| format!("{}: {}", role, content))
                            .collect::<Vec<_>>()
                            .join("\n");
                        if learner.learn_from_session(&summary).await.unwrap_or(false) {
                            let last_id = messages.last().map(|(id, ..)| *id).unwrap_or(0);
                            let _ = jq_clone.mark_chats_as_distilled(&channel_id, last_id).await;
                        }
                    }
                }
            }

            // 9. Knowledge Indexing (Phase 21-B) - Refresh project knowledge every 12 cycles (~1 hour)
            // Trigger on first cycle (counter=1) for immediate indexing
            if wakeup_counter == 1 || wakeup_counter % 12 == 0 {
                let ws_root =
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                let indexer = infrastructure::knowledge_indexer::ProjectKnowledgeIndexer::new(
                    artifact_store.clone(),
                    jq_clone.get_pool().clone(),
                    ws_root,
                );
                let _ = indexer.run_indexing().await;
            }

            // 10. SQLite Global Backup (Tier 5: Architecture) - Every 2 hours (24 cycles)
            if wakeup_counter % 24 == 0 {
                info!("💾 [BackgroundWorker] Starting SQLite periodic backup...");
                let backup_dir = std::path::Path::new("workspace/backups");
                if !backup_dir.exists() {
                    let _ = std::fs::create_dir_all(backup_dir);
                }
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
                let backup_path = backup_dir.join(format!("aiome_{}.db", timestamp));

                // Using VACUUM INTO for online backup (safe even if the file is being written to)
                let pool = jq_clone.get_pool();
                // Ensure we use the absolute path for SQLite
                if let Ok(abs_backup_path) = std::fs::canonicalize(backup_dir)
                    .map(|p| p.join(format!("aiome_{}.db", timestamp)))
                {
                    let query = format!(
                        "VACUUM INTO '{}'",
                        abs_backup_path
                            .to_str()
                            .unwrap_or_default()
                            .replace("'", "''")
                    );
                    match sqlx::query(&query).execute(pool).await {
                        Ok(_) => info!(
                            "💾 [BackgroundWorker] Backup successful: {:?}",
                            abs_backup_path
                        ),
                        Err(e) => warn!("⚠️ [BackgroundWorker] Backup failed: {:?}", e),
                    }
                } else {
                    // Fallback to relative if canonicalize fails (e.g. dir just created)
                    let query = format!(
                        "VACUUM INTO '{}'",
                        backup_path.to_str().unwrap_or_default().replace("'", "''")
                    );
                    match sqlx::query(&query).execute(pool).await {
                        Ok(_) => info!(
                            "💾 [BackgroundWorker] Backup successful (relative): {:?}",
                            backup_path
                        ),
                        Err(e) => warn!("⚠️ [BackgroundWorker] Backup failed: {:?}", e),
                    }
                }

                // Cleanup old backups (keep last 5)
                if let Ok(entries) = std::fs::read_dir(backup_dir) {
                    let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
                    paths.sort_by(|a, b| {
                        let ma = a
                            .metadata()
                            .and_then(|m| m.modified())
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        let mb = b
                            .metadata()
                            .and_then(|m| m.modified())
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        mb.cmp(&ma)
                    });

                    if paths.len() > 5 {
                        for old_path in paths.iter().skip(5) {
                            let _ = std::fs::remove_file(old_path);
                        }
                    }
                }
            }

            // Sleep for 5 minutes before next maintenance cycle (Pattern B: longer interval for Ollama background)
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(300)) => {},
                _ = token.cancelled() => {
                    info!("🛑 [BackgroundWorker] Cancellation received. Exiting.");
                    break;
                }
            }
        }
    });

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            error!("🚨 Failed to bind to addr {}: {}", addr, e);
            std::process::exit(1);
        });
    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(token))
        .await
    {
        error!("🚨 Server error: {}", e);
    }
}

async fn shutdown_signal(token: CancellationToken) {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!("🚨 Failed to install Ctrl+C handler: {}", e);
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => {
                error!("🚨 Failed to install signal handler: {}", e);
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("🔴 [api-server] Received Ctrl+C signal. Initiating graceful shutdown...");
        },
        _ = terminate => {
            info!("🔴 [api-server] Received Terminate signal. Initiating graceful shutdown...");
        },
    }

    token.cancel();

    // Give background workers some time to cleanup
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    info!("👋 [api-server] Graceful shutdown complete.");
}

pub fn build_app(
    state: AppState,
    cors_layer: CorsLayer,
    static_path: &str,
    #[cfg(feature = "nurture")] nurture_state: nurture_api::state::SharedState,
    plugin_registry: plugin_loader::PluginRegistry,
) -> Router {
    let internal_router: Router<AppState> = Router::new()
        // --- Protected Routes (Require Authentication) ---
        .route("/api/wiki", get(routes::general::list_wiki_files))
        .route(
            "/api/wiki/:filename",
            get(routes::general::get_wiki_content),
        )
        .route(
            "/api/synergy/graph",
            get(routes::karma::synergy_graph_handler),
        )
        .route(
            "/api/synergy/test/failure",
            axum::routing::post(routes::karma::trigger_failure_demo),
        )
        .route(
            "/api/synergy/test/security",
            axum::routing::post(routes::karma::trigger_security_demo),
        )
        .route(
            "/api/synergy/test/federation",
            axum::routing::post(routes::karma::trigger_federation_demo),
        )
        .route(
            "/api/synergy/rules",
            get(routes::karma::get_immune_rules_handler)
                .post(routes::karma::add_immune_rule_handler)
                .put(routes::karma::add_immune_rule_handler),
        )
        .route(
            "/api/synergy/rules/:id",
            axum::routing::delete(routes::karma::delete_immune_rule_handler),
        )
        .route(
            "/api/artifacts",
            get(routes::artifacts::list_artifacts_handler),
        )
        .route(
            "/api/artifacts/:id",
            get(routes::artifacts::get_artifact_handler)
                .delete(routes::artifacts::delete_artifact_handler),
        )
        .route(
            "/api/artifacts/:id/edges",
            get(routes::artifacts::get_artifact_edges_handler),
        )
        .route(
            "/api/artifacts/:id/files/:filename",
            get(routes::artifacts::download_artifact_file_handler),
        )
        .route(
            "/api/agent/chat",
            axum::routing::post(routes::agent::trigger_agent_chat).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(3)), // 1 chat per 3s
            ),
        )
        .route(
            "/api/agent/feedback",
            axum::routing::post(routes::agent::handle_karma_feedback),
        )
        .route(
            "/api/system/evolution",
            get(routes::karma::get_evolution_history_handler),
        )
        .route("/api/soul/status", get(routes::soul::get_soul_status))
        .route(
            "/api/v1/settings",
            get(routes::settings::get_settings).put(routes::settings::update_setting),
        )
        .route(
            "/api/v1/settings/test",
            axum::routing::post(routes::settings::test_connection).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(5, std::time::Duration::from_secs(1)), // 5 tests per sec
            ),
        )
        .route(
            "/api/v1/ollama/models",
            get(routes::settings::get_ollama_models),
        )
        .route(
            "/api/v1/commerce/balance/:agent_id",
            get(routes::commerce::get_balance),
        )
        .route(
            "/api/v1/commerce/purchase/:agent_id",
            axum::routing::post(routes::commerce::execute_purchase).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(2)), // 1 purchase per 2s
            ),
        )
        .route("/api/v1/logs", get(routes::general::get_logs))
        .route("/api/biome/status", get(routes::biome::biome_status))
        .route(
            "/api/biome/topics",
            get(routes::biome::list_topics)
                .post(routes::biome::create_topic)
                .route_layer(
                    tower::ServiceBuilder::new()
                        .layer(axum::error_handling::HandleErrorLayer::new(
                            handle_rate_limit,
                        ))
                        .buffer(5)
                        .rate_limit(1, std::time::Duration::from_secs(5)), // 1 topic per 5s
                ),
        )
        .route("/api/biome/list", get(routes::biome::list_messages))
        .route(
            "/api/biome/send",
            axum::routing::post(routes::biome::send_message).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(2, std::time::Duration::from_secs(1)), // 2 messages per sec (p2p)
            ),
        )
        .route(
            "/api/biome/autonomous/start",
            axum::routing::post(routes::biome::autonomous_start).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(2)), // 1 toggle per 2s
            ),
        )
        .route(
            "/api/biome/autonomous/stop",
            axum::routing::post(routes::biome::autonomous_stop),
        )
        .route(
            "/api/biome/autonomous/status",
            get(routes::biome::autonomous_status),
        )
        .route(
            "/api/expression/status",
            get(routes::expression::expression_status),
        )
        .route(
            "/api/expression/generate",
            axum::routing::post(routes::expression::generate_expression).route_layer(
                tower::ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        handle_rate_limit,
                    ))
                    .buffer(5)
                    .rate_limit(1, std::time::Duration::from_secs(10)), // 1 generation per 10s (costly)
            ),
        )
        .route(
            "/api/expression/list",
            get(routes::expression::list_expressions),
        )
        .route(
            "/api/expression/auto",
            axum::routing::post(routes::expression::toggle_auto_expression),
        )
        .route("/api/skills", get(routes::skill::list_skills))
        .route(
            "/api/skills/import",
            axum::routing::post(routes::skill::import_skill),
        )
        .route(
            "/api/skills/mcp/spawn",
            axum::routing::post(routes::skill::spawn_mcp_server),
        )
        .route("/api/health", get(routes::general::get_health_status))
        // Apply timeout only to standard routes (avoids killing long-lived SSE/WS)
        .layer(TimeoutLayer::new(Duration::from_secs(30)));

    let streaming_router: Router<AppState> = Router::new()
        .route("/api/synergy/karma", get(routes::karma::get_karma_stream))
        .route(
            "/api/agent/chat/stream",
            axum::routing::post(stream::trigger_agent_chat_stream),
        )
        .nest("/api/v1/mcp", mcp::router())
        .route(
            "/api/system/vitality",
            get(stream::trigger_system_vitality_stream),
        )
        .route("/api/v1/watchtower/ws", get(routes::watchtower::ws_handler));

    let mut router: Router<AppState> = Router::new().merge(internal_router).merge(streaming_router);

    #[cfg(feature = "nurture")]
    {
        // nurture routes are merged after with_state, so handle separately
    }

    // Apply auth middleware BEFORE with_state, using from_fn_with_state
    let state_for_auth = state.clone();
    let router = router.with_state(state);

    #[cfg(feature = "nurture")]
    let router = router.merge(nurture_api::routes::nurture_routes(nurture_state));

    // Dynamic Route Merging from PluginRegistry (Phase A-2)
    let router = plugin_registry.merge_routes(router);

    router
        .route_layer(axum::middleware::from_fn_with_state(state_for_auth, auth::auth_middleware))

        // --- Public Routes (Internal Monitoring / SSE / WS) ---
        .merge(utoipa_swagger_ui::SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api::ApiDoc::openapi()))
        .fallback_service(ServeDir::new(static_path).append_index_html_on_directories(true))
        // --- Layer 3: Security Headers (Defense in Depth) ---
        .layer(SetResponseHeaderLayer::if_not_present(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")))
        .layer(SetResponseHeaderLayer::if_not_present(X_FRAME_OPTIONS, HeaderValue::from_static("DENY")))
        .layer(SetResponseHeaderLayer::if_not_present(STRICT_TRANSPORT_SECURITY, HeaderValue::from_static("max-age=31536000; includeSubDomains")))
        .layer(SetResponseHeaderLayer::if_not_present(CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self' ws: wss: http: https:; object-src 'none'; base-uri 'self';")))
        // --- Layer 2: Dynamic CORS (Whitelisting) ---
        // Sources: 1) ALLOWED_ORIGINS env var (defaults)  2) DB system_settings (dynamic)
        .layer(cors_layer)
        // --- Layer 1: Global Rate Limiting & DoS Protection ---
        .layer(
            tower::ServiceBuilder::new()
                .layer(axum::error_handling::HandleErrorLayer::new(|err: tower::BoxError| async move {
                    (StatusCode::INTERNAL_SERVER_ERROR, format!("Security Layer Error: {}", err))
                }))
                .buffer(1024)
                .rate_limit(50, std::time::Duration::from_secs(1)) // Spike protection
                .into_inner()
        )
        // --- Layer 0: Payload Protection ---
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // 10MB max request body
}

#[cfg(test)]
mod api_integration_tests;

async fn handle_rate_limit(_err: tower::BoxError) -> (axum::http::StatusCode, &'static str) {
    (
        axum::http::StatusCode::TOO_MANY_REQUESTS,
        "Rate Limit Exceeded",
    )
}

async fn handle_global_error(err: tower::BoxError) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        format!("Security Layer Error: {}", err),
    )
}
