/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

pub mod heartbeat;
pub mod watchtower;

use crate::AppState;
use std::sync::Arc;
use tracing::info;

/// 内部バックグラウンドタスクを起動する。
pub async fn spawn_all(state: AppState) {
    info!("🚀 Spawning unified internal services (Watchtower & Heartbeat)...");

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
}
