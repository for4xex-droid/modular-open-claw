/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::UniversalJobQueue;
use crate::{sql_exec, sql_fetch_all, sql_fetch_one, sql_fetch_optional};
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use base64::{prelude::BASE64_STANDARD, Engine};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use sqlx::Row;
use tracing::warn;

#[async_trait]
pub trait SwarmOps {
    async fn do_get_node_id(&self) -> Result<String, AiomeError>;
    async fn do_sign_swarm_payload(&self, payload: &str) -> Result<String, AiomeError>;
    async fn do_tick_local_clock(&self) -> Result<u64, AiomeError>;
    async fn do_sync_local_clock(&self, remote_clock: u64) -> Result<u64, AiomeError>;
    async fn do_get_global_api_failures(&self) -> Result<i64, AiomeError>;
    async fn do_record_global_api_failure(&self) -> Result<i64, AiomeError>;
    async fn do_record_global_api_success(&self) -> Result<(), AiomeError>;
    async fn do_get_system_agent_id(&self) -> Result<uuid::Uuid, AiomeError>;
    // Biome
    async fn do_get_biome_topic_status(
        &self,
        topic_id: &str,
    ) -> Result<Option<(i32, Option<String>)>, AiomeError>;
    async fn do_advance_biome_turn(
        &self,
        topic_id: &str,
        cooldown_minutes: i64,
    ) -> Result<i32, AiomeError>;
    async fn do_fetch_biome_messages(
        &self,
        topic_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError>;
    async fn do_store_biome_message(
        &self,
        message: &aiome_contracts::biome::BiomeMessage,
    ) -> Result<(), AiomeError>;
    async fn do_update_biome_reputation(&self, pubkey: &str, delta: f64)
        -> Result<f64, AiomeError>;
    async fn do_archive_biome_topic(&self, topic_id: &str) -> Result<(), AiomeError>;
}

#[async_trait]
impl SwarmOps for UniversalJobQueue {
    async fn do_get_biome_topic_status(
        &self,
        topic_id: &str,
    ) -> Result<Option<(i32, Option<String>)>, AiomeError> {
        let q = format!(
            "SELECT turn_count, status FROM biome_topics WHERE topic_id = {}",
            self.pool.ph(0)
        );
        let opt: Option<(i32, String)> =
            crate::sql_fetch_optional!(&self.pool, (i32, String), &q, topic_id).unwrap_or(None);
        Ok(opt.map(|(c, s)| (c, Some(s))))
    }

    async fn do_advance_biome_turn(
        &self,
        topic_id: &str,
        cooldown_minutes: i64,
    ) -> Result<i32, AiomeError> {
        let q_check = format!(
            "SELECT turn_count FROM biome_topics WHERE topic_id = {}",
            self.pool.ph(0)
        );
        let current: i32 = crate::sql_fetch_optional!(&self.pool, (i32,), &q_check, topic_id)
            .unwrap_or(None)
            .map(|r| r.0)
            .unwrap_or(0);

        let next = current + 1;
        let q_upsert = match &self.pool {
            crate::db::DatabasePool::Sqlite(_) => format!("INSERT INTO biome_topics (topic_id, peer_pubkey, status, turn_count, cooldown_until) VALUES ({0}, 'peer', 'Active', {1}, datetime('now', '+{2} minutes')) ON CONFLICT(topic_id) DO UPDATE SET turn_count = biome_topics.turn_count + 1, cooldown_until = datetime('now', '+{2} minutes')", self.pool.ph(0), self.pool.ph(1), self.pool.ph(2)),
            crate::db::DatabasePool::Postgres(_) => format!("INSERT INTO biome_topics (topic_id, peer_pubkey, status, turn_count, cooldown_until) VALUES ({0}, 'peer', 'Active', {1}, NOW() + interval '{2} minutes') ON CONFLICT(topic_id) DO UPDATE SET turn_count = biome_topics.turn_count + 1, cooldown_until = NOW() + interval '{2} minutes'", self.pool.ph(0), self.pool.ph(1), self.pool.ph(2)),
        };
        crate::sql_exec!(
            &self.pool,
            &q_upsert,
            topic_id,
            1,
            cooldown_minutes.to_string()
        )?;
        Ok(next)
    }

