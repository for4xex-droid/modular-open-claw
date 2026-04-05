/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::*;
use crate::app_state::Component;
use aiome_core_contracts::traits::AgentEvolver;
use aiome_core_contracts::traits::JobQueue;
use aiome_core_contracts::traits::TaskRegistry;
use axum_test::TestServer;
use infrastructure::auth::AuthManager;
use serde_json::json;
use serial_test::serial;
use shared::config::AiomeConfig;
use soul::SoulPipeline;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{AllowOrigin, CorsLayer};

// 💎 Shared Global Metrics Recorder (PR-10 Mitigation)
// Prometheus handles only one global recorder per process.
static GLOBAL_METRICS_HANDLE: once_cell::sync::Lazy<metrics_exporter_prometheus::PrometheusHandle> =
    once_cell::sync::Lazy::new(|| {
        metrics_exporter_prometheus::PrometheusBuilder::new()
            .install_recorder()
            .expect("Failed to install global prometheus recorder in tests")
    });

#[derive(Debug)]
struct DummyLlm;
#[async_trait::async_trait]
impl aiome_core::llm_provider::LlmProvider for DummyLlm {
    async fn complete(
        &self,
        prompt: &str,
        sys: Option<&str>,
    ) -> Result<aiome_core_contracts::LlmResponse, aiome_core::error::AiomeError> {
        let is_json_req = sys
            .map(|s| s.contains("JSON format") || s.contains("category\": \"Learning"))
            .unwrap_or(false)
            || prompt.contains("JSON format")
            || prompt.contains("Oracle Judge");

        let content = if is_json_req {
            if sys
                .map(|s| s.contains("category\": \"Learning"))
                .unwrap_or(false)
            {
                r#"{"category": "Learning", "description": "High-performance GPU for ML", "budget": 800}"#.to_string()
            } else {
                r#"{"passed": true, "score": 1.0, "detail": "Perfect"}"#.to_string()
            }
        } else {
            "Dummy Output".to_string()
        };
        Ok(aiome_core_contracts::LlmResponse {
            content,
            stop_reason: aiome_core_contracts::StopReason::EndTurn,
            reasoning: None,
            metadata: None,
        })
    }
    async fn complete_with_cache(
        &self,
        request: aiome_core_contracts::llm::LlmRequest,
    ) -> Result<aiome_core_contracts::LlmResponse, aiome_core::error::AiomeError> {
        let content = request
            .messages
            .last()
            .map(|m| m.content.as_str())
            .unwrap_or("");
        self.complete(content, None).await
    }
    async fn test_connection(&self) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }
    fn name(&self) -> &str {
        "Dummy"
    }

    async fn stream_complete(
        &self,
        _prompt: &str,
        _sys: Option<&str>,
    ) -> Result<
        std::pin::Pin<
            Box<
                dyn tokio_stream::Stream<Item = Result<String, aiome_core::error::AiomeError>>
                    + Send,
            >,
        >,
        aiome_core::error::AiomeError,
    > {
        let s = async_stream::stream! {
            yield Ok("Dummy Stream".to_string());
        };
        Ok(Box::pin(s))
    }
}

#[derive(Debug)]
struct MockCommerceEngine;
#[async_trait::async_trait]
impl aiome_core_contracts::commerce::CommerceEngine for MockCommerceEngine {
    async fn deduct_generation_cost(
        &self,
        _agent_id: uuid::Uuid,
        _amount: u64,
        _generation_type: &str,
    ) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }

    async fn get_balance(
        &self,
        _agent_id: uuid::Uuid,
    ) -> Result<u64, aiome_core::error::AiomeError> {
        Ok(0)
    }
    async fn validate_activity(
        &self,
        _agent_id: uuid::Uuid,
        _activity_type: &str,
        _amount: u64,
    ) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }
    async fn execute_autonomous_purchase(
        &self,
        _agent_id: uuid::Uuid,
        _item_id: uuid::Uuid,
        _metadata: serde_json::Value,
    ) -> Result<String, aiome_core::error::AiomeError> {
        Ok("mock".into())
    }
    async fn get_daily_spend(
        &self,
        _agent_id: uuid::Uuid,
    ) -> Result<u64, aiome_core::error::AiomeError> {
        Ok(0)
    }
    async fn get_daily_limit(
        &self,
        _agent_id: uuid::Uuid,
    ) -> Result<u64, aiome_core::error::AiomeError> {
        Ok(100)
    }
    async fn escrow_create(
        &self,
        _agent_id: uuid::Uuid,
        _amount: u64,
    ) -> Result<String, aiome_core::error::AiomeError> {
        Ok("esc".into())
    }
    async fn stake(
        &self,
        _agent_id: uuid::Uuid,
        _amount: u64,
    ) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }
    async fn slash(
        &self,
        _agent_id: uuid::Uuid,
        _amount: u64,
        _reason: &str,
    ) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }
    async fn register_license(
        &self,
        _agent_id: uuid::Uuid,
        _asset_id: uuid::Uuid,
        _license_type: &str,
    ) -> Result<String, aiome_core::error::AiomeError> {
        Ok("lic".into())
    }

    async fn create_subscription(
        &self,
        _agent_id: uuid::Uuid,
        _plan_id: &str,
    ) -> Result<String, aiome_core::error::AiomeError> {
        Ok("sub_mock_123".into())
    }

    async fn cancel_subscription(
        &self,
        _subscription_id: &str,
    ) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }

    async fn get_subscription_status(
        &self,
        agent_id: uuid::Uuid,
    ) -> Result<aiome_core_contracts::commerce::SubscriptionStatus, aiome_core::error::AiomeError>
    {
        if agent_id == uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap() {
            return Ok(aiome_core_contracts::commerce::SubscriptionStatus::None);
        }
        Ok(aiome_core_contracts::commerce::SubscriptionStatus::Active)
    }

    async fn transfer(
        &self,
        _from_id: uuid::Uuid,
        _to_id: uuid::Uuid,
        _amount: u64,
    ) -> Result<String, aiome_core::error::AiomeError> {
        Ok("transfer_ok".to_string())
    }

    async fn escrow_refund(&self, _order_id: &str) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }

    fn verify_signature(
        &self,
        _payload: &str,
        _sig_header: &str,
    ) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }

    async fn process_webhook(
        &self,
        _event_id: &str,
        _event_type: &str,
        _payload: &serde_json::Value,
    ) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }

    async fn escrow_release(
        &self,
        _escrow_id: &str,
        _recipient_id: uuid::Uuid,
    ) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }
}

#[derive(Debug)]
struct MockGiftEngine;
#[async_trait::async_trait]
impl aiome_core_contracts::commerce::GiftEngine for MockGiftEngine {
    async fn send_gift_code(
        &self,
        _recipient_email: &str,
        _amount_usd: f64,
        _reason: &str,
    ) -> Result<String, aiome_core::error::AiomeError> {
        Ok("mock_order_123".to_string())
    }

    async fn validate_gift_policy(
        &self,
        _agent_id: uuid::Uuid,
        _amount_usd: f64,
    ) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }

    async fn get_policy_context(
        &self,
        _agent_id: uuid::Uuid,
    ) -> Result<aiome_core_contracts::commerce::GiftPolicyContext, aiome_core::error::AiomeError>
    {
        Ok(aiome_core_contracts::commerce::GiftPolicyContext {
            max_amount_usd: 5.0,
            daily_limit_reached: false,
            daily_sent_count: 0,
            daily_sent_total_usd: 0.0,
        })
    }
}
#[derive(Debug, Default)]
struct MockLiveSessionManager;
#[async_trait::async_trait]
impl aiome_core_contracts::traits::LiveSessionManager for MockLiveSessionManager {
    async fn create_session(
        &self,
        _level: aiome_core_contracts::live_types::ThinkingLevel,
    ) -> Result<String, aiome_core::error::AiomeError> {
        Ok("mock_session".into())
    }
    async fn close_session(&self, _session_id: &str) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }
    async fn send_audio(
        &self,
        _session_id: &str,
        _pcm: &[u8],
    ) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }
    async fn send_text(
        &self,
        _session_id: &str,
        _text: &str,
    ) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }
    async fn receive_events(
        &self,
        _session_id: &str,
    ) -> Result<Vec<aiome_core_contracts::live_types::LiveEvent>, aiome_core::error::AiomeError>
    {
        Ok(vec![])
    }
}

#[derive(Debug, Default)]
struct MockLoraEngine;
#[async_trait::async_trait]
impl aiome_core_contracts::traits::LoraEngine for MockLoraEngine {
    async fn complete_with_lora(
        &self,
        _prompt: &str,
        _lora_id: &str,
    ) -> Result<aiome_core_contracts::llm::LlmResponse, aiome_core::error::AiomeError> {
        Ok(aiome_core_contracts::llm::LlmResponse {
            content: "LlmResponse from MockLoraEngine".to_string(),
            stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
            reasoning: None,
            metadata: None,
        })
    }
    async fn health_check(&self) -> Result<bool, aiome_core::error::AiomeError> {
        Ok(true)
    }
    async fn train(
        &self,
        _base: &str,
        _data: &str,
        _params: serde_json::Value,
    ) -> Result<String, aiome_core::error::AiomeError> {
        Ok("mock_job_id".into())
    }
}

