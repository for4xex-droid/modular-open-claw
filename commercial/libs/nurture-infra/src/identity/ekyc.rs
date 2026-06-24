/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use async_trait::async_trait;
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use nurture_bridge::db::DatabasePool;
use nurture_bridge::error::AiomeError;
use nurture_bridge::{sql_exec, sql_fetch_optional_map};
use sqlx::Row;

#[async_trait]
pub trait EkycStore: Send + Sync {
    async fn is_verified(&self, actor_id: &ActorId) -> Result<bool, NurtureError>;
    async fn set_verified(&self, actor_id: &ActorId, session_id: &str) -> Result<(), NurtureError>;
}

pub struct SQLiteEkycStore {
    pool: DatabasePool,
}

impl SQLiteEkycStore {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EkycStore for SQLiteEkycStore {
    async fn is_verified(&self, actor_id: &ActorId) -> Result<bool, NurtureError> {
        let status = sql_fetch_optional_map!(
            &self.pool,
            sqlite: "SELECT status FROM nurture_kyc_status WHERE actor_id = ?",
            |row| Ok::<String, AiomeError>(row.get("status")),
            pg: "SELECT status FROM nurture_kyc_status WHERE actor_id = $1",
            |row| Ok::<String, AiomeError>(row.get("status")),
            actor_id.0.to_string()
        )
        .map_err(|e| NurtureError::Infrastructure(format!("Database error in KYC check: {}", e)))?;

        Ok(status.is_some_and(|s| s == "verified"))
    }

    async fn set_verified(&self, actor_id: &ActorId, session_id: &str) -> Result<(), NurtureError> {
        sql_exec!(
            &self.pool,
            sqlite: "INSERT INTO nurture_kyc_status (actor_id, status, verified_at, stripe_session_id) 
                     VALUES (?, 'verified', CURRENT_TIMESTAMP, ?) 
                     ON CONFLICT(actor_id) DO UPDATE SET status = 'verified', stripe_session_id = excluded.stripe_session_id, verified_at = CURRENT_TIMESTAMP",
            pg: "INSERT INTO nurture_kyc_status (actor_id, status, verified_at, stripe_session_id) 
                     VALUES ($1, 'verified', CURRENT_TIMESTAMP, $2) 
                     ON CONFLICT(actor_id) DO UPDATE SET status = 'verified', stripe_session_id = excluded.stripe_session_id, verified_at = CURRENT_TIMESTAMP",
            actor_id.0.to_string(),
            session_id
        )
        .map(|_| ())
        .map_err(|e| NurtureError::Infrastructure(format!("Database error saving KYC status: {}", e)))
    }
}

#[cfg(any(test, debug_assertions))]
pub struct MockEkycStore {
    pub always_verified: bool,
}

#[cfg(any(test, debug_assertions))]
#[async_trait]
impl EkycStore for MockEkycStore {
    async fn is_verified(&self, _actor_id: &ActorId) -> Result<bool, NurtureError> {
        Ok(self.always_verified)
    }

    async fn set_verified(
        &self,
        _actor_id: &ActorId,
        _session_id: &str,
    ) -> Result<(), NurtureError> {
        Ok(())
    }
}
