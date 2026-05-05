/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::job_queue::{CostOps, SettingsOps};
use aiome_core::error::AiomeError;
use sqlx::Row;
use std::sync::Arc;
use tracing::{info, warn};

/// コスト集計結果
pub struct CostStatus {
    /// 過去24時間の合計使用額 (USD)
    pub total_usd_24h: f64,
    /// 24時間の利用制限額 (USD)
    pub limit_usd: f64,
    /// 過去30日間の合計使用額 (USD)
    pub total_usd_30d: Option<f64>,
    /// 30日間の月次利用制限額 (USD)
    pub monthly_limit_usd: Option<f64>,
    /// 制限に達したかどうか (サーキットブレーカーの状態)
    pub is_tripped: bool,
}

/// コストに基づくサーキットブレーカー
pub struct CostCircuitBreaker {
    ops: Arc<dyn CostOps>,
    /// 24時間あたりのデフォルトコスト上限 (USD)
    default_limit_usd: f64,
}

impl CostCircuitBreaker {
    /// CostCircuitBreaker の新規インスタンスを生成する
    pub fn new(ops: Arc<dyn CostOps>, default_limit_usd: f64) -> Self {
        Self {
            ops,
            default_limit_usd,
        }
    }

    /// 現在のコスト状態を確認
    pub async fn check_state(&self) -> Result<CostStatus, AiomeError> {
        // 設定からカスタム上限を取得（なければデフォルト）
        let limit_usd = self
            .ops
            .get_setting_value("cost_limit_24h")
            .await
            .ok()
            .flatten()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(self.default_limit_usd);

        let monthly_limit_usd = self
            .ops
            .get_setting_value("cost_limit_monthly")
            .await
            .ok()
            .flatten()
            .and_then(|s| s.parse::<f64>().ok());

        // 過去24時間の累計コストを集計
        let total_usd_24h = self.ops.aggregate_cost_hours(24).await?;

        // バイパススイッチ（手動拡張）の確認
        let bypass_amount = self
            .ops
            .get_setting_value("cost_bypass_amount")
            .await
            .ok()
            .flatten()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        // 過去30日（月次ローリング）の累計コストを集計
        let mut is_monthly_tripped = false;
        let mut final_total_usd_30d = None;
        let mut final_monthly_limit = None;

        if let Some(m_limit) = monthly_limit_usd {
            let total_usd_30d = self.ops.aggregate_cost_days(30).await?;

            final_total_usd_30d = Some(total_usd_30d);
            final_monthly_limit = Some(m_limit + bypass_amount);

            if bypass_amount > 0.0 {
                if total_usd_30d >= (m_limit + bypass_amount) {
                    is_monthly_tripped = true;
                }
            } else if total_usd_30d >= m_limit {
                is_monthly_tripped = true;
            }

            if is_monthly_tripped {
                warn!(
                    "🚨 [CostCircuitBreaker] TRIP! 30d rolling monthly cost ${:.4} exceeds limit ${:.4}",
                    total_usd_30d,
                    m_limit + bypass_amount
                );
            }
        }

        let is_24h_tripped = total_usd_24h >= limit_usd;

        let final_24h_tripped = if bypass_amount > 0.0 {
            total_usd_24h >= (limit_usd + bypass_amount)
        } else {
            is_24h_tripped
        };

        if final_24h_tripped {
            warn!(
                "🚨 [CostCircuitBreaker] TRIP! 24h cost ${:.4} exceeds limit ${:.4}",
                total_usd_24h,
                limit_usd + bypass_amount
            );
        }

        Ok(CostStatus {
            total_usd_24h,
            limit_usd: limit_usd + bypass_amount,
            total_usd_30d: final_total_usd_30d,
            monthly_limit_usd: final_monthly_limit,
            is_tripped: final_24h_tripped || is_monthly_tripped,
        })
    }

    /// 実行前にチェックし、エラーを投げる
    pub async fn enforce(&self) -> Result<(), AiomeError> {
        let status = self.check_state().await?;
        if status.is_tripped {
            // AS-1.8: (Future) Emit SSE event for UI notification when budget is exceeded.
            // This will be integrated with the api-server's event_sender in Phase 30.
            return Err(AiomeError::Infrastructure {
                reason: format!("Cost limit exceeded: 24h usage ${:.4} >= limit ${:.4}. Monthly usage: ${:.4} / limit: ${:.4}. Please expand quota in settings.",
                                 status.total_usd_24h, status.limit_usd, status.total_usd_30d.unwrap_or(0.0), status.monthly_limit_usd.unwrap_or(0.0)),
            });
        }
        Ok(())
    }

