/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::belief_consistency_gate::*;
    use crate::slm_bridge::SlmBridge;
    use aiome_core_contracts::error::AiomeError;
    use aiome_core_contracts::llm::{LlmProvider, LlmResponse, StopReason};
    use async_trait::async_trait;
    use std::sync::Arc;

    #[derive(Debug)]
    struct MockLlm {
        response_content: String,
    }

    #[async_trait]
    impl LlmProvider for MockLlm {
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            Ok(LlmResponse {
                content: self.response_content.clone(),
                stop_reason: StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "mock"
        }
    }

    #[allow(clippy::unwrap_used)]
    #[tokio::test]
    async fn test_consistent_karma_passes() {
        let llm = Arc::new(MockLlm {
            response_content: "CONSISTENT".into(),
        });
        let beliefs = vec!["I like Python.".into()];
        let gate = BeliefConsistencyGate::new(llm, None, beliefs, None);

        let result = gate
            .check_belief_consistency("Python is a great tool.")
            .await
            .unwrap();
        assert_eq!(result, BeliefCheckResult::Consistent);
    }

    #[allow(clippy::unwrap_used)]
    #[tokio::test]
    async fn test_contradicting_karma_flagged() {
        // Note: In RED phase, this will fail because the mock implementation always returns Consistent
        let llm = Arc::new(MockLlm {
            response_content: "CONTRADICTED: User expressed strong dislike for Python.".into(),
        });
        let beliefs = vec!["I like Python.".into()];
        let gate = BeliefConsistencyGate::new(llm, None, beliefs, None);

        let result = gate
            .check_belief_consistency("Python is terrible and should never be used.")
            .await
            .unwrap();

        // This is expected to FAIL in RED phase
        if let BeliefCheckResult::Contradicted { flag } = result {
            assert!(flag.contains("CONTRADICTED"));
        } else {
            panic!("Expected Contradicted result, got {:?}", result);
        }
    }

    #[allow(clippy::unwrap_used)]
    #[tokio::test]
    async fn test_evidence_accumulation_and_revision_threshold() {
        let llm = Arc::new(MockLlm {
            response_content: "REVISION_CANDIDATE".into(),
        });
        let beliefs = vec!["I like Python.".into()];
        let config = BeliefGateConfig {
            contradiction_threshold: 0.7,
            revision_evidence_count: 2,
        };
        let gate = BeliefConsistencyGate::new(llm, None, beliefs, Some(config));

        assert!(!gate.has_sufficient_evidence_for_revision().await);

        gate.accumulate_evidence(
            "Python",
            Evidence {
                content: "User said they prefer Rust now.".into(),
                source: "chat".into(),
                timestamp: "2026-03-28".into(),
                strength: 0.8,
            },
        )
        .await;

        assert!(!gate.has_sufficient_evidence_for_revision().await);

        gate.accumulate_evidence(
            "Python",
            Evidence {
                content: "User again stated Rust is better.".into(),
                source: "chat".into(),
                timestamp: "2026-03-28".into(),
                strength: 0.9,
            },
        )
        .await;

        assert!(gate.has_sufficient_evidence_for_revision().await);
    }

    #[allow(clippy::unwrap_used)]
    #[tokio::test]
    async fn test_slm_fallback_to_llm() {
        // SLM がエラーを返す環境を模倣（無効なコマンド）
        let slm = Arc::new(SlmBridge::new_with_command("invalid-slm"));
        let llm = Arc::new(MockLlm {
            response_content: "CONTRADICTED: LLM Fallback Success".into(),
        });
        let beliefs = vec!["I like Python.".into()];

        let gate = BeliefConsistencyGate::new(llm, Some(slm), beliefs, None);

        // SLM が失敗しても LLM が判定を下すはず
        let result = gate
            .check_belief_consistency("Python is bad.")
            .await
            .unwrap();

        if let BeliefCheckResult::Contradicted { flag } = result {
            assert!(flag.contains("LLM Fallback Success"));
        } else {
            panic!("Expected Contradicted from LLM fallback, got {:?}", result);
        }
    }
}
