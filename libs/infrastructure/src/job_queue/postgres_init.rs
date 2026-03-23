/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::error::AiomeError;
use sqlx::{Pool, Postgres};
use tracing::info;

pub struct PostgresInitializer;

impl PostgresInitializer {
    pub async fn init_db(pool: &Pool<Postgres>) -> Result<(), AiomeError> {
        info!("🐘 [PostgresInitializer] Initializing schema...");

        // 1. Audit Table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS audit_ledger_global (
                id SERIAL PRIMARY KEY,
                table_name TEXT NOT NULL,
                operation TEXT NOT NULL,
                record_id TEXT NOT NULL,
                new_data TEXT NOT NULL,
                prev_hash TEXT NOT NULL,
                current_hash TEXT NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        // 2. Jobs Table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                category TEXT NOT NULL DEFAULT 'default',
                topic TEXT NOT NULL,
                style_name TEXT NOT NULL,
                karma_directives JSONB NOT NULL,
                status TEXT NOT NULL,
                started_at TIMESTAMPTZ,
                last_heartbeat TIMESTAMPTZ,
                tech_karma_extracted INTEGER NOT NULL DEFAULT 0,
                creative_rating INTEGER,
                execution_log TEXT,
                error_message TEXT,
                sns_platform TEXT,
                sns_content_id TEXT,
                published_at TIMESTAMPTZ,
                output_artifacts JSONB,
                permission_manifest JSONB,
                retry_count INTEGER NOT NULL DEFAULT 0,
                agent_id TEXT,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        // 3. Karma Logs (using pgvector later if needed)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS karma_logs (
                id TEXT PRIMARY KEY,
                job_id TEXT REFERENCES jobs(id) ON DELETE SET NULL,
                karma_type TEXT NOT NULL,
                related_skill TEXT NOT NULL,
                lesson TEXT NOT NULL,
                weight INTEGER NOT NULL DEFAULT 100,
                soul_version_hash TEXT,
                karma_embedding BYTEA,
                is_federated INTEGER NOT NULL DEFAULT 0,
                clone_origin_id TEXT,
                domain TEXT DEFAULT 'general',
                subtopic TEXT,
                tier TEXT NOT NULL DEFAULT 'WARM',
                apply_count INTEGER NOT NULL DEFAULT 0,
                lamport_clock BIGINT NOT NULL DEFAULT 0,
                node_id TEXT DEFAULT '',
                signature TEXT,
                is_archived INTEGER NOT NULL DEFAULT 0,
                last_applied_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        // ... Add more tables from SQLite migrations here ...
        // Note: For now we implement core tables to verify connectivity.

        Ok(())
    }
}
