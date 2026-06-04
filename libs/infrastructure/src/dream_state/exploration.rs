/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::DreamState;
use crate::trend_sonar::ExternalTrendSonar;
use aiome_core_contracts::traits::{JobQueue, TrendSource};
use rand::Rng;
use std::error::Error;
use tracing::{info, warn};

impl DreamState {
    /// 探索夢
    pub(super) async fn explorative_dream(
        &self,
        job_queue: &dyn JobQueue,
        trend_sonar: &ExternalTrendSonar,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Explorative — Searching for new creative horizons...");

        let seeds = [
            "cyberpunk aesthetics",
            "ancient lost technology",
            "biomimicry",
            "lo-fi horror",
            "solarpunk architecture",
        ];
        let seed = seeds[rand::thread_rng().gen_range(0..seeds.len())];

        match trend_sonar.get_trends(seed).await {
            Ok(trends) if !trends.is_empty() => {
                let best = &trends[0];
                info!(
                    "🔮 [DreamState] Dreamt of a new possibility: '{}'. Seeded into the cycle.",
                    best.keyword
                );

                let directives_json = serde_json::json!({
                    "dream_born": true,
                    "seed": seed,
                    "phantom": true
                });
                let directives = directives_json.to_string();
                job_queue
                    .enqueue(
                        "data_processing",
                        &best.keyword,
                        "auto",
                        Some(&directives),
                        None,
                        None,
                        0,
                    )
                    .await?;
                return Ok(Some(format!("Explored a new seed: {}", best.keyword)));
            }
            Ok(_) => warn!("💤 [DreamState] The dream was a void. No trends found."),
            Err(e) => warn!("💤 [DreamState] Dream vision blurred: {}", e),
        }

        Ok(None)
    }
}
