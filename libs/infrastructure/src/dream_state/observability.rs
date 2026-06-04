/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::DreamState;
use std::error::Error;
use tracing::{info, warn};

impl DreamState {
    /// Observability夢 (Phase 3-D)
    pub(crate) async fn observability_dream(
        &self,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Observability — Reviewing performance metrics of LLM providers...");
        if let Some(logger) = &self.eval_logger {
            let stats = logger.get_all_provider_stats(7).await?;
            if stats.is_empty() {
                info!("💤 [DreamState] No recent provider stats found.");
                return Ok(None);
            }

            let mut insights = Vec::new();

            for stat in stats {
                // Latency Threshold Check
                if stat.average_latency_ms > 2000.0 {
                    let msg = format!(
                        "⚠️ Provider '{}' ({}) is experiencing high average latency ({:.1} ms).",
                        stat.provider, stat.model, stat.average_latency_ms
                    );
                    warn!("🚨 [DreamState] Observability Alert: {}", msg);
                    insights.push(format!("high latency for {}", stat.model));
                }

                // Cost Threshold Check (Phase 3-D Requirement)
                if stat.total_cost_usd > 1.0 {
                    // $1.0 in 7 days as a warning threshold
                    let msg = format!(
                        "💰 Provider '{}' ({}) has accrued significant costs: ${:.4}.",
                        stat.provider, stat.model, stat.total_cost_usd
                    );
                    warn!("🚨 [DreamState] Observability Cost Alert: {}", msg);
                    insights.push(format!("high cost for {}", stat.model));
                }
            }

            if !insights.is_empty() {
                Ok(Some(format!(
                    "Observability Insights: {}",
                    insights.join("; ")
                )))
            } else {
                info!(
                    "✅ [DreamState] All LLM providers are operating within acceptable parameters."
                );

                // Phase 3-D+ DB GC Logic (Fire and Forget)
                let gc_logger = logger.clone();
                tokio::spawn(async move {
                    match gc_logger.garbage_collect(90).await {
                        Ok(cleaned) if cleaned > 0 => info!(
                            "🧹 [DreamState] GC removed {} old observability records.",
                            cleaned
                        ),
                        Err(e) => warn!("⚠️ [DreamState] GC for observability logs failed: {}", e),
                        _ => {}
                    }
                });

                Ok(None)
            }
        } else {
            info!("💤 [DreamState] Observability logger is not connected.");
            Ok(None)
        }
    }
}
