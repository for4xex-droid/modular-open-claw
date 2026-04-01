/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type AgentRegistry = Arc<RwLock<HashMap<String, AgentInfo>>>;

#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub did: String,
    pub ip: String,
    pub port: u16,
    pub last_seen: std::time::Instant,
}

pub async fn update_registry(registry: &AgentRegistry, did: String, ip: String, port: u16) {
    const MAX_REGISTRY_SIZE: usize = 1000;
    const AGENT_TTL: std::time::Duration = std::time::Duration::from_secs(300);

    let mut reg = registry.write().await;
    let now = std::time::Instant::now();

    // 1. Proactive TTL Cleanup
    reg.retain(|_, info| now.duration_since(info.last_seen) < AGENT_TTL);

    // 2. Bound Registry Size
    if reg.len() >= MAX_REGISTRY_SIZE && !reg.contains_key(&did) {
        let oldest_did = reg
            .iter()
            .min_by_key(|(_, info)| info.last_seen)
            .map(|(id, _)| id.clone());
        if let Some(id) = oldest_did {
            reg.remove(&id);
        }
    }

    reg.insert(
        did.clone(),
        AgentInfo {
            did,
            ip,
            port,
            last_seen: now,
        },
    );
}

pub fn start_mdns_listener(registry: AgentRegistry) -> Result<ServiceDaemon, String> {
    let mdns = ServiceDaemon::new().map_err(|e| e.to_string())?;
    let service_type = "_aiome._tcp.local.";

    let receiver = mdns.browse(service_type).map_err(|e| e.to_string())?;

    let registry_clone = registry.clone();
    tokio::spawn(async move {
        while let Ok(event) = receiver.recv_async().await {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    let did = info
                        .get_property_val_str("did")
                        .unwrap_or("unknown_did")
                        .to_string();
                    let ip = info
                        .get_addresses()
                        .iter()
                        .next()
                        .map(|ip| ip.to_string())
                        .unwrap_or_default();
                    let port = info.get_port();

                    tracing::info!("🔍 [mDNS] Discovered Agent (DID: {}): {}:{}", did, ip, port);
                    update_registry(&registry_clone, did, ip, port).await;
                }
                ServiceEvent::ServiceRemoved(_, full_name) => {
                    tracing::info!("🔌 [mDNS] Service removed: {}", full_name);
                }
                _ => {}
            }
        }
    });

    Ok(mdns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mdns_registry_oom_protection() {
        let registry = Arc::new(RwLock::new(HashMap::new()));

        // Simulate filling up the registry using the actual logic
        for i in 0..1050 {
            let did = format!("did:aiome:test-{}", i);
            update_registry(&registry, did, "127.0.0.1".into(), 8080).await;
        }

        let reg = registry.read().await;
        assert!(
            reg.len() <= 1000,
            "Registry should be capped at 1000, but was {}",
            reg.len()
        );
    }
}
