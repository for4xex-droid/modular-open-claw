#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
#![allow(clippy::default_constructed_unit_structs)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::type_complexity)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::manual_inspect)]
/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::app_state::Component;
use crate::internal_services;
use crate::logging;
use crate::mcp;
use crate::plugin_loader;
use crate::AppState;
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

use super::*;
use aiome_core::expression::tts_worker::TtsWorker;
use aiome_core::traits::JobQueue;
use shared::health::HealthMonitor;

pub async fn spawn_background_workers(
    state: &crate::app_state::AppState,
    belief_gate: &std::sync::Arc<infrastructure::belief_consistency_gate::BeliefConsistencyGate>,
    cancel_token: tokio_util::sync::CancellationToken,
) -> anyhow::Result<()> {
    let resolver = &state.config.get_inner().resolver;
    let job_queue = state.job_queue.get_inner().clone();

    // [Step 1.8.5] Spawn Unified Internal Services (Watchtower & Heartbeat)
    internal_services::spawn_all(state.clone()).await;

    // [Step 1.8.6] Spawn BlobStorageAdapter (Event-Driven Physical Asset Purging)
    let blob_adapter = std::sync::Arc::new(infrastructure::blob_storage::BlobStorageAdapter::new(
        resolver.root().to_path_buf(),
    ));
    let blob_rx = job_queue.event_bus.subscribe();
    blob_adapter.start_event_listener(blob_rx).await;

    // [Step 1.9] Initialize and Spawn TtsWorker Background Loop
    let tts_worker_jq = state.job_queue.get_inner().clone();
    let tts_worker_provider = state.tts_provider.get_inner().clone();
    let tts_worker_speaker = state
        .config
        .get_inner()
        .xtts_speaker
        .clone()
        .unwrap_or_else(|| "p225".to_string());
    let tts_worker_artifacts = resolver.resolve("artifacts");

    let tts_cancel = cancel_token.clone();
    tokio::spawn(async move {
        tracing::info!("🎙️ [TtsWorker] Starting background synthesis loop...");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            tokio::select! {
                _ = tts_cancel.cancelled() => {
                    tracing::info!("🛑 [TtsWorker] Shutting down cleanly...");
                    break;
                }
                _ = interval.tick() => {
                    if let Err(e) = TtsWorker::process_pending_tts(
                        &*tts_worker_jq,
                        &*tts_worker_provider,
                        &tts_worker_speaker,
                        &tts_worker_artifacts,
                    )
                    .await
                    {
                        tracing::error!("🚨 [TtsWorker] Loop error: {}", e);
                    }
                }
            }
        }
    });

    // [Step 1.10] Initialize and Spawn CortexCompiler Background Loop
    let compiler_provider = state.provider.get_inner().clone();
    let compiler_pool = state.db_pool.get_inner().clone();
    let compiler_semaphore = state.compute_semaphore.get_inner().clone();
    let compiler_gate = Some(belief_gate.clone());
    let compiler_projector = state.cortex_projector.get_inner().clone();
    let cortex_cancel = cancel_token.clone();
    tokio::spawn(async move {
        tracing::info!("📚 [Cortex] Starting compilation loop...");
        let compiler = infrastructure::cortex_compiler::CortexCompiler::new(
            compiler_provider,
            (*compiler_pool).clone(),
            compiler_gate,
            compiler_semaphore,
        )
        .with_file_projector(compiler_projector);
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1800));
        loop {
            tokio::select! {
                _ = cortex_cancel.cancelled() => {
                    tracing::info!("🛑 [Cortex] Shutting down cleanly...");
                    break;
                }
                _ = interval.tick() => {
                    match compiler.run_compilation_cycle().await {
                        Ok(ref report) if report.new_articles > 0 || report.updated_articles > 0 => {
                            tracing::info!(
                                "📚 [Cortex] Compilation: {} new, {} updated",
                                report.new_articles,
                                report.updated_articles
                            );
                        }
                        Err(e) => tracing::error!("🚨 [Cortex] Compilation error: {}", e),
                        _ => {} // Ignore empty cycles
                    }
                }
            }
        }
    });

    Ok(())
}
