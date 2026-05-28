/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
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
        message: &aiome_core_contracts::biome::BiomeMessage,
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
        const Q_STATUS_SQLITE: &str =
            "SELECT turn_count, status FROM biome_topics WHERE topic_id = ?";
        const Q_STATUS_PG: &str = "SELECT turn_count, status FROM biome_topics WHERE topic_id = $1";

        let opt: Option<(i32, String)> = crate::sql_fetch_optional!(
            &self.pool,
            (i32, String),
            sqlite: Q_STATUS_SQLITE,
            pg: Q_STATUS_PG,
            topic_id
        )
        .unwrap_or(None);
        Ok(opt.map(|(c, s)| (c, Some(s))))
    }

    async fn do_advance_biome_turn(
        &self,
        topic_id: &str,
        cooldown_minutes: i64,
    ) -> Result<i32, AiomeError> {
        const Q_CHECK_SQLITE: &str = "SELECT turn_count FROM biome_topics WHERE topic_id = ?";
        const Q_CHECK_PG: &str = "SELECT turn_count FROM biome_topics WHERE topic_id = $1";

        let current: i32 = crate::sql_fetch_optional!(
            &self.pool,
            (i32,),
            sqlite: Q_CHECK_SQLITE,
            pg: Q_CHECK_PG,
            topic_id
        )
        .unwrap_or(None)
        .map(|r| r.0)
        .unwrap_or(0);

        let next = current + 1;
        let cooldown_ts =
            (chrono::Utc::now() + chrono::Duration::minutes(cooldown_minutes)).to_rfc3339();

        const Q_UPSERT_SQLITE: &str = "INSERT INTO biome_topics (topic_id, peer_pubkey, status, turn_count, cooldown_until) VALUES (?, 'peer', 'Active', ?, ?) ON CONFLICT(topic_id) DO UPDATE SET turn_count = biome_topics.turn_count + 1, cooldown_until = ?";
        const Q_UPSERT_PG: &str = "INSERT INTO biome_topics (topic_id, peer_pubkey, status, turn_count, cooldown_until) VALUES ($1, 'peer', 'Active', $2, $3::timestamptz) ON CONFLICT(topic_id) DO UPDATE SET turn_count = biome_topics.turn_count + 1, cooldown_until = $3::timestamptz";

        crate::sql_exec!(
            &self.pool,
            sqlite: Q_UPSERT_SQLITE,
            pg: Q_UPSERT_PG,
            topic_id,
            1,
            &cooldown_ts,
            &cooldown_ts
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
        _message: &aiome_core_contracts::biome::BiomeMessage,
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
        const Q_ARC_SQLITE: &str = "UPDATE biome_topics SET status = 'Archived' WHERE topic_id = ?";
        const Q_ARC_PG: &str = "UPDATE biome_topics SET status = 'Archived' WHERE topic_id = $1";

        crate::sql_exec!(
            &self.pool,
            sqlite: Q_ARC_SQLITE,
            pg: Q_ARC_PG,
            topic_id
        )?;
        Ok(())
    }

    async fn do_get_node_id(&self) -> Result<String, AiomeError> {
        const Q1_SQLITE: &str = "SELECT value FROM system_state WHERE key = ?";
        const Q1_PG: &str = "SELECT value FROM system_state WHERE key = $1";

        let opt: Option<String> = crate::sql_fetch_optional!(
            &self.pool,
            (String,),
            sqlite: Q1_SQLITE,
            pg: Q1_PG,
            "node_id"
        )
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
            const Q2_SQLITE: &str = "INSERT INTO system_state (key, value) VALUES (?, ?)";
            const Q2_PG: &str = "INSERT INTO system_state (key, value) VALUES ($1, $2)";

            crate::sql_tx_exec!(
                &mut tx,
                sqlite: Q2_SQLITE,
                pg: Q2_PG,
                "node_id",
                &pubkey_b64
            )?;

            crate::sql_tx_exec!(
                &mut tx,
                sqlite: Q2_SQLITE,
                pg: Q2_PG,
                "node_privkey",
                &privkey_b64
            )?;
            tx.commit().await.map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
            Ok(pubkey_b64)
        }
    }

    async fn do_sign_swarm_payload(&self, payload: &str) -> Result<String, AiomeError> {
        let _ = self.do_get_node_id().await?;
        const Q_KEY_SQLITE: &str = "SELECT value FROM system_state WHERE key = ?";
        const Q_KEY_PG: &str = "SELECT value FROM system_state WHERE key = $1";

        let opt: Option<String> = crate::sql_fetch_optional!(
            &self.pool,
            (String,),
            sqlite: Q_KEY_SQLITE,
            pg: Q_KEY_PG,
            "node_privkey"
        )
        .unwrap_or(None)
        .map(|r| r.0);

        if let Some(privkey_b64) = opt {
            let priv_bytes =
                BASE64_STANDARD
                    .decode(privkey_b64)
                    .map_err(|_| AiomeError::Infrastructure {
                        reason: "Corrupt node key".to_string(),
                    })?;
            if priv_bytes.len() != 32 {
                return Err(AiomeError::Infrastructure {
                    reason: "Corrupt node key (invalid length)".to_string(),
                });
            }
            let mut key_arr = zeroize::Zeroizing::new([0u8; 32]);
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
        const Q1_SQLITE: &str = "SELECT value FROM system_state WHERE key = ?";
        const Q1_PG: &str = "SELECT value FROM system_state WHERE key = $1";

        let current: i64 = crate::sql_tx_fetch_one!(
            &mut tx,
            (String,),
            sqlite: Q1_SQLITE,
            pg: Q1_PG,
            "logical_clock"
        )
        .map(|r| r.0.parse().unwrap_or(0))
        .unwrap_or(0);

        let next = current + 1;

        const Q2_SQLITE: &str = "UPDATE system_state SET value = ? WHERE key = ?";
        const Q2_PG: &str = "UPDATE system_state SET value = $1 WHERE key = $2";

        crate::sql_tx_exec!(
            &mut tx,
            sqlite: Q2_SQLITE,
            pg: Q2_PG,
            next.to_string(),
            "logical_clock"
        )?;
        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;
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
        const Q1_SQLITE: &str = "SELECT value FROM system_state WHERE key = ?";
        const Q1_PG: &str = "SELECT value FROM system_state WHERE key = $1";

        let current: i64 = crate::sql_tx_fetch_one!(
            &mut tx,
            (String,),
            sqlite: Q1_SQLITE,
            pg: Q1_PG,
            "logical_clock"
        )
        .map(|r| r.0.parse().unwrap_or(0))
        .unwrap_or(0);

        if remote_clock > (current as u64) + 100_000 {
            warn!(
                "⚠️ Potential Clock Poisoning attempt or severe skew detected: {} vs {}",
                remote_clock, current
            );
            let _ = tx.rollback().await;
            return Ok(current as u64);
        }

        let next = std::cmp::max(current as u64, remote_clock) + 1;

        const Q2_SQLITE: &str = "UPDATE system_state SET value = ? WHERE key = ?";
        const Q2_PG: &str = "UPDATE system_state SET value = $1 WHERE key = $2";

        crate::sql_tx_exec!(
            &mut tx,
            sqlite: Q2_SQLITE,
            pg: Q2_PG,
            next.to_string(),
            "logical_clock"
        )?;
        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;
        Ok(next)
    }

    async fn do_get_global_api_failures(&self) -> Result<i64, AiomeError> {
        const Q_FAIL_SQLITE: &str = "SELECT value FROM system_state WHERE key = ?";
        const Q_FAIL_PG: &str = "SELECT value FROM system_state WHERE key = $1";

        let opt: Option<String> = crate::sql_fetch_optional!(
            &self.pool,
            (String,),
            sqlite: Q_FAIL_SQLITE,
            pg: Q_FAIL_PG,
            "consecutive_api_failures"
        )
        .unwrap_or(None)
        .map(|r| r.0);
        Ok(opt.map(|v| v.parse().unwrap_or(0)).unwrap_or(0))
    }

    async fn do_record_global_api_failure(&self) -> Result<i64, AiomeError> {
        let current = self.do_get_global_api_failures().await?;
        let next = current + 1;
        const SQLITE_Q: &str = "INSERT OR REPLACE INTO system_state (key, value, updated_at) VALUES (?, ?, datetime('now'))";
        const PG_Q: &str = "INSERT INTO system_state (key, value, updated_at) VALUES ($1, $2, NOW()) ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at";

        crate::sql_exec!(
            &self.pool,
            sqlite: SQLITE_Q,
            pg: PG_Q,
            "consecutive_api_failures",
            next.to_string()
        )?;
        Ok(next)
    }

    async fn do_record_global_api_success(&self) -> Result<(), AiomeError> {
        let sqlite_q = format!(
            "INSERT OR REPLACE INTO system_state (key, value, updated_at) VALUES (?, ?, {})",
            self.pool.now_fn()
        );
        let pg_q = format!("INSERT INTO system_state (key, value, updated_at) VALUES ($1, $2, {}) ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at", self.pool.now_fn());

        crate::sql_exec!(
            &self.pool,
            sqlite: &sqlite_q,
            pg: &pg_q,
            "consecutive_api_failures",
            "0"
        )?;
        Ok(())
    }

    async fn do_get_system_agent_id(&self) -> Result<uuid::Uuid, AiomeError> {
        const Q1_SQLITE: &str = "SELECT value FROM system_state WHERE key = ?";
        const Q1_PG: &str = "SELECT value FROM system_state WHERE key = $1";

        let opt: Option<String> = crate::sql_fetch_optional!(
            &self.pool,
            (String,),
            sqlite: Q1_SQLITE,
            pg: Q1_PG,
            "system_agent_id"
        )
        .unwrap_or(None)
        .map(|r| r.0);

        if let Some(val) = opt {
            uuid::Uuid::parse_str(&val).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Corrupt system_agent_id: {}", e),
            })
        } else {
            let new_id = uuid::Uuid::new_v4();
            let val = new_id.to_string();

            const Q2_SQLITE: &str = "INSERT INTO system_state (key, value) VALUES (?, ?)";
            const Q2_PG: &str = "INSERT INTO system_state (key, value) VALUES ($1, $2)";

            crate::sql_exec!(
                &self.pool,
                sqlite: Q2_SQLITE,
                pg: Q2_PG,
                "system_agent_id",
                &val
            )?;
            Ok(new_id)
        }
    }
}
