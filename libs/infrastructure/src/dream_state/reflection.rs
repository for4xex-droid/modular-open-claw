/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::DreamState;
use aiome_core_contracts::traits::{JobQueue, JobStatus};
use std::error::Error;
use tracing::{info, warn};

impl DreamState {
    /// 省察夢
    pub(super) async fn reflective_dream(
        &self,
        job_queue: &dyn JobQueue,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Reflective — Contemplating past scars and lessons...");

        let recent = job_queue.fetch_all_karma(10).await?;
        if recent.is_empty() {
            info!("💤 [DreamState] No memories to reflect upon yet.");
            return Ok(None);
        }

        let recent_jobs = job_queue.fetch_recent_jobs(20).await?;
        let failed_jobs: Vec<_> = recent_jobs
            .iter()
            .filter(|j| matches!(j.status, JobStatus::Failed))
            .collect();

        if let Some(fail) = failed_jobs.first() {
            info!("🩹 [DreamState] Remembering the failure of '{}'. Dreaming of a redemption version...", fail.topic);
            let redemption_topic = format!("{} (Redemption Remix)", fail.topic);
            let directives_json = serde_json::json!({
                "remix_of": fail.id,
                "dream_born": true
            });
            let directives = directives_json.to_string();
            job_queue
                .enqueue(
                    "data_processing",
                    &redemption_topic,
                    "auto",
                    Some(&directives),
                    None,
                    None,
                    0,
                )
                .await?;
            // X-1: Dream insight → Karma write-back (経済活動→夢→学習ループ)
            if let Err(e) = job_queue
                .store_karma(
                    &fail.id,
                    "dream_reflection",
                    &format!(
                        "Reflected on failure of '{}' and initiated redemption remix",
                        fail.topic
                    ),
                    "Synthesized",
                    "autonomous_dream_system",
                    Some("meta_cognition"),
                    Some("reflective_dream"),
                    None,
                    false,
                )
                .await
            {
                warn!(
                    "⚠️ [DreamState] Failed to store dream reflection karma: {}",
                    e
                );
            }
            return Ok(Some(format!("Reflected on failure of '{}'", fail.topic)));
        } else {
            info!("✨ [DreamState] The past is clear. No recent failures haunt my dreams.");
        }

        let sentinel = crate::cognitive_sentinel::CognitiveSentinel::new(
            crate::cognitive_sentinel::CognitiveThresholds::default(),
        );
        match sentinel.diagnose(job_queue, "system_agent").await {
            Ok(Some(alert)) => {
                warn!("🚨 [DreamState] {}", alert);
                return Ok(Some(alert));
            }
            Err(e) => {
                warn!(
                    "⚠️ [DreamState] CognitiveSentinel failed to diagnose: {}",
                    e
                );
            }
            _ => {}
        }

        Ok(None)
    }
}