    async fn do_fetch_biome_messages(
        &self,
        _topic_id: &str,
        _limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        Ok(vec![])
    }
    async fn do_store_biome_message(
        &self,
        _message: &aiome_contracts::biome::BiomeMessage,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn do_update_biome_reputation(
        &self,
        _pubkey: &str,
        _delta: f64,
    ) -> Result<f64, AiomeError> {
        Ok(0.0)
    }

    async fn do_archive_biome_topic(&self, topic_id: &str) -> Result<(), AiomeError> {
        let q = format!(
            "UPDATE biome_topics SET status = 'Archived' WHERE topic_id = {}",
            self.pool.ph(0)
        );
        crate::sql_exec!(&self.pool, &q, topic_id)?;
        Ok(())
    }

    async fn do_get_node_id(&self) -> Result<String, AiomeError> {
        let q1 = format!(
            "SELECT value FROM system_state WHERE key = {}",
            self.pool.ph(0)
        );
        let opt: Option<String> = crate::sql_fetch_optional!(&self.pool, (String,), &q1, "node_id")
            .unwrap_or(None)
            .map(|r| r.0);

        if let Some(val) = opt {
            Ok(val)
        } else {
            let mut csprng = OsRng;
            let signing_key = SigningKey::generate(&mut csprng);
            let pubkey_b64 = BASE64_STANDARD.encode(signing_key.verifying_key().as_bytes());
            let privkey_b64 = BASE64_STANDARD.encode(signing_key.to_bytes());

            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;
            let q2 = format!(
                "INSERT INTO system_state (key, value) VALUES ({}, {})",
                self.pool.ph(0),
                self.pool.ph(1)
            );
            match &mut tx {
                crate::db::DatabaseTransaction::Sqlite(t) => {
                    let _ = sqlx::query(&q2)
                        .bind("node_id")
                        .bind(&pubkey_b64)
                        .execute(&mut **t)
                        .await;
                    let _ = sqlx::query(&q2)
                        .bind("node_privkey")
                        .bind(&privkey_b64)
                        .execute(&mut **t)
                        .await;
                }
                crate::db::DatabaseTransaction::Postgres(t) => {
                    let _ = sqlx::query(&q2)
                        .bind("node_id")
                        .bind(&pubkey_b64)
                        .execute(&mut **t)
                        .await;
                    let _ = sqlx::query(&q2)
                        .bind("node_privkey")
                        .bind(&privkey_b64)
                        .execute(&mut **t)
                        .await;
                }
            }
            let _ = tx.commit().await;
            Ok(pubkey_b64)
        }
    }

    async fn do_sign_swarm_payload(&self, payload: &str) -> Result<String, AiomeError> {
        let _ = self.do_get_node_id().await?;
        let q = format!(
            "SELECT value FROM system_state WHERE key = {}",
            self.pool.ph(0)
        );
        let opt: Option<String> = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar(&q)
                .bind("node_privkey")
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar(&q)
                .bind("node_privkey")
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
        };

        if let Some(privkey_b64) = opt {
            let priv_bytes =
                BASE64_STANDARD
                    .decode(privkey_b64)
                    .map_err(|_| AiomeError::Infrastructure {
                        reason: "Corrupt node key".to_string(),
                    })?;
            let mut key_arr = [0u8; 32];
            key_arr.copy_from_slice(&priv_bytes);
            let signing_key = SigningKey::from_bytes(&key_arr);
            let signature = signing_key.sign(payload.as_bytes());
            Ok(BASE64_STANDARD.encode(signature.to_bytes()))
        } else {
            Err(AiomeError::Infrastructure {
                reason: "Node private key not found after initialization".to_string(),
            })
        }
    }

    async fn do_tick_local_clock(&self) -> Result<u64, AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        let q1 = format!(
            "SELECT value FROM system_state WHERE key = {}",
            self.pool.ph(0)
        );
        let current: i64 = match &mut tx {
            crate::db::DatabaseTransaction::Sqlite(t) => sqlx::query(&q1)
                .bind("logical_clock")
                .fetch_one(&mut **t)
                .await
                .map(|r| r.get::<String, _>("value").parse().unwrap_or(0))
                .unwrap_or(0),
            crate::db::DatabaseTransaction::Postgres(t) => sqlx::query(&q1)
                .bind("logical_clock")
                .fetch_one(&mut **t)
                .await
                .map(|r| r.get::<String, _>("value").parse().unwrap_or(0))
                .unwrap_or(0),
        };

        let next = current + 1;
        let q2 = format!(
            "UPDATE system_state SET value = {} WHERE key = {}",
            self.pool.ph(0),
            self.pool.ph(1)
        );
        match &mut tx {
            crate::db::DatabaseTransaction::Sqlite(t) => {
                let _ = sqlx::query(&q2)
                    .bind(next.to_string())
                    .bind("logical_clock")
                    .execute(&mut **t)
                    .await;
            }
            crate::db::DatabaseTransaction::Postgres(t) => {
                let _ = sqlx::query(&q2)
                    .bind(next.to_string())
                    .bind("logical_clock")
                    .execute(&mut **t)
                    .await;
            }
        }
        let _ = tx.commit().await;
        Ok(next as u64)
    }

