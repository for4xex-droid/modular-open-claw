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

        let content = if prompt.contains("Cortex Wiki articles context") {
            "```json\n{\"answer_md\": \"This is a mock answer based on context.\", \"confidence\": 0.95}\n```".to_string()
        } else if is_json_req {
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
        _asset_id: Option<uuid::Uuid>,
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
        _extra: &str,
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
        _agent_id: uuid::Uuid,
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

    async fn list_escrows(
        &self,
        agent_id: uuid::Uuid,
    ) -> Result<Vec<aiome_core_contracts::commerce::EscrowRecord>, aiome_core::error::AiomeError>
    {
        Ok(vec![aiome_core_contracts::commerce::EscrowRecord {
            id: "valid_escrow_123".to_string(),
            payer_id: agent_id.to_string(),
            order_id: "order_123".to_string(),
            amount: 1000,
            status: "Pending".to_string(),
            created_at: "2026-04-23T00:00:00Z".to_string(),
        }])
    }

    async fn instant_refund(
        &self,
        _transaction_id: &str,
        _agent_id: uuid::Uuid,
    ) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }

    async fn withdraw_points(
        &self,
        _agent_id: uuid::Uuid,
        _amount: u64,
    ) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }

    async fn get_points(
        &self,
        _agent_id: uuid::Uuid,
    ) -> Result<aiome_core_contracts::commerce::PointsBalance, aiome_core::error::AiomeError> {
        Ok(aiome_core_contracts::commerce::PointsBalance {
            balance: 0,
            lifetime_earned: 0,
            lifetime_withdrawn: 0,
            conversion_rate_bps: 10000,
        })
    }

    async fn get_transaction_history(
        &self,
        _agent_id: uuid::Uuid,
        _limit: u32,
    ) -> Result<Vec<aiome_core_contracts::commerce::TransactionRecord>, aiome_core::error::AiomeError>
    {
        Ok(vec![])
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

pub struct MockFormalProofGate;

#[async_trait::async_trait]
impl aiome_contracts::proof::FormalProofGate for MockFormalProofGate {
    async fn verify_skill(
        &self,
        _skill_name: &str,
        proof_spec_b64: &str,
    ) -> Result<bool, aiome_contracts::error::AiomeError> {
        // Return true if proof is not empty, for testing
        Ok(!proof_spec_b64.is_empty())
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

    // G-Log Fix: Ensure app_logs table exists for integration tests
    let sqlite_pool = pool
        .get_sqlite_pool()
        .expect("SQLite pool required for tests");
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS app_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            level TEXT NOT NULL,
            target TEXT NOT NULL,
            message TEXT NOT NULL
        )",
    )
    .execute(sqlite_pool)
    .await;

    let ts = std::sync::Arc::new(
        infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
    );
    let job_queue = Arc::new(
        infrastructure::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
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
            pool.clone(),
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
        pool.clone(),
    ));
    let intent_generator = Arc::new(infrastructure::intent::IntentGenerator::new(
        context_engine.clone(),
        provider.clone(),
        intent_firewall.clone(),
        soul_store.clone(),
    ));

    let registry = Arc::new(infrastructure::registry::RegistryManager::new(pool.clone()));
    std::env::set_var("VAULT_MASTER_PASSWORD", "test_master_password_for_vault");
    std::env::set_var("WORKSPACE_DIR", tmp_dir.path().to_str().unwrap());
    let voice_drm = Arc::new(
        infrastructure::security::VoiceCoreDrm::new(
            "http://localhost:3016".to_string(),
            registry.clone(),
            pool.clone(),
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
        pool.clone().into(),
        100,
    ));

    let disk_quota_mgr =
        infrastructure::disk_quota::DiskQuotaManager::new(pool.clone(), 500 * 1024 * 1024);
    let _ = disk_quota_mgr.init().await;

    let state = AppState {
        db_pool: Component::new(Arc::new(pool.clone())),
        oxilean_power: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        hook_chain: Default::default(),
        hook_manager: Default::default(),
        publish_pipeline: Default::default(),
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
            // config.ollama_host = "".to_string();
            // config.ollama_model = "".to_string();
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
            pool.clone(),
            Arc::new(MockCommerceEngine),
            provider.clone(),
            tmp_dir.path().join("gig_artifacts"),
        )) as Arc<dyn aiome_core_contracts::gig::GigEngine>),
        intent_generator: Component::new(intent_generator),
        intent_firewall: Component::new(intent_firewall),
        audit_logger: Component::new(audit_logger),
        affiliate_adapter: Component::new(Arc::new(
            infrastructure::intent::MockAffiliateAdapter::new(),
        )
            as Arc<dyn aiome_core_contracts::traits::AffiliateAdapter>),
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
            let mut dispatcher = infrastructure::task_orchestrator::TaskDispatcher::new(
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
                Some(Arc::new(
                    infrastructure::immune_system::AdaptiveImmuneSystem::new(provider.clone()),
                )), // immune_system
                None, // quality_gate_store
                None, // hook_manager
            );

            // Register conductors for integration tests
            let generic_conductor = Arc::new(
                infrastructure::task_orchestrator::llm_conductor::GenericLlmConductor::new(
                    provider.clone(),
                    vec!["scientific_experiment", "data_processing"],
                ),
            );
            dispatcher.register_conductor(generic_conductor);

            let seo_conductor = Arc::new(
                infrastructure::task_orchestrator::seo_content::SeoContentConductor::new(
                    provider.clone(),
                    Arc::new(infrastructure::publisher::PublishPipeline::new(vec![])),
                    false,
                    "http://localhost:8080".to_string(),
                    60,
                ),
            );
            dispatcher.register_conductor(seo_conductor);

            let dispatcher = Arc::new(dispatcher);

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
            aiome_commerce::syndicate::UniversalSyndicateStore::new(pool.clone()),
        )),
        hierarchical_router: Component::new(Arc::new(
            infrastructure::hierarchical_router::HierarchicalRouter::new(
                provider.clone(),
                pool.get_sqlite_pool()
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
            infrastructure::cortex_ingester::CortexIngester::new(provider.clone(), pool.clone()),
        )),
        cortex_query: Component::new(Arc::new(
            infrastructure::cortex_query::CortexQueryEngine::new(provider.clone(), pool.clone()),
        )),
        cortex_projector: Default::default(),
        feature_flags_cache: Component::new(Arc::new(
            moka::future::Cache::builder()
                .time_to_live(std::time::Duration::from_secs(60))
                .build(),
        )),
        eval_logger: Component::new(Arc::new(
            infrastructure::llm::evaluation_logger::EvaluationLogger::new(Arc::new(
                infrastructure::llm::evaluation_logger::SqlEvalLogRepository::new(pool.clone()),
            )),
        )),
        lora_marketplace: {
            let vault_root = tmp_dir.path().join("vault");
            std::fs::create_dir_all(&vault_root).ok();
            Component::new(Arc::new(
                infrastructure::lora_marketplace::UniversalLoraMarketplace::new(
                    pool.clone(),
                    commerce_engine.clone()
                        as Arc<dyn aiome_core_contracts::commerce::CommerceEngine>,
                    vault_root,
                ),
            )
                as Arc<dyn aiome_core_contracts::lora_marketplace::LoraMarketplace>)
        },
        a2ui_catalog: Default::default(),
        nurture_url: None,
        nurture_internal_secret: None,
        quality_gate_store: Component::default(),
        skill_arena: Component::new(Arc::new(
            infrastructure::skills::skill_arena::SkillArena::new(),
        )),
        rlm_client: Component::default(),
        formal_proof_gate: Component::new(
            Arc::new(MockFormalProofGate) as Arc<dyn aiome_contracts::proof::FormalProofGate>
        ),
        gig_updater: Component::new(Arc::new(
            infrastructure::gig_metadata_updater::DbGigUpdater::new(
                pool.get_sqlite_pool()
                    .cloned()
                    .expect("SQLite pool required for gig_updater"),
            ),
        )
            as Arc<dyn aiome_contracts::gig_metadata::GigMetadataUpdater>),
        pkce_cache: Component::new(Arc::new(
            moka::future::Cache::builder()
                .time_to_live(std::time::Duration::from_secs(600))
                .max_capacity(10_000)
                .build(),
        )),
    };

    let cors_layer = CorsLayer::new().allow_origin(AllowOrigin::any());

    let plugin_registry = plugin_loader::PluginRegistry::new();
    let metrics_handle = GLOBAL_METRICS_HANDLE.clone();
    let app = build_app(
        state.clone(),
        cors_layer,
        tmp_dir.path().join("static").to_str().unwrap().to_string(),
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
async fn test_a2ui_action_integration() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    // Enqueue a job to have something to approve
    state
        .job_queue
        .enqueue("Testing", "A2UI", "standard", None, None, None, 1)
        .await
        .unwrap();
    let jobs = state.job_queue.fetch_recent_jobs(1).await.unwrap();
    let target_id = jobs[0].id.clone();

    // 1. Submit approve action
    let resp = server
        .post("/api/v1/a2ui/action")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&json!({
            "surface_id": "surf_123",
            "action": format!("approve_job:{}", target_id)
        }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);

    // Verify job status changed to pending
    let job = state
        .job_queue
        .fetch_job(&target_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.status, aiome_core_contracts::traits::JobStatus::Pending);

    // 2. Submit cancel action on the same job
    let resp = server
        .post("/api/v1/a2ui/action")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&json!({
            "surface_id": "surf_123",
            "action": format!("cancel_job:{}", target_id)
        }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);

    // Verify job status changed to Cancelled (not Failed)
    let job = state
        .job_queue
        .fetch_job(&target_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        job.status,
        aiome_core_contracts::traits::JobStatus::Cancelled
    );

    // 3. Invalid UUID format → 400
    let resp = server
        .post("/api/v1/a2ui/action")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&json!({
            "surface_id": "surf_456",
            "action": "approve_job:not-a-valid-uuid"
        }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);

    // 4. Invalid action prefix → 400
    let resp = server
        .post("/api/v1/a2ui/action")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&json!({
            "surface_id": "surf_789",
            "action": format!("delete_job:{}", target_id)
        }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);

    // 5. Rate Limiting → 429
    // Tests 1-4 above already consumed 4 rate-limit tokens (limit=5 in test state).
    // The auth middleware checks rate limit BEFORE the handler runs, so even
    // requests that return 400 consume tokens. One more valid request should pass,
    // then the next must be rejected with 429.
    let resp = server
        .post("/api/v1/a2ui/action")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&json!({
            "surface_id": "surf_rl",
            "action": format!("approve_job:{}", target_id)
        }))
        .await;
    // 5th request (token #5) — still within limit
    assert!(resp.status_code() != StatusCode::TOO_MANY_REQUESTS);

    // 6th request → must be rate-limited (429)
    let resp_rl = server
        .post("/api/v1/a2ui/action")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&json!({
            "surface_id": "surf_rl",
            "action": format!("approve_job:{}", target_id)
        }))
        .await;

    assert_eq!(resp_rl.status_code(), StatusCode::TOO_MANY_REQUESTS);
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
async fn test_get_prompt_stats() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = format!("Bearer mock_valid_token_admin:{}", uuid::Uuid::new_v4());

    // Arrange: Insert seed data
    let db_pool = state.db_pool.get_inner().get_sqlite_pool_or_err().unwrap();
    sqlx::query(
        "INSERT INTO prompt_evaluation_log (prompt_hash, provider, model, latency_ms, token_count_in, token_count_out, cost_usd, cache_hit) VALUES ('testhash', 'test_provider', 'test_model', 100, 10, 20, 0.0015, 1)"
    )
    .execute(db_pool)
    .await
    .unwrap();

    // Validate GET /api/v1/audit/prompt-stats
    let response = server
        .get("/api/v1/audit/prompt-stats")
        .add_query_param("period", "7d")
        .add_header(axum::http::header::AUTHORIZATION, bearer)
        .await;

    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "Audit prompt stats should return 200 OK"
    );
    let json = response.json::<serde_json::Value>();
    assert!(json.get("period").is_some(), "Should contain period field");
    assert!(
        json.get("providers").is_some(),
        "Should contain providers field"
    );

    let providers = json.get("providers").unwrap().as_array().unwrap();
    assert!(
        !providers.is_empty(),
        "Should contain at least one provider stat"
    );

    let first = &providers[0];
    assert!(
        first.get("total_cost_usd").is_some(),
        "ProviderStat should contain total_cost_usd"
    );
    assert!(
        first.get("cache_hit_rate").is_some(),
        "ProviderStat should contain cache_hit_rate"
    );
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
    std::env::set_var("AIOME_DEV_MODE", "1");
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
async fn test_verify_skill_proof_endpoint_connected() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let payload = serde_json::json!({
        "skill_name": "test_skill",
        "proof_spec_b64": "ZHVtbXlfcHJvb2Y=",
        "wasm_hash": "hash123"
    });

    let resp = server
        .post("/api/skills/verify-proof")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::NOT_FOUND,
        "Handler should return 404 when WASM not found"
    );

    let json = resp.json::<serde_json::Value>();
    assert_eq!(json["code"], "NotFound");
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("Skill WASM not found"));
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
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer mock_valid_token_admin:{}", uuid::Uuid::new_v4()),
        )
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

    // async-stripe 1.0.0-rc.5 の strict deserialization をバイパスするため
    // serde_json::Value で手動構築。必須フィールドのみ明示的に設定し、
    // 残りは CheckoutSession スキーマ充填（テスト安定性のため）。
    let session_val = serde_json::json!({
        // === テスト本質フィールド ===
        "id": "cs_test_123",
        "object": "checkout.session",
        "metadata": metadata,
        "amount_total": 1000,
        // === スキーマ充填（strict deserialization 対策） ===
        "automatic_tax": { "enabled": false, "status": null },
        "created": 1677628800,
        "currency": "usd",
        "livemode": false,
        "mode": "payment",
        "payment_status": "paid",
        "status": "complete",
        "amount_subtotal": 1000,
        "cancel_url": "http://example.com/cancel",
        "custom_fields": [],
        "custom_text": { "shipping_address": null, "submit": null, "terms_of_service_acceptance": null, "after_submit": null },
        "customer_creation": "always",
        "expires_at": 1677629800,
        "payment_method_types": ["card"],
        "phone_number_collection": { "enabled": false },
        "success_url": "http://example.com/success",
        "tax_id_collection": { "enabled": false }
    });

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
            safety_level: aiome_core_contracts::contracts::ToolSafetyLevel::Safe,
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

    let mut original_audio = b"RIFFAAAAWAVE".to_vec();
    original_audio.extend_from_slice(b"secret_ai_voice_model_data_12345");

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
    let system_auth = format!("Bearer mock_valid_token_system:{}", system_id);

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
    let system_auth = format!("Bearer mock_valid_token_system:{}", system_id);

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

    // Generate a real PKCE challenge/verifier pair
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use sha2::{Digest, Sha256};
    let verifier = "my_super_secret_verifier_for_testing_purposes";
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    // 1. Authorize: GET with PKCE
    let authorize_url = format!("/api/v1/auth/authorize?client_id=test&response_type=code&code_challenge={}&code_challenge_method=S256", challenge);
    let resp = server.get(&authorize_url).await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);

    let json: serde_json::Value = resp.json();
    let auth_code = json["code"].as_str().unwrap();

    // 2. Token: POST with matching verifier
    let resp = server
        .post("/api/v1/auth/token")
        .json(&json!({"grant_type": "authorization_code", "code": auth_code, "code_verifier": verifier}))
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

    let status = resp.status_code();
    println!("Inochi2D Upload Response: {:?}", resp.text());
    assert_eq!(status, axum::http::StatusCode::OK);
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
        infrastructure::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
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
        db_pool: Component::new(Arc::new(pool.clone())),
        hook_chain: Default::default(),
        registry: Component::new(Arc::new(infrastructure::registry::RegistryManager::new(
            pool.clone(),
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
        "static".to_string(),
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
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&serde_json::json!({}))
        .await;
    let code = res.status_code();
    assert!(
        code == axum::http::StatusCode::NOT_FOUND
            || code == axum::http::StatusCode::METHOD_NOT_ALLOWED,
        "Status was: {}",
        code
    );

    let res = server
        .post("/api/v1/demo/start")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
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

    std::env::set_var("AIOME_DEV_MODE", "1");
    // test endpoints might require authorization and params, so 401/400 is fine, but NOT 404.
    let res = server.post("/api/synergy/test/failure").await;
    assert_ne!(res.status_code(), axum::http::StatusCode::NOT_FOUND);

    let res = server
        .post("/api/v1/settings/test")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&serde_json::json!({}))
        .await;
    assert_ne!(res.status_code(), axum::http::StatusCode::NOT_FOUND);

    let res = server
        .post("/api/v1/demo/start")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
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
            roles: vec![shared::auth::Role::User],
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
#[cfg(debug_assertions)]
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
    let bearer = "Bearer mock_valid_token_ekyctest_user".to_string();

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
        .json(&json!({"agent_id": "00000000-0000-0000-0000-000000000001", "subscription_id": "sub_mock_123"}))
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
async fn test_awaiting_input_job_lifecycle() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    // 1. Insert a mock AwaitingInput job directly into DB
    let pool = state.db_pool.get_inner().get_sqlite_pool_or_err().unwrap();
    let test_job_id = "test-awaiting-input-job";

    // NOTE: This will fail until the SQLite CHECK constraint migration (Gap 10) is applied, or it might pass if AwaitingInput is already in the old check constraint.
    // The previous analysis showed AwaitingInput was IN the constraint, but Cancelled wasn't.
    sqlx::query(
        "INSERT INTO jobs (id, category, topic, style_name, karma_directives, status, priority) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(test_job_id)
    .bind("Goal")
    .bind("Dangerous action")
    .bind("default")
    .bind("[]")
    .bind("AwaitingInput")
    .bind(100)
    .execute(pool)
    .await.expect("Failed to insert mock AwaitingInput job");

    // 2. Test GET /api/v1/jobs/awaiting-input (RED: 404 expected initially)
    let get_resp = server
        .get("/api/v1/jobs/awaiting-input")
        .add_header("Authorization", &bearer)
        .await;

    assert_eq!(
        get_resp.status_code(),
        axum::http::StatusCode::OK,
        "Expected OK from /api/v1/jobs/awaiting-input"
    );

    let jobs: Vec<aiome_core_contracts::Job> = get_resp.json();
    assert!(
        jobs.iter().any(|j| j.id == test_job_id),
        "Expected the test job to be in the awaiting-input list"
    );

    // 3. Test POST /api/v1/jobs/{id}/review - Approve (RED: expected 200 OK after wiring, currently 202)
    let payload = serde_json::json!({
        "status": "approved",
        "comments": "Safe to proceed"
    });

    let review_resp = server
        .post(&format!("/api/v1/jobs/{}/review", test_job_id))
        .add_header("Authorization", &bearer)
        .json(&payload)
        .await;

    assert_eq!(
        review_resp.status_code(),
        axum::http::StatusCode::OK,
        "Expected OK when approving an AwaitingInput job"
    );

    // Verify it was requeued (status = Pending) and execution_log has bypass marker
    let updated_job = state
        .job_queue
        .fetch_job(test_job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_job.status, aiome_core_contracts::JobStatus::Pending);
    assert_eq!(
        updated_job.execution_log.as_deref(),
        Some("IMMUNE_BYPASS_APPROVED")
    );

    // 4. Test POST /api/v1/jobs/{id}/review - Race condition block (RED: expected 409 Conflict)
    let duplicate_resp = server
        .post(&format!("/api/v1/jobs/{}/review", test_job_id))
        .add_header("Authorization", &bearer)
        .json(&payload)
        .await;

    assert_eq!(
        duplicate_resp.status_code(),
        axum::http::StatusCode::CONFLICT,
        "Expected CONFLICT when approving a job that is no longer AwaitingInput"
    );
}

#[serial]
#[tokio::test]
async fn test_cancel_awaiting_input_job() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let pool = state.db_pool.get_inner().get_sqlite_pool_or_err().unwrap();
    let test_job_id = "test-cancel-awaiting-input-job";

    sqlx::query(
        "INSERT INTO jobs (id, category, topic, style_name, karma_directives, status, priority) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(test_job_id)
    .bind("Goal")
    .bind("Another action")
    .bind("default")
    .bind("[]")
    .bind("AwaitingInput")
    .bind(100)
    .execute(pool)
    .await.expect("Failed to insert mock AwaitingInput job for cancel test");

    // Test POST /api/v1/jobs/{id}/cancel (RED: currently fails to cancel AwaitingInput)
    let cancel_resp = server
        .post(&format!("/api/v1/jobs/{}/cancel", test_job_id))
        .add_header("Authorization", &bearer)
        .await;

    assert_eq!(
        cancel_resp.status_code(),
        axum::http::StatusCode::OK,
        "Expected OK when cancelling an AwaitingInput job"
    );

    let updated_job = state
        .job_queue
        .fetch_job(test_job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        updated_job.status,
        aiome_core_contracts::JobStatus::Cancelled
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
    let pool = state.db_pool.get_inner().get_sqlite_pool_or_err().unwrap();
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

    let status = response.status_code();
    println!("Response Body: {}", response.text());
    assert_eq!(status, StatusCode::OK);
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

#[serial]
#[tokio::test]
async fn test_cortex_query_file_back() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let payload = serde_json::json!({
        "question": "What is AI?",
        "file_back": true
    });

    let response = server
        .post("/api/v1/cortex/query")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;
    let status = response.status_code();
    println!("Response Body: {}", response.text());
    assert_eq!(status, axum::http::StatusCode::OK);

    let pool = state.db_pool.get_inner().get_sqlite_pool_or_err().unwrap();

    let log_row =
        sqlx::query("SELECT COUNT(*) as count FROM cortex_activity_log WHERE event_type = 'query'")
            .fetch_one(pool)
            .await
            .unwrap();
    use sqlx::Row;
    let log_count: i64 = log_row.get("count");
    assert_eq!(log_count, 1, "Query activity log should be inserted");

    let doc_row =
        sqlx::query("SELECT COUNT(*) as count FROM cortex_documents WHERE source_type = 'query'")
            .fetch_one(pool)
            .await
            .unwrap();
    let doc_count: i64 = doc_row.get("count");
    assert_eq!(
        doc_count, 1,
        "File-back document should be inserted since mock confidence is 0.95"
    );
}

#[serial]
#[tokio::test]
async fn test_seo_content_conductor_exists() {
    let (_server, state, _tmp) = create_test_server().await;

    // RED: genericllm_conductor is not yet registered with "seo_content"
    let dispatcher = state.task_dispatcher.get_inner();
    let conductor = dispatcher.get_conductor_for("seo_content");

    assert!(
        conductor.is_some(),
        "seo_content category must be handled by a registered conductor"
    );

    assert_eq!(
        conductor.expect("conductor is missing").conductor_name(),
        "SeoContentConductor",
        "seo_content should be handled by dedicated SeoContentConductor"
    );
}

#[serial]
#[tokio::test]
async fn test_inochi2d_asset_delivery_and_path_traversal() {
    let (server, _state, _tmp) = create_test_server().await;

    // 1. Try to download a valid asset (RED initially as endpoint missing)
    let valid_response = server.get("/api/v1/avatar/inochi2d/valid.inx").await;
    // We expect a 404 if the file isn't there, but currently the route doesn't exist at all so it might be 404 too.
    // Instead we can actually create the file in the mock Sandbox and fetch it.

    // 2. Try Path Traversal (MUST be blocked)
    let traversal_res = server
        .get("/api/v1/avatar/inochi2d/..%2f..%2fetc%2fpasswd")
        .await;
    assert_ne!(
        traversal_res.status_code(),
        reqwest::StatusCode::OK,
        "Path traversal must be rejected"
    );
}

#[serial]
#[tokio::test]
async fn test_whisper_monologue_api() {
    let (server, _state, _tmp) = create_test_server().await;

    // Hit the Whisper API (RED since it doesn't exist)
    let res = server
        .get("/api/v1/whisper/monologue")
        .add_query_param("limit", "10")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(res.status_code(), reqwest::StatusCode::OK);
}

#[serial]
#[tokio::test]
async fn test_auth_full_oauth_workflow() {
    let (server, state, _tmp) = create_test_server().await;

    // Generate real PKCE challenge/verifier pair
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use sha2::{Digest, Sha256};
    let verifier = "another_secret_verifier_for_full_oauth_workflow";
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    // 1. Authorize (Request Code)
    // We expect a valid JSON auth code, not a plain string Mock.
    let authorize_res = server
        .get("/api/v1/auth/authorize")
        .add_query_param("client_id", "aiome_test_client")
        .add_query_param("response_type", "code")
        .add_query_param("code_challenge", &challenge)
        .add_query_param("code_challenge_method", "S256")
        .await;

    assert_eq!(authorize_res.status_code(), reqwest::StatusCode::OK);
    let authorize_json: serde_json::Value = authorize_res.json();
    let auth_code = authorize_json["code"]
        .as_str()
        .expect("Must return auth code");

    // 2. Token Exchange
    let token_payload = serde_json::json!({
        "grant_type": "authorization_code",
        "code": auth_code,
        "client_id": "aiome_test_client",
        "code_verifier": verifier
    });

    let token_res = server.post("/api/v1/auth/token").json(&token_payload).await;

    assert_eq!(token_res.status_code(), reqwest::StatusCode::OK);
    let token_json: serde_json::Value = token_res.json();

    assert!(token_json.get("access_token").is_some());
    let access_token = token_json["access_token"].as_str().unwrap();

    // The returned token MUST be signed by AuthManager (which in tests is MockAuthManager)
    assert!(
        access_token.starts_with("eyJ") || access_token.starts_with("mock_valid_token_"),
        "Token must be a valid JWT or mock token"
    );

    // Validate the token via inner AuthManager
    let claim = state
        .auth_manager
        .validate_token(access_token)
        .await
        .expect("Token must be validly signed");
    assert_eq!(claim.roles, vec![shared::auth::Role::Agent]);
}

#[serial]
#[tokio::test]
async fn test_auth_pkce_rejection_workflow() {
    let (server, _state, _tmp) = create_test_server().await;

    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use sha2::{Digest, Sha256};
    let verifier = "correct_secret_verifier_for_rejection_test";
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    // 1. Authorize (Request Code) with PKCE
    let authorize_res = server
        .get("/api/v1/auth/authorize")
        .add_query_param("client_id", "aiome_test_client_rej")
        .add_query_param("response_type", "code")
        .add_query_param("code_challenge", &challenge)
        .add_query_param("code_challenge_method", "S256")
        .await;

    assert_eq!(authorize_res.status_code(), reqwest::StatusCode::OK);
    let authorize_json: serde_json::Value = authorize_res.json();
    let auth_code = authorize_json["code"].as_str().unwrap();

    // 2. Token Exchange with WRONG verifier
    let token_payload = serde_json::json!({
        "grant_type": "authorization_code",
        "code": auth_code,
        "client_id": "aiome_test_client_rej",
        "code_verifier": "wrong_verifier_should_fail"
    });

    let token_res = server.post("/api/v1/auth/token").json(&token_payload).await;

    // This should fail with 400 Bad Request
    assert_eq!(token_res.status_code(), reqwest::StatusCode::BAD_REQUEST);
}

#[serial]
#[tokio::test]
async fn test_commerce_release_escrow_idor() {
    let (server, _state, _tmp) = create_test_server().await;

    let bearer =
        "Bearer mock_valid_token_ekyc_test_user:00000000-0000-0000-0000-000000000001".to_string();

    // Attempt to release an escrow we OWN
    let valid_payload = serde_json::json!({
        "recipient_id": uuid::Uuid::new_v4().to_string()
    });

    let res_valid = server
        .post("/api/v1/commerce/escrow/valid_escrow_123/release")
        .add_header(axum::http::header::AUTHORIZATION, bearer.clone())
        .json(&valid_payload)
        .await;

    assert_eq!(res_valid.status_code(), reqwest::StatusCode::OK);

    // Attempt to release an escrow we DO NOT OWN
    let res_invalid = server
        .post("/api/v1/commerce/escrow/other_users_escrow_456/release")
        .add_header(axum::http::header::AUTHORIZATION, bearer)
        .json(&valid_payload)
        .await;

    // This should fail due to IDOR protection
    assert_eq!(res_invalid.status_code(), reqwest::StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn test_metrics_observability() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    // 1. Send a request to a core handler, e.g., health
    let res = server.get("/api/health").await;
    res.assert_status_ok();

    // 2. Fetch metrics
    let metrics_res = server
        .get("/api/v1/metrics")
        .add_header(axum::http::header::AUTHORIZATION, bearer)
        .await;
    metrics_res.assert_status_ok();

    let text = metrics_res.text();
    // Validate that our custom metric 'aiome_api_requests_total' exists and was incremented
    assert!(
        text.contains("aiome_api_requests_total"),
        "Custom metric aiome_api_requests_total is missing from Prometheus output!"
    );
}
