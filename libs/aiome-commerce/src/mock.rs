/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::commerce::CommerceEngine;
use aiome_core::error::AiomeError;
use aiome_core_contracts::commerce::EscrowRecord;
use async_trait::async_trait;
use tracing::info;
use uuid::Uuid;

use dashmap::DashMap;
use std::sync::Arc;

/// OSS 版向けのモック経済エンジン
#[cfg(any(test, debug_assertions))]
#[derive(Clone, Default)]
pub struct MockCommerceEngine {
    /// エージェント別の残高
    balances: Arc<DashMap<Uuid, u64>>,
    /// エスクロー（保留中）の金額 (sender_id, amount)
    escrows: Arc<DashMap<String, (Uuid, u64)>>,
}

#[cfg(any(test, debug_assertions))]
impl MockCommerceEngine {
    /// 新規モックエンジンを生成する
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(any(test, debug_assertions))]
#[async_trait]
impl CommerceEngine for MockCommerceEngine {
    async fn get_balance(&self, agent_id: Uuid) -> Result<u64, AiomeError> {
        Ok(*self.balances.entry(agent_id).or_insert(1000))
    }

    async fn validate_activity(
        &self,
        agent_id: Uuid,
        _activity_type: &str,
        amount: u64,
    ) -> Result<(), AiomeError> {
        let balance = self.get_balance(agent_id).await?;
        if balance < amount {
            return Err(AiomeError::Infrastructure {
                reason: "Insufficient funds".into(),
            });
        }
        Ok(())
    }

    async fn execute_autonomous_purchase(
        &self,
        agent_id: Uuid,
        _item_id: Uuid,
        _metadata: serde_json::Value,
    ) -> Result<String, AiomeError> {
        // Simple purchase: just deduct 10 units for demo
        let mut balance = self.balances.entry(agent_id).or_insert(1000);
        if *balance >= 10 {
            *balance -= 10;
            Ok(Uuid::new_v4().to_string())
        } else {
            Err(AiomeError::Infrastructure {
                reason: "Insufficient funds".into(),
            })
        }
    }

    async fn get_daily_spend(&self, _agent_id: Uuid) -> Result<u64, AiomeError> {
        Ok(0)
    }

    async fn get_daily_limit(&self, _agent_id: Uuid) -> Result<u64, AiomeError> {
        Ok(1000)
    }

    async fn escrow_create(&self, agent_id: Uuid, amount: u64) -> Result<String, AiomeError> {
        let mut balance = self.balances.entry(agent_id).or_insert(1000);
        if *balance >= amount {
            *balance -= amount;
            let escrow_id = format!("escrow-{}", Uuid::new_v4());
            self.escrows.insert(escrow_id.clone(), (agent_id, amount));
            Ok(escrow_id)
        } else {
            Err(AiomeError::Infrastructure {
                reason: "Insufficient funds for escrow".into(),
            })
        }
    }

    async fn list_escrows(&self, agent_id: Uuid) -> Result<Vec<EscrowRecord>, AiomeError> {
        let mut records = Vec::new();
        for entry in self.escrows.iter() {
            let escrow_id = entry.key();
            let (sender_id, amount) = entry.value();
            if sender_id == &agent_id {
                records.push(EscrowRecord {
                    id: escrow_id.clone(),
                    payer_id: sender_id.to_string(),
                    order_id: "mock_order".to_string(),
                    amount: *amount as i64,
                    status: "Locked".to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                });
            }
        }
        Ok(records)
    }

    async fn escrow_release(&self, escrow_id: &str, recipient_id: Uuid) -> Result<(), AiomeError> {
        if let Some((_, (_, amount))) = self.escrows.remove(escrow_id) {
            let mut balance = self.balances.entry(recipient_id).or_insert(1000);
            *balance += amount;
            Ok(())
        } else {
            Err(AiomeError::Infrastructure {
                reason: "Escrow not found".into(),
            })
        }
    }

