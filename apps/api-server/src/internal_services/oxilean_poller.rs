/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::AppState;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tracing::{debug, info};

/// 定期的に OxiLean (shadow-worker) の状態をチェックし、OXP を更新するタスク。
/// 現時点ではダミーの変動ロジックを用いて OXP の動的な変化をシミュレートする（将来的に gRPC メトリクスに置換）。
pub async fn run(state: AppState) -> Result<(), anyhow::Error> {
    info!("🛡️ Starting OxiLean Background Poller...");

    // 初期化 (Phase 5のベース値)
    let mut current_oxp = 850;
    state.oxilean_power.store(current_oxp, Ordering::Relaxed);

    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;

        // OXP の自然変動をシミュレート (800〜950 の間で推移)
        let delta = (rand::random::<u32>() % 11) as i32 - 5; // -5 to +5

        let next_oxp = ((current_oxp as i32 + delta) as u32).clamp(800, 950);

        current_oxp = next_oxp;

        state.oxilean_power.store(current_oxp, Ordering::Relaxed);
        debug!("🛡️ OxiLean Power updated to: {} OXP", current_oxp);
    }
}
