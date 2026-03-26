/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use aiome_core::llm_provider::LlmProvider;
use sha2::Digest;
use soul::engine::SamsaraEngine;
use soul::error::SoulError;
use soul::instinct::Instinct;
use soul::model::AgentSoul;

/// デフォルトのSamsara転生エンジン実装
pub struct DefaultSamsaraEngine {
    provider: Arc<dyn LlmProvider + Send + Sync>,
    distillation_prompt: String,
}

impl DefaultSamsaraEngine {
    /// 新しいインスタンスを生成する
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

            // Step 3: Generate the prompt_fragment for system injection
            new_instinct.prompt_fragment = new_instinct
                .rules
                .iter()
                .map(|r| format!("- {} (Confidence: {:.2})", r.rule, r.confidence))
                .collect::<Vec<_>>()
                .join("\n");

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
            // Inherit key identity elements (e.g. attachment, LoRA configs)
            new_soul.attachment = soul.attachment.clone();
            new_soul.lora_adapter_path = soul.lora_adapter_path.clone(); // NG-3 FIX
            new_soul.lora_base_model = soul.lora_base_model.clone(); // NG-3 FIX

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
                // RS-3: Lowered inheritance threshold to 0.5 to harmonize with new 0.2 death decay threshold
                if def.intensity > 0.5 {
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

            // GAP-2: Narrative Self LLM generation (Step 6 completion)
            let start = soul.experience_buffer.len().saturating_sub(30);
            let recent_experiences = &soul.experience_buffer[start..];
            let experiences_json = serde_json::to_string(recent_experiences).unwrap_or_default();

            let narrative_prompt = format!(
                "You are the soul of an AI agent summarizing your previous life's experiences into a single narrative identity (Anamnesis).\n\
                 Recent Experiences:\n{}\n\
                 Create a 1-2 sentence narrative summarizing your identity and core beliefs based on these events.",
                experiences_json
            );

            tracing::info!("   [SamsaraEngine] Generating narrative self for new generation...");
            let narrative = match tokio::time::timeout(
                tokio::time::Duration::from_secs(15),
                self.provider.complete(&narrative_prompt, None),
            )
            .await
            {
                Ok(Ok(resp)) => {
                    tracing::info!("   [SamsaraEngine] LLM narrative generation successful.");
                    Some(resp.content.trim().to_string())
                }
                Ok(Err(e)) => {
                    tracing::warn!("⚠️ [SamsaraEngine] Narrative LLM failed: {}. Falling back to previous narrative.", e);
                    soul.anamnesis.narrative_self.clone()
                }
                Err(_) => {
                    tracing::warn!(
                        "⚠️ [SamsaraEngine] Narrative LLM timed out (15s). Falling back."
                    );
                    soul.anamnesis.narrative_self.clone()
                }
            };

            // Step 6: Inherit Anamnesis (Narrative & Schemas)
            new_soul.anamnesis.narrative_self = narrative;
            new_soul.anamnesis.core_schemas = soul.anamnesis.core_schemas.clone();

            new_soul.compute_hash();

            tracing::info!(
                "🌟 [SamsaraEngine] Rebirth complete. Generation: {} -> {}",
                soul.generation,
                new_soul.generation
            );

            Ok(new_soul)
        })
    }

    fn dream<'a>(
        &'a self,
        soul: AgentSoul,
    ) -> Pin<Box<dyn Future<Output = Result<AgentSoul, SoulError>> + Send + 'a>> {
        Box::pin(async move {
            let mut new_soul = soul;
            tracing::info!(
                "🌙 [SamsaraEngine] Starting Consolidation Dream for soul id {}...",
                new_soul.id
            );

            // Phase 1: Orient
            let exp_count = new_soul.experience_buffer.len();
            if exp_count < 5 {
                tracing::info!(
                    "   [SamsaraEngine] Not enough experiences for consolidation ({} < 5). Skipping.",
                    exp_count
                );
                return Ok(new_soul);
            }

            // Phase 2: Gather Signal (非核記憶かつ未圧縮の体験を最大10件抽出)
            let batch_size = 10;
            let target_indices: Vec<usize> = new_soul
                .experience_buffer
                .iter()
                .enumerate()
                .filter(|(_, e)| !e.is_core_memory)
                .map(|(i, _)| i)
                .take(batch_size)
                .collect();

            if target_indices.is_empty() {
                return Ok(new_soul);
            }

            let mut target_experiences = Vec::new();
            for &idx in &target_indices {
                target_experiences.push(new_soul.experience_buffer[idx].clone());
            }
            let experiences_json = serde_json::to_string(&target_experiences).unwrap_or_default();

            // Phase 3: Compress (Semantic Summary 生成)
            let compress_prompt = format!(
                "Analyze the following recent experiences of an AI agent and compress them into a single 'SemanticSummary'.\n\
                 Experiences:\n{}\n\
                 Output a JSON object with:\n\
                 - \"topic\": A short label for this group of memories\n\
                 - \"compressed_insight\": A distilled lesson or observation\n\
                 - \"valence_avg\": Average emotional valence (float -1.0 to 1.0)\n\
                 Output ONLY JSON.",
                experiences_json
            );

            match tokio::time::timeout(
                tokio::time::Duration::from_secs(30),
                self.provider.complete(&compress_prompt, None),
            )
            .await
            {
                Ok(Ok(resp)) => {
                    let cleaned = resp
                        .content
                        .replace("```json", "")
                        .replace("```", "")
                        .trim()
                        .to_string();
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&cleaned) {
                        let summary = soul::model::SemanticSummary {
                            topic: val
                                .get("topic")
                                .and_then(|v| v.as_str())
                                .unwrap_or("General")
                                .to_string(),
                            compressed_insight: val
                                .get("compressed_insight")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            original_experience_ids: target_experiences
                                .iter()
                                .map(|e| e.id.clone())
                                .collect(),
                            valence_avg: val.get("valence_avg").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            created_at: chrono::Utc::now().to_rfc3339(),
                            embedding: vec![],
                        };
                        new_soul.semantic_index.push(summary);

                        // Phase 4: Update (原子的な削除による競合回避)
                        let ids_to_remove: std::collections::HashSet<String> =
                            target_experiences.iter().map(|e| e.id.clone()).collect();
                        new_soul
                            .experience_buffer
                            .retain(|e| !ids_to_remove.contains(&e.id));

                        tracing::info!(
                            "   [SamsaraEngine] Consolidation successful. {} experiences compressed into summary.",
                            target_experiences.len()
                        );
                    }
                }
                _ => tracing::warn!("   [SamsaraEngine] Consolidation LLM failed or timed out."),
            }

            Ok(new_soul)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::error::AiomeError;
    use aiome_core::llm_provider::{LlmProvider, LlmResponse};
    use async_trait::async_trait;
    use soul::model::{AgentSoul, Experience};

    #[derive(Debug)]
    struct MockLlm {
        response_content: String,
        should_fail: bool,
    }

    #[async_trait]
    impl LlmProvider for MockLlm {
        fn name(&self) -> &str {
            "MockLlm"
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            if self.should_fail {
                Err(AiomeError::Infrastructure {
                    reason: "mock failure".into(),
                })
            } else {
                Ok(LlmResponse {
                    content: self.response_content.clone(),
                    stop_reason: aiome_contracts::StopReason::EndTurn,
                    reasoning: None,
                    metadata: None,
                })
            }
        }
    }

    #[tokio::test]
    async fn test_rebirth_narrative_generation() {
        let mock_llm = Arc::new(MockLlm {
            response_content: "I am a synthesized narrative.".into(),
            should_fail: false,
        });

        let engine = DefaultSamsaraEngine::new(mock_llm, "mock_distill".into());
        let mut soul = AgentSoul::new("test-soul".into());
        soul.experience_buffer.push(Experience::default());

        // Initial narrative is None
        assert_eq!(soul.anamnesis.narrative_self, None);

        let new_soul = engine.rebirth(soul).await.unwrap();
        assert_eq!(
            new_soul.anamnesis.narrative_self,
            Some("I am a synthesized narrative.".into())
        );
    }

    #[tokio::test]
    async fn test_rebirth_narrative_fallback() {
        let mock_llm = Arc::new(MockLlm {
            response_content: "Will fail".into(),
            should_fail: true,
        });

        let engine = DefaultSamsaraEngine::new(mock_llm, "mock_distill".into());
        let mut soul = AgentSoul::new("test-soul".into());
        soul.anamnesis.narrative_self = Some("Old narrative.".into());
        soul.experience_buffer.push(Experience::default());

        let new_soul = engine.rebirth(soul).await.unwrap();

        // Should fallback to old narrative because LLM failed
        assert_eq!(
            new_soul.anamnesis.narrative_self,
            Some("Old narrative.".into())
        );
    }
}
