/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::job_queue::UniversalJobQueue;
use crate::{sql_exec, sql_fetch_all, sql_fetch_one, sql_fetch_optional};
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use sqlx::Row;

use crate::job_queue::expression::ExpressionOps;
use aiome_core_contracts::traits::SettingsOps;

#[async_trait]
pub trait CostOps: SettingsOps {
    /// 過去 N 時間のコスト合計を取得する
    async fn aggregate_cost_hours(&self, hours: i64) -> Result<f64, AiomeError>;
    /// 過去 N 日のコスト合計を取得する
    async fn aggregate_cost_days(&self, days: i64) -> Result<f64, AiomeError>;
    /// 特定のジョブのコスト合計を取得する
    async fn aggregate_cost_by_job(&self, job_id: &str) -> Result<f64, AiomeError>;
}

#[async_trait]
impl SettingsOps for UniversalJobQueue {
    async fn do_get_setting(&self, key: &str) -> Result<Option<String>, AiomeError> {
        let q = format!(
            "SELECT value, CAST(is_secret AS INTEGER) FROM system_settings WHERE key = {}",
            self.pool.ph(0)
        );

        let opt: Option<(String, i32)> =
            crate::sql_fetch_optional!(&self.pool, (String, i32), &q, key).map_err(|e| {
                AiomeError::Infrastructure {
                    reason: format!("Get setting failed for key '{}': {}", key, e),
                }
            })?;

        match opt {
            Some((value, is_secret)) if is_secret != 0 => {
                // Transparent decryption for secret values (§CISO-1)
                match crate::security::crypto::decrypt_setting(&value) {
                    Ok(plaintext) => Ok(Some(plaintext)),
                    Err(_) => {
                        // Fallback: value may be a legacy unencrypted secret
                        tracing::warn!(
                            "⚠️ [Settings] Failed to decrypt secret '{}' — treating as plaintext (legacy migration)",
                            key
                        );
                        Ok(Some(value))
                    }
                }
            }
            Some((value, _)) => Ok(Some(value)),
            None => Ok(None),
        }
    }

    async fn do_set_setting(
        &self,
        key: &str,
        value: &str,
        category: &str,
        is_secret: bool,
    ) -> Result<(), AiomeError> {
        let q = match &self.pool {
            crate::db::DatabasePool::Sqlite(_) => format!("INSERT OR REPLACE INTO system_settings (key, value, category, is_secret, updated_at) VALUES ({0}, {1}, {2}, {3}, {4})", self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.now_fn()),
            crate::db::DatabasePool::Postgres(_) => format!("INSERT INTO system_settings (key, value, category, is_secret, updated_at) VALUES ({0}, {1}, {2}, {3}, {4}) ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, category = EXCLUDED.category, is_secret = EXCLUDED.is_secret, updated_at = EXCLUDED.updated_at", self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.now_fn()),
        };
        sql_exec!(&self.pool, &q, key, value, category, is_secret as i32).map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("Update setting failed: {}", e),
            }
        })?;
        Ok(())
    }

    async fn do_get_all_settings(
        &self,
    ) -> Result<Vec<aiome_core::contracts::SystemSetting>, AiomeError> {
        let q = "SELECT key, value, category, is_secret, updated_at FROM system_settings";
        let entries =
            match &self.pool {
                crate::db::DatabasePool::Sqlite(p) => {
                    let rows = sqlx::query(q).fetch_all(p).await.map_err(|e| {
                        AiomeError::Infrastructure {
                            reason: e.to_string(),
                        }
                    })?;
                    rows.into_iter().map(map_sqlite_row_to_setting).collect()
                }
                crate::db::DatabasePool::Postgres(p) => {
                    let rows = sqlx::query(q).fetch_all(p).await.map_err(|e| {
                        AiomeError::Infrastructure {
                            reason: e.to_string(),
                        }
                    })?;
                    rows.into_iter().map(map_postgres_row_to_setting).collect()
                }
            };
        Ok(entries)
    }

    async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
        self.do_get_auto_expression_enabled().await
    }

    async fn set_auto_expression_enabled(&self, enabled: bool) -> Result<(), AiomeError> {
        self.do_set_auto_expression_enabled(enabled).await
    }
}

