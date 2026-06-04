/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::AppState;
use aiome_core::traits::{ChatStore, SettingsOps};
use aiome_core_contracts::events::CoreEvent;
use aiome_core_contracts::traits::AgentEvolver;
use infrastructure::heartbeat_wakeup::HeartbeatWakeupService;
use infrastructure::score_tracker::ScoreTracker;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::info;

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
        (**state.db_pool.get_inner()).clone(),
    ));

    // 2. HeartbeatWakeupService の初期化
    // state.job_queue は AgentEvolver を継承しているため、そのまま渡せる。
    let lora_service: Option<Arc<dyn aiome_core_contracts::traits::LoraEngine>> =
        state.lora_engine.as_opt().cloned();

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
        // Removed: snapshot is already recorded inside wakeup_service.run_wakeup_ping()

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

        // C. Intent-First Suggestion
        check_suggestion(&state).await;
    }
}

async fn check_suggestion(state: &AppState) {
    if !is_suggestion_enabled(state).await {
        return;
    }

    // Default channel history
    if let Ok(history) = state.job_queue.fetch_chat_history("default", 10).await {
        if history.len() >= 5 {
            // V1: Simple trigger for suggestion based on activity volume
            let _ = state
                .event_sender
                .get_inner()
                .send(CoreEvent::ProactiveTalk {
                    message: "【Suggestion】最近のチャット履歴から、システム診断の実行を推奨します。実行しますか？".to_string(),
                    channel_id: 0,
                });
        }
    }
}

async fn is_suggestion_enabled(state: &AppState) -> bool {
    let flag = state
        .job_queue
        .get_setting_value("feature_flag.intent_first_suggestion")
        .await
        .ok()
        .flatten();
    // Default to true if not explicitly set to false
    flag.as_deref() != Some("false")
}
