/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::job_queue::UniversalJobQueue;
use aiome_core::error::AiomeError;
use sqlx::Row;
use std::sync::Arc;
use tracing::{info, warn};

/// コスト集計結果
pub struct CostStatus {
    /// 過去24時間の合計使用額 (USD)
    pub total_usd_24h: f64,
    /// 利用制限額 (USD)
    pub limit_usd: f64,
    /// 制限に達したかどうか (サーキットブレーカーの状態)
    pub is_tripped: bool,
}

/// コストに基づくサーキットブレーカー
pub struct CostCircuitBreaker {
    jq: Arc<UniversalJobQueue>,
    /// 24時間あたりのデフォルトコスト上限 (USD)
    default_limit_usd: f64,
}

impl CostCircuitBreaker {
    /// CostCircuitBreaker の新規インスタンスを生成する
    pub fn new(jq: Arc<UniversalJobQueue>, default_limit_usd: f64) -> Self {
        Self {
            jq,
            default_limit_usd,
        }
    }

    /// 現在のコスト状態を確認
    pub async fn check_state(&self) -> Result<CostStatus, AiomeError> {
        // 設定からカスタム上限を取得（なければデフォルト）
        let limit_usd = self
            .jq
            .get_setting_value("cost_limit_24h")
            .await
            .ok()
            .flatten()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(self.default_limit_usd);

        // 過去24時間の累計コストを集計
        let res_opt = match &self.jq.get_pool() {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query("SELECT SUM(estimated_cost_usd) FROM resource_usage_logs WHERE created_at > datetime('now', '-1 day')")
                .fetch_one(p)
                .await
                .map(|row| row.get::<Option<f64>, _>(0)),
            crate::db::DatabasePool::Postgres(p) => sqlx::query("SELECT SUM(estimated_cost_usd) FROM resource_usage_logs WHERE created_at > NOW() - INTERVAL '1 day'")
                .fetch_one(p)
                .await
                .map(|row| row.get::<Option<f64>, _>(0)),
        };

        let total_usd: f64 =
            res_opt
                .map(|opt| opt.unwrap_or(0.0))
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to aggregate costs: {}", e),
                })?;

        let is_tripped = total_usd >= limit_usd;

        // バイパススイッチ（手動拡張）の確認
        let bypass_amount = self
            .jq
            .get_setting_value("cost_bypass_amount")
            .await
            .ok()
            .flatten()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let final_tripped = if bypass_amount > 0.0 {
            total_usd >= (limit_usd + bypass_amount)
        } else {
            is_tripped
        };

        if final_tripped {
            warn!(
                "🚨 [CostCircuitBreaker] TRIP! 24h cost ${:.4} exceeds limit ${:.4}",
                total_usd,
                limit_usd + bypass_amount
            );
        }

        Ok(CostStatus {
            total_usd_24h: total_usd,
            limit_usd: limit_usd + bypass_amount,
            is_tripped: final_tripped,
        })
    }

    /// 実行前にチェックし、エラーを投げる
    pub async fn enforce(&self) -> Result<(), AiomeError> {
        let status = self.check_state().await?;
        if status.is_tripped {
            // AS-1.8: (Future) Emit SSE event for UI notification when budget is exceeded.
            // This will be integrated with the api-server's event_sender in Phase 30.
            return Err(AiomeError::Infrastructure {
                reason: format!("Cost limit exceeded: 24h usage ${:.4} >= limit ${:.4}. Please expand quota in settings.", 
                                 status.total_usd_24h, status.limit_usd),
            });
        }
        Ok(())
    }

    /// セッション単位のコスト制限を確認 (GAP-6)
    pub async fn enforce_session_limit(
        &self,
        session_id: &str,
        current_session_cost: f64,
    ) -> Result<(), AiomeError> {
        let limit = self
            .jq
            .get_setting_value("cost_limit_per_session")
            .await
            .ok()
            .flatten()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.50); // デフォルト $0.50

        if current_session_cost >= limit {
            warn!(
                "🚨 [CostCircuitBreaker] Session {} cost ${:.4} exceeds limit ${:.4}",
                session_id, current_session_cost, limit
            );
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Session cost limit exceeded: ${:.4} >= ${:.4}",
                    current_session_cost, limit
                ),
            });
        }
        Ok(())
    }
}

/// 特定のトランザクションを保護するためのバイパススイッチ
pub struct CostBypassSwitch {
    jq: Arc<UniversalJobQueue>,
}

impl CostBypassSwitch {
    /// CostBypassSwitch の新規インスタンスを生成する
    pub fn new(jq: Arc<UniversalJobQueue>) -> Self {
        Self { jq }
    }

    /// 24時間有効なクォータ一時拡張
    pub async fn expand_quota(&self, amount_usd: f64) -> Result<(), AiomeError> {
        let current = self
            .jq
            .get_setting_value("cost_bypass_amount")
            .await
            .ok()
            .flatten()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        self.jq
            .update_setting(
                "cost_bypass_amount",
                &(current + amount_usd).to_string(),
                "system",
                false,
            )
            .await?;
        info!(
            "✅ [CostBypassSwitch] Quota expanded by ${:.2}.",
            amount_usd
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_queue::UniversalJobQueue;

    #[tokio::test]
    async fn test_session_cost_limit_green() {
        let jq = Arc::new(
            UniversalJobQueue::new("sqlite::memory:", None)
                .await
                .unwrap(),
        );
        let breaker = CostCircuitBreaker::new(jq.clone(), 10.0);

        // 通常範囲内 ($0.10) は OK
        let result = breaker.enforce_session_limit("session_123", 0.1).await;
        assert!(result.is_ok());

        // デフォルト制限 ($0.50) を超えるとエラー
        let result = breaker.enforce_session_limit("session_456", 0.6).await;
        assert!(result.is_err());
    }
}
