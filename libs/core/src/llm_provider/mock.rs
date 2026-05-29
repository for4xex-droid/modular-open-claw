/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AiomeError;
use async_trait::async_trait;
use std::pin::Pin;

pub use aiome_core_contracts::llm::{LlmProvider, LlmResponse};

/// Infrastructure テスト用のモックLLM
#[cfg(any(test, debug_assertions))]
#[derive(Debug, Clone, Default)]
pub struct MockLlmProvider {
    /// Mock response content.
    pub response: String,
    /// Force the mock to return an error.
    pub should_fail: bool,
}

#[cfg(any(test, debug_assertions))]
#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete(
        &self,
        _prompt: &str,
        _system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        if self.should_fail {
            return Err(AiomeError::Infrastructure {
                reason: "Mock failure".into(),
            });
        }
        Ok(LlmResponse {
            content: if self.response.is_empty() {
                "{\"winner\": \"Skill A\", \"reasoning\": \"Mock victory\"}".to_string()
            } else {
                self.response.clone()
            },
            ..Default::default()
        })
    }
    async fn stream_complete(
        &self,
        _prompt: &str,
        _system: Option<&str>,
    ) -> Result<
        Pin<Box<dyn tokio_stream::Stream<Item = Result<String, AiomeError>> + Send>>,
        AiomeError,
    > {
        if self.should_fail {
            return Err(AiomeError::Infrastructure {
                reason: "Mock failure".into(),
            });
        }
        let response = if self.response.is_empty() {
            "{\"winner\": \"Skill A\", \"reasoning\": \"Mock victory\"}".to_string()
        } else {
            self.response.clone()
        };

        let stream = tokio_stream::iter(vec![Ok(response)]);
        Ok(Box::pin(stream))
    }
    async fn test_connection(&self) -> Result<(), AiomeError> {
        Ok(())
    }
    fn name(&self) -> &str {
        "MockLLM"
    }
}
