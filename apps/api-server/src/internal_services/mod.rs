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

pub async fn spawn_all(state: AppState) {
    info!("🚀 Spawning unified internal services (Watchtower & Heartbeat & OxiLean & Dream) with TaskSupervisor...");

    let supervisor = infrastructure::supervisor::TaskSupervisor::new(10, 300);
    let cancel_token = tokio_util::sync::CancellationToken::new();

    // 1. Watchtower Task (Discord/Telegram Bridge)
    struct WatchtowerTask {
        state: AppState,
    }
    impl infrastructure::supervisor::SupervisedTask for WatchtowerTask {
        fn name(&self) -> &'static str {
            "Watchtower"
        }
        fn run(
            &self,
            ct: tokio_util::sync::CancellationToken,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
            let state = self.state.clone();
            Box::pin(async move {
                loop {
                    if let Err(e) = watchtower::run(state.clone()).await {
                        tracing::error!(
                            "❌ Internal Watchtower service failed: {:?}. Restarting in 5s...",
                            e
                        );
                        tokio::select! {
                            _ = ct.cancelled() => { tracing::info!("🛑 Watchtower shutdown requested"); return; }
                            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
                        }
                    } else {
                        break;
                    }
                }
            })
        }
    }
    supervisor.spawn_supervised(
        WatchtowerTask {
            state: state.clone(),
        },
        cancel_token.clone(),
    );

    // 2. Heartbeat Task (Autonomous Pings & Plateau Detection)
    struct HeartbeatTask {
        state: AppState,
    }
    impl infrastructure::supervisor::SupervisedTask for HeartbeatTask {
        fn name(&self) -> &'static str {
            "Heartbeat"
        }
        fn run(
            &self,
            ct: tokio_util::sync::CancellationToken,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
            let state = self.state.clone();
            Box::pin(async move {
                loop {
                    if let Err(e) = heartbeat::run(state.clone()).await {
                        tracing::error!(
                            "❌ Internal Heartbeat service failed: {:?}. Restarting in 5s...",
                            e
                        );
                        tokio::select! {
                            _ = ct.cancelled() => { tracing::info!("🛑 Heartbeat shutdown requested"); return; }
                            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
                        }
                    } else {
                        break;
                    }
                }
            })
        }
    }
    supervisor.spawn_supervised(
        HeartbeatTask {
            state: state.clone(),
        },
        cancel_token.clone(),
    );

    // 3. Dream Task (Hypothesis, Review, and Insight Generation)
    struct DreamTask {
        state: AppState,
    }
    impl infrastructure::supervisor::SupervisedTask for DreamTask {
        fn name(&self) -> &'static str {
            "Dream"
        }
        fn run(
            &self,
            ct: tokio_util::sync::CancellationToken,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
            let state = self.state.clone();
            Box::pin(async move {
                loop {
                    if let Err(e) = dream::run(state.clone()).await {
                        tracing::error!(
                            "❌ Internal Dream service failed: {:?}. Restarting in 5s...",
                            e
                        );
                        tokio::select! {
                            _ = ct.cancelled() => { tracing::info!("🛑 Dream shutdown requested"); return; }
                            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
                        }
                    } else {
                        break;
                    }
                }
            })
        }
    }
    supervisor.spawn_supervised(
        DreamTask {
            state: state.clone(),
        },
        cancel_token.clone(),
    );

    // 4. OxiLean Poller Task
    struct OxiLeanTask {
        state: AppState,
    }
    impl infrastructure::supervisor::SupervisedTask for OxiLeanTask {
        fn name(&self) -> &'static str {
            "OxiLean"
        }
        fn run(
            &self,
            ct: tokio_util::sync::CancellationToken,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
            let state = self.state.clone();
            Box::pin(async move {
                loop {
                    if let Err(e) = oxilean_poller::run(state.clone()).await {
                        tracing::error!(
                            "❌ Internal OxiLean Poller service failed: {:?}. Restarting in 5s...",
                            e
                        );
                        tokio::select! {
                            _ = ct.cancelled() => { tracing::info!("🛑 OxiLean shutdown requested"); return; }
                            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
                        }
                    } else {
                        break;
                    }
                }
            })
        }
    }
    supervisor.spawn_supervised(
        OxiLeanTask {
            state: state.clone(),
        },
        cancel_token.clone(),
    );
}