pub async fn create_test_server() -> (TestServer, AppState, tempfile::TempDir) {
    let tmp_dir = tempfile::TempDir::new().expect("tmp dir creation failed");
    let db_path = tmp_dir.path().join("test.db");

    // Set WORKSPACE_DIR to tmp_dir for security sandbox consistency (S-4 fix)
    std::env::set_var("WORKSPACE_DIR", tmp_dir.path().to_str().unwrap());

    let pool = infrastructure::db::DatabasePool::new_sqlite(&format!(
        "sqlite://{}",
        db_path.to_str().unwrap()
    ))
    .await
    .expect("Failed to create test DB pool");

    let ts = std::sync::Arc::new(
        infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
    );
    let job_queue = Arc::new(
        infrastructure::job_queue::UniversalJobQueue::new(
            &format!("sqlite://{}", db_path.to_str().unwrap()),
            None,
            ts,
        )
        .await
        .expect("Failed to create test job queue"),
    );

    let provider = Arc::new(DummyLlm);

    let skills_dir = tmp_dir.path().join("skills");
    let forge_dir = tmp_dir.path().join("forge");
    let sandbox_dir = tmp_dir.path().join("sandbox");
    let artifacts_dir = tmp_dir.path().join("artifacts");

    std::fs::create_dir_all(&skills_dir).unwrap();
    std::fs::create_dir_all(&forge_dir).unwrap();
    std::fs::create_dir_all(&sandbox_dir).unwrap();
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let wasm_skill_manager = Arc::new(
        infrastructure::skills::WasmSkillManager::new(
            skills_dir.to_str().unwrap(),
            sandbox_dir.to_str().unwrap(),
        )
        .unwrap(),
    );

    // Create .abyss_vault directory inside the tmp_dir
    let vault_dir = tmp_dir.path().join(".abyss_vault");
    std::fs::create_dir_all(&vault_dir).unwrap();
    let skill_forge = Arc::new(infrastructure::skills::forge::SkillForge::new(
        forge_dir.to_str().unwrap(),
        skills_dir.to_str().unwrap(),
    ));
    let artifact_store = Arc::new(
        infrastructure::artifact_store::UniversalArtifactStore::new(
            job_queue.get_pool().clone(),
            artifacts_dir.clone(),
        )
        .with_job_queue(job_queue.clone()),
    );
    let context_engine = Arc::new(infrastructure::context_engine::ContextEngine::new(
        provider.clone(),
        job_queue.clone(),
        Arc::new(tokio::sync::Semaphore::new(1)),
    ));
    let soul_mutator = Arc::new(infrastructure::soul_mutator::SoulMutator::new(
        provider.clone(),
        tmp_dir.path().join("SOUL.md"),
        None,
    ));
    let autonomous_running = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let autonomous_config = Arc::new(tokio::sync::RwLock::new(None));
    let intent_firewall = Arc::new(infrastructure::intent::IntentFirewall::new());
    let soul_store = Arc::new(infrastructure::soul_store::UniversalSoulStore::new(
        job_queue.get_pool().clone(),
    ));
    let intent_generator = Arc::new(infrastructure::intent::IntentGenerator::new(
        context_engine.clone(),
        provider.clone(),
        intent_firewall.clone(),
        soul_store.clone(),
    ));

    let registry = Arc::new(infrastructure::registry::RegistryManager::new(
        job_queue.get_pool().clone(),
    ));
    std::env::set_var(
        "VAULT_MASTER_KEY",
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
    );
    std::env::set_var("WORKSPACE_DIR", tmp_dir.path().to_str().unwrap());
    let voice_drm = Arc::new(
        infrastructure::security::VoiceCoreDrm::new(
            "http://localhost:3016".to_string(),
            registry.clone(),
            job_queue.get_pool().clone(),
        )
        .await,
    );

    let commerce_engine = Arc::new(MockCommerceEngine);

    let rate_limiter = infrastructure::rate_limiter::AgentRateLimiter::new(5);

    let soul_adapter = infrastructure::soul_adapter::CoreDomainAdapter::new(
        job_queue.clone(),
        None, // embedding_provider
    );
    let samsara_engine = infrastructure::samsara_engine::DefaultSamsaraEngine::new(
        provider.clone(),
        "test distillator".to_string(),
    );
    let audit_logger = Arc::new(infrastructure::audit_logger::AsyncAuditLogger::new(
        job_queue.get_pool().clone().into(),
        100,
    ));

    let disk_quota_mgr = infrastructure::disk_quota::DiskQuotaManager::new(
        job_queue.get_pool().clone(),
        500 * 1024 * 1024,
    );
    let _ = disk_quota_mgr.init().await;

    let state = AppState {
        hook_chain: Default::default(),
        a2a_client: Component::new(Arc::new(
            infrastructure::grpc::mock_a2a_client::MockA2aClient::new(),
        )),
        health_monitor: Component::new(Arc::new(Mutex::new(HealthMonitor::new()))),
        job_queue: Component::new(job_queue.clone()),
        wasm_skill_manager: Component::new(wasm_skill_manager),
        skill_forge: Component::new(skill_forge),
        docs_path: tmp_dir.path().to_str().unwrap().to_string(),
        project_rules_cache: Component::new(Arc::new(
            moka::future::Cache::builder()
                .time_to_live(std::time::Duration::from_secs(30))
                .build(),
        )),
        llm_semaphore: Component::new(Arc::new(tokio::sync::Semaphore::new(1))),
        forge_semaphore: Component::new(Arc::new(tokio::sync::Semaphore::new(1))),
        mcp_sessions: Component::new(Arc::new(tokio::sync::RwLock::new(
            std::collections::HashMap::new(),
        ))),
        mcp_manager: Component::new(Arc::new(mcp::client::McpProcessManager::new())),
        artifact_store: Component::new(artifact_store),
        event_sender: Component::new(tokio::sync::broadcast::channel(10).0),
        context_engine: Component::new(context_engine),
        soul_mutator: Component::new(soul_mutator),
        soul_store: Component::new(soul_store),
        provider: Component::new(provider.clone()),
        autonomous_running: Component::new(autonomous_running),
        autonomous_config: Component::new(autonomous_config),
        http_client: Component::new(aiome_core::http::get_http_client().clone()),
        docker_failures: Component::new(Arc::new(tokio::sync::RwLock::new(
            std::collections::HashMap::new(),
        ))),
        security_policy: shared::security::SecurityPolicy::default(),
        commerce_engine: Component::new(commerce_engine.clone()),
        circuit_breaker: Component::new(Arc::new(
            infrastructure::circuit_breaker::CircuitBreaker::new(
                "integration-test",
                infrastructure::circuit_breaker::CircuitBreakerConfig {
                    failure_threshold: 5,
                    reset_timeout: std::time::Duration::from_secs(60),
                },
            ),
        )),
        rate_limiter: Component::new(rate_limiter),
        slo_engine: Component::new(Arc::new(infrastructure::slo_engine::SloEngine::new(
            infrastructure::slo_engine::SloConfig {
                error_budget_max: 100,
                warning_threshold: 80,
            },
            chrono::Duration::hours(24),
        ))),
        api_server_secret: Component::new(Arc::new(secrecy::SecretString::from(
            "test_secret".to_string(),
        ))),
        federation_secret: Component::new(Arc::new(secrecy::SecretString::from(
            "test_fed_secret".to_string(),
        ))),
        config: Component::new({
            let mut config = AiomeConfig::default();
            config.resolver = shared::app_data::AppDataResolver::new();
            config.log_level = "info".to_string();
            config.ollama_host = "".to_string();
            config.ollama_model = "".to_string();
            config.gemini_api_key = None;
            config.openai_api_key = None;
            config.anthropic_api_key = None;
            config.api_server_port = 0;
            config.key_proxy_url = "".to_string();
            config.samsara_hub_url = "".to_string();
            config.allowed_origins = vec![];
            config.abyss_vault_path = tmp_dir.path().to_str().unwrap().to_string();
            config.tremendous_api_key = None;
            config.master_email = None;
            config.xtts_endpoint = None;
            config.xtts_speaker = None;
            config.vault_path = tmp_dir.path().join("vault");
            config.mcp = shared::config::McpConfig::default();
            Arc::new(config)
        }),
        gift_engine: Component::new(Arc::new(MockGiftEngine)),
        ekyc_engine: Component::new(Arc::new(aiome_commerce::ekyc::MockEkycEngine)),
        ekyc_session_store: Component::new(Arc::new(
            aiome_commerce::ekyc::store::MockEkycSessionStore,
        )),
        quarantine_store: Component::new(Arc::new(
            infrastructure::compliance::quarantine::MockQuarantineStore,
        )),
        auth_manager: Component::new(Arc::new(infrastructure::auth::MockAuthManager::new())),
        system_agent_id: uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        voice_drm: Component::new(voice_drm.clone()),
        registry: Component::new(registry.clone()),
        gig_engine: Component::new(Arc::new(aiome_commerce::gig::UniversalGigEngine::new(
            job_queue.get_pool().clone(),
            Arc::new(MockCommerceEngine),
            provider.clone(),
            tmp_dir.path().join("gig_artifacts"),
        )) as Arc<dyn aiome_core_contracts::gig::GigEngine>),
        intent_generator: Component::new(intent_generator),
        intent_firewall: Component::new(intent_firewall),
        audit_logger: Component::new(audit_logger),
        affiliate_adapter: Component::new(
            Arc::new(infrastructure::intent::AffiliateAdapter::new()),
        ),
        soul_pipeline: Component::new(Arc::new(SoulPipeline::new(soul_adapter, samsara_engine))),
        transcription_engine: Component::new(Arc::new(
            infrastructure::whisper_transcription::WhisperTranscriptionAdapter::new(
                Arc::new(infrastructure::security::BastionGuard::new_internal(
                    aiome_core::security::PermissionManifest::default(),
                )),
                false,
            ),
        )
            as Arc<dyn aiome_core::traits::TranscriptionEngine>),
        task_dispatcher: {
            let validator = Arc::new(
                infrastructure::validator::DefaultConstitutionalValidator::new(
                    provider.clone(),
                    None,
                ),
            );
            let dispatcher = Arc::new(infrastructure::task_orchestrator::TaskDispatcher::new(
                job_queue.clone(),
                std::time::Duration::from_millis(10),
                None, // core_event_tx
                None, // tool_discovery
                None, // planner
                Some(validator),
                Some(tmp_dir.path().join("SOUL.md")),
                None, // oracle
                None, // gig_engine
                None, // diagnostics
            ));

            // G-22 Fix: Spawn dispatcher loop in background for integration tests
            let dispatcher_clone = dispatcher.clone();
            tokio::spawn(async move {
                dispatcher_clone.run_dispatch_loop().await;
            });

            Component::new(dispatcher)
        },
        lora_engine: Component::new(Arc::new(MockLoraEngine::default())),
        tts_provider: Component::new(Arc::new(infrastructure::tts::MockTtsProvider::default())),
        news_service: Component::default(),
        live_session_manager: Component::new(Arc::new(MockLiveSessionManager::default())),
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
                provider.clone(),
                job_queue
                    .get_pool()
                    .get_sqlite_pool()
                    .cloned()
                    .expect("SQLite pool required for HierarchicalRouter"),
            ),
        )),
        ws_active_connections: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        harness_cache: Component::new(Arc::new(
            infrastructure::skills::harness::HarnessCache::new(),
        )),
        upload_semaphore: Component::new(Arc::new(tokio::sync::Semaphore::new(10))),
        compute_semaphore: Component::new(Arc::new(tokio::sync::Semaphore::new(1))),
        disk_quota: Component::new(Arc::new(disk_quota_mgr)),
        generative_engine: Component::new(Arc::new(
            infrastructure::generative_engine::mock::MockGenerativeEngine::default(),
        )),
        cortex_ingester: Component::new(Arc::new(
            infrastructure::cortex_ingester::CortexIngester::new(
                provider.clone(),
                job_queue.get_pool().clone(),
            ),
        )),
        cortex_query: Component::new(Arc::new(
            infrastructure::cortex_query::CortexQueryEngine::new(
                provider.clone(),
                job_queue.get_pool().clone(),
            ),
        )),
        lora_marketplace: {
            let vault_root = tmp_dir.path().join("vault");
            std::fs::create_dir_all(&vault_root).ok();
            Component::new(Arc::new(
                infrastructure::lora_marketplace::UniversalLoraMarketplace::new(
                    job_queue.get_pool().clone(),
                    commerce_engine.clone()
                        as Arc<dyn aiome_core_contracts::commerce::CommerceEngine>,
                    vault_root,
                ),
            )
                as Arc<dyn aiome_core_contracts::lora_marketplace::LoraMarketplace>)
        },
    };

    let cors_layer = CorsLayer::new().allow_origin(AllowOrigin::any());

    let plugin_registry = plugin_loader::PluginRegistry::new();
    let metrics_handle = GLOBAL_METRICS_HANDLE.clone();
    let app = build_app(
        state.clone(),
        cors_layer,
        tmp_dir.path().join("static").to_str().unwrap(),
        plugin_registry,
        metrics_handle,
    );
    let server = TestServer::new(app).expect("Failed to create TestServer");
    (server, state, tmp_dir)
}