    /// ジョブ単位のコスト制限を確認 (GAP-6, B-4)
    pub async fn enforce_job_limit(
        &self,
        job_id: &str,
        max_job_cost_usd: f64,
    ) -> Result<(), AiomeError> {
        let current_cost = self.ops.aggregate_cost_by_job(job_id).await?;

        if current_cost >= max_job_cost_usd {
            warn!(
                "🚨 [CostCircuitBreaker] Job {} cost ${:.4} exceeds limit ${:.4}",
                job_id, current_cost, max_job_cost_usd
            );
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Job cost limit exceeded: ${:.4} >= ${:.4}",
                    current_cost, max_job_cost_usd
                ),
            });
        }
        Ok(())
    }
}

/// 特定のトランザクションを保護するためのバイパススイッチ
pub struct CostBypassSwitch {
    jq: Arc<dyn SettingsOps>,
}

impl CostBypassSwitch {
    /// CostBypassSwitch の新規インスタンスを生成する
    pub fn new(jq: Arc<dyn SettingsOps>) -> Self {
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
    async fn test_job_cost_limit_green() {
        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = Arc::new(
            UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(),
        );

        crate::job_queue::migrations::DbInitializer::init_db(&*jq)
            .await
            .unwrap();

        let raw_pool = jq.pool.get_sqlite_pool().unwrap();

        sqlx::query("INSERT INTO jobs (id, category, topic, style_name, karma_directives, status) VALUES ('job_123', 'cat', 'topic', 'style', '{}', 'Pending')")
            .execute(raw_pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO resource_usage_logs (job_id, provider_name, model_name, usage_type, amount, estimated_cost_usd, created_at) VALUES ('job_123', 'p', 'm', 't', 1, 0.1, datetime('now'))")
            .execute(raw_pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO jobs (id, category, topic, style_name, karma_directives, status) VALUES ('job_456', 'cat', 'topic', 'style', '{}', 'Pending')")
            .execute(raw_pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO resource_usage_logs (job_id, provider_name, model_name, usage_type, amount, estimated_cost_usd, created_at) VALUES ('job_456', 'p', 'm', 't', 1, 0.6, datetime('now'))")
            .execute(raw_pool)
            .await
            .unwrap();

        let breaker = CostCircuitBreaker::new(jq.clone(), 10.0);

        // 通常範囲内 ($0.10) は OK
        let result = breaker.enforce_job_limit("job_123", 0.50).await;
        assert!(result.is_ok());

        // 指定制限 ($0.50) を超えるとエラー
        let result = breaker.enforce_job_limit("job_456", 0.50).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_monthly_cost_limit() {
        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = Arc::new(
            UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(),
        );

        // Initialize migrations to create resource_usage_logs table
        crate::job_queue::migrations::DbInitializer::init_db(&*jq)
            .await
            .unwrap();

        let pool = jq.pool.get_sqlite_pool().unwrap();

        // 0. Insert dummy jobs to satisfy foreign keys
        // (id, category, topic, style_name, karma_directives, status)
        sqlx::query("INSERT INTO jobs (id, category, topic, style_name, karma_directives, status) VALUES ('1', 'cat', 'topic', 'style', '{}', 'Pending')")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO jobs (id, category, topic, style_name, karma_directives, status) VALUES ('2', 'cat', 'topic', 'style', '{}', 'Pending')")
            .execute(pool)
            .await
            .unwrap();

        // 1. Insert a log entry from 15 days ago with a massive cost ($1000)
        sqlx::query("INSERT INTO resource_usage_logs (job_id, provider_name, model_name, usage_type, amount, estimated_cost_usd, created_at) VALUES ('1', 'p', 'm', 't', 1, 1000.0, datetime('now', '-15 days'))")
            .execute(pool)
            .await
            .unwrap();

        // 2. Insert a small recent cost ($1) so 24h limit is NOT triggered (assuming 24h limit is $10 by default)
        sqlx::query("INSERT INTO resource_usage_logs (job_id, provider_name, model_name, usage_type, amount, estimated_cost_usd, created_at) VALUES ('2', 'p', 'm', 't', 1, 1.0, datetime('now', '-1 hour'))")
            .execute(pool)
            .await
            .unwrap();

        // 3. Set the monthly limit to $500.
        jq.update_setting("cost_limit_monthly", "500.0", "system", false)
            .await
            .unwrap();

        let breaker = CostCircuitBreaker::new(jq.clone(), 10.0);

        // 4. check_state should result in an error or tripped state! (Since $1001 > $500)
        let result = breaker.check_state().await;
        assert!(result.is_ok()); // check_state won't return Err but returns CostStatus
        let status = result.unwrap();

        // RED test: This should fail initially because `is_tripped` only checks 24h ($1), while monthly is $1001. So is_tripped will be false, but our assert will expect true!
        assert!(
            status.is_tripped,
            "Breaker should trip due to monthly limit of $500, current: $1001"
        );
    }
}
