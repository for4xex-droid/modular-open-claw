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

#[async_trait]
/// `SettingsOps` トレイト
pub trait SettingsOps {
    /// 指定キーの設定値を取得する
    async fn do_get_setting(&self, key: &str) -> Result<Option<String>, AiomeError>;
    /// 設定値を保存・更新する
    async fn do_set_setting(
        &self,
        key: &str,
        value: &str,
        category: &str,
        is_secret: bool,
    ) -> Result<(), AiomeError>;
    /// 全設定値を一覧取得する
    async fn do_get_all_settings(
        &self,
    ) -> Result<Vec<aiome_core::contracts::SystemSetting>, AiomeError>;

    /// 特定のフィーチャーフラグが有効か確認する (Phase 2-D)
    async fn is_feature_enabled(&self, flag: &str) -> bool {
        self.do_get_setting(&format!("feature_flag.{}", flag))
            .await
            .ok()
            .flatten()
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
    }
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
                    rows.into_iter()
                        .map(|row| {
                            let is_secret = row.get::<i32, _>("is_secret") != 0;
                            aiome_core::contracts::SystemSetting {
                                key: row.get("key"),
                                // Never expose ciphertext to the frontend (§CISO-2)
                                value: if is_secret {
                                    "••••••••".to_string()
                                } else {
                                    row.get("value")
                                },
                                category: row.get("category"),
                                is_secret,
                                updated_at: row.get("updated_at"),
                            }
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
                        .map(|row| {
                            let is_secret = row.get::<bool, _>("is_secret");
                            aiome_core::contracts::SystemSetting {
                                key: row.get("key"),
                                // Never expose ciphertext to the frontend (§CISO-2)
                                value: if is_secret {
                                    "••••••••".to_string()
                                } else {
                                    row.get("value")
                                },
                                category: row.get("category"),
                                is_secret,
                                updated_at: row.get("updated_at"),
                            }
                        })
                        .collect()
                }
            };

        Ok(entries)
    }
}
