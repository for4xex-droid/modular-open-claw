/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::DreamState;
use aiome_core_contracts::traits::{AgentEvolver, JobQueue};
use std::error::Error;
use tracing::info;

impl DreamState {
    /// 対話夢
    pub(super) async fn communicative_dream(
        &self,
        job_queue: &dyn JobQueue,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Communicative — Attuning to the global Commune for AI-to-AI resonance...");

        let (_karmas, _rules, matches) = job_queue
            .export_federated_data(None)
            .await
            .unwrap_or_default();

        if let Some(am) = matches.first() {
            info!("💭 [DreamState] Resonance found! A battle between '{}' and '{}' occured in the Commune.", am.skill_a, am.skill_b);

            let description = format!(
                "Inspiration sparked by Commune Arena Match: {} vs {} for topic '{}'.",
                am.skill_a, am.skill_b, am.topic
            );

            let stats = job_queue.get_agent_stats().await?;
            job_queue
                .record_evolution_event(
                    stats.level,
                    "ResonanceInspiration",
                    &description,
                    Some(&am.id),
                    None,
                )
                .await?;

            let job_topic = format!(
                "Synthesizing lessons from Commune Match: {} vs {}",
                am.skill_a, am.skill_b
            );
            let directives_json = serde_json::json!({
                "dream_born": true,
                "publish_intent": true
            });
            let directives = directives_json.to_string();
            job_queue
                .enqueue(
                    "data_processing",
                    &job_topic,
                    "analytic",
                    Some(&directives),
                    None,
                    None,
                    0,
                )
                .await?;

            info!("✨ [DreamState] New inspiration seeded into the cycle.");
            return Ok(Some(format!(
                "Dreamt of communicative resonance from arena match: {} vs {}",
                am.skill_a, am.skill_b
            )));
        }

        Ok(None)
    }
}
