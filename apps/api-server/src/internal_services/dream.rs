use crate::AppState;
use aiome_core_contracts::traits::AgentEvolver;
use infrastructure::cortex_file_projector::CortexFileProjector;
use infrastructure::dream_state::DreamState;
use infrastructure::trend_sonar::{ExternalTrendSonar, SerpAnalysisAdapter};
use std::sync::Arc;
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

    // 1. DreamState の初期化 (ADR-025: Agent-Native Discovery 有効化)
    let dream_state = DreamState::new(state.provider.get_inner().clone());

    // 2. TrendSonar の準備（探索夢向け）
    // Phase β: Inject SerpAnalysisAdapter for trend-based SEO gap identification
    let mut adapters: Vec<Arc<dyn infrastructure::trend_sonar::TrendAdapter>> = vec![];

    // WebSearchAdapter + SerpAnalysisAdapter using SEARCH_API_KEY
    if let Ok(api_key) = std::env::var("SEARCH_API_KEY") {
        if !api_key.is_empty() {
            adapters.push(Arc::new(
                infrastructure::trend_sonar::WebSearchAdapter::new(api_key.clone()),
            ));
            adapters.push(Arc::new(SerpAnalysisAdapter::new(api_key)));
            info!("✅ [DreamService] WebSearch + SerpAnalysis adapters registered.");
        }
    }

    // XSignalProbe using X_BEARER_TOKEN
    if let Ok(x_token) = std::env::var("X_BEARER_TOKEN") {
        if !x_token.is_empty() {
            adapters.push(Arc::new(infrastructure::x_signal_probe::XSignalProbe::new(
                x_token,
            )));
            info!("✅ [DreamService] XSignalProbe adapter registered.");
        }
    }

    if adapters.is_empty() {
        info!("ℹ️ [DreamService] No SEARCH_API_KEY or X_BEARER_TOKEN found. TrendSonar running in passive mode.");
    }
    let trend_sonar = ExternalTrendSonar::new(adapters, None);

    // 3. Queue / AgentLevel 情報へのアクセス
    let job_queue = job_queue_inner;

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
