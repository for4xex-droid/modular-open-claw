use axum::Router;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod mcp_server;
mod mdns_broadcaster;
mod routes;

#[tokio::main]
async fn main() {
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

        // MVP: Using a stub implementation for the GigEngine/Validator to satisfy the Gateway
        // In a real production deployment, this would inject the UniversalGigEngine and ConstitutionalValidator.
        use ::uuid::Uuid;
        use aiome_contracts::error::AiomeError;
        use aiome_contracts::gig::{
            GigBid, GigDeliverable, GigEngine, GigIntent, VerificationResult,
        };
        use aiome_contracts::traits::ConstitutionalValidator;
        use async_trait::async_trait;
        use infrastructure::gig_gateway::SecureGigGateway;
        use infrastructure::rate_limiter::AgentRateLimiter;
        use std::sync::Arc;

        struct DummyGigEngine;
        #[async_trait]
        impl GigEngine for DummyGigEngine {
            async fn publish_intent(&self, _intent: GigIntent) -> Result<Uuid, AiomeError> {
                Ok(Uuid::new_v4())
            }
            async fn submit_bid(&self, _bid: GigBid) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn accept_bid(&self, _intent_id: Uuid, _bid_id: Uuid) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn deliver(&self, _deliverable: GigDeliverable) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn verify_and_settle(
                &self,
                _order_id: Uuid,
            ) -> Result<VerificationResult, AiomeError> {
                unimplemented!()
            }
        }

        struct DummyValidator;
        #[async_trait]
        impl ConstitutionalValidator for DummyValidator {
            async fn verify_constitutional(
                &self,
                _output: &str,
                _soul_md: &str,
            ) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        let engine = Arc::new(DummyGigEngine);
        let validator = Arc::new(DummyValidator);
        let limiter = AgentRateLimiter::new(60); // 60 requests per minute
        let gateway = SecureGigGateway::new(engine, validator, limiter);

        let mcp_server = mcp_server::McpServer::new(gateway);
        mcp_server.run().await;
        return;
    }

    let app = setup_router();

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Listening on {}", addr);

    // Start mDNS Broadcaster
    let did = "did:key:z6MkhaXgBZDvotDkL5257faiztiuC2ZXpu258wtVGnQkERfN"; // Placeholder for Phase 52
    let _mdns_daemon = mdns_broadcaster::start_mdns_broadcaster(8080, did).unwrap();

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

pub fn setup_router() -> Router {
    Router::new().nest("/.well-known", routes::well_known_routes())
}
