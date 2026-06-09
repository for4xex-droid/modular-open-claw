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

const MAX_CLOCK_SKEW: u64 = 100_000;

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
    // Commune
    async fn do_get_commune_topic_status(
        &self,
        topic_id: &str,
    ) -> Result<Option<(i32, Option<String>)>, AiomeError>;
    async fn do_advance_commune_turn(
        &self,
        topic_id: &str,
        cooldown_minutes: i64,
    ) -> Result<i32, AiomeError>;
    async fn do_fetch_commune_messages(
        &self,
        topic_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError>;
    async fn do_store_commune_message(
        &self,
        message: &aiome_core_contracts::commune::CommuneMessage,
    ) -> Result<(), AiomeError>;
    async fn do_update_commune_reputation(
        &self,
        pubkey: &str,
        delta: f64,
    ) -> Result<f64, AiomeError>;
    async fn do_archive_commune_topic(&self, topic_id: &str) -> Result<(), AiomeError>;
    async fn do_store_shared_genome(
        &self,
        topic_id: &str,
        blueprint_json: &str,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_shared_genomes(
        &self,
        topic_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError>;
}

#[async_trait]
impl SwarmOps for UniversalJobQueue {
    async fn do_get_commune_topic_status(
        &self,
        topic_id: &str,
    ) -> Result<Option<(i32, Option<String>)>, AiomeError> {
        const Q_STATUS_SQLITE: &str =
            "SELECT turn_count, status FROM commune_topics WHERE topic_id = ?";
        const Q_STATUS_PG: &str =
            "SELECT turn_count, status FROM commune_topics WHERE topic_id = $1";

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

    async fn do_advance_commune_turn(
        &self,
        topic_id: &str,
        cooldown_minutes: i64,
    ) -> Result<i32, AiomeError> {
        const Q_CHECK_SQLITE: &str = "SELECT turn_count FROM commune_topics WHERE topic_id = ?";
        const Q_CHECK_PG: &str = "SELECT turn_count FROM commune_topics WHERE topic_id = $1";

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

        const Q_UPSERT_SQLITE: &str = "INSERT INTO commune_topics (topic_id, peer_pubkey, status, turn_count, cooldown_until) VALUES (?, 'peer', 'Active', ?, ?) ON CONFLICT(topic_id) DO UPDATE SET turn_count = commune_topics.turn_count + 1, cooldown_until = ?";
        const Q_UPSERT_PG: &str = "INSERT INTO commune_topics (topic_id, peer_pubkey, status, turn_count, cooldown_until) VALUES ($1, 'peer', 'Active', $2, $3::timestamptz) ON CONFLICT(topic_id) DO UPDATE SET turn_count = commune_topics.turn_count + 1, cooldown_until = $3::timestamptz";

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

    async fn do_fetch_commune_messages(
        &self,
        topic_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        let q = format!(
            "SELECT sender_pubkey, recipient_pubkey, topic_id, content, karma_root_cid, signature, lamport_clock, encryption, payload_type, created_at \
             FROM commune_messages WHERE topic_id = {} ORDER BY created_at DESC LIMIT {}",
            self.pool.ph(0), self.pool.ph(1)
        );

        /// Converts raw sqlx rows into a Vec of JSON values.
        /// Macro is required because SqliteRow and PgRow are distinct types
        /// that share the same `try_get` API but differ in their Database associated type.
        macro_rules! commune_rows_to_json {
            ($rows:expr) => {{
                let mut results = Vec::with_capacity($rows.len());
                for r in $rows {
                    let lamport: i64 = r.try_get("lamport_clock").unwrap_or(0);
                    results.push(serde_json::json!({
                        "sender_pubkey": r.try_get::<String, _>("sender_pubkey").unwrap_or_default(),
                        "recipient_pubkey": r.try_get::<String, _>("recipient_pubkey").unwrap_or_default(),
                        "topic_id": r.try_get::<String, _>("topic_id").unwrap_or_default(),
                        "content": r.try_get::<String, _>("content").unwrap_or_default(),
                        "karma_root_cid": r.try_get::<String, _>("karma_root_cid").unwrap_or_default(),
                        "signature": r.try_get::<String, _>("signature").unwrap_or_default(),
                        "lamport_clock": lamport as u64,
                        "encryption": r.try_get::<String, _>("encryption").unwrap_or_default(),
                        "payload_type": r.try_get::<Option<String>, _>("payload_type").unwrap_or_default(),
                        "created_at": r.try_get::<String, _>("created_at").unwrap_or_default(),
                    }));
                }
                results
            }};
        }

        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(&q)
                    .bind(topic_id)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                Ok(commune_rows_to_json!(rows))
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(&q)
                    .bind(topic_id)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                Ok(commune_rows_to_json!(rows))
            }
        }
    }

    async fn do_store_commune_message(
        &self,
        message: &aiome_core_contracts::commune::CommuneMessage,
    ) -> Result<(), AiomeError> {
        crate::sql_exec!(
            &self.pool,
            sqlite: "INSERT INTO commune_messages (sender_pubkey, recipient_pubkey, topic_id, content, karma_root_cid, signature, lamport_clock, encryption, payload_type) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            pg: "INSERT INTO commune_messages (sender_pubkey, recipient_pubkey, topic_id, content, karma_root_cid, signature, lamport_clock, encryption, payload_type) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            &message.sender_pubkey,
            &message.recipient_pubkey,
            &message.topic_id,
            &message.content,
            &message.karma_root_cid,
            &message.signature,
            message.lamport_clock as i64,
            &message.encryption,
            &message.payload_type
        )?;
        Ok(())
    }

    async fn do_update_commune_reputation(
        &self,
        pubkey: &str,
        delta: f64,
    ) -> Result<f64, AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        const Q_SELECT_SQLITE: &str = "SELECT reputation_score FROM commune_peers WHERE pubkey = ?";
        const Q_SELECT_PG: &str = "SELECT reputation_score FROM commune_peers WHERE pubkey = $1";

        let opt: Option<(i32,)> = crate::sql_tx_fetch_optional!(
            &mut tx,
            (i32,),
            sqlite: Q_SELECT_SQLITE,
            pg: Q_SELECT_PG,
            pubkey
        )?;

        let current_score = opt.map(|r| r.0).unwrap_or(100);
        let new_score_f = current_score as f64 + delta;
        let new_score = new_score_f.round() as i32;

        const Q_UPSERT_SQLITE: &str =
            "INSERT OR REPLACE INTO commune_peers (pubkey, reputation_score) VALUES (?, ?)";
        const Q_UPSERT_PG: &str = "INSERT INTO commune_peers (pubkey, reputation_score) VALUES ($1, $2) \
                                   ON CONFLICT(pubkey) DO UPDATE SET reputation_score = EXCLUDED.reputation_score, last_seen_at = CURRENT_TIMESTAMP";

        crate::sql_tx_exec!(
            &mut tx,
            sqlite: Q_UPSERT_SQLITE,
            pg: Q_UPSERT_PG,
            pubkey,
            new_score
        )?;

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        Ok(new_score as f64 / 100.0)
    }

    async fn do_archive_commune_topic(&self, topic_id: &str) -> Result<(), AiomeError> {
        const Q_ARC_SQLITE: &str =
            "UPDATE commune_topics SET status = 'Archived' WHERE topic_id = ?";
        const Q_ARC_PG: &str = "UPDATE commune_topics SET status = 'Archived' WHERE topic_id = $1";

        crate::sql_exec!(
            &self.pool,
            sqlite: Q_ARC_SQLITE,
            pg: Q_ARC_PG,
            topic_id
        )?;
        Ok(())
    }

    async fn do_store_shared_genome(
        &self,
        topic_id: &str,
        blueprint_json: &str,
    ) -> Result<(), AiomeError> {
        crate::sql_exec!(
            &self.pool,
            sqlite: "INSERT INTO commune_shared_genomes (topic_id, blueprint_json) VALUES (?, ?)",
            pg: "INSERT INTO commune_shared_genomes (topic_id, blueprint_json) VALUES ($1, $2)",
            topic_id,
            blueprint_json
        )?;
        Ok(())
    }

    async fn do_fetch_shared_genomes(
        &self,
        topic_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        let q = format!(
            "SELECT topic_id, blueprint_json, created_at FROM commune_shared_genomes WHERE topic_id = {} ORDER BY created_at DESC LIMIT {}",
            self.pool.ph(0), self.pool.ph(1)
        );

        macro_rules! genome_rows_to_json {
            ($rows:expr) => {{
                let mut results = Vec::with_capacity($rows.len());
                for r in $rows {
                    results.push(serde_json::json!({
                        "topic_id": r.try_get::<String, _>("topic_id").unwrap_or_default(),
                        "blueprint_json": r.try_get::<String, _>("blueprint_json").unwrap_or_default(),
                        "created_at": r.try_get::<String, _>("created_at").unwrap_or_default(),
                    }));
                }
                results
            }};
        }

        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(&q)
                    .bind(topic_id)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                Ok(genome_rows_to_json!(rows))
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(&q)
                    .bind(topic_id)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                Ok(genome_rows_to_json!(rows))
            }
        }
    }

    async fn do_get_node_id(&self) -> Result<String, AiomeError> {
        let mut attempts = 0;
        loop {
            match self.do_get_node_id_inner().await {
                Ok(val) => return Ok(val),
                Err(e) => {
                    attempts += 1;
                    if attempts >= 10 {
                        return Err(e);
                    }
                    let sleep_ms = 10 + (rand::random::<u64>() % 50);
                    tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
                }
            }
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

        if remote_clock > (current as u64) + MAX_CLOCK_SKEW {
            warn!(
                "⚠️ Potential Clock Poisoning attempt or severe skew detected: {} vs {}",
                remote_clock, current
            );
            if let Err(e) = tx.rollback().await {
                tracing::error!("[Swarm] Transaction rollback failed: {}", e);
            }
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
        crate::sql_exec!(
            &self.pool,
            sqlite: "INSERT OR REPLACE INTO system_state (key, value, updated_at) VALUES (?, ?, datetime('now'))",
            pg: "INSERT INTO system_state (key, value, updated_at) VALUES ($1, $2, CURRENT_TIMESTAMP) ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at",
            "consecutive_api_failures",
            "0"
        )?;
        Ok(())
    }

    async fn do_get_system_agent_id(&self) -> Result<uuid::Uuid, AiomeError> {
        let mut attempts = 0;
        loop {
            match self.do_get_system_agent_id_inner().await {
                Ok(val) => return Ok(val),
                Err(e) => {
                    attempts += 1;
                    if attempts >= 10 {
                        return Err(e);
                    }
                    let sleep_ms = 10 + (rand::random::<u64>() % 50);
                    tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
                }
            }
        }
    }
}

impl UniversalJobQueue {
    async fn do_get_node_id_inner(&self) -> Result<String, AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        const Q_SELECT_SQLITE: &str = "SELECT value FROM system_state WHERE key = ?";
        const Q_SELECT_PG: &str = "SELECT value FROM system_state WHERE key = $1";

        let opt: Option<String> = crate::sql_tx_fetch_optional!(
            &mut tx,
            (String,),
            sqlite: Q_SELECT_SQLITE,
            pg: Q_SELECT_PG,
            "node_id"
        )
        .unwrap_or(None)
        .map(|r| r.0);

        if let Some(val) = opt {
            if let Err(e) = tx.rollback().await {
                tracing::error!("[Swarm] Transaction rollback failed: {}", e);
            }
            return Ok(val);
        }

        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let pubkey_b64 = BASE64_STANDARD.encode(signing_key.verifying_key().as_bytes());
        let privkey_b64 = BASE64_STANDARD.encode(signing_key.to_bytes());

        const Q_INSERT_SQLITE: &str =
            "INSERT INTO system_state (key, value) VALUES (?, ?) ON CONFLICT(key) DO NOTHING";
        const Q_INSERT_PG: &str =
            "INSERT INTO system_state (key, value) VALUES ($1, $2) ON CONFLICT(key) DO NOTHING";

        crate::sql_tx_exec!(
            &mut tx,
            sqlite: Q_INSERT_SQLITE,
            pg: Q_INSERT_PG,
            "node_id",
            &pubkey_b64
        )?;

        crate::sql_tx_exec!(
            &mut tx,
            sqlite: Q_INSERT_SQLITE,
            pg: Q_INSERT_PG,
            "node_privkey",
            &privkey_b64
        )?;

        let final_opt: Option<String> = crate::sql_tx_fetch_optional!(
            &mut tx,
            (String,),
            sqlite: Q_SELECT_SQLITE,
            pg: Q_SELECT_PG,
            "node_id"
        )
        .unwrap_or(None)
        .map(|r| r.0);

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        final_opt.ok_or_else(|| AiomeError::Infrastructure {
            reason: "Failed to resolve node_id after insertion".to_string(),
        })
    }

    async fn do_get_system_agent_id_inner(&self) -> Result<uuid::Uuid, AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        const Q1_SQLITE: &str = "SELECT value FROM system_state WHERE key = ?";
        const Q1_PG: &str = "SELECT value FROM system_state WHERE key = $1";

        let opt: Option<String> = crate::sql_tx_fetch_optional!(
            &mut tx,
            (String,),
            sqlite: Q1_SQLITE,
            pg: Q1_PG,
            "system_agent_id"
        )
        .unwrap_or(None)
        .map(|r| r.0);

        if let Some(val) = opt {
            if let Err(e) = tx.rollback().await {
                tracing::error!("[Swarm] Transaction rollback failed: {}", e);
            }
            return uuid::Uuid::parse_str(&val).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Corrupt system_agent_id: {}", e),
            });
        }

        let new_id = uuid::Uuid::new_v4();
        let val = new_id.to_string();

        const Q2_SQLITE: &str =
            "INSERT INTO system_state (key, value) VALUES (?, ?) ON CONFLICT(key) DO NOTHING";
        const Q2_PG: &str =
            "INSERT INTO system_state (key, value) VALUES ($1, $2) ON CONFLICT(key) DO NOTHING";

        crate::sql_tx_exec!(
            &mut tx,
            sqlite: Q2_SQLITE,
            pg: Q2_PG,
            "system_agent_id",
            &val
        )?;

        let final_opt: Option<String> = crate::sql_tx_fetch_optional!(
            &mut tx,
            (String,),
            sqlite: Q1_SQLITE,
            pg: Q1_PG,
            "system_agent_id"
        )
        .unwrap_or(None)
        .map(|r| r.0);

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        let resolved_val = final_opt.ok_or_else(|| AiomeError::Infrastructure {
            reason: "Failed to resolve system_agent_id after insertion".to_string(),
        })?;

        uuid::Uuid::parse_str(&resolved_val).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Corrupt system_agent_id after resolution: {}", e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_do_get_node_id_concurrent() {
        let sql_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let db_pool = crate::db::DatabasePool::Sqlite(sql_pool.clone());
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(db_pool.clone()),
        );

        sqlx::query(
            "CREATE TABLE system_state (key TEXT PRIMARY KEY, value TEXT, updated_at TEXT);",
        )
        .execute(&sql_pool)
        .await
        .unwrap();

        let queue = Arc::new(UniversalJobQueue::from_pool(db_pool, ts));

        let q1 = queue.clone();
        let q2 = queue.clone();

        let handle1 = tokio::spawn(async move { q1.do_get_node_id().await });
        let handle2 = tokio::spawn(async move { q2.do_get_node_id().await });

        let res1 = handle1.await.unwrap();
        let res2 = handle2.await.unwrap();

        assert!(res1.is_ok(), "First task failed: {:?}", res1);
        assert!(res2.is_ok(), "Second task failed: {:?}", res2);
        assert_eq!(
            res1.unwrap(),
            res2.unwrap(),
            "Returned node IDs do not match!"
        );
    }

    #[tokio::test]
    async fn test_do_get_system_agent_id_concurrent() {
        let sql_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let db_pool = crate::db::DatabasePool::Sqlite(sql_pool.clone());
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(db_pool.clone()),
        );

        sqlx::query(
            "CREATE TABLE system_state (key TEXT PRIMARY KEY, value TEXT, updated_at TEXT);",
        )
        .execute(&sql_pool)
        .await
        .unwrap();

        let queue = Arc::new(UniversalJobQueue::from_pool(db_pool, ts));

        let q1 = queue.clone();
        let q2 = queue.clone();

        let handle1 = tokio::spawn(async move { q1.do_get_system_agent_id().await });
        let handle2 = tokio::spawn(async move { q2.do_get_system_agent_id().await });

        let res1 = handle1.await.unwrap();
        let res2 = handle2.await.unwrap();

        assert!(res1.is_ok(), "First task failed: {:?}", res1);
        assert!(res2.is_ok(), "Second task failed: {:?}", res2);
        assert_eq!(
            res1.unwrap(),
            res2.unwrap(),
            "Returned system agent IDs do not match!"
        );
    }

    #[tokio::test]
    async fn test_commune_messages_and_reputation() {
        let sql_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let db_pool = crate::db::DatabasePool::Sqlite(sql_pool.clone());
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(db_pool.clone()),
        );

        sqlx::query("CREATE TABLE commune_messages (id INTEGER PRIMARY KEY AUTOINCREMENT, sender_pubkey TEXT NOT NULL, recipient_pubkey TEXT NOT NULL, topic_id TEXT NOT NULL, content TEXT NOT NULL, karma_root_cid TEXT NOT NULL, signature TEXT NOT NULL, lamport_clock INTEGER NOT NULL, encryption TEXT NOT NULL, payload_type TEXT DEFAULT NULL, created_at TEXT DEFAULT (datetime('now')));")
            .execute(&sql_pool)
            .await
            .unwrap();

        sqlx::query("CREATE TABLE commune_peers (pubkey TEXT PRIMARY KEY, last_seen_at TEXT DEFAULT (datetime('now')), reputation_score INTEGER NOT NULL DEFAULT 100);")
            .execute(&sql_pool)
            .await
            .unwrap();

        let queue = UniversalJobQueue::from_pool(db_pool, ts);

        let msg = aiome_core_contracts::commune::CommuneMessage {
            sender_pubkey: "sender".to_string(),
            recipient_pubkey: "recipient".to_string(),
            topic_id: "topic-1".to_string(),
            content: "hello".to_string(),
            karma_root_cid: "cid-1".to_string(),
            signature: "sig-1".to_string(),
            lamport_clock: 42,
            timestamp: "2026-06-07T00:00:00Z".to_string(),
            encryption: "none".to_string(),
            payload_type: None,
        };

        let store_res = queue.do_store_commune_message(&msg).await;
        assert!(
            store_res.is_ok(),
            "Failed to store message: {:?}",
            store_res
        );

        let fetch_res = queue.do_fetch_commune_messages("topic-1", 10).await;
        assert!(
            fetch_res.is_ok(),
            "Failed to fetch messages: {:?}",
            fetch_res
        );
        let messages = fetch_res.unwrap();
        assert_eq!(messages.len(), 1);
        let fetched_msg = &messages[0];
        assert_eq!(fetched_msg["content"], "hello");
        assert_eq!(fetched_msg["sender_pubkey"], "sender");

        let rep_res1 = queue.do_update_commune_reputation("peer-1", 10.0).await;
        assert!(rep_res1.is_ok());
        assert_eq!(rep_res1.unwrap(), 1.10);

        let rep_res2 = queue.do_update_commune_reputation("peer-1", -25.0).await;
        assert!(rep_res2.is_ok());
        assert_eq!(rep_res2.unwrap(), 0.85);
    }
}