    async fn do_sync_local_clock(&self, remote_clock: u64) -> Result<u64, AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        let q1 = format!(
            "SELECT value FROM system_state WHERE key = {}",
            self.pool.ph(0)
        );
        let current: i64 = match &mut tx {
            crate::db::DatabaseTransaction::Sqlite(t) => sqlx::query(&q1)
                .bind("logical_clock")
                .fetch_one(&mut **t)
                .await
                .map(|r| r.get::<String, _>("value").parse().unwrap_or(0))
                .unwrap_or(0),
            crate::db::DatabaseTransaction::Postgres(t) => sqlx::query(&q1)
                .bind("logical_clock")
                .fetch_one(&mut **t)
                .await
                .map(|r| r.get::<String, _>("value").parse().unwrap_or(0))
                .unwrap_or(0),
        };

        if remote_clock > (current as u64) + 100_000 {
            warn!(
                "⚠️ Potential Clock Poisoning attempt or severe skew detected: {} vs {}",
                remote_clock, current
            );
            let _ = tx.rollback().await;
            return Ok(current as u64);
        }

        let next = std::cmp::max(current as u64, remote_clock) + 1;
        let q2 = format!(
            "UPDATE system_state SET value = {} WHERE key = {}",
            self.pool.ph(0),
            self.pool.ph(1)
        );
        match &mut tx {
            crate::db::DatabaseTransaction::Sqlite(t) => {
                let _ = sqlx::query(&q2)
                    .bind(next.to_string())
                    .bind("logical_clock")
                    .execute(&mut **t)
                    .await;
            }
            crate::db::DatabaseTransaction::Postgres(t) => {
                let _ = sqlx::query(&q2)
                    .bind(next.to_string())
                    .bind("logical_clock")
                    .execute(&mut **t)
                    .await;
            }
        }
        let _ = tx.commit().await;
        Ok(next)
    }

    async fn do_get_global_api_failures(&self) -> Result<i64, AiomeError> {
        let q = format!(
            "SELECT value FROM system_state WHERE key = {}",
            self.pool.ph(0)
        );
        let opt: Option<String> = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar(&q)
                .bind("consecutive_api_failures")
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar(&q)
                .bind("consecutive_api_failures")
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
        };
        Ok(opt.map(|v| v.parse().unwrap_or(0)).unwrap_or(0))
    }

    async fn do_record_global_api_failure(&self) -> Result<i64, AiomeError> {
        let current = self.do_get_global_api_failures().await?;
        let next = current + 1;
        let q = match &self.pool {
            crate::db::DatabasePool::Sqlite(_) => format!("INSERT OR REPLACE INTO system_state (key, value, updated_at) VALUES ({0}, {1}, {2})", self.pool.ph(0), self.pool.ph(1), self.pool.now_fn()),
            crate::db::DatabasePool::Postgres(_) => format!("INSERT INTO system_state (key, value, updated_at) VALUES ({0}, {1}, {2}) ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at", self.pool.ph(0), self.pool.ph(1), self.pool.now_fn()),
        };
        sql_exec!(&self.pool, &q, "consecutive_api_failures", next.to_string())?;
        Ok(next)
    }

    async fn do_record_global_api_success(&self) -> Result<(), AiomeError> {
        let q = match &self.pool {
            crate::db::DatabasePool::Sqlite(_) => format!("INSERT OR REPLACE INTO system_state (key, value, updated_at) VALUES ({0}, {1}, {2})", self.pool.ph(0), self.pool.ph(1), self.pool.now_fn()),
            crate::db::DatabasePool::Postgres(_) => format!("INSERT INTO system_state (key, value, updated_at) VALUES ({0}, {1}, {2}) ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at", self.pool.ph(0), self.pool.ph(1), self.pool.now_fn()),
        };
        sql_exec!(&self.pool, &q, "consecutive_api_failures", "0")?;
        Ok(())
    }

    async fn do_get_system_agent_id(&self) -> Result<uuid::Uuid, AiomeError> {
        let q1 = format!(
            "SELECT value FROM system_state WHERE key = {}",
            self.pool.ph(0)
        );
        let opt: Option<String> = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar(&q1)
                .bind("system_agent_id")
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar(&q1)
                .bind("system_agent_id")
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
        };

        if let Some(val) = opt {
            uuid::Uuid::parse_str(&val).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Corrupt system_agent_id: {}", e),
            })
        } else {
            let new_id = uuid::Uuid::new_v4();
            let val = new_id.to_string();
            let q2 = format!(
                "INSERT INTO system_state (key, value) VALUES ({}, {})",
                self.pool.ph(0),
                self.pool.ph(1)
            );
            sql_exec!(&self.pool, &q2, "system_agent_id", &val)?;
            Ok(new_id)
        }
    }
}