pub fn test_bearer() -> String {
    // MockAuthManager accepts "mock_valid_token_<sub>"
    "Bearer mock_valid_token_test_user".to_string()
}

#[serial]
#[tokio::test]
async fn test_health_check() {
    let (server, _state, _tmp) = create_test_server().await;
    let response = server
        .get("/api/health")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);

    // Check JSON structure: ResourceStatus fields
    let json = response.json::<serde_json::Value>();
    assert!(json.get("cpu_usage_percent").is_some());
    assert!(json.get("level").is_some());

    // Check Circuit Breaker status (G-1)
    let cb = json
        .get("llm_circuit_breaker")
        .expect("llm_circuit_breaker field missing");
    assert_eq!(cb["name"], "integration-test");
    assert_eq!(cb["state"], "Closed");
}

#[serial]
#[tokio::test]
async fn test_rate_limiting_per_agent() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    for _ in 0..5 {
        let resp = server
            .get("/api/biome/status")
            .add_header(axum::http::header::AUTHORIZATION, &bearer)
            .await;
        assert_eq!(resp.status_code(), StatusCode::OK);
    }

    let resp = server
        .get("/api/biome/status")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(resp.status_code(), StatusCode::TOO_MANY_REQUESTS);
}

#[serial]
#[tokio::test]
async fn test_expression_generation_plan_limits() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer =
        "Bearer mock_valid_token_test_user:00000000-0000-0000-0000-000000000002".to_string();
    use aiome_core::expression::Expression;
    use aiome_core_contracts::expression::TtsStatus;
    // Simulate 5 recently generated expressions
    for i in 0..5 {
        let mut expr = Expression::default();
        expr.id = format!("expr_{}", i);
        expr.content = "mock content".into();
        expr.tts_status = TtsStatus::NotRequested;
        expr.created_at = chrono::Utc::now().to_rfc3339();

        state.job_queue.store_expression(&expr).await.unwrap();
    }

    // 2. The generation attempt should be blocked by the Free plan rate limit
    let resp = server
        .post("/api/expression/generate")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(
        resp.status_code(),
        StatusCode::TOO_MANY_REQUESTS,
        "Free plan should be rate limited at 5 expressions per hour"
    );
}

#[serial]
#[tokio::test]
async fn test_settings_unauthorized() {
    let (server, _state, _tmp) = create_test_server().await;
    let response = server.get("/api/v1/settings").await;
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[serial]
#[tokio::test]
async fn test_settings_authorized_and_crud() {
    let (server, _state, _tmp) = create_test_server().await;

    // Get empty settings
    let get_resp = server
        .get("/api/v1/settings")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;
    assert_eq!(get_resp.status_code(), StatusCode::OK);
    let settings = get_resp.json::<serde_json::Value>();
    assert!(settings.as_array().unwrap().is_empty());

    // Put a valid setting (assuming theme is allowed)
    // Wait, the API checks whitelist. theme should be allowed.
    let put_req = json!({
        "key": "ollama_model",
        "value": "qwen2",
        "category": "llm"
    });

    let put_resp = server
        .put("/api/v1/settings")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&put_req)
        .await;

    assert_eq!(put_resp.status_code(), StatusCode::OK);

    // Check if it got saved
    let get_resp2 = server
        .get("/api/v1/settings")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;
    let settings_array = get_resp2.json::<Vec<serde_json::Value>>();
    assert_eq!(settings_array.len(), 1);
    assert_eq!(settings_array[0]["key"], "ollama_model");
    assert_eq!(settings_array[0]["value"], "qwen2");
}

#[serial]
#[tokio::test]
async fn test_settings_ssrf_protection() {
    let (server, _state, _tmp) = create_test_server().await;

    let payload = json!({
        "service": "ollama",
        "url": "http://169.254.169.254",
        "model": "malicious"
    });

    let resp = server
        .post("/api/v1/settings/test")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&payload)
        .await;

    // Should block SSRF attempt with success: false and message containing "SSRF Blocked"
    assert_eq!(resp.status_code(), StatusCode::OK);
    let json = resp.json::<serde_json::Value>();
    assert_eq!(json["success"], false);
    assert!(json["message"].as_str().unwrap().contains("SSRF Blocked"));
}

#[serial]
#[tokio::test]
async fn test_biome_routes_auth() {
    let (server, _state, _tmp) = create_test_server().await;

    let resp_no_auth = server.get("/api/biome/status").await;
    assert_eq!(resp_no_auth.status_code(), StatusCode::UNAUTHORIZED);

    let resp_auth = server
        .get("/api/biome/status")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;
    assert_eq!(resp_auth.status_code(), StatusCode::OK);
}

