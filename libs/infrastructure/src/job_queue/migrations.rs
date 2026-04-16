/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use async_trait::async_trait;
use sqlx::Row;
use tracing::{error, info, warn};

use super::UniversalJobQueue;

#[async_trait]
pub trait DbInitializer {
    async fn init_db(&self) -> Result<(), AiomeError>;
}

#[async_trait]
impl DbInitializer for UniversalJobQueue {
    /// The Immortal Samsara Schema (完全不可侵DDL)
    async fn init_db(&self) -> Result<(), AiomeError> {
        let pool = self
            .pool
            .get_sqlite_pool()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason:
                    "init_db for generic migrations assumes SQLite. Postgres uses postgres_init.rs."
                        .to_string(),
            })?;

        sqlx::migrate!("migrations/sqlite")
            .set_ignore_missing(true)
            .run(pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to run SQLite migrations: {}", e),
            })?;

        // Initialize default agent_stats row for SQLite
        sqlx::query(
            "INSERT OR IGNORE INTO agent_stats (id, level, exp, resonance, creativity, fatigue) VALUES (1, 1, 0, 0, 0, 0)"
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to initialize agent_stats default row: {}", e),
        })?;

        Ok(())
    }
}
