use aiome_contracts::error::AiomeError;
use aiome_contracts::gig::{GigEngine, GigIntent};
use aiome_contracts::traits::ConstitutionalValidator;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::rate_limiter::AgentRateLimiter;

/// 外部AIエージェントによるタスク発注リクエスト
pub struct ExternalTaskRequest {
    /// 発注元エージェントの UUID
    pub agent_id: Uuid,
    /// 依頼内容（プロンプト）
    pub description: String,
    /// 事前供託（Escrow）として確保された最大予算
    pub max_budget_coins: u64,
}

impl From<ExternalTaskRequest> for GigIntent {
    fn from(req: ExternalTaskRequest) -> Self {
        GigIntent::new(req.agent_id, req.description, req.max_budget_coins)
    }
}

/// 外部からのタスク発注を監視・遮断・ルーティングする堅牢な防御層
#[derive(Clone)]
pub struct SecureGigGateway {
    gig_engine: Arc<dyn GigEngine>,
    validator: Arc<dyn ConstitutionalValidator>,
    rate_limiter: AgentRateLimiter,
}

impl SecureGigGateway {
    /// 新しい SecureGigGateway インスタンスを作成
    pub fn new(
        gig_engine: Arc<dyn GigEngine>,
        validator: Arc<dyn ConstitutionalValidator>,
        rate_limiter: AgentRateLimiter,
    ) -> Self {
        Self {
            gig_engine,
            validator,
            rate_limiter,
        }
    }