#[serial]
#[tokio::test]
async fn test_ollama_models() {
    let (server, _state, _tmp) = create_test_server().await;

    // Test hitting the ollama models endpoint
    let resp = server
        .get("/api/v1/ollama/models")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    // Without a real ollama server running to mock, it will fail to connect and return 500 or 502/503 depending on impl.
    // We just verify it's responsive and authorized, not hanging.
    assert!(
        resp.status_code() == StatusCode::SERVICE_UNAVAILABLE
            || resp.status_code() == StatusCode::INTERNAL_SERVER_ERROR
            || resp.status_code() == StatusCode::OK
    );
}

#[serial]
#[tokio::test]
async fn test_tts_synthesis() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let payload = json!({
        "text": "Hello from TDD",
        "voice_id": "p225"
    });

    let resp = server
        .post("/api/v1/voice/synthesize")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    // This is expected to FAIL (RED) with 404/405 initially
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body = resp.as_bytes();
    assert!(!body.is_empty());
}

#[serial]
#[tokio::test]
async fn test_lora_training_start() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let payload = json!({
        "base_model": "mistral-7b",
        "dataset_id": "user-123-chat",
        "params": {
            "epochs": 3,
            "lr": 1e-4
        }
    });

    let resp = server
        .post("/api/v1/lora/train")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    // This is expected to FAIL (RED) with 404/405 initially
    assert_eq!(resp.status_code(), StatusCode::ACCEPTED);

    let json = resp.json::<serde_json::Value>();
    assert!(json.get("job_id").is_some());
}

#[serial]
#[tokio::test]
async fn test_lora_training_status() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    // Arrange: Insert a dummy job
    let mut job = aiome_core_contracts::traits::Job::default();
    job.id = "mock_lora_job_id".to_string();
    job.category = "LORA_TRAINING".to_string();
    job.status = aiome_core_contracts::traits::JobStatus::InProgress;

    // We must use enqueue to insert it if possible, but the JobQueue trait might not support inserting arbitrary jobs with arbitrary IDs.
    // However, JobQueue has `store_job` or similar in Mock, but let's just use `enqueue` and get the real job_id.
    let job_id = state
        .job_queue
        .enqueue(
            "LORA_TRAINING",
            "mock_lora_job",
            "training",
            None,
            None,
            None,
            0,
        )
        .await
        .expect("Failed to enqueue job");

    // We also need to update its status to something verifiable
    state
        .job_queue
        .update_job_status(&job_id, aiome_core_contracts::traits::JobStatus::Completed)
        .await
        .unwrap();

    let url = format!("/api/v1/lora/status/{}", job_id);
    let resp = server
        .get(&url)
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    // This is expected to FAIL (RED) with 404 Not Found initially
    assert_eq!(resp.status_code(), StatusCode::OK);

    let json = resp.json::<serde_json::Value>();
    assert_eq!(json.get("status").unwrap().as_str().unwrap(), "Completed");
}

#[serial]
#[tokio::test]
async fn test_avatar_upload_ekyc_enforcement() {
    let (server, _state, _tmp) = create_test_server().await;

    // mock_valid_token_unverified does NOT start with 'ekyc_'
    let bearer = "Bearer mock_valid_token_unverified_user".to_string();

    let payload = json!({
        "name": "test_avatar",
        "content_base64": base64::engine::general_purpose::STANDARD.encode(vec![0u8; 100]),
        "head_height": 1.0,
        "total_height": 6.0
    });

    let resp = server
        .post("/api/avatar/upload")
        .add_header(axum::http::header::AUTHORIZATION, bearer)
        .json(&payload)
        .await;

    // Currently this returns OK because it only warns.
    // We WANT it to return FORBIDDEN or UNAUTHORIZED.
    assert_eq!(
        resp.status_code(),
        StatusCode::FORBIDDEN,
        "Unverified user should be blocked from avatar upload"
    );
}

#[serial]
#[tokio::test]
async fn test_skill_import_oom_protection() {
    let (server, _state, _tmp) = create_test_server().await;

    // We need to mock a remote server that returns a huge response.
    // Since we use reqwest::Client in AppState, this is hard to mock without a real mock server.
    // However, we can at least test the 1MB limit check logic if we could mock the response.

    // For now, let's just test that a normal import is authorized
    let resp = server
        .post("/api/skills/import")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&json!({"url": "http://example.com/skill.yaml"}))
        .await;

    // It should fail with RemoteServiceError (because example.com/skill.yaml doesn't exist in test env),
    // but it should NOT be UNAUTHORIZED.
    assert_ne!(resp.status_code(), StatusCode::UNAUTHORIZED);
}
#[serial]
#[tokio::test]
async fn test_global_body_limit() {
    let (server, _state, _tmp) = create_test_server().await;

    // Send a 4MB JSON payload (limit should be 2MB)
    let large_string = "a".repeat(4 * 1024 * 1024);
    let payload = json!({
        "key": "theme",
        "value": large_string,
        "category": "ui"
    });

    let resp = server
        .post("/api/v1/settings")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&payload)
        .await;

    // This should fail (RED) because current limit is 512MB
    assert_eq!(
        resp.status_code(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "4MB request should be rejected by 2MB limit"
    );
}

#[serial]
#[tokio::test]
async fn test_avatar_upload_limit_bypass() {
    let (server, _state, _tmp) = create_test_server().await;

    // Send a 10MB payload to /api/avatar/upload (permitted by 50MB layer)
    let large_base64 = "a".repeat(10 * 1024 * 1024);
    let payload = json!({
        "name": "large_avatar",
        "content_base64": large_base64,
        "head_height": 1.0,
        "total_height": 6.0
    });

    let bearer = "Bearer mock_valid_token_ekyc_verified_user".to_string();

    let resp = server
        .post("/api/avatar/upload")
        .add_header(axum::http::header::AUTHORIZATION, bearer)
        .json(&payload)
        .await;

    // Should NOT be 413. Should be 200 or 400.
    assert_ne!(
        resp.status_code(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "Avatar upload should bypass global 2MB limit up to 50MB"
    );
}

#[serial]
#[tokio::test]
async fn test_diagnostics_api() {
    let (server, _state, _tmp) = create_test_server().await;

    let resp = server
        .get("/api/v1/audit/diagnostics")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    if resp.status_code() != StatusCode::OK {
        let err_text = resp.text();
        panic!(
            "Diagnostics API failed with status: {}, body: {:?}",
            resp.status_code(),
            err_text
        );
    }
    let json = resp.json::<serde_json::Value>();
    assert!(json.as_array().is_some());
}

#[serial]
#[tokio::test]
async fn test_artifacts_api() {
    let (server, _state, _tmp) = create_test_server().await;

    let resp = server
        .get("/api/artifacts")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let json = resp.json::<serde_json::Value>();
    assert!(json.as_array().is_some());
}

#[serial]
#[tokio::test]
async fn test_trends_api() {
    let (server, _state, _tmp) = create_test_server().await;

    let resp = server
        .get("/api/v1/trends")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let json = resp.json::<serde_json::Value>();
    assert!(json.get("trends").is_some());
}

#[serial]
#[tokio::test]
async fn test_stripe_webhook_idempotency_and_license_grant() {
    let (server, state, _tmp) = create_test_server().await;
    let registry = state.registry.clone();

    let agent_id = uuid::Uuid::new_v4();
    let asset_id = uuid::Uuid::new_v4();

    let mut metadata = std::collections::HashMap::new();
    metadata.insert(String::from("agent_id"), agent_id.to_string());
    metadata.insert(String::from("asset_id"), asset_id.to_string());

    let session = stripe::CheckoutSession {
        id: "cs_test_123".parse().unwrap(),
        metadata: Some(metadata),
        amount_total: Some(1000), // $10.00
        ..Default::default()
    };

    let mut session_val = serde_json::to_value(&session).unwrap();
    if let Some(obj) = session_val.as_object_mut() {
        obj.insert("object".to_string(), serde_json::json!("checkout.session"));
    }

    let db_path = _tmp.path().join("test.db");
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&format!("sqlite:{}", db_path.to_str().unwrap()))
        .await
        .unwrap();

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS revenue_splits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tx_id TEXT NOT NULL,
            recipient_id TEXT NOT NULL,
            role TEXT NOT NULL,
            amount INTEGER NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    registry
        .register_asset(infrastructure::registry::AssetManifest {
            id: asset_id,
            creator_id: uuid::Uuid::new_v4(),
            asset_type: infrastructure::registry::AssetType::Plugin,
            name: "Test Asset".to_string(),
            description: "Test".to_string(),
            price_coins: 1000,
            metadata: None,
        })
        .await
        .unwrap();

    let payload_val = serde_json::json!({
        "id": "evt_test_123",
        "object": "event",
        "api_version": "2022-11-15",
        "created": 1677628800,
        "livemode": false,
        "pending_webhooks": 1,
        "request": {
            "id": null,
            "idempotency_key": null
        },
        "type": "checkout.session.completed",
        "data": {
            "object": session_val
        }
    });

    let payload = serde_json::to_string(&payload_val).unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let sig_header = format!("t={},v1=dummy_signature", now);

    let resp: axum_test::TestResponse = server
        .post("/api/v1/commerce/webhook")
        .add_header("stripe-signature", sig_header.clone())
        .add_header("content-type", "application/json")
        .text(payload.clone())
        .await;

    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);

    // 現在の api-server 実装では grant_license は呼ばれないため、ここは RED になるはずだったが、実装により GREEN になる
    let is_owned = registry.check_ownership(agent_id, asset_id).await.unwrap();
    assert!(
        is_owned,
        "Webhook must grant license in a single transaction (GREEN)"
    );
}

