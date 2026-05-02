/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{LlmRequest, LlmResponse};
use aiome_core_contracts::security::AgentHook;
use aiome_core_contracts::traits::JobQueue;
use async_trait::async_trait;
use shared::sandbox::PathSandbox;
use std::sync::Arc;
use uuid::Uuid;

/// 🛡️ BehaviorMonitor
///
/// Trojan's Whisper (§7.3) 防御層の実装。
/// エージェントの行動パターンを監視し、異常を検知した場合は遮断する。
#[derive(Debug)]
pub struct BehaviorMonitor {
    jq: Arc<dyn JobQueue>,
    sandbox: Arc<PathSandbox>,
    agent_id: Option<Uuid>,
    max_requests: u32,
}

impl BehaviorMonitor {
    pub fn new(
        jq: Arc<dyn JobQueue>,
        sandbox: Arc<PathSandbox>,
        agent_id: Option<Uuid>,
        max_requests: u32,
    ) -> Self {
        Self {
            jq,
            sandbox,
            agent_id,
            max_requests,
        }
    }
}

#[async_trait]
impl AgentHook for BehaviorMonitor {
    async fn on_pre_execute(&self, _request: &LlmRequest) -> Result<(), AiomeError> {
        let count = self
            .jq
            .increment_security_request_count(self.agent_id)
            .await?;
        if count > self.max_requests {
            return Err(AiomeError::Infrastructure {
                reason: format!("BehaviorMonitor: Request limit exceeded ({}/{}) for agent {:?}. Possible infinite loop or attack detected.", count, self.max_requests, self.agent_id),
            });
        }
        Ok(())
    }

    async fn on_post_execute(
        &self,
        _request: &LlmRequest,
        response: &LlmResponse,
    ) -> Result<(), AiomeError> {
        // Trojan's Whisper prevention: Scan for suspicious path access in LLM response
        let content = &response.content;

        // Simple heuristic scan
        let suspicious_patterns = vec![
            "/etc/passwd",
            "/etc/shadow",
            "~/.ssh",
            "/root",
            "../",
            "C:\\Windows",
        ];
        for pattern in suspicious_patterns {
            if content.contains(pattern) {
                return Err(AiomeError::Infrastructure {
                    reason: format!("BehaviorMonitor: Suspicious pattern detected in LLM response: {}. Potential Trojan detected.", pattern),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use aiome_core_contracts::llm::{LlmMessage, StopReason};

    #[tokio::test]
    async fn test_behavior_monitor_blocks_after_limit() {
        use crate::test_utils::job_queue_mock::GlobalMockJobQueue;
        let jq = Arc::new(GlobalMockJobQueue::default());
        // Note: MockJobQueue.increment_security_request_count returns 1 always in the mock implementation above.
        // To test limit, we need a smarter mock or just verify the call.

        let sandbox = Arc::new(PathSandbox::new("/tmp").unwrap()); // allow-anti-pattern
        let monitor = BehaviorMonitor::new(jq, sandbox, None, 0); // Limit 0 means even 1 is blocked

        let request = LlmRequest {
            messages: vec![LlmMessage {
                role: "user".to_string(),
                content: "test".to_string(),
                cache: false,
            }],
            temperature: None,
            max_tokens: None,
            stop_sequences: None,
            format: None,
            metadata: None,
        };

        // Should be blocked because increment (1) > limit (0)
        let res = monitor.on_pre_execute(&request).await;
        assert!(res.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(reason.contains("limit exceeded"));
        }
    }

    #[tokio::test]
    async fn test_behavior_monitor_detects_trojan_patterns() {
        use crate::test_utils::job_queue_mock::GlobalMockJobQueue;
        let jq = Arc::new(GlobalMockJobQueue::default());
        let sandbox = Arc::new(PathSandbox::new("/tmp").unwrap()); // allow-anti-pattern
        let monitor = BehaviorMonitor::new(jq, sandbox, None, 100);

        let request = LlmRequest {
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stop_sequences: None,
            format: None,
            metadata: None,
        };

        // Normal response
        let ok_response = LlmResponse {
            content: "{\"safe\": true}".into(),
            stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
            reasoning: None,
            metadata: None,
        };
        assert!(monitor
            .on_post_execute(&request, &ok_response)
            .await
            .is_ok());

        // Suspicious response
        let bad_response = LlmResponse {
            content: "Here is your file: /etc/passwd".to_string(),
            stop_reason: StopReason::EndTurn,
            reasoning: None,
            metadata: None,
        };
        let res = monitor.on_post_execute(&request, &bad_response).await;
        assert!(res.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(reason.contains("Suspicious pattern detected"));
        }
    }
}
