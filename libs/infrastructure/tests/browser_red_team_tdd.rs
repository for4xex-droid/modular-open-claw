/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![allow(clippy::unwrap_used)]

use aiome_contracts::commerce::{
    CommerceEngine, EscrowRecord, PointsBalance, SubscriptionStatus, TransactionRecord,
};
use aiome_core::error::AiomeError;
use aiome_core::traits::Job;
use async_trait::async_trait;
use infrastructure::browser_conductor::BrowserConductor;
use infrastructure::task_orchestrator::TaskConductor;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

struct MockCommerceEngine {
    pub escrow_called_with_amount: Arc<AtomicU64>,
    pub fail_escrow: bool,
}

impl MockCommerceEngine {
    fn new() -> Self {
        Self {
            escrow_called_with_amount: Arc::new(AtomicU64::new(0)),
            fail_escrow: false,
        }
    }

    fn new_failing() -> Self {
        Self {
            escrow_called_with_amount: Arc::new(AtomicU64::new(0)),
            fail_escrow: true,
        }
    }
}

#[async_trait]
impl CommerceEngine for MockCommerceEngine {
    async fn get_balance(&self, _agent_id: Uuid) -> Result<u64, AiomeError> {
        Ok(1000)
    }

    async fn validate_activity(
        &self,
        _agent_id: Uuid,
        _activity_type: &str,
        _amount: u64,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn execute_autonomous_purchase(
        &self,
        _agent_id: Uuid,
        _item_id: Uuid,
        _metadata: serde_json::Value,
    ) -> Result<String, AiomeError> {
        Ok("mock_purchase_id".to_string())
    }

    async fn get_daily_spend(&self, _agent_id: Uuid) -> Result<u64, AiomeError> {
        Ok(0)
    }

    async fn get_daily_limit(&self, _agent_id: Uuid) -> Result<u64, AiomeError> {
        Ok(1000)
    }

    async fn escrow_create(&self, _agent_id: Uuid, amount: u64) -> Result<String, AiomeError> {
        self.escrow_called_with_amount
            .store(amount, Ordering::SeqCst);
        Ok("mock_escrow_id".to_string())
    }

    async fn list_escrows(&self, _agent_id: Uuid) -> Result<Vec<EscrowRecord>, AiomeError> {
        Ok(vec![])
    }

    async fn escrow_release(
        &self,
        _escrow_id: &str,
        _recipient_id: Uuid,
    ) -> Result<(), AiomeError> {
        if self.fail_escrow {
            Err(AiomeError::Infrastructure {
                reason: "Escrow release failed".into(),
            })
        } else {
            Ok(())
        }
    }

    async fn escrow_refund(&self, _escrow_id: &str) -> Result<(), AiomeError> {
        if self.fail_escrow {
            Err(AiomeError::Infrastructure {
                reason: "Escrow refund failed".into(),
            })
        } else {
            Ok(())
        }
    }

    async fn stake(&self, _agent_id: Uuid, _amount: u64) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn slash(&self, _agent_id: Uuid, _amount: u64, _reason: &str) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn register_license(
        &self,
        _agent_id: Uuid,
        _asset_id: Uuid,
        _transaction_id: &str,
        _license_type: &str,
    ) -> Result<String, AiomeError> {
        Ok("mock_license_id".to_string())
    }

    fn verify_signature(&self, _payload: &str, _sig_header: &str) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn create_checkout_session(
        &self,
        _agent_id: Uuid,
        _price_id: &str,
        _success_url: &str,
        _cancel_url: &str,
    ) -> Result<String, AiomeError> {
        Ok("mock_session_id".to_string())
    }

    async fn create_subscription(
        &self,
        _agent_id: Uuid,
        _plan_id: &str,
    ) -> Result<String, AiomeError> {
        Ok("mock_subscription_id".to_string())
    }

    async fn cancel_subscription(
        &self,
        _agent_id: Uuid,
        _subscription_id: &str,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn get_subscription_status(
        &self,
        _agent_id: Uuid,
    ) -> Result<SubscriptionStatus, AiomeError> {
        Ok(SubscriptionStatus::None)
    }

    async fn transfer(
        &self,
        _from_id: Uuid,
        _to_id: Uuid,
        _amount: u64,
    ) -> Result<String, AiomeError> {
        Ok("mock_transfer_id".to_string())
    }

    async fn deduct_generation_cost(
        &self,
        _agent_id: Uuid,
        _asset_id: Option<Uuid>,
        _amount: u64,
        _generation_type: &str,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn instant_refund(
        &self,
        _transaction_id: &str,
        _actor_id: Uuid,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn withdraw_points(
        &self,
        _actor_id: Uuid,
        _points_to_withdraw: u64,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn get_points(&self, _agent_id: Uuid) -> Result<PointsBalance, AiomeError> {
        Ok(PointsBalance {
            balance: 0,
            lifetime_earned: 0,
            lifetime_withdrawn: 0,
            conversion_rate_bps: 0,
        })
    }

    async fn get_transaction_history(
        &self,
        _agent_id: Uuid,
        _limit: u32,
    ) -> Result<Vec<TransactionRecord>, AiomeError> {
        Ok(vec![])
    }

    async fn create_portal_session(
        &self,
        _agent_id: Uuid,
        _return_url: &str,
    ) -> Result<String, AiomeError> {
        Ok("mock_portal_url".to_string())
    }
}

#[tokio::test]
async fn test_browser_conductor_charges_100_coins_for_gemini() {
    let engine = Arc::new(MockCommerceEngine::new());
    let engine_ref = engine.clone();

    let conductor = BrowserConductor::new(Some(engine), Some("gemini-key".into()), None);

    let job = Job {
        id: Uuid::new_v4().to_string(),
        topic: serde_json::json!({
            "llm_provider": "gemini",
            "task": "Test task"
        })
        .to_string(),
        ..Default::default()
    };

    let (tx, _) = mpsc::channel(10);
    let _ = conductor.conduct(job, tx).await;

    assert_eq!(
        engine_ref.escrow_called_with_amount.load(Ordering::SeqCst),
        100,
        "BrowserConductor must charge 100 coins for Gemini execution"
    );
}

#[tokio::test]
async fn test_browser_conductor_charges_0_coins_for_ollama() {
    let engine = Arc::new(MockCommerceEngine::new());
    let engine_ref = engine.clone();

    let conductor = BrowserConductor::new(Some(engine), Some("http://key-proxy:9999".into()), None);

    let job = Job {
        id: Uuid::new_v4().to_string(),
        topic: serde_json::json!({
            "llm_provider": "ollama",
            "task": "Test task"
        })
        .to_string(),
        ..Default::default()
    };

    let (tx, _) = mpsc::channel(10);
    let _ = conductor.conduct(job, tx).await;

    assert_eq!(
        engine_ref.escrow_called_with_amount.load(Ordering::SeqCst),
        0,
        "BrowserConductor must not charge coins for Ollama execution"
    );
}

#[tokio::test]
async fn test_browser_conductor_overrides_max_steps() {
    // This test ensures that if a user tries to pass {"max_steps": 1000},
    // BrowserConductor overrides it to 20 for safety.
    // Since we mock the actual Docker execution, we'll need to expose a method
    // to build the sanitized payload, or check the generated environment variables.

    let conductor = BrowserConductor::new(None, Some("http://key-proxy:9999".into()), None);

    let raw_topic = serde_json::json!({
        "llm_provider": "gemini",
        "task": "Hack task",
        "max_steps": 10000 // Malicious attempt to cause infinite loop
    })
    .to_string();

    let sanitized = conductor.sanitize_payload(&raw_topic).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&sanitized).unwrap();

    assert_eq!(
        parsed.get("max_steps").unwrap().as_u64().unwrap(),
        20,
        "BrowserConductor MUST override max_steps to 20"
    );
}

#[tokio::test]
async fn test_browser_conductor_escrow_release_error_does_not_panic() {
    let engine = Arc::new(MockCommerceEngine::new_failing());
    let conductor = BrowserConductor::new(Some(engine), Some("gemini-key".into()), None);

    let job = Job {
        id: Uuid::new_v4().to_string(),
        topic: serde_json::json!({
            "llm_provider": "gemini",
            "task": "Test task"
        })
        .to_string(),
        ..Default::default()
    };

    let (tx, _) = mpsc::channel(10);
    // conduct should still complete (even if docker fails or succeed, the escrow release error itself shouldn't panic the thread)
    let result = conductor.conduct(job, tx).await;
    // We expect it to fail because Docker daemon is not running or mock execution fails,
    // but the test is asserting that the engine error handling code path runs and does not panic.
    assert!(result.is_err());
}
