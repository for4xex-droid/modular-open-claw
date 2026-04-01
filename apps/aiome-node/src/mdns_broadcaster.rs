/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use mdns_sd::{ServiceDaemon, ServiceInfo};

pub fn start_mdns_broadcaster(port: u16, did: &str) -> Result<ServiceDaemon, String> {
    let mdns = ServiceDaemon::new().map_err(|e| e.to_string())?;

    let service_type = "_aiome._tcp.local.";
    let instance_name = format!(
        "aiome-node-{}",
        uuid::Uuid::new_v4()
            .as_simple()
            .to_string()
            .chars()
            .take(8)
            .collect::<String>()
    );
    let host_name = format!("{}.local.", instance_name);

    let mut properties = std::collections::HashMap::new();
    properties.insert("did".to_string(), did.to_string());

    // mdns-sd 0.11 requires IP address string. We'll use 0.0.0.0 or get local IP.
    // For local dev, 127.0.0.1 is fine, but for actual P2P we should bind to all.
    let my_service = ServiceInfo::new(
        service_type,
        &instance_name,
        &host_name,
        "127.0.0.1",
        port,
        Some(properties),
    )
    .map_err(|e| e.to_string())?;

    mdns.register(my_service).map_err(|e| e.to_string())?;

    tracing::info!(
        "📡 [mDNS] Broadcasting _aiome._tcp service for {} on port {}",
        did,
        port
    );

    Ok(mdns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdns_broadcaster() {
        let daemon = start_mdns_broadcaster(8080, "did:key:test");
        assert!(daemon.is_ok());
    }
}
