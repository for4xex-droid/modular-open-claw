/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::AppState;
use aiome_core_contracts::traits::AgentEvolver;
use infrastructure::dream_state::DreamState;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, warn};

/// Dream サービスを起動する。
/// 自発的な仮説生成・実験・省察（Dream State）を定期的に実行する。
pub async fn run(state: AppState) -> anyhow::Result<()> {
    info!("💤 [DreamService] Initializing Dream State Loop...");

    // ADR-025: CortexFileProjector の初期化 — Agent-Native Discovery
    let resolver = shared::app_data::AppDataResolver::new();
    let cortex_fs_root = resolver.resolve("cortex_fs");
    let job_queue_inner = state.job_queue.get_inner().clone();
    let projector = state.cortex_projector.get_inner().clone();

    // 起動時に初回投影を実行
    match projector.project_to_filesystem().await {
        Ok(report) => {
            info!(
                "📂 [DreamService] Initial Cortex FS projection: {} created, {} updated across {} categories",
                report.files_created, report.files_updated, report.categories_count
            );
        }
        Err(e) => {
            warn!(
                "⚠️ [DreamService] Initial Cortex FS projection failed (non-fatal): {}",
                e
            );
        }
    }

    // 1. DreamState の初期化 (ADR-025: Agent-Native Discovery 有効化、Phase 3-D Observability 有効化)
    let dream_state = DreamState::new(state.provider.get_inner().clone())
        .with_eval_logger(state.eval_logger.get_inner().clone());

    // 2. Queue / AgentLevel 情報へのアクセス
    let job_queue = job_queue_inner;
    let llm_provider = state.provider.0.clone();

    // 3. 定期実行ループ (心拍の裏で長時間走る)
    let mut timer = interval(Duration::from_secs(600)); // 10分おき

    loop {
        timer.tick().await;

        // TrendSonar をループ毎に再構築し、最新の DB 設定を反映する
        // (State Staleness 防止: API キーの動的変更に追従)
        let trend_sonar =
            infrastructure::trend_sonar::build_active_trend_sonar(job_queue.as_ref(), llm_provider.clone())
                .await;

        let level = job_queue
            .get_agent_stats()
            .await
            .map(|stats| stats.level)
            .unwrap_or(1);

        info!("💤 [DreamService] Agent Level: {}. Contemplating...", level);

        match dream_state.dream(&*job_queue, &trend_sonar, level).await {
            Ok(Some(insight)) => {
                info!("🌌 [DreamService] Dream Insight: {}", insight);
            }
            Ok(None) => {
                // Preempted or void dream
            }
            Err(e) => {
                error!("❌ [DreamService] Dream sequence failed: {:?}", e);
            }
        }
    }
}