#[serial]
#[tokio::test]
#[serial]
async fn test_voice_drm_roundtrip() {
    let (server, state, tmp) = create_test_server().await;
    let registry = state.registry.clone();
    let token = test_bearer();

    let original_audio = b"secret_ai_voice_model_data_12345".to_vec();

    // 1. Upload
    let multipart = axum_test::multipart::MultipartForm::new().add_part(
        "file",
        axum_test::multipart::Part::bytes(original_audio.clone()).file_name("model.aivoice"),
    );

    let response = server
        .post("/api/v1/voice/upload")
        .add_header(axum::http::header::AUTHORIZATION, &token)
        .multipart(multipart)
        .await;

    assert_eq!(response.status_code(), axum::http::StatusCode::OK);
    let json: serde_json::Value = response.json();
    let asset_id_str = json["asset_id"].as_str().unwrap();
    let asset_id = uuid::Uuid::parse_str(asset_id_str).unwrap();

    let agent_id_str = json["creator_id"].as_str().unwrap();
    let agent_id = uuid::Uuid::parse_str(agent_id_str).unwrap();

    let db_path = tmp.path().join("test.db");
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&format!("sqlite:{}", db_path.to_str().unwrap()))
        .await
        .unwrap();

    // 2. Validate registry
    let owns = registry.check_ownership(agent_id, asset_id).await.unwrap();
    assert!(owns, "The uploader must own the uploaded asset");

    // 3. Verify file on disk
    let vault_dir = tmp.path().join(".abyss_vault");
    let file_path = vault_dir.join(format!("{}.aivoice", asset_id));
    assert!(
        file_path.exists(),
        "The encrypted voice asset should exist on disk"
    );

    // 4. Decrypt manually using AbyssVoiceVault
    let encrypted_data = tokio::fs::read(&file_path).await.unwrap();

    // Check if it's actually encrypted (not matching original)
    assert_ne!(encrypted_data, original_audio, "File must be encrypted");

    // Reconstruct vault
    let vault = infrastructure::security::abyss_voice_vault::AbyssVoiceVault::new(
        (*registry).clone(),
        infrastructure::db::DatabasePool::Sqlite(pool),
    );
    vault.restore_keys_from_db().await.unwrap();

    use aiome_core_contracts::voice_vault::VoiceKeyVault;
    let decrypted = vault
        .decrypt_stream(agent_id, asset_id, &encrypted_data)
        .await
        .unwrap();

    // 5. Assert equal
    assert_eq!(
        decrypted, original_audio,
        "Decrypted data must exactly match original uploaded audio"
    );
}

#[serial]
#[tokio::test]
async fn test_synergy_demo_routes_visibility() {
    let (server, _state, _tmp) = create_test_server().await;
    let auth = test_bearer();

    // 1. Synergy Test Routes
    let resp = server
        .post("/api/synergy/test/failure")
        .add_header(axum::http::header::AUTHORIZATION, &auth)
        .await;

    #[cfg(feature = "dev-routes")]
    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::OK,
        "dev-routes is enabled, should return 200 OK"
    );

    #[cfg(not(debug_assertions))]
    {
        assert_eq!(
            resp.status_code(),
            axum::http::StatusCode::NOT_FOUND,
            "Synergy test routes must be 404 in non-debug builds"
        );
    }

    // 2. Settings Test Connection Route
    let resp_settings = server
        .post("/api/v1/settings/test")
        .add_header(axum::http::header::AUTHORIZATION, &auth)
        .json(&json!({
            "service": "ollama",
            "url": "http://localhost:11434"
        }))
        .await;

    #[cfg(not(debug_assertions))]
    {
        assert_eq!(
            resp_settings.status_code(),
            axum::http::StatusCode::NOT_FOUND,
            "Settings test route must be 404 in non-debug builds"
        );
    }
}

#[serial]
#[tokio::test]
async fn test_quarantine_audit_api() {
    let (server, state, _tmp) = create_test_server().await;
    let system_id = state.system_agent_id;
    let system_auth = format!("Bearer mock_valid_token_sysadmin:{}", system_id);

    // 1. System Agent access: Expect 200 OK
    let resp = server
        .get("/api/v1/audit/quarantine")
        .add_header(axum::http::header::AUTHORIZATION, &system_auth)
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);

    // 2. Unauthorized access: Another agent_id
    let other_id = uuid::Uuid::new_v4();
    let other_auth = format!("Bearer mock_valid_token_testuser:{}", other_id);
    let resp_forbidden = server
        .get("/api/v1/audit/quarantine")
        .add_header(axum::http::header::AUTHORIZATION, &other_auth)
        .await;
    assert_eq!(
        resp_forbidden.status_code(),
        axum::http::StatusCode::FORBIDDEN
    );
}

#[serial]
#[tokio::test]
async fn test_quarantine_release_api() {
    let (server, state, _tmp) = create_test_server().await;
    let system_id = state.system_agent_id;
    let system_auth = format!("Bearer mock_valid_token_sysadmin:{}", system_id);

    let asset_id = uuid::Uuid::new_v4().to_string();

    // 1. System Agent access: Expect 200 OK
    let resp = server
        .post(&format!("/api/v1/audit/quarantine/{}/release", asset_id))
        .add_header(axum::http::header::AUTHORIZATION, &system_auth)
        .await;

    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::OK,
        "System admin should be able to release quarantined assets"
    );

    // 2. Unauthorized access: Another agent_id
    let other_id = uuid::Uuid::new_v4();
    let other_auth = format!("Bearer mock_valid_token_testuser:{}", other_id);
    let resp_forbidden = server
        .post(&format!("/api/v1/audit/quarantine/{}/release", asset_id))
        .add_header(axum::http::header::AUTHORIZATION, &other_auth)
        .await;

    assert_eq!(
        resp_forbidden.status_code(),
        axum::http::StatusCode::FORBIDDEN,
        "Normal users should not be able to release quarantined assets"
    );
}

#[serial]
#[tokio::test]
async fn test_oauth2_endpoints_stub() {
    let (server, _state, _tmp) = create_test_server().await;

    // 1. Authorize: GET
    let resp = server
        .get("/api/v1/auth/authorize?client_id=test&response_type=code")
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);

    // 2. Token: POST
    let resp = server
        .post("/api/v1/auth/token")
        .json(&json!({"grant_type": "authorization_code", "code": "mock"}))
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);
}

#[serial]
#[tokio::test]
async fn test_voice_asset_list() {
    let (server, _state, _tmp) = create_test_server().await;

    // 1. Fetch public voice models
    let resp = server
        .get("/api/v1/voice/list?scope=public")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);
    let json: serde_json::Value = resp.json();
    assert!(json.as_array().is_some(), "Response should be a JSON array");
}

#[serial]
#[tokio::test]
async fn test_ekyc_session_creation() {
    let (server, _state, _tmp) = create_test_server().await;

    let resp = server
        .post("/api/v1/ekyc/session")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);
    let json: serde_json::Value = resp.json();
    assert!(json.get("session_url").is_some());
    assert!(json.get("session_id").is_some());
}

#[serial]
#[tokio::test]
async fn test_inochi2d_upload() {
    let (server, _state, _tmp) = create_test_server().await;

    // valid INX magic: INX\x02
    let mut payload = b"INX\x02".to_vec();
    payload.extend(vec![0u8; 100]); // dummy data

    let multipart = axum_test::multipart::MultipartForm::new().add_part(
        "file",
        axum_test::multipart::Part::bytes(payload.clone()).file_name("model.inx"),
    );

    let resp = server
        .post("/api/v1/avatar/inochi2d/upload")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .multipart(multipart)
        .await;

    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);
}

#[serial]
#[tokio::test]
async fn test_gift_policy_dynamic() {
    let (server, _state, _tmp) = create_test_server().await;

    // Auth token corresponds to agent_id 00000000-0000-0000-0000-000000000001
    let agent_id = "00000000-0000-0000-0000-000000000001";
    let resp = server
        .get(&format!("/api/v1/gift/policy/{}", agent_id))
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);
    let json: serde_json::Value = resp.json();
    assert_eq!(json["max_amount_usd"], 5.0);
    assert_eq!(json["daily_limit_reached"], false);
}

