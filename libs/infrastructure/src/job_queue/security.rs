/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use super::UniversalJobQueue;
use crate::db::DatabasePool;
use aiome_core::error::AiomeError;
use aiome_core_contracts::contracts::SystemEvent;
use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

#[async_trait]
pub trait SecurityOps {
    async fn do_get_security_request_count(
        &self,
        agent_id: Option<Uuid>,
    ) -> Result<u32, AiomeError>;
    async fn do_increment_security_request_count(
        &self,
        agent_id: Option<Uuid>,
    ) -> Result<u32, AiomeError>;
    async fn forget_actor(&self, agent_id: Uuid) -> Result<(), AiomeError>;
}

#[async_trait]
impl SecurityOps for UniversalJobQueue {
    async fn do_get_security_request_count(
        &self,
        agent_id: Option<Uuid>,
    ) -> Result<u32, AiomeError> {
        let id_str = agent_id
            .map(|u| u.to_string())
            .unwrap_or_else(|| "SYSTEM".to_string());

        match &self.pool {
            DatabasePool::Sqlite(p) => {
                let row =
                    sqlx::query("SELECT request_count FROM security_audit WHERE agent_id = ?")
                        .bind(&id_str)
                        .fetch_optional(p)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;

                Ok(row.map(|r| r.get::<i64, _>(0) as u32).unwrap_or(0))
            }
            DatabasePool::Postgres(p) => {
                let row =
                    sqlx::query("SELECT request_count FROM security_audit WHERE agent_id = $1")
                        .bind(&id_str)
                        .fetch_optional(p)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;

                Ok(row.map(|r| r.get::<i32, _>(0) as u32).unwrap_or(0))
            }
        }
    }

    async fn do_increment_security_request_count(
        &self,
        agent_id: Option<Uuid>,
    ) -> Result<u32, AiomeError> {
        let id_str = agent_id
            .map(|u| u.to_string())
            .unwrap_or_else(|| "SYSTEM".to_string());

        match &self.pool {
            DatabasePool::Sqlite(p) => {
                sqlx::query("INSERT INTO security_audit (agent_id, request_count) VALUES (?, 1) ON CONFLICT(agent_id) DO UPDATE SET request_count = request_count + 1, updated_at = datetime('now')")
                    .bind(&id_str)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

                self.do_get_security_request_count(agent_id).await
            }
            DatabasePool::Postgres(p) => {
                sqlx::query("INSERT INTO security_audit (agent_id, request_count) VALUES ($1, 1) ON CONFLICT(agent_id) DO UPDATE SET request_count = security_audit.request_count + 1, updated_at = now()")
                    .bind(&id_str)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

                self.do_get_security_request_count(agent_id).await
            }
        }
    }

    async fn forget_actor(&self, agent_id: Uuid) -> Result<(), AiomeError> {
        let agent_id_str = agent_id.to_string();

        match &self.pool {
            DatabasePool::Sqlite(p) => {
                let mut tx = p.begin().await.map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;

                sqlx::query("DELETE FROM ekyc_sessions WHERE user_id = ?")
                    .bind(&agent_id_str)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("ekyc_sessions: {}", e),
                    })?;

                sqlx::query("DELETE FROM jobs WHERE agent_id = ?")
                    .bind(&agent_id_str)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("jobs: {}", e),
                    })?;

                sqlx::query("DELETE FROM guild_members WHERE agent_id = ?")
                    .bind(&agent_id_str)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("guild_members: {}", e),
                    })?;

                sqlx::query("DELETE FROM chat_history WHERE channel_id = ?")
                    .bind(&agent_id_str)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("chat_history: {}", e),
                    })?;

                sqlx::query("DELETE FROM chat_memory_summaries WHERE channel_id = ?")
                    .bind(&agent_id_str)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("chat_memory_summaries: {}", e),
                    })?;

                sqlx::query("DELETE FROM system_settings WHERE category = 'identity'")
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("system_settings: {}", e),
                    })?;

                sqlx::query("DELETE FROM security_audit WHERE agent_id = ?")
                    .bind(&agent_id_str)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("security_audit: {}", e),
                    })?;

                let record_id = uuid::Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('SYSTEM', 'FORGET_ACTOR', ?, ?, 'FORGET', 'FORGET')")
                    .bind(&record_id)
                    .bind(&agent_id_str)
                    .execute(&mut *tx).await.map_err(|e| AiomeError::Infrastructure { reason: format!("audit_ledger_global: {}", e) })?;

                tx.commit().await.map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;

                // ブロードキャスト
                if self
                    .event_bus
                    .send(SystemEvent::ActorForgotten(agent_id))
                    .is_err()
                {
                    tracing::warn!(
                        "[Security] No subscribers for ActorForgotten event (agent={})",
                        agent_id
                    );
                }

                Ok(())
            }
            DatabasePool::Postgres(p) => {
                let mut tx = p.begin().await.map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;

                sqlx::query("DELETE FROM ekyc_sessions WHERE user_id = $1")
                    .bind(&agent_id_str)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("ekyc_sessions: {}", e),
                    })?;

                sqlx::query("DELETE FROM jobs WHERE agent_id = $1")
                    .bind(&agent_id_str)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("jobs: {}", e),
                    })?;

                sqlx::query("DELETE FROM guild_members WHERE agent_id = $1")
                    .bind(&agent_id_str)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("guild_members: {}", e),
                    })?;

                sqlx::query("DELETE FROM chat_history WHERE channel_id = $1")
                    .bind(&agent_id_str)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("chat_history: {}", e),
                    })?;

                sqlx::query("DELETE FROM chat_memory_summaries WHERE channel_id = $1")
                    .bind(&agent_id_str)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("chat_memory_summaries: {}", e),
                    })?;

                sqlx::query("DELETE FROM system_settings WHERE category = 'identity'")
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("system_settings: {}", e),
                    })?;

                sqlx::query("DELETE FROM security_audit WHERE agent_id = $1")
                    .bind(&agent_id_str)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("security_audit: {}", e),
                    })?;

                let record_id = uuid::Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) VALUES ('SYSTEM', 'FORGET_ACTOR', $1, $2, 'FORGET', 'FORGET')")
                    .bind(&record_id)
                    .bind(&agent_id_str)
                    .execute(&mut *tx).await.map_err(|e| AiomeError::Infrastructure { reason: format!("audit_ledger_global: {}", e) })?;

                tx.commit().await.map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;

                // ブロードキャスト
                if self
                    .event_bus
                    .send(SystemEvent::ActorForgotten(agent_id))
                    .is_err()
                {
                    tracing::warn!(
                        "[Security] No subscribers for ActorForgotten event (agent={})",
                        agent_id
                    );
                }

                Ok(())
            }
        }
    }
}
