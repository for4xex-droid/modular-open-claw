/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::{DreamResult, DreamState, HotSwapRequest};
use aiome_core_contracts::events::CoreEvent;
use std::error::Error;
use tracing::{info, warn};

impl DreamState {
    /// Aegis Sentinel 夢 (Phase 1 & Phase 2)
    pub async fn aegis_sentinel_dream(
        &self,
    ) -> Result<Option<DreamResult>, Box<dyn Error + Send + Sync>> {
        let repo = match &self.incident_repo {
            Some(r) => r,
            None => return Ok(None),
        };

        let stats = match repo.compute_weekly_stats().await {
            Ok(s) => s,
            Err(e) => {
                warn!("⚠️ [DreamState] Failed to compute Aegis stats: {}", e);
                return Ok(None);
            }
        };

        let mut alert_level = "Info";
        if stats.total_incidents_7d >= 50 || stats.unresolved >= 20 {
            alert_level = "Critical";
        } else if stats.total_incidents_7d >= 10 || stats.distinct_skills >= 5 {
            alert_level = "Warning";
        }

        let mut alert_msg = None;
        if alert_level != "Info" {
            let msg = format!(
                "Aegis {} Alert: {} total incidents in last 7 days. Top failing skill: {:?}",
                alert_level, stats.total_incidents_7d, stats.top_failing_skill
            );
            warn!("🚨 [DreamState] {}", msg);

            if let Some(sender) = &self.event_sender {
                if let Err(e) = sender.send(CoreEvent::AegisSentinel {
                    level: alert_level.to_string(),
                    message: msg.clone(),
                    total_incidents: stats.total_incidents_7d,
                    top_skill: stats.top_failing_skill.clone(),
                }) {
                    warn!("⚠️ [DreamState] Failed to broadcast AegisSentinel event (no subscribers?): {}", e);
                }
            }
            alert_msg = Some(msg);
        }

        let mut hot_swaps = Vec::new();

        // Phase 2: AegisProver Batch Loop for auto-remediation (ADR-040)
        let prover = crate::aegis::prover::AegisProver::new(self.llm.clone());
        let open_incidents = match repo.fetch_open_incidents(5).await {
            Ok(incidents) => incidents,
            Err(e) => {
                warn!(
                    "⚠️ [DreamState] Failed to fetch open incidents for Aegis batch loop: {}",
                    e
                );
                Vec::new()
            }
        };

        for mut incident in open_incidents {
            info!(
                "🛡️ [DreamState] AegisProver analyzing incident: {}",
                incident.id
            );
            if let Err(e) = repo
                .update_status(&incident.id, crate::aegis::types::IncidentStatus::Analyzing)
                .await
            {
                warn!(
                    "⚠️ [DreamState] Failed to set Analyzing status for {}: {}",
                    incident.id, e
                );
            }

            match prover.generate_patch(&incident).await {
                Ok(patch_code) => {
                    if let Err(e) = repo
                        .update_status(
                            &incident.id,
                            crate::aegis::types::IncidentStatus::PatchGenerated,
                        )
                        .await
                    {
                        warn!(
                            "⚠️ [DreamState] Failed to set PatchGenerated status for {}: {}",
                            incident.id, e
                        );
                    }

                    if let Err(e) = repo
                        .update_status(
                            &incident.id,
                            crate::aegis::types::IncidentStatus::KaniVerifying,
                        )
                        .await
                    {
                        warn!(
                            "⚠️ [DreamState] Failed to set KaniVerifying status for {}: {}",
                            incident.id, e
                        );
                    }
                    match prover.verify_with_kani(&patch_code).await {
                        Ok(true) => {
                            info!(
                                "✅ [DreamState] Kani verification SUCCEEDED for incident: {}",
                                incident.id
                            );
                            if let Err(e) = repo
                                .update_status(
                                    &incident.id,
                                    crate::aegis::types::IncidentStatus::KaniSuccess,
                                )
                                .await
                            {
                                warn!(
                                    "⚠️ [DreamState] Failed to set KaniSuccess status for {}: {}",
                                    incident.id, e
                                );
                            }

                            // Return HotSwapRequest for Phase 2 automation
                            hot_swaps.push(HotSwapRequest {
                                incident_id: incident.id.clone(),
                                skill_name: incident.skill_name.clone(),
                                patch_code,
                            });
                        }
                        _ => {
                            warn!(
                                "❌ [DreamState] Kani verification FAILED for incident: {}",
                                incident.id
                            );
                            if let Err(e) = repo.increment_retry_count(&incident.id).await {
                                warn!(
                                    "⚠️ [DreamState] Failed to increment retry count for {}: {}",
                                    incident.id, e
                                );
                            }
                            incident.retry_count += 1; // local tracking for loop-internal threshold check

                            if incident.retry_count >= crate::aegis::prover::MAX_KANI_RETRIES {
                                warn!("⛔ [DreamState] Max Kani retries reached for incident: {}. Transitioning to WontFix.", incident.id);
                                if let Err(e) = repo
                                    .update_status(
                                        &incident.id,
                                        crate::aegis::types::IncidentStatus::WontFix,
                                    )
                                    .await
                                {
                                    warn!(
                                        "⚠️ [DreamState] Failed to set WontFix status for {}: {}",
                                        incident.id, e
                                    );
                                }
                                metrics::counter!("aegis_kani_wontfix_total").increment(1);
                            } else {
                                // Revert to Open for next dream cycle
                                if let Err(e) = repo
                                    .update_status(
                                        &incident.id,
                                        crate::aegis::types::IncidentStatus::Open,
                                    )
                                    .await
                                {
                                    warn!(
                                        "⚠️ [DreamState] Failed to revert Open status for {}: {}",
                                        incident.id, e
                                    );
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "⚠️ [DreamState] AegisProver failed to generate patch: {}",
                        e
                    );
                    if let Err(e) = repo.increment_retry_count(&incident.id).await {
                        warn!(
                            "⚠️ [DreamState] Failed to increment retry count for {}: {}",
                            incident.id, e
                        );
                    }
                    incident.retry_count += 1;

                    if incident.retry_count >= crate::aegis::prover::MAX_KANI_RETRIES {
                        warn!("⛔ [DreamState] Max Kani retries reached (patch generation failed). Transitioning to WontFix.");
                        if let Err(e) = repo
                            .update_status(
                                &incident.id,
                                crate::aegis::types::IncidentStatus::WontFix,
                            )
                            .await
                        {
                            warn!(
                                "⚠️ [DreamState] Failed to set WontFix status for {}: {}",
                                incident.id, e
                            );
                        }
                        metrics::counter!("aegis_kani_wontfix_total").increment(1);
                    } else if let Err(e) = repo
                        .update_status(&incident.id, crate::aegis::types::IncidentStatus::Open)
                        .await
                    {
                        warn!(
                            "⚠️ [DreamState] Failed to revert Open status for {}: {}",
                            incident.id, e
                        );
                    }
                }
            }
        }

        if alert_msg.is_some() || !hot_swaps.is_empty() {
            let fallback_msg = "Aegis Sentinel dream complete: analyzed recent incidents and ran auto-remediation batch loop.".to_string();
            let insight = alert_msg.or(Some(fallback_msg));
            return Ok(Some(DreamResult { insight, hot_swaps }));
        }

        Ok(Some(DreamResult::from_insight(
            "Aegis Sentinel dream complete: no incidents to process.".to_string(),
        )))
    }
}
