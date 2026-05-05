/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use aiome_contracts::gig_metadata::GigMetadataUpdater;
use async_trait::async_trait;

#[derive(Clone)]
pub struct DbGigUpdater {
    pool: sqlx::SqlitePool,
}

impl DbGigUpdater {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GigMetadataUpdater for DbGigUpdater {
    async fn mark_as_verified(&self, skill_name: &str, oxp: u32) -> Result<(), String> {
        let update_query = r#"
            UPDATE ai_artifacts
            SET
                file_manifest = json_set(file_manifest, '$.oxilean_verified', true, '$.oxilean_oxp', ?2)
            WHERE
                id = ?1
        "#;

        sqlx::query(update_query)
            .bind(skill_name) // Currently using skill_name as artifact id/job_ref conceptually, this might need refinement depending on actual schema
            .bind(oxp)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to update gig metadata: {}", e))?;

        Ok(())
    }
}
