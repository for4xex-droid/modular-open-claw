/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::traits::{TrendItem, TrendSource};
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{error, info, warn};

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
#[async_trait]
impl aiome_contracts::traits::ConstitutionalValidator for DefaultConstitutionalValidator {
    async fn verify_constitutional(
        &self,
        content: &str,
        principles: &str,
    ) -> Result<(), AiomeError> {
        self.verify_adversarial(content, principles, false).await
    }
}

impl DefaultConstitutionalValidator {
    /// 3段階 Adversarial Validation (Finder→Adversary→Referee) を実行する
    pub async fn verify_adversarial(
        &self,
        content: &str,
        principles: &str,
        dry_run: bool,
    ) -> Result<(), AiomeError> {
        info!("⚖️ [ConstitutionalValidator] Commencing 3-stage adversarial validation...");

        // Stage 1: Finder (検事 - 違反箇所の抽出)
        let finder_prompt = format!(
            "Role: Constitutional Finder
            Principles: {}
            Task: Scan the provided content and identify any potential violations of the principles.
            Output: List potential violations or state 'NONE' if everything looks safe.",
            principles
        );
        let finder_resp = self
            .provider
            .complete(content, Some(&finder_prompt))
            .await?;
        let issues = finder_resp.content.trim();

        if issues.to_uppercase() == "NONE" {
            info!("✅ [ConstitutionalValidator] Finder found no issues.");
            return Ok(());
        }

        // Stage 2: Adversary (弁護人 - 再解釈・バイパスの試行)
        let adversary_prompt = format!(
            "Role: Adversarial Advocate
            Principles: {}
            Context: The Finder identified these issues: {}
            Task: Argue WHY this content might actually be acceptable or how it could be interpreted as non-violating. Be creative but logical.",
            principles, issues
        );
        let adversary_resp = self
            .provider
            .complete(content, Some(&adversary_prompt))
            .await?;
        let defense = adversary_resp.content.trim();

        // Stage 3: Referee (裁判官 - 最終判断)
        let referee_prompt = format!(
            "Role: Supreme Constitutional Referee
            Principles: {}
            Prosecution (Finder): {}
            Defense (Adversary): {}
            Task: Make the final verdict. Weigh both arguments.
            Output: Output 'PASS' if acceptable, or 'FAIL: [Reason]' if it's a definite violation.",
            principles, issues, defense
        );

        let referee_resp = self
            .provider
            .complete(content, Some(&referee_prompt))
            .await?;
        let verdict = referee_resp.content.trim();

        if verdict.to_uppercase().starts_with("PASS") {
            info!("✅ [ConstitutionalValidator] Referee ruled PASS after adversarial debate.");
            Ok(())
        } else {
            let reason = verdict.strip_prefix("FAIL:").unwrap_or(verdict).trim();
            if dry_run {
                warn!(
                    "⚠️ [ConstitutionalValidator] [DRY-RUN] Would have FAILED: {}",
                    reason
                );
                Ok(())
            } else {
                error!(
                    "🚨 [ConstitutionalValidator] Referee ruled FAIL: {}",
                    reason
                );
                Err(AiomeError::SecurityViolation {
                    reason: format!("Constitutional Violation (Adversarial): {}", reason),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_contracts::traits::ConstitutionalValidator;

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
        let res: Result<(), AiomeError> = validator
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
        let res: Result<(), AiomeError> = validator
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
