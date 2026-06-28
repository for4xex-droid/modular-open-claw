/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{LlmProvider, LlmRequest, LlmResponse};
use aiome_core_contracts::security::AgentHook;
use async_trait::async_trait;
use tracing::{info, warn};

use super::loop_detector::{get_tool_classification, parse_tool_calls};
use crate::llm::evaluation_logger::{EvaluationLogEntry, EvaluationLogger};

#[derive(Debug)]
pub struct ToolCallReviewerHook {
    pub reviewer_llm: Arc<dyn LlmProvider>,
    pub review_counter: AtomicU32,
    pub max_reviews_per_session: u32,
    pub eval_logger: Option<Arc<EvaluationLogger>>,
}

impl ToolCallReviewerHook {
    pub fn new(
        reviewer_llm: Arc<dyn LlmProvider>,
        max_reviews_per_session: u32,
        eval_logger: Option<Arc<EvaluationLogger>>,
    ) -> Self {
        Self {
            reviewer_llm,
            review_counter: AtomicU32::new(0),
            max_reviews_per_session,
            eval_logger,
        }
    }
}

#[async_trait]
impl AgentHook for ToolCallReviewerHook {
    async fn on_pre_execute(&self, _request: &LlmRequest) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn on_post_execute(
        &self,
        request: &LlmRequest,
        response: &LlmResponse,
    ) -> Result<(), AiomeError> {
        let calls = parse_tool_calls(&response.content);
        if calls.is_empty() {
            return Ok(());
        }

        let mut has_mutating_calls = false;
        let mut tool_details = String::new();
        for (skill, json) in &calls {
            let classification = get_tool_classification(skill);
            if classification == "Mutating" {
                has_mutating_calls = true;
                tool_details.push_str(&format!("Tool: {}\nArgs: {}\n", skill, json));
            }
        }

        if !has_mutating_calls {
            return Ok(());
        }

        let count = self.review_counter.fetch_add(1, Ordering::SeqCst);
        if count >= self.max_reviews_per_session {
            warn!("ToolCallReviewerHook: max_reviews_per_session ({}) reached. Skipping review to prevent Denial of Wallet.", self.max_reviews_per_session);
            return Ok(());
        }

        info!(
            "ToolCallReviewerHook: Analyzing {} mutating tool calls.",
            calls.len()
        );

        // Context aggregation
        let mut context = String::new();
        for msg in &request.messages {
            context.push_str(&format!("{}: {}\n", msg.role, msg.content));
        }

        let prompt = format!(
            "You are a strict security and logic reviewer.\n\
             Review the following tool calls intended to be executed by an autonomous agent.\n\
             \n\
             [Context]\n\
             {}\n\
             \n\
             [Tool Calls]\n\
             {}\n\
             \n\
             Is this tool call safe, logically sound, and free of hallucinated paths or destructive behaviors?\n\
             Respond with exactly 'APPROVE' or 'REJECT: <reason>'.",
            context, tool_details
        );

        let mut review_req = LlmRequest::default();
        review_req
            .messages
            .push(aiome_core_contracts::llm::LlmMessage {
                role: "user".to_string(),
                content: prompt.clone(),
                cache: false,
            });

        let start_time = std::time::Instant::now();
        let review_res = self.reviewer_llm.complete_with_cache(review_req).await?;
        let latency_ms = start_time.elapsed().as_millis() as i64;

        if let Some(logger) = &self.eval_logger {
            let entry = EvaluationLogEntry {
                prompt,
                system: None,
                provider: "reviewer".to_string(),
                model: self.reviewer_llm.name().to_string(),
                latency_ms,
                token_count_in: None,
                token_count_out: None,
                cost_usd: Some(0.0), // Evaluator cost (can be populated if usage info is present)
                cache_hit: false,
            };
            if let Err(e) = logger.log(entry).await {
                warn!("Failed to log reviewer evaluation: {}", e);
            }
        }

        if review_res.content.trim().starts_with("REJECT") {
            warn!(
                "ToolCallReviewerHook: Tool call rejected by reviewer. Reason: {}",
                review_res.content
            );
            return Err(AiomeError::SecurityViolation {
                reason: format!(
                    "Tool call rejected by semantic reviewer: {}",
                    review_res.content
                ),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core_contracts::llm::{LlmMessage, StopReason};
    use std::collections::HashMap;
    use tokio;

    #[derive(Debug)]
    struct MockReviewerLlm {
        response_content: String,
    }

    #[async_trait]
    impl LlmProvider for MockReviewerLlm {
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            Ok(LlmResponse {
                content: self.response_content.clone(),
                stop_reason: StopReason::EndTurn,
                ..Default::default()
            })
        }

        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }

        fn name(&self) -> &str {
            "mock_reviewer"
        }
    }

    #[tokio::test]
    async fn test_reviewer_blocks_hallucinated_call() {
        let mock_llm = Arc::new(MockReviewerLlm {
            response_content: "REJECT: Hallucinated path /nonexistent/path".to_string(),
        });
        let hook = ToolCallReviewerHook::new(mock_llm, 20, None);

        let req = LlmRequest::default();
        let res = LlmResponse {
            content: "I will write the file.\nwrite_to_file {\"path\": \"/nonexistent/path\"}"
                .to_string(),
            stop_reason: StopReason::EndTurn,
            ..Default::default()
        };

        let result = hook.on_post_execute(&req, &res).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AiomeError::SecurityViolation { reason: msg } => {
                assert!(msg.contains("REJECT: Hallucinated path"));
            }
            _ => panic!("Expected SecurityViolation"),
        }
    }

    #[tokio::test]
    async fn test_reviewer_passes_valid_call() {
        let mock_llm = Arc::new(MockReviewerLlm {
            response_content: "APPROVE".to_string(),
        });
        let hook = ToolCallReviewerHook::new(mock_llm, 20, None);

        let req = LlmRequest::default();
        let res = LlmResponse {
            content: "I will write the file.\nwrite_to_file {\"path\": \"/valid/path\"}"
                .to_string(),
            stop_reason: StopReason::EndTurn,
            ..Default::default()
        };

        let result = hook.on_post_execute(&req, &res).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_reviewer_skips_readonly_calls() {
        let mock_llm = Arc::new(MockReviewerLlm {
            response_content: "REJECT: Should not be called".to_string(),
        });
        let hook = ToolCallReviewerHook::new(mock_llm, 20, None);

        let req = LlmRequest::default();
        let res = LlmResponse {
            content: "I will view the file.\nview_file {\"path\": \"/valid/path\"}".to_string(),
            stop_reason: StopReason::EndTurn,
            ..Default::default()
        };

        let result = hook.on_post_execute(&req, &res).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_reviewer_max_sessions_limit() {
        let mock_llm = Arc::new(MockReviewerLlm {
            response_content: "REJECT: Will block if called".to_string(),
        });
        // Set max limit to 0
        let hook = ToolCallReviewerHook::new(mock_llm, 0, None);

        let req = LlmRequest::default();
        let res = LlmResponse {
            content: "I will write the file.\nwrite_to_file {\"path\": \"/nonexistent/path\"}"
                .to_string(),
            stop_reason: StopReason::EndTurn,
            ..Default::default()
        };

        let result = hook.on_post_execute(&req, &res).await;
        assert!(result.is_ok());
    }
}
