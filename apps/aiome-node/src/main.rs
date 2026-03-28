use axum::Router;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
