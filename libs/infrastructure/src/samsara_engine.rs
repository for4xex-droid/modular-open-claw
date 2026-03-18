use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use aiome_core::llm_provider::LlmProvider;
use sha2::Digest;
use soul::engine::SamsaraEngine;
use soul::error::SoulError;
use soul::instinct::Instinct;
use soul::model::AgentSoul;

pub struct DefaultSamsaraEngine {
    provider: Arc<dyn LlmProvider + Send + Sync>,
    distillation_prompt: String,
}

impl DefaultSamsaraEngine {
    pub fn new(provider: Arc<dyn LlmProvider + Send + Sync>, distillation_prompt: String) -> Self {
        Self {
            provider,
            distillation_prompt,
        }
    }
}

impl SamsaraEngine for DefaultSamsaraEngine {
    fn distill<'a>(
        &'a self,
        soul: &'a AgentSoul,
    ) -> Pin<Box<dyn Future<Output = Result<Instinct, SoulError>> + Send + 'a>> {
        Box::pin(async move {
            tracing::info!(
                "🧬 [SamsaraEngine] Distilling soul generation {}...",
                soul.generation
            );

            // Extract max 30 recent experiences to avoid LLM context overflow without cloning the whole buffer (GAP-2)
            let start = soul.experience_buffer.len().saturating_sub(30);
            let recent_experiences = &soul.experience_buffer[start..];
            let experiences_json = serde_json::to_string(recent_experiences).unwrap_or_default();
            let markers_json = serde_json::to_string(&soul.somatic_markers).unwrap_or_default();

            // Incorporate distillation system prompt (GAP-3)
            let prompt = format!(
                "{}\n\
                 You are distilling the experiences of an autonomous agent into InstinctRules.\n\
                 Recent Experiences:\n{}\n\
                 Somatic Markers:\n{}\n\
                 Create a concise JSON response containing an array of InstinctRules based on these experiences.\n\
                 Each InstinctRule must have:\n\
                 - \"rule\" (string): The distilled core directive or lesson\n\
                 - \"confidence\" (float 0.0-1.0): How strongly this rule is believed.\n\
                 Output ONLY the valid JSON array of objects, no markdown formatting.",
                self.distillation_prompt, experiences_json, markers_json
            );

            let new_hash = format!(
                "{:x}",
                sha2::Sha256::digest(
                    format!(
                        "distilled_{}_{}",
                        soul.soul_hash,
                        chrono::Utc::now().timestamp()
                    )
                    .as_bytes()
                )
            );

            let llm_result = match tokio::time::timeout(
                tokio::time::Duration::from_secs(30),
                self.provider.complete(&prompt, None),
            )
            .await
            {
                Ok(Ok(response)) => {
                    tracing::info!("   [SamsaraEngine] LLM distillation successful.");
                    let cleaned = response
                        .content
                        .replace("```json", "")
                        .replace("```", "")
                        .trim()
                        .to_string();
                    cleaned
                }
                Ok(Err(e)) => {
                    tracing::warn!("⚠️ [Distill] LLM execution failed: {}. Falling back to default distillation.", e);
                    "[]".to_string()
                }
                Err(_) => {
                    tracing::warn!("⚠️ [Distill] LLM timeout (30s). Falling back.");
                    "[]".to_string()
                }
            };

            let mut new_instinct = soul.instinct.clone();

            if let Ok(parsed_rules) = serde_json::from_str::<Vec<serde_json::Value>>(&llm_result) {
                for rule_val in parsed_rules {
                    if let (Some(r), Some(c)) = (
                        rule_val.get("rule").and_then(|v| v.as_str()),
                        rule_val.get("confidence").and_then(|v| v.as_f64()),
                    ) {
                        new_instinct.rules.push(soul::instinct::InstinctRule {
                            generation_origin: soul.generation,
                            rule: r.to_string(),
                            confidence: c,
                        });
                    }
                }
            } else {
                tracing::warn!(
                    "⚠️ [Distill] Failed to parse LLM response. Output snippet: {}",
                    llm_result.chars().take(100).collect::<String>()
                );
            }

            // Limit rules to top 10 by confidence
            new_instinct.rules.sort_by(|a, b| {
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            new_instinct.rules.truncate(10);
            new_instinct.hash = new_hash;

            Ok(new_instinct)
        })
    }

    fn rebirth<'a>(
        &'a self,
        soul: AgentSoul,
    ) -> Pin<Box<dyn Future<Output = Result<AgentSoul, SoulError>> + Send + 'a>> {
        Box::pin(async move {
            tracing::info!(
                "🌟 [SamsaraEngine] Triggering rebirth for soul id {}...",
                soul.id
            );

            let new_instinct = self.distill(&soul).await?;

            let mut new_soul = AgentSoul::new(soul.id.clone());
            new_soul.generation = soul.generation + 1;
            new_soul.instinct = new_instinct;
            // Inherit some elements (e.g. attachment, aesthetics in the future)
            new_soul.attachment = soul.attachment.clone();

            // GAP-4 Design Intent Clarification:
            // The `predictive_model` is intentionally left as `PredictiveModel::default()`.
            // If prediction accuracy was inherited, `local_plasticity` would remain low,
            // trapping the AI in a state of high exploitation and low exploration (zero surprise).
            // A new generation must start fresh, retaining only wisdom (Instincts) and scars (Defenses).

            // Note: somatic_markers is reset here, but strong defenses are inherited (DS-5)
            tracing::debug!(
                "   [SamsaraEngine] Resetting L1 somatic_markers for gen {}",
                new_soul.generation
            );

            let mut inherited_defenses = Vec::new();
            for def in &soul.defenses {
                if def.intensity > 0.7 {
                    let mut inherited = def.clone();
                    inherited.intensity *= 0.9; // Slight decay upon rebirth
                    inherited_defenses.push(inherited);
                }
            }
            if !inherited_defenses.is_empty() {
                tracing::info!(
                    "   [SamsaraEngine] Inherited {} strong defenses into new generation.",
                    inherited_defenses.len()
                );
            }
            new_soul.defenses = inherited_defenses;

            // Step 6: Inherit Anamnesis (Narrative & Schemas)
            new_soul.anamnesis = soul.anamnesis.clone();

            new_soul.compute_hash();

            tracing::info!(
                "🌟 [SamsaraEngine] Rebirth complete. Generation: {} -> {}",
                soul.generation,
                new_soul.generation
            );

            Ok(new_soul)
        })
    }
}
