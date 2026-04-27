/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use sqlx::{Pool, Postgres};
use tracing::info;

pub struct PostgresInitializer;

impl PostgresInitializer {
    pub async fn init_db(pool: &Pool<Postgres>) -> Result<(), AiomeError> {
        info!("🐘 [PostgresInitializer] Running PostgreSQL migrations via sqlx::migrate!...");

        sqlx::migrate!("migrations/postgres")
            .run(pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to run Postgres migrations: {}", e),
            })?;

        // 17. Extensions
        if let Err(e) = sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_trgm;")
            .execute(pool)
            .await
        {
            tracing::warn!("Failed to create pg_trgm extension: {}", e);
        }

        Ok(())
    }
}
