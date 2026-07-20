/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

pub use crate::app_state::Component;
pub use crate::*;
pub use aiome_core_contracts::traits::AgentEvolver;
pub use aiome_core_contracts::traits::JobQueue;
pub use aiome_core_contracts::traits::TaskRegistry;
pub use axum_test::TestServer;
pub use infrastructure::auth::AuthManager;
pub use shared::config::AiomeConfig;
pub use shared::health::HealthMonitor;
pub use soul::SoulPipeline;
pub use std::sync::Arc;
pub use tokio::sync::Mutex;
pub use tower_http::cors::{AllowOrigin, CorsLayer};

// 💎 Shared Global Metrics Recorder (PR-10 Mitigation)
// Prometheus handles only one global recorder per process.
pub static GLOBAL_METRICS_HANDLE: once_cell::sync::Lazy<
    metrics_exporter_prometheus::PrometheusHandle,
> = once_cell::sync::Lazy::new(|| {
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

        let is_constitutional = sys
            .map(|s| s.contains("Constitutional") || s.contains("Referee"))
            .unwrap_or(false);

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
        } else if is_constitutional {
            "PASS".to_string()
        } else {
            "Dummy Output".to_string()
        };
        Ok(aiome_core_contracts::LlmResponse {
            content,
            stop_reason: aiome_core_contracts::StopReason::EndTurn,
            ..Default::default()
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
impl aiome_core_contracts::commerce::FiatPaymentRails for MockCommerceEngine {
    fn verify_signature(
        &self,
        _payload: &str,
        sig_header: &str,
    ) -> Result<(), aiome_core::error::AiomeError> {
        // Negative テスト用マーカーのみ拒否。
        // "bad" の部分一致は HMAC hex（0-9a-f）に偶然含まれてフレークするため禁止。
        // "invalid" は hex に現れないので contains で安全（polar: v1,invalid_sig）。
        let marker = sig_header.trim();
        if marker.contains("invalid") || marker == "bad" || marker.starts_with("bad_") {
            return Err(aiome_core::error::AiomeError::Unauthorized {
                reason: "Invalid signature".into(),
            });
        }
        Ok(())
    }

    async fn create_checkout_session(
        &self,
        _agent_id: uuid::Uuid,
        price_id: &str,
        _success_url: &str,
        _cancel_url: &str,
    ) -> Result<String, aiome_core::error::AiomeError> {
        if price_id == "price_test_overwrite_99999" {
            return Ok("cs_test_overwritten".into());
        }
        Ok("cs_test_mock".into())
    }

    async fn create_portal_session(
        &self,
        _agent_id: uuid::Uuid,
        _return_url: &str,
    ) -> Result<String, aiome_core::error::AiomeError> {
        Ok("https://example.com/portal-session-mock".to_string())
    }

    async fn create_subscription(
        &self,
        _agent_id: uuid::Uuid,
        plan_id: &str,
    ) -> Result<String, aiome_core::error::AiomeError> {
        if plan_id == "price_test_overwrite_99999" {
            return Ok("sub_mock_overwritten".into());
        }
        Ok("sub_mock_123".into())
    }

    async fn cancel_subscription(
        &self,
        _agent_id: uuid::Uuid,
        _subscription_id: &str,
    ) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl aiome_core_contracts::commerce::Web3PaymentRails for MockCommerceEngine {
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
}

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
        agent_id: uuid::Uuid,
        _activity_type: &str,
        _amount: u64,
    ) -> Result<(), aiome_core::error::AiomeError> {
        if agent_id.to_string() == "00000000-0000-0000-0000-fa1100000000" {
            return Err(aiome_core::error::AiomeError::Infrastructure {
                reason: "Insufficient funds".into(),
            });
        }
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

    async fn register_license(
        &self,
        _agent_id: uuid::Uuid,
        _asset_id: uuid::Uuid,
        _license_type: &str,
        _extra: &str,
    ) -> Result<String, aiome_core::error::AiomeError> {
        Ok("lic".into())
    }

    async fn get_subscription_status(
        &self,
        agent_id: uuid::Uuid,
    ) -> Result<aiome_core_contracts::commerce::SubscriptionStatus, aiome_core::error::AiomeError>
    {
        let id_str = agent_id.to_string();
        if id_str.ends_with("0002") {
            return Ok(aiome_core_contracts::commerce::SubscriptionStatus::None);
        } else if id_str.ends_with("0003") {
            return Ok(aiome_core_contracts::commerce::SubscriptionStatus::Trialing);
        } else if id_str.ends_with("0004") {
            return Ok(aiome_core_contracts::commerce::SubscriptionStatus::PastDue);
        } else if id_str.ends_with("0005") {
            return Ok(aiome_core_contracts::commerce::SubscriptionStatus::Cancelled);
        } else if id_str.ends_with("0006") {
            return Ok(aiome_core_contracts::commerce::SubscriptionStatus::Unpaid);
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
            ..Default::default()
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
    create_test_server_with_limit(5).await
}

pub async fn create_test_server_with_limit(
    limit: u32,
) -> (TestServer, AppState, tempfile::TempDir) {
    let tmp_dir = tempfile::TempDir::new().expect("tmp dir creation failed");
    let db_path = tmp_dir.path().join("test.db");

    // Set WORKSPACE_DIR to tmp_dir for security sandbox consistency (S-4 fix)
    std::env::set_var("WORKSPACE_DIR", tmp_dir.path().to_str().unwrap());
    // Set AIOME_DEV_MODE to 1 for integration tests to allow localhost / test redirect URLs
    std::env::set_var("AIOME_DEV_MODE", "1");

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
    let intent_firewall = Arc::new(infrastructure::intent::IntentFirewall::new().unwrap());
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

    let rate_limiter = infrastructure::rate_limiter::AgentRateLimiter::new(limit)
        .expect("Rate limit value is valid");

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

    let vault_backend = Arc::new(
        infrastructure::security::sqlite_vault_backend::UniversalVaultBackend::new(pool.clone()),
    ) as Arc<dyn aiome_core_contracts::vault_backend::VaultBackend>;

    let state = AppState {
        tokens_css: String::new(),
        vault_backend: Component::new(vault_backend),
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
        fast_provider: Component::new(provider.clone()), // テストでは DummyLlm を共用
        autonomous_running: Component::new(autonomous_running),
        autonomous_config: Component::new(autonomous_config),
        http_client: Component::new(aiome_core::http::get_http_client().clone()),
        docker_failures: Component::new(Arc::new(tokio::sync::RwLock::new(
            std::collections::HashMap::new(),
        ))),
        security_policy: shared::security::SecurityPolicy::default(),
        commerce_engine: Component::new(commerce_engine.clone()),
        x402_negotiator: Component::new(None),
        circuit_breaker: Component::new(Arc::new(
            infrastructure::circuit_breaker::CircuitBreaker::new(
                "integration-test",
                infrastructure::circuit_breaker::CircuitBreakerConfig {
                    failure_threshold: 5,
                    reset_timeout: std::time::Duration::from_secs(60),
                },
            ),
        )),
        alert_manager: Component::new(Arc::new(infrastructure::alerts::AlertManager::new())),
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
            config.resolver = shared::app_data::AppDataResolver::new().unwrap();

            config.gemini_api_key = None;
            config.openai_api_key = None;
            config.anthropic_api_key = None;
            config.api_server_port = 0;
            config.key_proxy_url = std::env::var("TEST_KEY_PROXY_URL").unwrap_or_default();
            config.vault_secret = std::env::var("TEST_VAULT_SECRET")
                .ok()
                .map(secrecy::SecretString::from);
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
        ban_store: Component::new(Arc::new(
            infrastructure::compliance::ban_store::MockBanStore::new(),
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
        nurture_url: std::env::var("NURTURE_API_URL").ok(),
        nurture_internal_secret: std::env::var("NURTURE_INTERNAL_SECRET").ok(),
        nurture_s2s: None,
        quality_gate_store: Component::default(),
        skill_arena: Component::new(Arc::new(
            infrastructure::skills::skill_arena::SkillArena::new(),
        )),
        rlm_client: Component::default(),
        prompt_registry: Component::new(Arc::new(
            infrastructure::prompt_registry::MockPromptRegistry,
        )
            as Arc<dyn infrastructure::prompt_registry::PromptRegistry>),
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
        spec_provider: Component::new(Arc::new(infrastructure::spec_provider::FsSpecProvider::new(
            tmp_dir.path().join("workflows"),
        ))
            as Arc<dyn infrastructure::spec_provider::SpecProvider>),
        mcp_oauth_secrets: {
            let mut m = std::collections::HashMap::new();
            if let (Ok(id), Ok(secret)) = (
                std::env::var("GITHUB_CLIENT_ID"),
                std::env::var("GITHUB_CLIENT_SECRET"),
            ) {
                m.insert(
                    "github".to_string(),
                    crate::mcp::discovery::OAuthCredentials {
                        client_id: id,
                        client_secret: secrecy::SecretString::from(secret),
                    },
                );
            }
            m
        },
        buzz_generator: Component::new(Arc::new(
            infrastructure::buzz::generator::BuzzContentGenerator::new(provider.clone()),
        )),
        buzz_scheduler: Component::new(Arc::new(
            infrastructure::buzz::scheduler::BuzzScheduler::new(90, 4),
        )),
        stripe_price_subscription_monthly: std::env::var("STRIPE_PRICE_SUBSCRIPTION_MONTHLY").ok(),
        stripe_api_key: None,
        biome_engine: Component::new(std::sync::Arc::new(tokio::sync::RwLock::new(
            biome_engine::BiomeEngine::new(42),
        ))),
        workflow_execution_tracker: Component::new(std::sync::Arc::new(
            crate::workflow_execution_tracker::WorkflowExecutionTracker::new(),
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

pub struct E2eMockForecastProvider;
#[async_trait::async_trait]
impl aiome_core_contracts::forecast::ForecastProvider for E2eMockForecastProvider {
    async fn forecast(
        &self,
        series: Vec<Vec<f64>>,
        horizon: usize,
        _config: aiome_core_contracts::forecast::ForecastConfig,
    ) -> Result<aiome_core_contracts::forecast::ForecastResult, aiome_core::error::AiomeError> {
        let point_forecast: Vec<Vec<f64>> = series
            .iter()
            .map(|s| {
                let last_val = s.last().copied().unwrap_or(0.0);
                (0..horizon).map(|i| last_val + (i as f64 * 0.01)).collect()
            })
            .collect();
        Ok(aiome_core_contracts::forecast::ForecastResult {
            point_forecast,
            quantile_forecast: None,
            model_version: "mock".to_string(),
        })
    }
    async fn detect_anomaly(
        &self,
        _historical: Vec<f64>,
        _recent: Vec<f64>,
        _threshold_sigma: f64,
    ) -> Result<aiome_core_contracts::forecast::AnomalyResult, aiome_core::error::AiomeError> {
        Ok(aiome_core_contracts::forecast::AnomalyResult {
            is_anomaly: false,
            deviation_sigma: 0.0,
            predicted_values: vec![],
        })
    }
    fn name(&self) -> &str {
        "E2eMockForecast"
    }
}

#[derive(Default, Debug)]
pub struct E2eMockLoraEngine {
    pub train_called: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
#[async_trait::async_trait]
impl aiome_core_contracts::traits::LoraEngine for E2eMockLoraEngine {
    async fn complete_with_lora(
        &self,
        _prompt: &str,
        _lora_id: &str,
    ) -> Result<aiome_core_contracts::llm::LlmResponse, aiome_core::error::AiomeError> {
        Ok(aiome_core_contracts::llm::LlmResponse {
            content: "LlmResponse from E2eMockLoraEngine".to_string(),
            stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
            ..Default::default()
        })
    }
    async fn train(
        &self,
        _base_model: &str,
        _dataset_id: &str,
        _params: serde_json::Value,
    ) -> Result<String, aiome_core::error::AiomeError> {
        self.train_called
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok("job_test".to_string())
    }
    async fn health_check(&self) -> Result<bool, aiome_core::error::AiomeError> {
        Ok(true)
    }
}
