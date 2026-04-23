/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

pub mod dream;
pub mod heartbeat;
pub mod watchtower;

pub mod oxilean_poller;

use crate::AppState;
use std::sync::Arc;
use tracing::info;

/// 内部バックグラウンドタスクを起動する。
pub async fn spawn_all(state: AppState) {
    info!("🚀 Spawning unified internal services (Watchtower & Heartbeat & OxiLean)...");

    // 1. Watchtower Task (Discord/Telegram Bridge)
    let state_clone = state.clone();
    tokio::spawn(async move {
        if let Err(e) = watchtower::run(state_clone).await {
            tracing::error!("❌ Internal Watchtower service failed: {:?}", e);
        }
    });

    // 2. Heartbeat Task (Autonomous Pings & Plateau Detection)
    let state_clone = state.clone();
    tokio::spawn(async move {
        if let Err(e) = heartbeat::run(state_clone).await {
            tracing::error!("❌ Internal Heartbeat service failed: {:?}", e);
        }
    });

    // 3. Dream Task (Hypothesis, Review, and Insight Generation)
    let state_clone = state.clone();
    tokio::spawn(async move {
        if let Err(e) = dream::run(state_clone).await {
            tracing::error!("❌ Internal Dream service failed: {:?}", e);
        }
    });

    // 4. OxiLean Poller Task
    let state_clone = state.clone();
    tokio::spawn(async move {
        if let Err(e) = oxilean_poller::run(state_clone).await {
            tracing::error!("❌ Internal OxiLean Poller service failed: {:?}", e);
        }
    });
}
