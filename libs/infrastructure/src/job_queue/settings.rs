/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::job_queue::UniversalJobQueue;
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use sqlx::Row;

#[async_trait]
/// `SettingsOps` トレイト
pub trait SettingsOps {
    /// 指定キーの設定値を取得する
    async fn get_setting(&self, key: &str) -> Result<Option<String>, AiomeError>;
    /// 設定値を保存・更新する
    async fn set_setting(
        &self,
        key: &str,
        value: &str,
        category: &str,
        is_secret: bool,
    ) -> Result<(), AiomeError>;
    /// 全設定値を一覧取得する
    async fn get_all_settings(
        &self,
    ) -> Result<Vec<aiome_core::contracts::SystemSetting>, AiomeError>;
}

#[async_trait]
impl SettingsOps for UniversalJobQueue {
    async fn get_setting(&self, key: &str) -> Result<Option<String>, AiomeError> {
        let q = format!(
            "SELECT value FROM system_settings WHERE key = {}",
            self.pool.ph(0)
        );
        let opt: Option<String> = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar(&q)
                .bind(key)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar(&q)
                .bind(key)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
        };
        Ok(opt)
    }

    async fn set_setting(
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

    async fn get_all_settings(
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
                    rows.into_iter()
                        .map(|row| aiome_core::contracts::SystemSetting {
                            key: row.get("key"),
                            value: row.get("value"),
                            category: row.get("category"),
                            is_secret: row.get::<i32, _>("is_secret") != 0,
                            updated_at: row.get("updated_at"),
                        })
                        .collect()
                }
                crate::db::DatabasePool::Postgres(p) => {
                    let rows = sqlx::query(q).fetch_all(p).await.map_err(|e| {
                        AiomeError::Infrastructure {
                            reason: e.to_string(),
                        }
                    })?;
                    rows.into_iter()
                        .map(|row| aiome_core::contracts::SystemSetting {
                            key: row.get("key"),
                            value: row.get("value"),
                            category: row.get("category"),
                            is_secret: row.get::<bool, _>("is_secret"),
                            updated_at: row.get("updated_at"),
                        })
                        .collect()
                }
            };

        Ok(entries)
    }
}