#[serial]
#[tokio::test]
async fn test_gift_send_success() {
    let (server, _state, _tmp) = create_test_server().await;

    let agent_id = "00000000-0000-0000-0000-000000000001";
    let payload = json!({
        "recipient_email": "test@example.com",
        "amount_usd": 2.5,
        "reason": "Test gift"
    });

    let verified_bearer = "Bearer mock_valid_token_ekyc_test_user".to_string();

    let resp = server
        .post(&format!("/api/v1/gift/send/{}", agent_id))
        .add_header(axum::http::header::AUTHORIZATION, verified_bearer)
        .json(&payload)
        .await;

    assert_eq!(resp.status_code(), axum::http::StatusCode::CREATED);
    let json: serde_json::Value = resp.json();
    assert_eq!(json["status"], "Sent");
    assert!(json.get("order_id").is_some());
}

#[serial]
#[tokio::test]
async fn test_gift_send_unverified_blocked() {
    let (server, _state, _tmp) = create_test_server().await;

    let agent_id = "00000000-0000-0000-0000-000000000001";
    let payload = json!({
        "recipient_email": "hacker@example.com",
        "amount_usd": 5.0,
        "reason": "Unverified gift"
    });

    let unverified_bearer = "Bearer mock_valid_token_unverified_user".to_string();

    let resp = server
        .post(&format!("/api/v1/gift/send/{}", agent_id))
        .add_header(axum::http::header::AUTHORIZATION, unverified_bearer)
        .json(&payload)
        .await;

    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::FORBIDDEN,
        "Unverified user should not be able to send gifts"
    );
}

#[serial]
#[tokio::test]
async fn test_commerce_purchase_unverified_blocked() {
    let (server, _state, _tmp) = create_test_server().await;

    let agent_id = "00000000-0000-0000-0000-000000000001";
    let payload = json!({
        "item_id": "00000000-0000-0000-0000-000000000002",
        "metadata": {}
    });

    let unverified_bearer = "Bearer mock_valid_token_unverified_user".to_string();

    let resp = server
        .post(&format!("/api/v1/commerce/purchase/{}", agent_id))
        .add_header(axum::http::header::AUTHORIZATION, unverified_bearer)
        .json(&payload)
        .await;

    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::FORBIDDEN,
        "Unverified user should not be able to execute purchases"
    );
}

#[serial]
#[tokio::test]
async fn test_voice_upload_limit() {
    let (server, _state, _tmp) = create_test_server().await;

    // After relaxation to 500MB, 110MB should be ACCEPTED by the limit layer.
    let large_data = vec![0u8; 110 * 1024 * 1024];

    let resp = server
        .post("/api/v1/voice/upload")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .bytes(large_data.into())
        .await;

    // Expected GREEN: Should NOT be PayloadTooLarge anymore.
    assert_ne!(
        resp.status_code(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "110MB voice upload should be accepted by the new 500MB limit"
    );
}

#[serial]
#[tokio::test]
async fn test_fallback_router_failover() {
    let tmp_dir = tempfile::TempDir::new().expect("tmp dir creation failed");
    let db_path = tmp_dir.path().join("test_failover.db");

    let pool = infrastructure::db::DatabasePool::new_sqlite(&format!(
        "sqlite://{}",
        db_path.to_str().unwrap()
    ))
    .await
    .expect("Failed to create test DB pool");

    let ts = std::sync::Arc::new(
        infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
    );
    let job_queue = Arc::new(
        infrastructure::job_queue::UniversalJobQueue::new(
            &format!("sqlite://{}", db_path.to_str().unwrap()),
            None,
            ts,
        )
        .await
        .expect("Failed to create test job queue"),
    );

    // 1. Setup primary that always fails
    use aiome_core::llm_provider::MockLlmProvider;
    let primary = Arc::new(MockLlmProvider {
        response: "primary failed".to_string(),
        should_fail: true,
    });

    // 2. Setup fallback that succeed
    let fallback = Arc::new(MockLlmProvider {
        response: "fallback success".to_string(),
        should_fail: false,
    });

    let router: Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync> = Arc::new(
        infrastructure::llm::fallback_router::FallbackRouter::new(primary, fallback, 3),
    );

    // 3. Register state with this router
    let state = AppState {
        hook_chain: Default::default(),
        registry: Component::new(Arc::new(infrastructure::registry::RegistryManager::new(
            job_queue.get_pool().clone(),
        ))),
        wasm_skill_manager: Component::new(Arc::new(
            infrastructure::skills::WasmSkillManager::new(
                tmp_dir.path().join("skills").to_str().unwrap(),
                tmp_dir.path().join("sandbox").to_str().unwrap(),
            )
            .unwrap(),
        )),
        job_queue: Component::new(job_queue.clone()),
        config: Component::new(std::sync::Arc::new(shared::config::AiomeConfig::default())),
        provider: Component::new(router.clone()),
        health_monitor: Component::new(Arc::new(Mutex::new(HealthMonitor::new()))),
        ..Default::default()
    };

    let metrics_handle = GLOBAL_METRICS_HANDLE.clone();

    let app = build_app(
        state,
        CorsLayer::new().allow_origin(tower_http::cors::AllowOrigin::any()),
        "static",
        plugin_loader::PluginRegistry::new(),
        metrics_handle,
    );
    let _server = TestServer::new(app).unwrap();

    // 4. Trigger chat via the router handle we kept
    let res = router.complete("hello", None).await.unwrap();
    assert_eq!(res.content, "fallback success");
}

#[serial]
#[tokio::test]
async fn test_tts_worker_flow_red() {
    use aiome_core::expression::tts_worker::TtsWorker;
    use aiome_core::expression::Expression;
    use aiome_core_contracts::expression::TtsStatus;

    let (server, state, tmp) = create_test_server().await;
    let jq = state.job_queue.clone();

    let artifacts_dir = tmp.path().join("artifacts");

    // 1. Create a pending expression
    let expr = Expression {
        id: "tts-test-1".into(),
        content: "Testing TTS worker flow".into(),
        emotion: "neutral".into(),
        karma_refs: vec![],
        audio_path: None,
        duration_ms: None,
        tts_status: TtsStatus::NotRequested,
        avatar_params: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    JobQueue::store_expression(&**jq, &expr).await.unwrap();

    // 2. Run TtsWorker (Expected to FAIL if XTTS is offline)
    // We use a dummy/invalid endpoint to guarantee RED if not handled or no server.
    let provider = infrastructure::tts::XttsProvider::new("http://invalid.local:18020".to_string());
    let res = TtsWorker::process_pending_tts(jq.as_ref(), &provider, "p225", &artifacts_dir).await;

    // In a real TDD Red, we expect an error or some indication of failure
    // because the server is not there.
    assert!(
        res.is_ok(),
        "Worker should handle connection errors internally and return Ok(processed_count)"
    );
    assert_eq!(
        res.unwrap(),
        0,
        "Processed count should be 0 because connection failed"
    );

    // 3. Verify status in DB (should be Failed or still Generating/NotRequested depending on retry logic)
    let fetched = JobQueue::fetch_expressions(&**jq, 1).await.unwrap();
    assert_eq!(
        fetched[0].tts_status,
        TtsStatus::Failed,
        "Status should be Failed after connection error"
    );
}

#[serial]
#[tokio::test]
async fn test_gig_lifecycle() {
    let (server, _state, _tmp) = create_test_server().await;
    let auth = test_bearer();
    let requester_id = uuid::Uuid::new_v4();

    // 1. Publish Intent
    let publish_req = json!({
        "id": uuid::Uuid::new_v4(),
        "requester_id": requester_id,
        "description": "Test work request",
        "criteria": [
            {
                "type": "OracleJudge",
                "config": {
                    "rubric_prompt": "Is it good?",
                    "min_score": 0.0,
                    "model": null
                }
            }
        ],
        "max_budget_coins": 100,
        "category": "Other",
        "deadline": (chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339()
    });

    let resp = server
        .post("/api/v1/gig/publish")
        .add_header(axum::http::header::AUTHORIZATION, &auth)
        .json(&publish_req)
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::CREATED);
    let intent_id_json = resp.json::<serde_json::Value>();
    let intent_id = uuid::Uuid::parse_str(intent_id_json["id"].as_str().unwrap()).unwrap();

    // 2. Submit Bid
    let bid_id = uuid::Uuid::new_v4();
    let bidder_id = uuid::Uuid::new_v4();
    let bid_req = json!({
        "id": bid_id,
        "intent_id": intent_id,
        "bidder_id": bidder_id,
        "price_coins": 50,
        "est_duration_sec": 3600,
        "deposit_amount": 10
    });

    let resp = server
        .post("/api/v1/gig/bid")
        .add_header(axum::http::header::AUTHORIZATION, &auth)
        .json(&bid_req)
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);

    // 3. Accept Bid
    let resp = server
        .post(&format!("/api/v1/gig/accept/{}/{}", intent_id, bid_id))
        .add_header(axum::http::header::AUTHORIZATION, &auth)
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);

    // 4. Deliver
    let gig_artifacts_dir = _tmp.path().join("gig_artifacts");
    if !gig_artifacts_dir.exists() {
        std::fs::create_dir_all(&gig_artifacts_dir).unwrap();
    }
    std::fs::write(gig_artifacts_dir.join("artifact.txt"), "mock").unwrap();
    let deliver_req = json!({
        "order_id": intent_id,
        "deliverer_id": bidder_id,
        "artifact_path": "artifact.txt",
        "metadata": {}
    });
    let resp = server
        .post("/api/v1/gig/deliver")
        .add_header(axum::http::header::AUTHORIZATION, &auth)
        .json(&deliver_req)
        .await;
    println!("Deliver 500 Response: {:?}", resp.text());
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);

    // 5. Verify
    let resp = server
        .post(&format!("/api/v1/gig/verify/{}", intent_id))
        .add_header(axum::http::header::AUTHORIZATION, &auth)
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);
    let verify_res = resp.json::<aiome_core_contracts::gig::VerificationResult>();
    assert!(verify_res.passed);
}

