/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::commerce::{CommerceEngine, EconomicContext};
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use tracing::info;
use uuid::Uuid;

/// OSS 版向けのモック経済エンジン
#[cfg(any(test, debug_assertions))]
pub struct MockCommerceEngine;

#[cfg(any(test, debug_assertions))]
#[async_trait]
impl CommerceEngine for MockCommerceEngine {
    async fn get_balance(&self, _agent_id: Uuid) -> Result<u64, AiomeError> {
        Ok(1000) // 常に 1000 コイン
    }

    async fn validate_activity(
        &self,
        _agent_id: Uuid,
        _activity_type: &str,
        _amount: u64,
    ) -> Result<(), AiomeError> {
        Ok(()) // 常に許可
    }

    async fn execute_autonomous_purchase(
        &self,
        _agent_id: Uuid,
        _item_id: Uuid,
        _metadata: serde_json::Value,
    ) -> Result<String, AiomeError> {
        Ok(Uuid::new_v4().to_string()) // ダミーのトランザクションID
    }

    async fn get_daily_spend(&self, _agent_id: Uuid) -> Result<u64, AiomeError> {
        Ok(0) // デフォルトで 0 支出
    }

    async fn get_daily_limit(&self, _agent_id: Uuid) -> Result<u64, AiomeError> {
        Ok(1000) // 1000 回のデフォルトリミット
    }

    async fn escrow_create(&self, _agent_id: Uuid, _amount: u64) -> Result<String, AiomeError> {
        Ok(format!("escrow-{}", Uuid::new_v4()))
    }

    async fn escrow_release(
        &self,
        _escrow_id: &str,
        _recipient_id: Uuid,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn escrow_refund(&self, _escrow_id: &str) -> Result<(), AiomeError> {
        Ok(())
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
        asset_id: Uuid,
        _license_type: &str,
    ) -> Result<String, AiomeError> {
        info!(
            "🏷️ [MockCommerceEngine] Registering license for asset {}",
            asset_id
        );
        Ok(format!("lic_{}", Uuid::new_v4()))
    }

    fn verify_signature(&self, _payload: &str, _sig_header: &str) -> Result<(), AiomeError> {
        Ok(()) // モックなので常に成功
    }

    async fn process_webhook(
        &self,
        event_id: &str,
        _event_type: &str,
        _payload: &serde_json::Value,
    ) -> Result<(), AiomeError> {
        info!(
            "💡 [MockCommerceEngine] Mock processing webhook event: {}",
            event_id
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_escrow_lifecycle() {
        let engine = MockCommerceEngine;
        let agent_id = Uuid::new_v4();
        let amount = 100;

        // 1. Create escrow
        let escrow_id = engine.escrow_create(agent_id, amount).await.unwrap();
        assert!(escrow_id.starts_with("escrow-"));

        // 2. Release escrow (Should fail to compile until trait/impl updated)
        let result = engine.escrow_release(&escrow_id, agent_id).await;
        assert!(result.is_ok());

        // 3. Refund escrow (Should fail to compile)
        let refund_result = engine.escrow_refund(&escrow_id).await;
        assert!(refund_result.is_ok());
    }
}
