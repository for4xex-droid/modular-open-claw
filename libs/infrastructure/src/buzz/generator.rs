/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::templates::{build_user_prompt, get_system_prompt, BuzzTemplate};
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use std::sync::Arc;

/// A serializable draft for social media posting.
/// Contains the LLM-generated text, the template used, and engagement signal targets.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct BuzzDraft {
    pub text: String,
    pub template: BuzzTemplate,
    pub trend_source: String,
    pub estimated_signals: Vec<String>,
}

/// Generates social media drafts by delegating to an LLM provider
/// with template-specific system prompts.
pub struct BuzzContentGenerator {
    llm: Arc<dyn LlmProvider>,
}

impl BuzzContentGenerator {
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self { llm }
    }

    pub async fn generate(
        &self,
        trend_source: &str,
        template: BuzzTemplate,
        project_context: &str,
    ) -> Result<BuzzDraft, AiomeError> {
        let system_prompt = get_system_prompt(&template);
        let user_prompt = build_user_prompt(trend_source, project_context);

        let response = self
            .llm
            .complete(&user_prompt, Some(&system_prompt))
            .await?;

        Ok(BuzzDraft {
            text: response.content,
            estimated_signals: template.target_signals(),
            template,
            trend_source: trend_source.to_string(),
        })
    }
}
