use aiome_core::contracts::HubMessage;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

pub struct HubState {
    pub pool: shared::db::DatabasePool,
    pub secret: secrecy::SecretString,
    pub auth_manager: Arc<dyn shared::auth::AuthManager>,
    pub tx: broadcast::Sender<HubMessage>,
    pub active_connections: std::sync::atomic::AtomicUsize,
    pub agent_registry: crate::mdns_listener::AgentRegistry,
}

impl HubState {
    pub fn new(
        pool: shared::db::DatabasePool,
        secret: secrecy::SecretString,
        auth_manager: Arc<dyn shared::auth::AuthManager>,
        tx: broadcast::Sender<HubMessage>,
        agent_registry: crate::mdns_listener::AgentRegistry,
    ) -> Self {
        Self {
            pool,
            secret,
            auth_manager,
            tx,
            active_connections: std::sync::atomic::AtomicUsize::new(0),
            agent_registry,
        }
    }
}

pub async fn init_hub_db(pool: &shared::db::DatabasePool) -> anyhow::Result<()> {
    match pool {
        shared::db::DatabasePool::Sqlite(ref p) => {
            sqlx::migrate!("migrations/sqlite").run(p).await?;
        }
        shared::db::DatabasePool::Postgres(ref p) => {
            sqlx::migrate!("migrations/postgres").run(p).await?;
        }
    }
    info!("✅ Hub Database initialized (Approved & Quarantine layers + BFT/Reputation & Biome).");
    Ok(())
}
