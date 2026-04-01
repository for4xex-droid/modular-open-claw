/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use super::UniversalJobQueue;
use crate::db::DatabasePool;
use aiome_core::error::AiomeError;
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
}
