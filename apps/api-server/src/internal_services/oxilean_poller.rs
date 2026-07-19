/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::AppState;
use aiome_core_contracts::a2a::internal::proof_verifier_client::ProofVerifierClient;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tracing::{debug, error, info};

/// 定期的に OxiLean (shadow-worker) の状態をチェックし、OXP を更新するタスク。
pub async fn run(state: AppState) -> Result<(), anyhow::Error> {
    info!("🛡️ Starting OxiLean Background Poller...");

    let host = state.config.get_inner().shadow_clone_grpc_host.clone();
    let port = state.config.get_inner().shadow_clone_grpc_port.clone();
    let addr = format!("http://{}:{}", host, port);
    let auth_token = state
        .config
        .get_inner()
        .a2a_grpc_auth_token()
        .map(|s| {
            use secrecy::ExposeSecret;
            s.expose_secret().to_string()
        })
        .unwrap_or_default();

    let endpoint = tonic::transport::Endpoint::from_shared(addr)
        .map_err(|e| anyhow::anyhow!("Invalid gRPC endpoint: {}", e))?;

    // Use lazy connection pooling to avoid reconnecting every 60 seconds
    let channel = endpoint.connect_lazy();
    let mut client = ProofVerifierClient::new(channel);

    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;

        let mut request =
            tonic::Request::new(aiome_core_contracts::a2a::internal::GetOxiLeanStatusRequest {});
        if auth_token.is_empty() {
            tracing::warn!(
                "A2A gRPC auth token empty (A2A_AUTH_TOKEN / A2A_NODE_TOKEN). Skipping authorization header."
            );
        } else if let Ok(metadata_val) = tonic::metadata::MetadataValue::try_from(&auth_token) {
            request.metadata_mut().insert("authorization", metadata_val);
        } else {
            tracing::warn!(
                "Failed to parse A2A gRPC auth token as metadata value. Auth will be missing."
            );
        }

        match client.get_oxi_lean_status(request).await {
            Ok(response) => {
                let next_oxp = response.into_inner().current_oxp;
                state.oxilean_power.store(next_oxp, Ordering::Relaxed);
                debug!("🛡️ OxiLean Power updated to: {} OXP", next_oxp);
            }
            Err(e) => {
                error!("❌ Failed to fetch OxiLean status: {}", e);
            }
        }
    }
}
