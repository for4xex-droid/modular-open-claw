/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::DreamState;
use aiome_core_contracts::traits::{JobQueue, KarmaRegistry};
use serde_json::Value;
use std::error::Error;
use tracing::{info, warn};

impl DreamState {
    /// 仮説検証夢 (ADR-023)
    pub(super) async fn scientific_dream(
        &self,
        job_queue: &dyn JobQueue,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Scientific — Formulating improvement hypotheses...");

        // 1. Analyze existing Karma to find low-performance domains
        let recent_karma = job_queue.fetch_all_karma(20).await?;
        let karma_summary = serde_json::to_string(&recent_karma)?;

        // 2. Generate Hypothesis via LLM
        let prompt = format!(
            "Analyze the following Karma entries and hypothesize a way to improve the agent's performance.\n\nKarma:\n{}\n\nOutput a structured hypothesis in JSON: {{ \"domain\": \"string\", \"problem\": \"string\", \"hypothesis\": \"string\", \"experiment_design\": \"string\" }}",
            karma_summary
        );

        let resp = self
            .llm
            .complete(
                &prompt,
                Some("You are a Scientific AI Researcher. Generate innovative improvement hypotheses."),
            )
            .await?;

        let json_str = crate::llm::utils::extract_json(&resp.content)?;
        let manifest: Value = serde_json::from_str(json_str.as_ref())?;

        let domain = manifest["domain"].as_str().unwrap_or("General");
        let hypothesis = manifest["hypothesis"].as_str().unwrap_or("No hypothesis");

        info!(
            "🧪 [DreamState] New Hypothesis for {}: {}",
            domain, hypothesis
        );

        // 3. Dispatch Experiment Job
        let job_topic = format!(
            "[Experiment] {} - {}",
            domain,
            manifest["problem"].as_str().unwrap_or("")
        );
        let directives = serde_json::json!({
            "dream_born": true,
            "hypothesis": manifest,
            "scientific_loop": true
        })
        .to_string();

        job_queue
            .enqueue(
                "scientific_experiment",
                &job_topic,
                "experimental",
                Some(&directives),
                None,
                None,
                0,
            )
            .await?;

        // 4. AutoHarness Integration (Phase E-5)
        // If domain is security or safety, generate a Shadow harness
        let is_security_related = domain.to_lowercase().contains("security")
            || domain.to_lowercase().contains("safety")
            || hypothesis.to_lowercase().contains("vulnerability")
            || hypothesis.to_lowercase().contains("exploit");

        if is_security_related {
            info!(
                "🛡️ [DreamState] Security-related hypothesis detected. Generating AutoHarness..."
            );
            let harness_prompt = format!(
                "Based on this hypothesis: '{}', generate a simple WASM-compatible Rust code that returns 'true' if the action is safe and 'false' if it's risky.\n\
                Hypothesis: {}\n\
                Problem: {}\n\
                The code must be a valid Rust source that can be compiled to WASM. Just return the Rust code.",
                hypothesis,
                hypothesis,
                manifest["problem"].as_str().unwrap_or("Unknown")
            );

            if let Ok(h_resp) = self
                .llm
                .complete(
                    &harness_prompt,
                    Some("You are a Security Engineer. Generate a WASM harness."),
                )
                .await
            {
                // In a real implementation, we would compile this code.
                // For the MVP, we store the 'intent' of the code or a placeholder.
                // Here we simulate the generation.
                let harness_id = format!("auto_{}", uuid::Uuid::new_v4().simple());
                let record = aiome_core_contracts::contracts::HarnessRecord {
                    id: harness_id.clone(),
                    domain: domain.to_string(),
                    description: format!("Autonomous harness for: {}", hypothesis),
                    code_payload: h_resp.content, // Real implementation would compile to WASM bytes
                    status: aiome_core_contracts::contracts::HarnessStatus::Shadow,

                    severity: 70,
                    version: 1,
                    agent_id: None,
                    fire_count: 0,
                    false_positive_count: 0,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    last_fired_at: None,
                    safety_level: aiome_core_contracts::contracts::ToolSafetyLevel::Safe,
                };

                if let Err(e) = job_queue.store_harness_record(&record).await {
                    warn!("⚠️ [DreamState] Failed to store autonomous harness: {}", e);
                } else {
                    info!(
                        "✅ [DreamState] Autonomous Shadow harness '{}' registered.",
                        harness_id
                    );
                }
            }
        }

        Ok(Some(format!(
            "Hypothesized improvement for {}: {}",
            domain, hypothesis
        )))
    }
}
