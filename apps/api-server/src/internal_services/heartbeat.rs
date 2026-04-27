/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::AppState;
use aiome_core_contracts::events::CoreEvent;
use aiome_core_contracts::traits::AgentEvolver;
use infrastructure::heartbeat_wakeup::HeartbeatWakeupService;
use infrastructure::score_tracker::ScoreTracker;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info};

/// Heartbeat サービスを起動する。
/// 自律的な話しかけ（Wakeup Ping）や、Score Plateau（停滞）の検知を定期的に実行する。
pub async fn run(state: AppState) -> anyhow::Result<()> {
    info!("💓 [Heartbeat] Initializing Unified Heartbeat Service...");

    // 1. ScoreTracker の初期化 (Phase 3D / Sprint 3)
    let forecast_provider: Option<Arc<dyn aiome_core_contracts::forecast::ForecastProvider>> = {
        if let Ok(timesfm_url) = std::env::var("TIMESFM_URL") {
            let auth = std::env::var("TIMESFM_AUTH_TOKEN").unwrap_or_default();
            shared::security::scrub_env("TIMESFM_AUTH_TOKEN");
            Some(Arc::new(
                infrastructure::forecast::timesfm::TimesFmProvider::new(timesfm_url, auth),
            ))
        } else {
            None
        }
    };

    let score_tracker = Arc::new(ScoreTracker::new(
        forecast_provider,
        state.job_queue.get_inner().get_pool().clone(),
    ));

    // 2. HeartbeatWakeupService の初期化
    // state.job_queue は AgentEvolver を継承しているため、そのまま渡せる。
    let lora_service = state.lora_engine.as_opt().and_then(|_e| {
        // LoraEngine を LoraTrainingService にダウンキャストすることはできないが、
        // 実際には main.rs で LoraTrainingService が注入されている。
        None
    });

    let evolver: Arc<dyn AgentEvolver> = state.job_queue.get_inner().clone();

    let wakeup_service = HeartbeatWakeupService::new(
        state.provider.get_inner().clone(),
        state.llm_semaphore.get_inner().clone(),
        state.config.get_inner().resolver.root().to_path_buf(),
    )
    .with_evolution_tools(score_tracker.clone(), evolver.clone(), lora_service);

    // 3. 定期実行ループ (5分おき)
    let mut timer = interval(Duration::from_secs(300));

    loop {
        timer.tick().await;
        info!("💓 [Heartbeat] Running periodic heartbeat check...");

        // A. Snapshot 記録
        if let Err(e) = score_tracker.record_daily_snapshot(&evolver).await {
            error!("❌ [Heartbeat] Failed to record score snapshot: {:?}", e);
        }

        // B. Wakeup Ping (自律的な話しかけ)
        if let Some(message) = wakeup_service.run_wakeup_ping().await {
            info!("📢 [Heartbeat] Wakeup message generated: {}", message);

            // 全チャネル（またはデフォルトチャネル）にブロードキャスト
            let _ = state
                .event_sender
                .get_inner()
                .send(CoreEvent::ProactiveTalk {
                    message,
                    channel_id: 0, // 0 = Default Broadcast
                });
        }
    }
}