    async fn escrow_refund(&self, escrow_id: &str) -> Result<(), AiomeError> {
        if let Some((_, (sender_id, amount))) = self.escrows.remove(escrow_id) {
            let mut balance = self.balances.entry(sender_id).or_insert(1000);
            *balance += amount;
            Ok(())
        } else {
            Err(AiomeError::Infrastructure {
                reason: "Escrow for refund not found".into(),
            })
        }
    }

    async fn stake(&self, agent_id: Uuid, amount: u64) -> Result<(), AiomeError> {
        let mut balance = self.balances.entry(agent_id).or_insert(1000);
        if *balance >= amount {
            *balance -= amount;
            Ok(())
        } else {
            Err(AiomeError::Infrastructure {
                reason: "Insufficient funds for staking".into(),
            })
        }
    }

    async fn slash(&self, agent_id: Uuid, amount: u64, _reason: &str) -> Result<(), AiomeError> {
        // Slashed funds are gone
        let mut balance = self.balances.entry(agent_id).or_insert(1000);
        *balance = balance.saturating_sub(amount);
        Ok(())
    }

    async fn register_license(
        &self,
        _agent_id: Uuid,
        asset_id: Uuid,
        _transaction_id: &str,
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

    async fn create_subscription(
        &self,
        _agent_id: Uuid,
        _plan_id: &str,
    ) -> Result<String, AiomeError> {
        Ok(format!("sub_{}", Uuid::new_v4()))
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
    ) -> Result<aiome_core::commerce::SubscriptionStatus, AiomeError> {
        Ok(aiome_core::commerce::SubscriptionStatus::Active)
    }

    async fn transfer(
        &self,
        from_id: Uuid,
        to_id: Uuid,
        amount: u64,
    ) -> Result<String, AiomeError> {
        let mut from_balance = self.balances.entry(from_id).or_insert(1000);
        if *from_balance >= amount {
            *from_balance -= amount;
            drop(from_balance); // Release lock before next entry

            let mut to_balance = self.balances.entry(to_id).or_insert(1000);
            *to_balance += amount;

            Ok(Uuid::new_v4().to_string())
        } else {
            Err(AiomeError::Infrastructure {
                reason: "Insufficient funds for transfer".into(),
            })
        }
    }

    async fn deduct_generation_cost(
        &self,
        agent_id: Uuid,
        _asset_id: Option<Uuid>,
        amount: u64,
        generation_type: &str,
    ) -> Result<(), AiomeError> {
        let mut balance = self.balances.entry(agent_id).or_insert(1000);
        if *balance >= amount {
            *balance -= amount;
            info!(
                "💳 [MockCommerceEngine] Deducted {} for '{}'. Remaining: {}",
                amount, generation_type, *balance
            );
            Ok(())
        } else {
            Err(AiomeError::Infrastructure {
                reason: format!(
                    "Insufficient funds for {} generation. Needed: {}, Have: {}",
                    generation_type, amount, *balance
                ),
            })
        }
    }

    async fn instant_refund(
        &self,
        _transaction_id: &str,
        _agent_id: Uuid,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn withdraw_points(&self, _agent_id: Uuid, _amount: u64) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn get_points(
        &self,
        _agent_id: Uuid,
    ) -> Result<aiome_core_contracts::commerce::PointsBalance, AiomeError> {
        Ok(aiome_core_contracts::commerce::PointsBalance {
            balance: 0,
            lifetime_earned: 0,
            lifetime_withdrawn: 0,
            conversion_rate_bps: 10000,
        })
    }

    async fn get_transaction_history(
        &self,
        _agent_id: Uuid,
        _limit: u32,
    ) -> Result<Vec<aiome_core_contracts::commerce::TransactionRecord>, AiomeError> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_escrow_lifecycle() {
        let engine = MockCommerceEngine::new();
        let agent_id = Uuid::new_v4();
        let amount = 100;

        // 1. Create escrow
        assert_eq!(engine.get_balance(agent_id).await.unwrap(), 1000); // allow-anti-pattern
        let escrow_id = engine.escrow_create(agent_id, amount).await.unwrap(); // allow-anti-pattern
        assert!(escrow_id.starts_with("escrow-"));
        assert_eq!(engine.get_balance(agent_id).await.unwrap(), 900); // allow-anti-pattern

        // 2. Refund escrow (Expected to return balance to 1000)
        let refund_result = engine.escrow_refund(&escrow_id).await;
        assert!(refund_result.is_ok());

        // This will FAIL right now because escrow_refund doesn't restore balance
        assert_eq!(
            engine.get_balance(agent_id).await.unwrap(),
            1000,
            "Balance should be refunded"
        );
    }

    #[tokio::test]
    async fn test_mock_subscription_lifecycle() {
        let engine = MockCommerceEngine::new();
        let agent_id = Uuid::new_v4();
        let plan_id = "premium_monthly";

        let sub_id = engine.create_subscription(agent_id, plan_id).await.unwrap(); // allow-anti-pattern
        assert!(!sub_id.is_empty());

        let status = engine.get_subscription_status(agent_id).await.unwrap(); // allow-anti-pattern
        assert_eq!(status, aiome_core::commerce::SubscriptionStatus::Active);

        engine.cancel_subscription(agent_id, &sub_id).await.unwrap(); // allow-anti-pattern
    }

    #[tokio::test]
    async fn test_mock_transfer_logic() {
        let engine = MockCommerceEngine::new();
        let from_id = Uuid::new_v4();
        let to_id = Uuid::new_v4();
        let amount = 300;

        // 初期残高は 1000
        engine.transfer(from_id, to_id, amount).await.unwrap(); // allow-anti-pattern

        assert_eq!(engine.get_balance(from_id).await.unwrap(), 700); // allow-anti-pattern
        assert_eq!(engine.get_balance(to_id).await.unwrap(), 1300); // allow-anti-pattern
    }

    #[tokio::test]
    async fn test_mock_transfer_insufficient_funds() {
        let engine = MockCommerceEngine::new();
        let from_id = Uuid::new_v4();
        let to_id = Uuid::new_v4();
        let amount = 1500; // 初期 1000 より多い

        let result = engine.transfer(from_id, to_id, amount).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_escrow_flow_detailed() {
        let engine = MockCommerceEngine::new();
        let sender_id = Uuid::new_v4();
        let recipient_id = Uuid::new_v4();
        let amount = 200;

        // 1. エスクロー作成 (sender: 1000 -> 800)
        let escrow_id = engine.escrow_create(sender_id, amount).await.unwrap(); // allow-anti-pattern
        assert_eq!(engine.get_balance(sender_id).await.unwrap(), 800); // allow-anti-pattern

        // 2. エスクロー解放 (recipient: 1000 -> 1200)
        engine
            .escrow_release(&escrow_id, recipient_id)
            .await
            .unwrap(); // allow-anti-pattern
        assert_eq!(engine.get_balance(recipient_id).await.unwrap(), 1200); // allow-anti-pattern
    }

    #[tokio::test]
    async fn test_mock_deduct_generation_cost() {
        let engine = MockCommerceEngine::new();
        let agent_id = Uuid::new_v4();

        // 生成コストの天引き前: 1000
        assert_eq!(engine.get_balance(agent_id).await.unwrap(), 1000); // allow-anti-pattern

        // 50コイン天引きする (GenerativeEngine使用時などを想定)
        engine
            .deduct_generation_cost(agent_id, None, 50, "image_generation")
            .await
            .unwrap(); // allow-anti-pattern

        // 天引き後: 950
        assert_eq!(engine.get_balance(agent_id).await.unwrap(), 950); // allow-anti-pattern

        // 残高不足エラーの確認
        let result = engine
            .deduct_generation_cost(agent_id, None, 2000, "video_generation")
            .await;
        assert!(result.is_err());
    }
}
