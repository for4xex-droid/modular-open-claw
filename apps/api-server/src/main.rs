/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![cfg_attr(test, allow(clippy::unwrap_used))]
#![deny(unsafe_code)]
// dead_code: 個別 #[allow] への段階移行中。pre-push clippy (-D warnings) 通過まで暫定維持。
#![allow(dead_code)]
#![allow(clippy::default_constructed_unit_structs)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::type_complexity)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::manual_inspect)]

use std::net::SocketAddr;
use std::time::Duration;
use tracing::{error, info};

pub mod agent_engine;
mod api;
#[cfg(test)]
mod api_integration_tests;
mod app_state;
#[cfg(test)]
mod audit_auth_tests;
mod auth;
#[cfg(any(debug_assertions, feature = "demo"))]
mod autonomous_demo;
pub mod bootstrap;
#[cfg(test)]
mod commerce_e2e_tests;
mod docker;
mod error;
#[cfg(test)]
mod federation_e2e_tests;
pub mod internal_services;
#[cfg(test)]
mod job_management_tests;
mod logging;
mod mcp;
mod nurture_s2s;
mod plugin_loader;
mod router;
mod routes;
mod self_diagnosis;
mod skill_handler;
mod stream;
pub mod system_instructions;
#[cfg(test)]
mod test_helpers;
pub mod tool_call_processor;
pub mod tool_call_router;
mod workflow_execution_tracker;

pub use app_state::AppState;
pub use router::build_app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    shared::process_hardening::pre_main_hardening();

    if std::env::var("CELL_ID").unwrap_or_default().is_empty() {
        eprintln!("🚨 FATAL: CELL_ID is not set! The Sovereign Verifier architecture requires strict cellular isolation. No identity = No survival.");
        std::process::exit(1);
    }

    let boot_ctx = crate::bootstrap::boot_sequence().await?;
    let state = boot_ctx.state;
    let plugin_registry = boot_ctx.plugin_registry;
    let metrics_handle = boot_ctx.metrics_handle;
    let cancel_token = boot_ctx.cancel_token;
    let cors_layer = boot_ctx.cors_layer;
    let static_path = state.config.get_inner().frontend_static_path.clone();
    // Box::leak is no longer needed since ServeDir accepts String
    let job_queue = state.job_queue.get_inner().clone();

    // === 🏗️ STAGE 7/7: Network ===
    let app = build_app(
        state.clone(),
        cors_layer,
        static_path,
        plugin_registry,
        metrics_handle,
    );

    // G-23: Periodic Federated Metrics Push & Pull (Background Maintenance Loop)
    let jq_for_bg = job_queue.clone();
    let cancel_bg = cancel_token.clone();
    let state_for_bg = state.clone();
    tokio::spawn(async move {
        use infrastructure::job_queue::federation::FederationOps;
        let interval_secs = match std::env::var("FEDERATION_SYNC_INTERVAL") {
            Ok(val) => match val.parse::<u64>() {
                Ok(v) => v,
                Err(_) => {
                    tracing::warn!(
                        "⚠️ [Federation] FEDERATION_SYNC_INTERVAL='{}' is not a valid u64; defaulting to 300s.",
                        val
                    );
                    300
                }
            },
            Err(_) => 300,
        };
        // Thundering herd mitigation: jitter is computed once at startup.
        // Each process gets a stable offset (0–29s), ensuring nodes don't synchronize in lockstep.
        let jitter = rand::random::<u64>() % 30;
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs + jitter));
        loop {
            tokio::select! {
                _ = cancel_bg.cancelled() => {
                    info!("🛑 [Maintenance] Federation metrics sync stopped due to shutdown.");
                    break;
                }
                _ = interval.tick() => {
                    if state_for_bg.is_feature_enabled(shared::feature_flags::FEDERATION_V1_5_FLAG).await {
                        info!("♻️ [Maintenance] Running periodic federated metrics push & sync...");
                        if let Err(e) = jq_for_bg.do_push_federated_metrics().await {
                            error!("🚨 [Maintenance] Failed to push federated metrics: {}", e);
                        }
                        if let Err(e) = jq_for_bg.do_sync_federated_data().await {
                            error!("🚨 [Maintenance] Failed to sync federated metrics: {}", e);
                        }
                    }
                }
            }
        }
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], state.config.get_inner().api_server_port));
    info!("🚀 [api-server] Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| anyhow::anyhow!(
        "🚨 [api-server] Failed to bind to address http://{}. Check if the port is already in use. Error: {}",
        addr, e
    ))?;

    let cancel_serve = cancel_token.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            cancel_serve.cancelled().await;
            tracing::info!("🛑 [api-server] Graceful shutdown triggered.");
        })
        .await
        .map_err(|e| anyhow::anyhow!("🚨 [api-server] Failed to start Axum server: {}", e))?;

    Ok(())
}
