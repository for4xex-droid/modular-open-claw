/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::contracts::KarmaClassification;
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;

/// Sprint 3-B: Hierarchical Classification (Taxonomy)
/// 過去の教訓をドメインとサブトピックに自動分類する。
pub struct KarmaTaxonomy;

impl KarmaTaxonomy {
    /// 指定された教訓（lesson）をLLMを用いて分類する。
    pub async fn classify(
        provider: &dyn LlmProvider,
        lesson: &str,
    ) -> Result<KarmaClassification, AiomeError> {
        let system_prompt = r#"You are the Karma Classifier for Aiome OS.
Your task is to classify a "lesson" (karma) into a hierarchical taxonomy.

Output MUST be a strict JSON object matching this structure:
{
  "domain": "Technical | Creative | Governance | Social | Meta",
  "subtopic": "string",
  "reasoning": "string"
}

Domains:
- Technical: Code, Performance, Bugs, API, Infrastructure.
- Creative: Aesthetics, Style, Tone, Visuals.
- Governance: Security, Policy, Ethics, Compliance.
- Social: User interaction, Engagement, Empathy.
- Meta: System evolution, Learning patterns, Self-improvement.

Constraint: Output ONLY raw JSON. No markdown blocks."#;

        // VULN-62: Sanitize input to prevent prompt injection
        let max_len = 5000;
        let sanitized_lesson = if lesson.len() > max_len {
            &lesson[..max_len]
        } else {
            lesson
        };
        let sanitized_lesson = sanitized_lesson.replace('"', "'").replace('\\', " ");

        let prompt = format!("Lesson: \"{}\"", sanitized_lesson);

        match provider.complete(&prompt, Some(system_prompt)).await {
            Ok(resp) => {
                // R1 Defense: AI might still output markdown blocks
                let clean_json = resp
                    .content
                    .trim()
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();

                let mut taxonomy = serde_json::from_str::<KarmaClassification>(clean_json)
                    .map_err(|e| {
                        tracing::warn!(
                            "🧬 [Taxonomy] JSON Parse Error: {}. Raw: {}",
                            e,
                            resp.content
                        );
                        AiomeError::Infrastructure {
                            reason: format!("Invalid classification format: {}", e),
                        }
                    })?;

                // VULN-62: Strict domain whitelisting to prevent hallucinatory domains
                let valid_domains = ["Technical", "Creative", "Governance", "Social", "Meta"];
                if !valid_domains.contains(&taxonomy.domain.as_str()) {
                    tracing::warn!(
                        "🧬 [Taxonomy] Invalid domain returned from LLM: {}",
                        taxonomy.domain
                    );
                    taxonomy.domain = "General".to_string(); // Fallback
                }

                Ok(taxonomy)
            }
            Err(e) => Err(e),
        }
    }

    /// フォールバック値を生成する（LLMエラー時）
    pub fn fallback() -> KarmaClassification {
        KarmaClassification {
            domain: "general".to_string(),
            subtopic: "uncategorized".to_string(),
            reasoning: "LLM classification failed, using fallback.".to_string(),
        }
    }
}
