/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::UniversalJobQueue;
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use sqlx::Row;

#[async_trait]
pub trait EvolutionOps {
    async fn do_get_agent_stats(&self) -> Result<shared::watchtower::AgentStats, AiomeError>;
    async fn do_add_resonance(&self, amount: i32) -> Result<(), AiomeError>;
    async fn do_add_tech_exp(&self, amount: i32) -> Result<(), AiomeError>;
    async fn do_add_creativity(&self, amount: i32) -> Result<(), AiomeError>;
    async fn do_record_soul_mutation(
        &self,
        old_hash: &str,
        new_hash: &str,
        reason: &str,
    ) -> Result<(), AiomeError>;
    async fn do_sync_samsara_level(
        &self,
    ) -> Result<Option<aiome_core::contracts::SamsaraEvent>, AiomeError>;

    // Evolution Chronicle
    async fn do_record_evolution_event(
        &self,
        level: i32,
        event_type: &str,
        description: &str,
        inspiration: Option<&str>,
        karma_json: Option<&str>,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_evolution_history(
        &self,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError>;
}

#[async_trait]
impl EvolutionOps for UniversalJobQueue {
    async fn do_get_agent_stats(&self) -> Result<shared::watchtower::AgentStats, AiomeError> {
        let q = "SELECT level, exp, resonance, creativity, fatigue FROM agent_stats WHERE id = 1";
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let r =
                    sqlx::query(q)
                        .fetch_one(p)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                Ok(shared::watchtower::AgentStats {
                    level: r.get("level"),
                    exp: r.get("exp"),
                    resonance: r.get("resonance"),
                    creativity: r.get("creativity"),
                    fatigue: r.get("fatigue"),
                })
            }
            crate::db::DatabasePool::Postgres(p) => {
                let r =
                    sqlx::query(q)
                        .fetch_one(p)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                Ok(shared::watchtower::AgentStats {
                    level: r.get("level"),
                    exp: r.get("exp"),
                    resonance: r.get("resonance"),
                    creativity: r.get("creativity"),
                    fatigue: r.get("fatigue"),
                })
            }
        }
    }

    async fn do_add_resonance(&self, amount: i32) -> Result<(), AiomeError> {
        let q = format!(
            "UPDATE agent_stats SET resonance = resonance + {0}, updated_at = {1} WHERE id = 1",
            self.pool.ph(0),
            self.pool.now_fn()
        );
        sql_exec!(&self.pool, &q, amount).map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;
        Ok(())
    }

    async fn do_add_tech_exp(&self, amount: i32) -> Result<(), AiomeError> {
        let q = format!(
            "UPDATE agent_stats SET exp = exp + {0}, updated_at = {1} WHERE id = 1",
            self.pool.ph(0),
            self.pool.now_fn()
        );
        sql_exec!(&self.pool, &q, amount).map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;
        let _ = self.do_sync_samsara_level().await;
        Ok(())
    }

    async fn do_add_creativity(&self, amount: i32) -> Result<(), AiomeError> {
        let q = format!(
            "UPDATE agent_stats SET creativity = creativity + {0}, updated_at = {1} WHERE id = 1",
            self.pool.ph(0),
            self.pool.now_fn()
        );
        sql_exec!(&self.pool, &q, amount).map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;
        Ok(())
    }

    async fn do_record_soul_mutation(
        &self,
        old_hash: &str,
        new_hash: &str,
        reason: &str,
    ) -> Result<(), AiomeError> {
        let q = format!("INSERT INTO soul_mutation_history (old_hash, new_hash, mutation_reason) VALUES ({0}, {1}, {2})", self.pool.ph(0), self.pool.ph(1), self.pool.ph(2));
        sql_exec!(&self.pool, &q, old_hash, new_hash, reason).map_err(|e| {
            AiomeError::Infrastructure {
                reason: e.to_string(),
            }
        })?;
        Ok(())
    }

