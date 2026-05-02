/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![cfg_attr(test, allow(clippy::unwrap_used))]

#![deny(unsafe_code)]
#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
#![allow(clippy::default_constructed_unit_structs)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::type_complexity)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::manual_inspect)]

use crate::app_state::Component;
use aiome_core::llm_provider::{EmbeddingProvider, LlmProvider};
use aiome_core::traits::TranscriptionEngine;
use aiome_core_contracts::commerce::CommerceEngine;
use aiome_core_contracts::commerce::GiftEngine;
use aiome_core_contracts::commerce::GiftPolicyContext;
use aiome_core_contracts::ekyc::EkycEngine;
use aiome_core_contracts::ekyc::EkycSessionStore;
use infrastructure::audit_logger::AsyncAuditLogger;
use infrastructure::auth::AuthManager;
use infrastructure::belief_consistency_gate::BeliefConsistencyGate;
use infrastructure::circuit_breaker::CircuitBreaker;
use infrastructure::compliance::quarantine::QuarantineStore;
use infrastructure::memory_crystallizer::MemoryCrystallizer;
use infrastructure::slo_engine::SloEngine;
use infrastructure::whisper_transcription::WhisperTranscriptionAdapter;
use shared::config::AiomeConfig;

use async_trait::async_trait;
use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, STRICT_TRANSPORT_SECURITY, X_CONTENT_TYPE_OPTIONS,
    X_FRAME_OPTIONS,
};
use axum::http::HeaderValue;
use axum::{http::StatusCode, response::IntoResponse, response::Json, routing::get, Router};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use tracing::{debug, error, info, warn};
use utoipa::OpenApi;

pub mod agent_engine;
mod api;
#[cfg(test)]
mod api_integration_tests;
mod app_state;
#[cfg(test)]
mod audit_auth_tests;
mod auth;
mod autonomous_demo;
pub mod bootstrap;
pub mod bootstrap_builder;
mod docker;
mod error;
#[cfg(test)]
mod federation_e2e_tests;
pub mod internal_services;
#[cfg(test)]
mod job_management_tests;
mod logging;
mod mcp;
mod plugin_loader;
mod router;
mod routes;
mod self_diagnosis;
mod skill_handler;
mod stream;
pub mod system_instructions;
pub mod tool_call_processor;
pub mod tool_call_router;

pub use app_state::AppState;
pub use router::build_app;

use aiome_core::expression::tts_worker::TtsWorker;
use aiome_core::traits::JobQueue;
use shared::health::HealthMonitor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("CELL_ID").unwrap_or_default().is_empty() {
        eprintln!("🚨 FATAL: CELL_ID is not set! The Sovereign Verifier architecture requires strict cellular isolation. No identity = No survival.");
        std::process::exit(1);
    }

    let mut boot_ctx = crate::bootstrap::boot_sequence().await?;
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

    // G-23: Periodic Federated Metrics Push (Background Maintenance Loop)
    let jq_for_bg = job_queue.clone();
    let cancel_bg = cancel_token.clone();
    tokio::spawn(async move {
        use infrastructure::job_queue::federation::FederationOps;
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Every hour
        loop {
            tokio::select! {
                _ = cancel_bg.cancelled() => {
                    info!("🛑 [Maintenance] Federation metrics push stopped due to shutdown.");
                    break;
                }
                _ = interval.tick() => {
                    info!("♻️ [Maintenance] Running periodic federated metrics push...");
                    if let Err(e) = jq_for_bg.do_push_federated_metrics().await {
                        error!("🚨 [Maintenance] Failed to push federated metrics: {}", e);
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
