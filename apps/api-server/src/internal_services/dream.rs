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
    let incident_repo = std::sync::Arc::new(
        infrastructure::aegis::incident_repo::IncidentRepository::new(
            (*state.db_pool.get_inner()).as_ref().clone(),
        ),
    );

    let dream_state = DreamState::new(state.provider.get_inner().clone())
        .with_eval_logger(state.eval_logger.get_inner().clone())
        .with_incident_repo(incident_repo.clone())
        .with_event_sender(state.event_sender.get_inner().clone());

    // 2. Queue / AgentLevel 情報へのアクセス
    let job_queue = job_queue_inner;
    let llm_provider = state.provider.0.clone();

    // 3. 定期実行ループ (心拍の裏で長時間走る)
    let mut timer = interval(Duration::from_secs(600)); // 10分おき

    loop {
        timer.tick().await;

        // TrendSonar をループ毎に再構築し、最新の DB 設定を反映する
        // (State Staleness 防止: API キーの動的変更に追従)
        let trend_sonar = infrastructure::trend_sonar::build_active_trend_sonar(
            job_queue.as_ref(),
            llm_provider.clone(),
        )
        .await;

        let level = job_queue
            .get_agent_stats()
            .await
            .map(|stats| stats.level)
            .unwrap_or(1);

        info!("💤 [DreamService] Agent Level: {}. Contemplating...", level);

        match dream_state.dream(&*job_queue, &trend_sonar, level).await {
            Ok(Some(result)) => {
                if let Some(insight) = result.insight {
                    info!("🌌 [DreamService] Dream Insight: {}", insight);
                }

                // Phase 2: Aegis Sentinel HotSwap Auto-Remediation
                for request in result.hot_swaps {
                    info!(
                        "🔥 [DreamService] Executing HotSwap for incident: {}",
                        request.incident_id
                    );

                    // Call SkillForge to compile the new WASM
                    // retry_count=0: No forge-level retry needed since Aegis already validated via Kani.
                    // LLM=None: Patch code is pre-validated; forge self-heal is unnecessary.
                    let skill_forge = state.skill_forge.get_inner();
                    match skill_forge
                        .forge_skill(
                            &request.skill_name,
                            &request.patch_code,
                            0,
                            "Aegis Sentinel auto-remediated HotSwap",
                            None,
                        )
                        .await
                    {
                        Ok(new_wasm_path) => {
                            info!("✅ [DreamService] HotSwap successful for skill {}. New WASM Path: {}", request.skill_name, new_wasm_path.display());

                            // Invalidate cache so the next execution loads the new WASM
                            let wasm_manager = state.wasm_skill_manager.get_inner();
                            wasm_manager.invalidate_cache(&request.skill_name);

                            // Update incident status to Resolved
                            if let Err(e) = incident_repo
                                .update_status(
                                    &request.incident_id,
                                    infrastructure::aegis::types::IncidentStatus::Resolved,
                                )
                                .await
                            {
                                warn!(
                                    "⚠️ [DreamService] Failed to set Resolved status for {}: {}",
                                    request.incident_id, e
                                );
                            }
                            metrics::counter!("aegis_hotswap_success_total").increment(1);
                        }
                        Err(e) => {
                            error!(
                                "❌ [DreamService] SkillForge compilation failed for {}: {:?}",
                                request.skill_name, e
                            );

                            // Increment retry count to prevent infinite Open→HotSwap→Open loops.
                            // When MAX_KANI_RETRIES is exceeded, the next dream cycle will
                            // transition the incident to WontFix via the batch loop.
                            if let Err(e) = incident_repo
                                .increment_retry_count(&request.incident_id)
                                .await
                            {
                                warn!(
                                    "⚠️ [DreamService] Failed to increment retry count for {}: {}",
                                    request.incident_id, e
                                );
                            }
                            if let Err(e) = incident_repo
                                .update_status(
                                    &request.incident_id,
                                    infrastructure::aegis::types::IncidentStatus::Open,
                                )
                                .await
                            {
                                warn!(
                                    "⚠️ [DreamService] Failed to revert Open status for {}: {}",
                                    request.incident_id, e
                                );
                            }
                            metrics::counter!("aegis_hotswap_failure_total").increment(1);
                        }
                    }
                }
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
