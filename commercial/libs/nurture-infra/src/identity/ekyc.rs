/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use async_trait::async_trait;
use commerce_protocol::error::NurtureError;
use commerce_protocol::identity::ActorId;
use sqlx::SqlitePool;

#[async_trait]
pub trait EkycStore: Send + Sync {
    async fn is_verified(&self, actor_id: &ActorId) -> Result<bool, NurtureError>;
    async fn set_verified(&self, actor_id: &ActorId, session_id: &str) -> Result<(), NurtureError>;
}

pub struct SQLiteEkycStore {
    pool: SqlitePool,
}

impl SQLiteEkycStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EkycStore for SQLiteEkycStore {
    async fn is_verified(&self, actor_id: &ActorId) -> Result<bool, NurtureError> {
        let row = sqlx::query("SELECT status FROM nurture_kyc_status WHERE actor_id = ?")
            .bind(actor_id.0.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                NurtureError::Infrastructure(format!("Database error in KYC check: {}", e))
            })?;

        if let Some(r) = row {
            use sqlx::Row;
            let status: String = r.try_get("status").unwrap_or_else(|e| {
                tracing::warn!("KYC status column extraction failed (schema issue?): {}", e);
                "pending".to_string()
            });
            Ok(status == "verified")
        } else {
            Ok(false)
        }
    }

    async fn set_verified(&self, actor_id: &ActorId, session_id: &str) -> Result<(), NurtureError> {
        sqlx::query(
            "INSERT INTO nurture_kyc_status (actor_id, status, verified_at, stripe_session_id) 
             VALUES (?, 'verified', CURRENT_TIMESTAMP, ?) 
             ON CONFLICT(actor_id) DO UPDATE SET status = 'verified', stripe_session_id = excluded.stripe_session_id, verified_at = CURRENT_TIMESTAMP"
        )
        .bind(actor_id.0.to_string())
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| NurtureError::Infrastructure(format!("Database error saving KYC status: {}", e)))?;
        Ok(())
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
