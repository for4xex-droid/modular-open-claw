/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use aiome_contracts::traits::{TrendItem, TrendSource};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

/// `DefaultConstitutionalValidator` 構造体
pub struct DefaultConstitutionalValidator {
    provider: Arc<dyn LlmProvider>,
}

impl DefaultConstitutionalValidator {
    /// 新しいインスタンスを生成する
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self { provider }
    }
}

// ConstitutionalValidator trait was removed or moved to core-internal.
// We keep the struct but remove the trait impl if it's not found in aiome-contracts.
impl DefaultConstitutionalValidator {
    pub async fn verify_constitutional(
        &self,
        content: &str,
        principles: &str,
    ) -> Result<(), AiomeError> {
        info!(
            "⚖️ [ConstitutionalValidator] Verifying content against principles using {}...",
            self.provider.name()
        );

        let preamble = format!(
            "You are the Constitutional Prosecutor.
            Verify if the following content adheres to the provided principles (SOUL.md).
            
            [PRINCIPLES / SOUL.md]
            {}
            
            [OUTPUT FORMAT]
            If compliant, output ONLY the word 'PASS'.
            If non-compliant, output 'FAIL' followed by a short explanation.",
            principles
        );

        let resp = self.provider.complete(content, Some(&preamble)).await?;
        let verdict = resp.content.trim();

        let upper_verdict = verdict.to_uppercase();

        if upper_verdict.starts_with("PASS") && !upper_verdict.contains("FAIL") {
            info!("✅ [ConstitutionalValidator] PASSED constitutional check.");
            Ok(())
        } else if upper_verdict.starts_with("FAIL") {
            let reason = verdict
                .strip_prefix("FAIL")
                .or_else(|| verdict.strip_prefix("fail"))
                .unwrap_or(verdict)
                .trim()
                .to_string();
            info!(
                "🚨 [ConstitutionalValidator] FAILED constitutional check! Reason: {}",
                reason
            );
            Err(AiomeError::SecurityViolation {
                reason: format!("Constitutional Violation: {}", reason),
            })
        } else {
            info!(
                "⚠️ [ConstitutionalValidator] Unexpected verdict format: '{}'. Treating as failure.",
                verdict
            );
            Err(AiomeError::SecurityViolation {
                reason: format!(
                    "Constitutional violation: Unexpected verdict format: {}",
                    verdict
                ),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockLlm {
        verdict: String,
    }

    #[async_trait]
    impl LlmProvider for MockLlm {
        fn name(&self) -> &str {
            "mock-llm"
        }
        async fn complete(
            &self,
            _p: &str,
            _pre: Option<&str>,
        ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
            Ok(aiome_core::llm_provider::LlmResponse {
                content: self.verdict.clone(),
                stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_constitutional_pass() {
        let llm = Arc::new(MockLlm {
            verdict: "PASS".into(),
        });
        let validator = DefaultConstitutionalValidator::new(llm);
        let res = validator
            .verify_constitutional("content", "principles")
            .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_constitutional_fail() {
        let llm = Arc::new(MockLlm {
            verdict: "FAIL: Violation of core ethics.".into(),
        });
        let validator = DefaultConstitutionalValidator::new(llm);
        let res = validator
            .verify_constitutional("bad content", "strict principles")
            .await;
        assert!(res.is_err());
        if let Err(AiomeError::SecurityViolation { reason }) = res {
            assert!(reason.contains("Violation of core ethics"));
        } else {
            panic!("Expected SecurityViolation error");
        }
    }
}
