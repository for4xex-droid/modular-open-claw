/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![cfg_attr(test, allow(clippy::unwrap_used))]
#![allow(dead_code)]
use axum::Router;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod mcp_server;
#[cfg(feature = "federation")]
mod mdns_broadcaster;
mod routes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    shared::process_hardening::pre_main_hardening();

    // ログの初期化
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "aiome_node=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Aiome Node...");

    // Check if running in MCP mode
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "mcp" {
        tracing::info!("Starting in MCP Server Mode (via stdio)");

        // MVP: Node Stub -> P3 UniversalGigEngine Integration
        use ::uuid::Uuid;
        use aiome_commerce::gig::UniversalGigEngine;
        use aiome_core_contracts::commerce::{
            CommerceEngine, EscrowRecord, FiatPaymentRails, PointsBalance, SubscriptionStatus,
            TransactionRecord, Web3PaymentRails,
        };
        use aiome_core_contracts::error::AiomeError;
        use aiome_core_contracts::llm::{LlmProvider, LlmResponse};
        use aiome_core_contracts::traits::ConstitutionalValidator;
        use async_trait::async_trait;
        use infrastructure::gig_gateway::SecureGigGateway;
        use infrastructure::rate_limiter::AgentRateLimiter;
        use std::path::PathBuf;
        use std::sync::Arc;

        // Stub LLM Provider if not available locally, or we could use the real one.
        // For MCP server stub, we can just use a DisabledLLMProvider
        #[derive(Debug)]
        struct DisabledLlmProvider;
        #[async_trait]
        impl LlmProvider for DisabledLlmProvider {
            async fn complete(
                &self,
                _prompt: &str,
                _sys: Option<&str>,
            ) -> Result<LlmResponse, AiomeError> {
                Err(AiomeError::Infrastructure {
                    reason: "LLM disabled in node".into(),
                })
            }
            async fn stream_complete(
                &self,
                _prompt: &str,
                _sys: Option<&str>,
            ) -> Result<
                std::pin::Pin<
                    Box<dyn tokio_stream::Stream<Item = Result<String, AiomeError>> + Send>,
                >,
                AiomeError,
            > {
                Err(AiomeError::Infrastructure {
                    reason: "LLM disabled in node".into(),
                })
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
            fn name(&self) -> &str {
                "disabled"
            }
        }

        const COMMERCE_STUB_ERR: &str = "Edge nodes cannot process commerce";

        struct StubCommerceEngine;
        impl StubCommerceEngine {
            fn err<T>() -> Result<T, AiomeError> {
                Err(AiomeError::Infrastructure {
                    reason: COMMERCE_STUB_ERR.into(),
                })
            }
        }
        #[async_trait]
        impl FiatPaymentRails for StubCommerceEngine {
            fn verify_signature(&self, _p: &str, _s: &str) -> Result<(), AiomeError> {
                Self::err()
            }

            async fn create_checkout_session(
                &self,
                _agent_id: uuid::Uuid,
                _price_id: &str,
                _success_url: &str,
                _cancel_url: &str,
            ) -> Result<String, aiome_core_contracts::error::AiomeError> {
                Self::err()
            }

            async fn create_portal_session(
                &self,
                _agent_id: Uuid,
                _return_url: &str,
            ) -> Result<String, AiomeError> {
                Self::err()
            }

            async fn create_subscription(&self, _a: Uuid, _p: &str) -> Result<String, AiomeError> {
                Self::err()
            }

            async fn cancel_subscription(&self, _a: Uuid, _s: &str) -> Result<(), AiomeError> {
                Self::err()
            }
        }

        #[async_trait]
        impl Web3PaymentRails for StubCommerceEngine {
            async fn stake(&self, _id: Uuid, _amt: u64) -> Result<(), AiomeError> {
                Self::err()
            }

            async fn slash(&self, _id: Uuid, _amt: u64, _reason: &str) -> Result<(), AiomeError> {
                Self::err()
            }
        }

        #[async_trait]
        impl CommerceEngine for StubCommerceEngine {
            async fn get_balance(&self, _id: Uuid) -> Result<u64, AiomeError> {
                Self::err()
            }

            async fn validate_activity(
                &self,
                _id: Uuid,
                _typ: &str,
                _amt: u64,
            ) -> Result<(), AiomeError> {
                Self::err()
            }

            async fn execute_autonomous_purchase(
                &self,
                _a: Uuid,
                _i: Uuid,
                _m: serde_json::Value,
            ) -> Result<String, AiomeError> {
                Self::err()
            }

            async fn get_daily_spend(&self, _id: Uuid) -> Result<u64, AiomeError> {
                Self::err()
            }

            async fn get_daily_limit(&self, _id: Uuid) -> Result<u64, AiomeError> {
                Self::err()
            }

            async fn escrow_create(&self, _id: Uuid, _amt: u64) -> Result<String, AiomeError> {
                Self::err()
            }

            async fn list_escrows(&self, _id: Uuid) -> Result<Vec<EscrowRecord>, AiomeError> {
                Self::err()
            }

            async fn escrow_release(&self, _e: &str, _r: Uuid) -> Result<(), AiomeError> {
                Self::err()
            }

            async fn escrow_refund(&self, _e: &str) -> Result<(), AiomeError> {
                Self::err()
            }

            async fn register_license(
                &self,
                _a: Uuid,
                _asset: Uuid,
                _t: &str,
                _l: &str,
            ) -> Result<String, AiomeError> {
                Self::err()
            }

            async fn get_subscription_status(
                &self,
                _a: Uuid,
            ) -> Result<SubscriptionStatus, AiomeError> {
                Self::err()
            }

            async fn transfer(&self, _f: Uuid, _t: Uuid, _amt: u64) -> Result<String, AiomeError> {
                Self::err()
            }

            async fn deduct_generation_cost(
                &self,
                _a: Uuid,
                _asset: Option<Uuid>,
                _amt: u64,
                _type: &str,
            ) -> Result<(), AiomeError> {
                Self::err()
            }

            async fn instant_refund(&self, _t: &str, _a: Uuid) -> Result<(), AiomeError> {
                Self::err()
            }

            async fn withdraw_points(&self, _a: Uuid, _p: u64) -> Result<(), AiomeError> {
                Self::err()
            }

            async fn get_points(&self, _a: Uuid) -> Result<PointsBalance, AiomeError> {
                Self::err()
            }

            async fn get_transaction_history(
                &self,
                _a: Uuid,
                _limit: u32,
            ) -> Result<Vec<TransactionRecord>, AiomeError> {
                Self::err()
            }
        }

        struct BasicValidator;
        #[async_trait]
        impl ConstitutionalValidator for BasicValidator {
            async fn verify_constitutional(
                &self,
                _output: &str,
                _soul_md: &str,
            ) -> Result<(), AiomeError> {
                // Production Edge Node allows all pass by default until OxiLean is integrated locally.
                Ok(())
            }
        }

        let app_data =
            std::env::var("APP_DATA_DIR").unwrap_or_else(|_| ".gemini/antigravity".to_string());
        let db_url = format!("sqlite://{}/aiome.db", app_data);
        tracing::info!("📦 [aiome-node] Connecting to database: {}", db_url);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect(&db_url)
            .await;

        let db_pool = if let Ok(p) = pool {
            infrastructure::db::DatabasePool::Sqlite(p)
        } else {
            tracing::error!("Failed to connect to database for GigEngine");
            std::process::exit(1);
        };

        let commerce_stub = Arc::new(StubCommerceEngine);
        let llm_stub = Arc::new(DisabledLlmProvider);
        let workspace_dir = std::env::var("WORKSPACE_DIR").unwrap_or_else(|_| ".".to_string());
        let engine = Arc::new(UniversalGigEngine::new(
            db_pool.clone(),
            commerce_stub,
            llm_stub,
            PathBuf::from(&workspace_dir),
        ));

        let validator = Arc::new(BasicValidator);
        let limiter =
            AgentRateLimiter::new(60).map_err(|e| format!("Rate limiter init failed: {}", e))?;
        let gateway = SecureGigGateway::new(engine, validator, limiter);

        let path = std::path::Path::new(&workspace_dir);
        let wasm_manager = Arc::new(
            infrastructure::skills::WasmSkillManager::new(path, path).unwrap_or_else(|e| {
                tracing::error!("Failed to init WasmSkillManager: {}", e);
                std::process::exit(1);
            }),
        );

        let skill_arena =
            infrastructure::skills::skill_arena::SkillArena::new().with_db_pool(db_pool);

        let _mcp_server = mcp_server::McpServer::new(gateway)
            .with_wasm_manager(wasm_manager)
            .with_skill_arena(Arc::new(skill_arena));

        return Ok(());
    }

    let app = setup_router();

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Listening on {}", addr);

    // Start mDNS Broadcaster
    #[cfg(feature = "federation")]
    let _mdns_daemon = {
        let did = "did:key:z6MkhaXgBZDvotDkL5257faiztiuC2ZXpu258wtVGnQkERfN"; // Placeholder for Phase 52 # gitleaks:allow
        match mdns_broadcaster::start_mdns_broadcaster(8080, did) {
            Ok(d) => Some(d),
            Err(e) => {
                tracing::error!("Failed to start mdns broadcaster: {}", e);
                None
            }
        }
    };

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

pub fn setup_router() -> Router {
    let _config = shared::config::AiomeConfig::load().unwrap_or_default();
    let router = Router::new().nest("/.well-known", routes::well_known_routes());

    #[cfg(feature = "federation")]
    let router = router.nest(
        "/api/v1/federation",
        routes::federation::router().with_state(std::sync::Arc::new(config)),
    );

    router
}