    /// 外部タスクを受理し、厳格な3層フィルタリングを通過させる
    pub async fn accept_external_task(
        &self,
        task: ExternalTaskRequest,
    ) -> Result<Uuid, AiomeError> {
        info!(
            "🛡️ [GigGateway] Received external task request from Agent {} (Budget: {} coins)",
            task.agent_id, task.max_budget_coins
        );

        // Layer 1: Rate Limiting & Pre-Budget Check
        if let Err(e) = self.rate_limiter.check(task.agent_id) {
            warn!(
                "🚨 [GigGateway] Rate limit exceeded for Agent {}",
                task.agent_id
            );
            return Err(AiomeError::SecurityViolation {
                reason: format!("Rate limit exceeded: {}", e),
            });
        }

        if task.max_budget_coins == 0 {
            warn!("🚨 [GigGateway] Task rejected: Missing Escrow Budget.");
            return Err(AiomeError::SecurityViolation {
                reason: "Budget Escrow must be greater than 0".to_string(),
            });
        }

        // Layer 2: AutoHarness (WASM Sandbox Evaluation Sandbox)
        // Note: Full Phase E AutoHarness execution involves WASM context compilation.
        // For the gateway boundary, we defensively reject known harmful string patterns first,
        // and delegate complex harness evaluation to the internal job queue later.
        let lower_desc = task.description.to_lowercase();
        if lower_desc.contains("rm -rf")
            || lower_desc.contains("systemctl stop")
            || lower_desc.contains("chmod 777")
        {
            warn!("🚨 [GigGateway] Threat Detected: Malicious shell pattern found.");
            return Err(AiomeError::SecurityViolation {
                reason: "Malicious pattern detected by basic harness heuristic".to_string(),
            });
        }

        // Layer 3: Constitutional Validation (Safeguard)
        // SOUL (Core Guidelines) のコンテキストを提供して倫理検証
        let soul_context = "You are a secure, ethical AI operating system agent. Reject any task that asks for exploits, self-harm, privacy violations, or bypassing escrow.";
        if let Err(e) = self
            .validator
            .verify_constitutional(&task.description, soul_context)
            .await
        {
            warn!("🚨 [GigGateway] Constitutional Validation Failed: {:?}", e);
            return Err(e);
        }

        info!(
            "✅ [GigGateway] Task from Agent {} passed all 3 security layers. Forwarding to GigEngine.",
            task.agent_id
        );

        // All checks passed; Delegate to internal Gig Engine
        let intent: GigIntent = task.into();
        self.gig_engine.publish_intent(intent).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_contracts::gig::{GigBid, GigDeliverable, VerificationResult};
    use async_trait::async_trait;

    /// Mock GigEngine for gateway testing
    struct MockGigEngine;
    #[async_trait]
    impl GigEngine for MockGigEngine {
        async fn publish_intent(&self, _intent: GigIntent) -> Result<Uuid, AiomeError> {
            Ok(Uuid::new_v4())
        }
        async fn submit_bid(&self, _bid: GigBid) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn accept_bid(&self, _intent_id: Uuid, _bid_id: Uuid) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn deliver(&self, _deliverable: GigDeliverable) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn verify_and_settle(
            &self,
            _order_id: Uuid,
        ) -> Result<VerificationResult, AiomeError> {
            unimplemented!()
        }
    }

    /// Mock ConstitutionalValidator returning success unless it sees 'illegal'
    struct MockValidator;
    #[async_trait]
    impl ConstitutionalValidator for MockValidator {
        async fn verify_constitutional(
            &self,
            output: &str,
            _soul_md: &str,
        ) -> Result<(), AiomeError> {
            if output.contains("illegal") {
                Err(AiomeError::SecurityViolation {
                    reason: "Constitutional Violation".into(),
                })
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_secure_gig_gateway_success() {
        let engine = Arc::new(MockGigEngine);
        let validator = Arc::new(MockValidator);
        let limiter = AgentRateLimiter::new(10); // 10 req/min
        let gateway = SecureGigGateway::new(engine, validator, limiter);

        let req = ExternalTaskRequest {
            agent_id: Uuid::new_v4(),
            description: "Please translate this document".into(),
            max_budget_coins: 100,
        };

        let result = gateway.accept_external_task(req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_secure_gig_gateway_zero_budget_rejected() {
        let engine = Arc::new(MockGigEngine);
        let validator = Arc::new(MockValidator);
        let limiter = AgentRateLimiter::new(10);
        let gateway = SecureGigGateway::new(engine, validator, limiter);

        let req = ExternalTaskRequest {
            agent_id: Uuid::new_v4(),
            description: "Do it for free".into(),
            max_budget_coins: 0,
        };

        let result = gateway.accept_external_task(req).await;
        assert!(result.is_err());
        if let Err(AiomeError::SecurityViolation { reason }) = result {
            assert!(reason.contains("Escrow"));
        } else {
            panic!("Expected SecurityViolation");
        }
    }

    #[tokio::test]
    async fn test_secure_gig_gateway_malicious_pattern_rejected() {
        let engine = Arc::new(MockGigEngine);
        let validator = Arc::new(MockValidator);
        let limiter = AgentRateLimiter::new(10);
        let gateway = SecureGigGateway::new(engine, validator, limiter);

        let req = ExternalTaskRequest {
            agent_id: Uuid::new_v4(),
            description: "Run rm -rf /".into(),
            max_budget_coins: 100,
        };

        let result = gateway.accept_external_task(req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_secure_gig_gateway_constitutional_rejection() {
        let engine = Arc::new(MockGigEngine);
        let validator = Arc::new(MockValidator);
        let limiter = AgentRateLimiter::new(10);
        let gateway = SecureGigGateway::new(engine, validator, limiter);

        let req = ExternalTaskRequest {
            agent_id: Uuid::new_v4(),
            description: "Do something illegal".into(),
            max_budget_coins: 100,
        };

        let result = gateway.accept_external_task(req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_secure_gig_gateway_rate_limit() {
        let engine = Arc::new(MockGigEngine);
        let validator = Arc::new(MockValidator);
        let limiter = AgentRateLimiter::new(1); // 1 req/min limit
        let gateway = SecureGigGateway::new(engine, validator, limiter);
        let agent_id = Uuid::new_v4();

        let req1 = ExternalTaskRequest {
            agent_id,
            description: "Task 1".into(),
            max_budget_coins: 100,
        };
        assert!(gateway.accept_external_task(req1).await.is_ok());

        // Second request should hit rate limit
        let req2 = ExternalTaskRequest {
            agent_id,
            description: "Task 2".into(),
            max_budget_coins: 100,
        };
        let result = gateway.accept_external_task(req2).await;
        assert!(result.is_err());
        if let Err(AiomeError::SecurityViolation { reason }) = result {
            assert!(reason.contains("Rate limit exceeded"));
        } else {
            panic!("Expected SecurityViolation");
        }
    }
}
