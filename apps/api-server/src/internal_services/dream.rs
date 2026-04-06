/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::AppState;
use aiome_core_contracts::traits::AgentEvolver;
use infrastructure::dream_state::DreamState;
use infrastructure::trend_sonar::{ExternalTrendSonar, SerpAnalysisAdapter};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info};

/// Dream サービスを起動する。
/// 自発的な仮説生成・実験・省察（Dream State）を定期的に実行する。
pub async fn run(state: AppState) -> anyhow::Result<()> {
    info!("💤 [DreamService] Initializing Dream State Loop...");

    // 1. DreamState の初期化
    let dream_state = DreamState::new(state.provider.get_inner().clone());

    // 2. TrendSonar の準備（探索夢向け）
    // Phase β: Inject SerpAnalysisAdapter for trend-based SEO gap identification
    let mut adapters: Vec<Arc<dyn infrastructure::trend_sonar::TrendAdapter>> = vec![];
    
    // WebSearchAdapter + SerpAnalysisAdapter using SEARCH_API_KEY
    if let Ok(api_key) = std::env::var("SEARCH_API_KEY") {
        if !api_key.is_empty() {
            adapters.push(Arc::new(infrastructure::trend_sonar::WebSearchAdapter::new(api_key.clone())));
            adapters.push(Arc::new(SerpAnalysisAdapter::new(api_key)));
            info!("✅ [DreamService] WebSearch + SerpAnalysis adapters registered.");
        }
    }
    if adapters.is_empty() {
        info!("ℹ️ [DreamService] No SEARCH_API_KEY found. TrendSonar running in passive mode.");
    }
    let trend_sonar = ExternalTrendSonar::new(adapters, None);

    // 3. Queue / AgentLevel 情報へのアクセス
    let job_queue = state.job_queue.get_inner().clone();

    // 4. 定期実行ループ (心拍の裏で長時間走る)
    let mut timer = interval(Duration::from_secs(600)); // 10分おき

    loop {
        timer.tick().await;

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