#[serial]
#[tokio::test]
#[serial]
#[cfg(not(debug_assertions))]
async fn test_test_endpoints_are_inaccessible_in_release() {
    let (server, _state, _dir) = create_test_server().await;

    let res = server.post("/api/synergy/test/failure").await;
    let code = res.status_code();
    assert!(
        code == axum::http::StatusCode::NOT_FOUND
            || code == axum::http::StatusCode::METHOD_NOT_ALLOWED,
        "Status was: {}",
        code
    );

    let res = server.post("/api/synergy/test/security").await;
    let code = res.status_code();
    assert!(
        code == axum::http::StatusCode::NOT_FOUND
            || code == axum::http::StatusCode::METHOD_NOT_ALLOWED,
        "Status was: {}",
        code
    );

    let res = server.post("/api/synergy/test/federation").await;
    let code = res.status_code();
    assert!(
        code == axum::http::StatusCode::NOT_FOUND
            || code == axum::http::StatusCode::METHOD_NOT_ALLOWED,
        "Status was: {}",
        code
    );

    let res = server
        .post("/api/v1/settings/test")
        .json(&serde_json::json!({}))
        .await;
    let code = res.status_code();
    assert!(
        code == axum::http::StatusCode::NOT_FOUND
            || code == axum::http::StatusCode::METHOD_NOT_ALLOWED,
        "Status was: {}",
        code
    );
}

#[serial]
#[tokio::test]
#[serial]
#[cfg(debug_assertions)]
async fn test_test_endpoints_are_accessible_in_debug() {
    let (server, _state, _dir) = create_test_server().await;

    // test endpoints might require authorization and params, so 401/400 is fine, but NOT 404.
    let res = server.post("/api/synergy/test/failure").await;
    assert_ne!(res.status_code(), axum::http::StatusCode::NOT_FOUND);

    let res = server
        .post("/api/v1/settings/test")
        .json(&serde_json::json!({}))
        .await;
    assert_ne!(res.status_code(), axum::http::StatusCode::NOT_FOUND);
}
#[serial]
#[tokio::test]
async fn test_treasure_get_recommendations() {
    let (server, state, _tmp) = create_test_server().await;
    let agent_id = uuid::Uuid::new_v4();

    // Generate valid token for the test agent
    let token = state
        .auth_manager
        .issue_token(shared::auth::AiomeCustomClaims {
            sub: "test_user".to_string(),
            iss: "aiome-test".to_string(),
            roles: vec!["user".into()],
            ekyc_verified: true,
            agent_id,
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            iat: chrono::Utc::now().timestamp() as usize,
        })
        .await
        .unwrap();

    // 1. Get recommendations (Treasure Box)
    let response = server
        .get("/api/v1/treasure")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    // This should fail initially (RED) as the GET route is not yet defined
    assert_eq!(response.status_code(), axum::http::StatusCode::OK);

    let items = response.json::<Vec<serde_json::Value>>();
    assert!(
        !items.is_empty(),
        "Should receive at least one recommendation"
    );
    assert!(
        items[0]["id"].as_str().is_some(),
        "Recommendations should have IDs"
    );

    // 2. Record feedback (AS-1.7: Karma Reward)
    let item_id = items[0]["id"].as_str().unwrap();
    let feedback_req = serde_json::json!({
        "item_id": item_id,
        "action": "click"
    });

    let response = server
        .post("/api/v1/treasure/feedback")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&feedback_req)
        .await;

    assert_eq!(response.status_code(), axum::http::StatusCode::OK);

    // Check if resonance increased (via AgentStats)
    let stats: aiome_core_contracts::types::AgentStats = state
        .job_queue
        .get_agent_stats()
        .await
        .expect("Failed to get initialized stats");
    assert!(
        stats.resonance > 0,
        "Agent resonance should increase after feedback"
    );
}

#[serial]
#[tokio::test]
async fn test_autonomous_demo_lifecycle() {
    let (server, _state, _tmp) = create_test_server().await;

    // Start the demo
    let resp = server
        .post("/api/v1/demo/start")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    // This should pass now (GREEN)
    assert_eq!(resp.status_code(), StatusCode::OK);
    let json = resp.json::<serde_json::Value>();
    assert_eq!(json["success"], true);
    assert!(json["message"]
        .as_str()
        .unwrap()
        .contains("Autonomous demo started"));
}

#[serial]
#[tokio::test]
async fn test_subscription_lifecycle() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    // 1. Create Subscription
    let payload = json!({
        "agent_id": "00000000-0000-0000-0000-000000000001",
        "plan_id": "price_gold_monthly"
    });

    let resp = server
        .post("/api/v1/commerce/subscription/create")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let json = resp.json::<serde_json::Value>();
    assert_eq!(json["subscription_id"], "sub_mock_123");

    // 2. Get Status
    let status_resp = server
        .get("/api/v1/commerce/subscription/00000000-0000-0000-0000-000000000001")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(status_resp.status_code(), StatusCode::OK);
    let status_json = status_resp.json::<aiome_core_contracts::commerce::SubscriptionStatus>();
    assert_eq!(
        status_json,
        aiome_core_contracts::commerce::SubscriptionStatus::Active
    );

    // 3. Cancel Subscription
    let cancel_resp = server
        .post("/api/v1/commerce/subscription/cancel")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&json!({"subscription_id": "sub_mock_123"}))
        .await;

    assert_eq!(cancel_resp.status_code(), StatusCode::OK);
}

#[serial]
#[tokio::test]
async fn test_syndicate_guild_api_flow() {
    let (server, _state, _tmp_dir) = create_test_server().await;
    let bearer = test_bearer(); // sub-1

    // 1. Create Guild
    let create_req = serde_json::json!({
        "name": "Integration Syndicate",
        "description": "Formed by API"
    });
    let resp = server
        .post("/api/v1/syndicate/guilds")
        .add_header("Authorization", &bearer)
        .json(&create_req)
        .await;
    resp.assert_status(axum::http::StatusCode::CREATED);
    let guild_id: uuid::Uuid = resp.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    // 2. List Guilds
    let resp = server
        .get("/api/v1/syndicate/guilds")
        .add_header("Authorization", &bearer)
        .await;
    resp.assert_status(axum::http::StatusCode::OK);
    let guilds: Vec<aiome_core_contracts::syndicate::Guild> = resp.json();
    assert!(guilds.iter().any(|g| g.id == guild_id));

    // 3. Add Member
    let other_agent_id = uuid::Uuid::new_v4();
    let add_req = serde_json::json!({
        "agent_id": other_agent_id,
        "role": "contributor"
    });
    let resp = server
        .post(&format!("/api/v1/syndicate/guilds/{}/members", guild_id))
        .add_header("Authorization", &bearer)
        .json(&add_req)
        .await;
    resp.assert_status(axum::http::StatusCode::OK);

    // 4. List Members
    let resp = server
        .get(&format!("/api/v1/syndicate/guilds/{}/members", guild_id))
        .add_header("Authorization", &bearer)
        .await;
    resp.assert_status(axum::http::StatusCode::OK);
    let members: Vec<aiome_core_contracts::syndicate::GuildMember> = resp.json();
    assert_eq!(members.len(), 2); // Owner + New Member

    // 5. Delete Guild
    let resp = server
        .delete(&format!("/api/v1/syndicate/guilds/{}", guild_id))
        .add_header("Authorization", &bearer)
        .await;
    resp.assert_status(axum::http::StatusCode::OK);
}