    async fn do_sync_samsara_level(
        &self,
    ) -> Result<Option<aiome_core::contracts::SamsaraEvent>, AiomeError> {
        let q1 = "SELECT SUM(weight) as total FROM karma_logs WHERE karma_type = 'Technical'";
        let total_weight: i64 = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query(q1)
                .fetch_one(p)
                .await
                .map(|r| r.get::<Option<i64>, _>("total"))
                .ok()
                .flatten()
                .unwrap_or(0),
            crate::db::DatabasePool::Postgres(p) => sqlx::query(q1)
                .fetch_one(p)
                .await
                .map(|r| r.get::<Option<i64>, _>("total"))
                .ok()
                .flatten()
                .unwrap_or(0),
        };

        let stats = self.do_get_agent_stats().await?;
        let original_level = stats.level;
        let mut current_level = original_level;

        while total_weight >= (current_level as i64 * current_level as i64 * 1000) {
            current_level += 1;
            if current_level >= 100 {
                break;
            }
        }

        if current_level > original_level {
            let q2 = format!(
                "UPDATE agent_stats SET level = {0}, updated_at = {1} WHERE id = 1",
                self.pool.ph(0),
                self.pool.now_fn()
            );
            sql_exec!(&self.pool, &q2, current_level).map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
            tracing::info!(
                "🌟 [SamsaraEngine] Level Up! {} -> {}",
                original_level,
                current_level
            );
            return Ok(Some(aiome_core::contracts::SamsaraEvent::LevelUp {
                old_level: original_level,
                new_level: current_level,
            }));
        }
        Ok(None)
    }

    async fn do_record_evolution_event(
        &self,
        level: i32,
        event_type: &str,
        description: &str,
        inspiration: Option<&str>,
        karma_json: Option<&str>,
    ) -> Result<(), AiomeError> {
        let q1 = "SELECT record_hash FROM evolution_chronicle ORDER BY id DESC LIMIT 1";
        let prev_hash = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query(q1)
                .fetch_optional(p)
                .await
                .map(|o| o.map(|r| r.get::<String, _>("record_hash")))
                .ok()
                .flatten()
                .unwrap_or_else(|| "GENESIS".to_string()),
            crate::db::DatabasePool::Postgres(p) => sqlx::query(q1)
                .fetch_optional(p)
                .await
                .map(|o| o.map(|r| r.get::<String, _>("record_hash")))
                .ok()
                .flatten()
                .unwrap_or_else(|| "GENESIS".to_string()),
        };

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(description.as_bytes());
        hasher.update(chrono::Utc::now().to_rfc3339().as_bytes());
        let record_hash = format!("{:x}", hasher.finalize());

        let q2 = format!("INSERT INTO evolution_chronicle (level_at, event_type, description, inspiration_source, karma_snapshot, prev_record_hash, record_hash) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6));
        sql_exec!(
            &self.pool,
            &q2,
            level,
            event_type,
            description,
            inspiration,
            karma_json,
            &prev_hash,
            &record_hash
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;
        Ok(())
    }

    async fn do_fetch_evolution_history(
        &self,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        let q = format!(
            "SELECT * FROM evolution_chronicle ORDER BY id DESC LIMIT {}",
            self.pool.ph(0)
        );
        let mut history = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(&q)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for row in rows {
                    history.push(serde_json::json!({ "id": row.get::<i64, _>("id"), "level": row.get::<i32, _>("level_at"), "event_type": row.get::<String, _>("event_type"), "description": row.get::<String, _>("description"), "inspiration": row.get::<Option<String>, _>("inspiration_source"), "karma": row.get::<Option<String>, _>("karma_snapshot"), "record_hash": row.get::<String, _>("record_hash"), "created_at": row.get::<String, _>("created_at") }));
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(&q)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for row in rows {
                    history.push(serde_json::json!({ "id": row.get::<i64, _>("id"), "level": row.get::<i32, _>("level_at"), "event_type": row.get::<String, _>("event_type"), "description": row.get::<String, _>("description"), "inspiration": row.get::<Option<String>, _>("inspiration_source"), "karma": row.get::<Option<String>, _>("karma_snapshot"), "record_hash": row.get::<String, _>("record_hash"), "created_at": row.get::<String, _>("created_at") }));
                }
            }
        }
        Ok(history)
    }
}