#[async_trait]
impl CostOps for UniversalJobQueue {
    async fn aggregate_cost_hours(&self, hours: i64) -> Result<f64, AiomeError> {
        let hours = hours.max(0);
        let res_opt = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let sql_modifier = format!("-{} hours", hours);
                sqlx::query("SELECT SUM(estimated_cost_usd) FROM resource_usage_logs WHERE created_at > datetime('now', ?)")
                    .bind(sql_modifier)
                    .fetch_one(p)
                    .await
                    .map(|row| row.get::<Option<f64>, _>(0))
            }
            crate::db::DatabasePool::Postgres(p) => {
                let interval = format!("{} hours", hours);
                sqlx::query("SELECT SUM(estimated_cost_usd) FROM resource_usage_logs WHERE created_at > NOW() - $1::interval")
                    .bind(interval)
                    .fetch_one(p)
                    .await
                    .map(|row| row.get::<Option<f64>, _>(0))
            }
        };

        res_opt
            .map(|opt| opt.unwrap_or(0.0))
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to aggregate hours costs: {}", e),
            })
    }

    async fn aggregate_cost_days(&self, days: i64) -> Result<f64, AiomeError> {
        let days = days.max(0);
        let res_opt = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let sql_modifier = format!("-{} days", days);
                sqlx::query("SELECT SUM(estimated_cost_usd) FROM resource_usage_logs WHERE created_at > datetime('now', ?)")
                    .bind(sql_modifier)
                    .fetch_one(p)
                    .await
                    .map(|row| row.get::<Option<f64>, _>(0))
            }
            crate::db::DatabasePool::Postgres(p) => {
                let interval = format!("{} days", days);
                sqlx::query("SELECT SUM(estimated_cost_usd) FROM resource_usage_logs WHERE created_at > NOW() - $1::interval")
                    .bind(interval)
                    .fetch_one(p)
                    .await
                    .map(|row| row.get::<Option<f64>, _>(0))
            }
        };

        res_opt
            .map(|opt| opt.unwrap_or(0.0))
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to aggregate days costs: {}", e),
            })
    }

    async fn aggregate_cost_by_job(&self, job_id: &str) -> Result<f64, AiomeError> {
        let res_opt = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query(
                "SELECT SUM(estimated_cost_usd) FROM resource_usage_logs WHERE job_id = ?",
            )
            .bind(job_id)
            .fetch_one(p)
            .await
            .map(|row| row.get::<Option<f64>, _>(0)),
            crate::db::DatabasePool::Postgres(p) => sqlx::query(
                "SELECT SUM(estimated_cost_usd) FROM resource_usage_logs WHERE job_id = $1",
            )
            .bind(job_id)
            .fetch_one(p)
            .await
            .map(|row| row.get::<Option<f64>, _>(0)),
        };

        res_opt
            .map(|opt| opt.unwrap_or(0.0))
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to aggregate job costs: {}", e),
            })
    }
}

fn map_sqlite_row_to_setting(row: sqlx::sqlite::SqliteRow) -> aiome_core::contracts::SystemSetting {
    use sqlx::Row;
    let is_secret = row.get::<i32, _>("is_secret") != 0;
    aiome_core::contracts::SystemSetting {
        key: row.get("key"),
        value: if is_secret {
            "••••••••".to_string()
        } else {
            row.get("value")
        },
        category: row.get("category"),
        is_secret,
        updated_at: row.get("updated_at"),
    }
}

fn map_postgres_row_to_setting(row: sqlx::postgres::PgRow) -> aiome_core::contracts::SystemSetting {
    use sqlx::Row;
    let is_secret = row.get::<bool, _>("is_secret");
    aiome_core::contracts::SystemSetting {
        key: row.get("key"),
        value: if is_secret {
            "••••••••".to_string()
        } else {
            row.get("value")
        },
        category: row.get("category"),
        is_secret,
        updated_at: row.get("updated_at"),
    }
}