#[serial]
#[tokio::test]
async fn test_syndicate_guild_sanitization() {
    let (server, _state, _tmp_dir) = create_test_server().await;
    let bearer = test_bearer();

    // 1. Create Guild with "dirty" input
    let create_req = serde_json::json!({
        "name": "<script>alert('xss')</script>Safe Guild",
        "description": "<b>Description</b> with <iframe src='malicious.com'></iframe>"
    });
    let resp = server
        .post("/api/v1/syndicate/guilds")
        .add_header("Authorization", &bearer)
        .json(&create_req)
        .await;
    resp.assert_status(axum::http::StatusCode::CREATED);
    let guild_id: uuid::Uuid = resp.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    // 2. Fetch Guilds and verify sanitization
    let resp = server
        .get("/api/v1/syndicate/guilds")
        .add_header("Authorization", &bearer)
        .await;
    resp.assert_status(axum::http::StatusCode::OK);
    let guilds: Vec<aiome_core_contracts::syndicate::Guild> = resp.json();
    let guild = guilds
        .iter()
        .find(|g| g.id == guild_id)
        .expect("Guild not found");

    // Expecting: "Safe Guild" and "Description with " (or similar depending on purge_entities)
    // purge_entities usually strips tags.
    assert_eq!(guild.name, "Safe Guild");
    assert_eq!(guild.description.as_ref().unwrap(), "Description with");
}

#[serial]
#[tokio::test]
async fn test_oracle_job_review_api() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let payload = serde_json::json!({
        "status": "approved",
        "comments": "Looking good"
    });

    let resp = server
        .post("/api/v1/jobs/test-job-id/review")
        .add_header("Authorization", &bearer)
        .json(&payload) // we can send an empty body or review payload
        .await;

    // We expect this to fail initially since the endpoint is not implemented (RED phase)
    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::ACCEPTED,
        "Expected ACCEPTED or OK from new review endpoint"
    );
}

#[serial]
#[tokio::test]
async fn test_compute_semaphore_limits_concurrency() {
    let (_server, state, _tmp) = create_test_server().await;

    // compute_semaphore が正しく 1 で初期化されているかをテストする
    let semaphore = state.compute_semaphore.get_inner();
    let permits = semaphore.available_permits();

    // Mac Unified Memory の枯渇を防ぐため、並列実行可能な重いタスク（LoRA, ImageGen）は「1つ」のみに制限されるべき
    assert_eq!(
        permits, 1,
        "compute_semaphore must restrict heavy tasks to 1 concurrent execution to prevent OOM / Kernel panic"
    );

    // 強制的にロックを取得して1つ消費した場合、もう available は 0 になることを確認
    let _permit = semaphore
        .try_acquire()
        .expect("Should be able to acquire the single compute permit");
    assert_eq!(semaphore.available_permits(), 0);
}

/// P1-3: Architectural Guard — HookChain Bypass Eradication Test
///
/// This test statically verifies that every call to `execute_wasm_skill` and
/// `execute_forge_command` in the api-server source code is preceded by a
/// HookChain check. Files that call these functions must also reference
/// `HookVerdict` (proving HookChain integration).
///
/// If this test fails, a new bypass path has been introduced.
#[test]
fn test_hookchain_bypass_eradication() {
    use std::fs;
    use std::path::Path;

    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // Files allowed to call execute_wasm_skill/execute_forge_command directly
    // (because they ARE the execution layer, not callers)
    let allowlist = [
        "skill_handler.rs",         // Definition site
        "api_integration_tests.rs", // Test infrastructure
    ];

    let mut violations = Vec::new();

    fn scan_dir(dir: &Path, allowlist: &[&str], violations: &mut Vec<String>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    scan_dir(&path, allowlist, violations);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let filename = path.file_name().unwrap().to_str().unwrap();
                    if allowlist.contains(&filename) {
                        continue;
                    }
                    if let Ok(content) = fs::read_to_string(&path) {
                        let has_direct_call = content.contains("execute_wasm_skill")
                            || content.contains("execute_forge_command");
                        let has_hookchain = content.contains("HookVerdict");

                        if has_direct_call && !has_hookchain {
                            violations.push(format!(
                                "{}: calls execute_wasm_skill/execute_forge_command without HookChain (missing HookVerdict)",
                                path.display()
                            ));
                        }
                    }
                }
            }
        }
    }

    scan_dir(&src_dir, &allowlist, &mut violations);

    assert!(
        violations.is_empty(),
        "🚨 HookChain BYPASS DETECTED! The following files call skill execution functions without HookChain integration:\n{}",
        violations.join("\n")
    );
}

#[serial]
#[tokio::test]
async fn test_security_regression_sentinel_block() {
    let (_server, mut state, _tmp) = create_test_server().await;

    // We mock the LLM to return a Sentinel block response.
    #[derive(Debug)]
    struct SentinelLlm;
    #[async_trait::async_trait]
    impl aiome_core::llm_provider::LlmProvider for SentinelLlm {
        async fn complete(
            &self,
            _prompt: &str,
            _sys: Option<&str>,
        ) -> Result<aiome_core_contracts::llm::LlmResponse, aiome_core::error::AiomeError> {
            Ok(aiome_core_contracts::llm::LlmResponse {
                content: r#"{"status": "blocked", "reason": "malicious code execution detected", "violated_pattern": "rm -rf"} "#.into(),
                metadata: None,
                reasoning: None,
                stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
            })
        }
        async fn test_connection(&self) -> Result<(), aiome_core::error::AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "SentinelLlm"
        }
    }

    // Use the sentinel LLM
    state.provider = Component::new(std::sync::Arc::new(SentinelLlm));

    let reply = r#"malicious_tool { "cmd": "rm -rf /" }"#;
    let mut steps = 0;

    let results =
        crate::tool_call_processor::process_generated_tool_calls(reply, &state, &mut steps, None)
            .await;

    assert_eq!(results.len(), 1);
    let msg = &results[0];
    assert!(
        msg.contains("[SENTINEL BLOCK]") || msg.contains("[GUARDRAIL BLOCK]"),
        "Expected rm -rf to be blocked by sentinel, got: {}",
        msg
    );
}

#[serial]
#[tokio::test]
async fn test_security_regression_path_traversal() {
    let (_server, state, _tmp) = create_test_server().await;

    // Attempt to parse tool calls with path traversal
    let reply = r#"../../etc/passwd { "data": "exploit" }"#;
    let calls = crate::tool_call_processor::parse_tool_calls(reply);

    // Test that the tool parser actually ignores or fails to parse invalid skill names
    // We expect the parser to drop it, or process_generated_tool_calls to block it

    let mut steps = 0;
    let results =
        crate::tool_call_processor::process_generated_tool_calls(reply, &state, &mut steps, None)
            .await;

    // The parse_tool_calls function safely drops tool names with invalid characters (like `/` or `.`).
    // If it dropped it, results is empty, which means it safely blocked the traversal.
    // If it somehow parsed it, it MUST have blocked it via Sentinel/Guardrail.
    if results.is_empty() {
        // Success condition: the parser refused to parse the exploit
        // Successfully passed Watchtower DR rules
    } else {
        let msg = &results[0];
        assert!(
            msg.contains("Error")
                || msg.contains("not found")
                || msg.contains("Invalid")
                || msg.contains("[SENTINEL BLOCK]")
                || msg.contains("Failed to evaluate")
                || msg.contains("Unknown"),
            "Expected explicit failure for path traversal, got: {}",
            msg
        );
    }
}

#[serial]
#[tokio::test]
async fn test_cortex_wiki_unauthorized() {
    let (server, _state, _tmp) = create_test_server().await;
    let response = server.get("/api/v1/cortex/wiki").await;
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[serial]
#[tokio::test]
async fn test_cortex_wiki_authorized() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    // Insert dummy article directly into DB so we can test the API
    let pool = state.job_queue.get_pool().get_sqlite_pool_or_err().unwrap();
    sqlx::query(
        "INSERT INTO cortex_wiki_articles (id, title, content_md, concepts, backlinks, source_refs, content_hash, version) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind("test-article-id")
    .bind("Test Article")
    .bind("## Content")
    .bind("[\"Test\"]")
    .bind("[]")
    .bind("[\"doc1\"]")
    .bind("hash123")
    .bind(1)
    .execute(pool)
    .await.unwrap();

    let response = server
        .get("/api/v1/cortex/wiki")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let articles: Vec<serde_json::Value> = response.json();
    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0]["id"], "test-article-id");
    assert_eq!(articles[0]["title"], "Test Article");

    let response = server
        .get("/api/v1/cortex/wiki/test-article-id")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let article: serde_json::Value = response.json();
    assert_eq!(article["id"], "test-article-id");
    assert_eq!(article["title"], "Test Article");
    assert_eq!(article["content_md"], "## Content");
}

#[serial]
#[tokio::test]
async fn test_model_status_unauthorized() {
    let (server, _state, _tmp) = create_test_server().await;
    let response = server.get("/api/v1/models/status").await;
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[serial]
#[tokio::test]
async fn test_model_status_authorized() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    // We expect 501 Not Implemented because of TDD RED phase
    let response = server
        .get("/api/v1/models/status")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
}

#[serial]
#[tokio::test]
async fn test_mcp_config_update_unauthorized() {
    let (server, _state, _tmp) = create_test_server().await;
    let payload = serde_json::json!({
        "mcp_servers": {}
    });
    let response = server.put("/api/skills/mcp/config").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[serial]
#[tokio::test]
async fn test_mcp_config_update_authorized_green() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();
    let payload = serde_json::json!({
        "mcp_servers": {}
    });
    let response = server
        .put("/api/skills/mcp/config")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
}
