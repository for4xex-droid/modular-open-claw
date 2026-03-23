/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
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
        let pool = self.pool.get_sqlite_pool().expect(
            "init_db for generic migrations assumes SQLite. Postgres uses postgres_init.rs.",
        );

        // Essential Audit Infrastructure
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS audit_ledger_global (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                table_name TEXT NOT NULL,
                operation TEXT NOT NULL,
                record_id TEXT NOT NULL,
                new_data TEXT NOT NULL,
                prev_hash TEXT NOT NULL,
                current_hash TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create audit_ledger_global: {}", e),
        })?;

        let trigger_tables = vec![
            ("jobs", "id"),
            ("karma_logs", "id"),
            ("system_state", "key"),
            ("ai_artifacts", "id"),
            ("revenue_splits", "asset_id"),
            ("gig_intents", "id"),
            ("gig_bids", "id"),
            ("escrows", "id"),
            ("gig_deliveries", "order_id"),
        ];

        for (table, pk_col) in trigger_tables {
            // Drop old triggers to ensure we replace them with the new PK-aware logic.
            // This MUST happen before any INSERT/UPDATE on these tables to avoid "no such column: NEW.id" errors
            // from lingering faulty triggers in the DB.
            let _ = sqlx::query(&format!("DROP TRIGGER IF EXISTS audit_insert_{}", table))
                .execute(pool)
                .await;
            let _ = sqlx::query(&format!("DROP TRIGGER IF EXISTS audit_update_{}", table))
                .execute(pool)
                .await;

            let trigger_sql = format!(
                "CREATE TRIGGER IF NOT EXISTS audit_insert_{0}
                 AFTER INSERT ON {0}
                 BEGIN
                    INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash)
                    VALUES (
                        '{0}',
                        'INSERT',
                        COALESCE(CAST(NEW.{1} AS TEXT), 'UNKNOWN'),
                        '{0}:INSERT:' || COALESCE(CAST(NEW.{1} AS TEXT), 'UNKNOWN'),
                        COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'),
                        hex(randomblob(16))
                    );
                 END;",
                table, pk_col
            );
            let _ = sqlx::query(&trigger_sql).execute(pool).await;

            let update_sql = format!(
                "CREATE TRIGGER IF NOT EXISTS audit_update_{0}
                 AFTER UPDATE ON {0}
                 BEGIN
                    INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash)
                    VALUES (
                        '{0}',
                        'UPDATE',
                        COALESCE(CAST(NEW.{1} AS TEXT), 'UNKNOWN'),
                        '{0}:UPDATE:' || COALESCE(CAST(NEW.{1} AS TEXT), 'UNKNOWN'),
                        COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'),
                        hex(randomblob(16))
                    );
                 END;",
                table, pk_col
            );
            let _ = sqlx::query(&update_sql).execute(pool).await;
        }

        // Use CREATE TABLE IF NOT EXISTS to prevent data loss on restart.
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY, 
                category TEXT NOT NULL,
                topic TEXT NOT NULL,
                style_name TEXT NOT NULL, 
                karma_directives TEXT NOT NULL CHECK(json_valid(karma_directives)), 
                status TEXT NOT NULL CHECK(status IN ('Pending', 'Processing', 'Completed', 'Failed')),
                started_at TEXT, 
                last_heartbeat TEXT,
                tech_karma_extracted INTEGER NOT NULL DEFAULT 0, 
                creative_rating INTEGER CHECK(creative_rating IN (-1, 0, 1)), 
                execution_log TEXT,
                error_message TEXT,
                sns_platform TEXT,
                sns_content_id TEXT,
                published_at TEXT,
                output_artifacts TEXT,
                permission_manifest TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );"
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create jobs table: {}", e) })?;

        // Embedded Migrations
        for migration in [
            "ALTER TABLE jobs ADD COLUMN last_heartbeat TEXT",
            "ALTER TABLE jobs ADD COLUMN execution_log TEXT",
            "ALTER TABLE jobs ADD COLUMN sns_platform TEXT",
            "ALTER TABLE jobs ADD COLUMN sns_content_id TEXT",
            "ALTER TABLE jobs ADD COLUMN published_at TEXT",
            "ALTER TABLE jobs ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE jobs ADD COLUMN output_artifacts TEXT",
            "ALTER TABLE jobs ADD COLUMN permission_manifest TEXT",
        ] {
            if let Err(e) = sqlx::query(migration).execute(pool).await {
                let msg = e.to_string();
                if !msg.contains("duplicate column name") && !msg.contains("already exists") {
                    warn!(
                        "⚠️ [DbInitializer] Embedded migration failed ({}): {}",
                        migration, e
                    );
                }
            }
        }

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS karma_logs (
                id TEXT PRIMARY KEY,
                job_id TEXT, 
                karma_type TEXT NOT NULL CHECK(karma_type IN ('Technical', 'Creative', 'Synthesized')),
                related_skill TEXT NOT NULL, 
                lesson TEXT NOT NULL,        
                weight INTEGER NOT NULL DEFAULT 100 CHECK(weight BETWEEN 0 AND 100), 
                soul_version_hash TEXT,
                karma_embedding BLOB,
                is_federated INTEGER NOT NULL DEFAULT 0,
                clone_origin_id TEXT,
                last_applied_at TEXT DEFAULT (datetime('now')),
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY(job_id) REFERENCES jobs(id) ON DELETE SET NULL
            );"
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create karma_logs table: {}", e) })?;

        // Indices
        if let Err(e) = sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_jobs_status_started ON jobs(status, started_at);",
        )
        .execute(pool)
        .await
        {
            info!(
                "💡 [DbInitializer] Index idx_jobs_status_started setup (might already exist): {}",
                e
            );
        }

        if let Err(e) = sqlx::query("CREATE INDEX IF NOT EXISTS idx_karma_logs_skill_weight ON karma_logs(related_skill, weight DESC);")
            .execute(pool).await {
            info!("💡 [DbInitializer] Index idx_karma_logs_skill_weight setup (might already exist): {}", e);
        }

        // The Metrics Ledger
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sns_metrics_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id TEXT NOT NULL,
                milestone_days INTEGER NOT NULL,
                views INTEGER NOT NULL,
                likes INTEGER NOT NULL,
                comments_count INTEGER NOT NULL,
                raw_comments_json TEXT,
                oracle_score_topic REAL,
                oracle_score_visual REAL,
                oracle_score_soul REAL,
                oracle_reason TEXT,
                hard_metric_score REAL,
                engagement_rate REAL,
                is_finalized INTEGER NOT NULL DEFAULT 0,
                recorded_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY(job_id) REFERENCES jobs(id) ON DELETE CASCADE
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create sns_metrics_history: {}", e),
        })?;

        if let Err(e) = sqlx::query("CREATE INDEX IF NOT EXISTS idx_sns_metrics_job ON sns_metrics_history(job_id, milestone_days);")
            .execute(pool).await {
             info!("💡 [DbInitializer] Index idx_sns_metrics_job setup (might already exist): {}", e);
        }

        for migration in [
            "ALTER TABLE jobs ADD COLUMN category TEXT NOT NULL DEFAULT 'default'",
            "ALTER TABLE sns_metrics_history ADD COLUMN raw_comments_json TEXT",
            "ALTER TABLE sns_metrics_history ADD COLUMN is_finalized INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE sns_metrics_history ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE sns_metrics_history ADD COLUMN hard_metric_score REAL",
            "ALTER TABLE sns_metrics_history ADD COLUMN engagement_rate REAL",
            "ALTER TABLE sns_metrics_history ADD COLUMN alignment_score REAL",
            "ALTER TABLE sns_metrics_history ADD COLUMN growth_score REAL",
            "ALTER TABLE sns_metrics_history ADD COLUMN lesson TEXT",
            "ALTER TABLE sns_metrics_history ADD COLUMN should_evolve INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE karma_logs ADD COLUMN tier TEXT NOT NULL DEFAULT 'WARM' CHECK(tier IN ('HOT', 'WARM', 'COLD'))",
            "ALTER TABLE karma_logs ADD COLUMN apply_count INTEGER NOT NULL DEFAULT 0",
            "CREATE INDEX IF NOT EXISTS idx_karma_tier ON karma_logs(tier)",
            "ALTER TABLE karma_logs ADD COLUMN lamport_clock INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE karma_logs ADD COLUMN node_id TEXT DEFAULT ''",
            "ALTER TABLE karma_logs ADD COLUMN signature TEXT",
            "ALTER TABLE karma_logs ADD COLUMN soul_version_hash TEXT",
            "ALTER TABLE karma_logs ADD COLUMN karma_embedding BLOB",
            "ALTER TABLE karma_logs ADD COLUMN is_federated INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE karma_logs ADD COLUMN clone_origin_id TEXT",
            "ALTER TABLE immune_rules ADD COLUMN is_federated INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE immune_rules ADD COLUMN lamport_clock INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE immune_rules ADD COLUMN node_id TEXT DEFAULT ''",
            "ALTER TABLE immune_rules ADD COLUMN signature TEXT",
            "ALTER TABLE immune_rules ADD COLUMN status TEXT DEFAULT 'Active'",
            "ALTER TABLE jobs ADD COLUMN agent_id TEXT",
        ] {
            if let Err(e) = sqlx::query(migration).execute(pool).await {
                let msg = e.to_string();
                if !msg.contains("duplicate column name") && !msg.contains("already exists") {
                    warn!("⚠️ [DbInitializer] Secondary migration failed ({}): {}", migration, e);
                }
            }
        }

        // Agent Evolution Stats
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS agent_stats (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                level INTEGER NOT NULL DEFAULT 1,
                exp INTEGER NOT NULL DEFAULT 0,
                resonance INTEGER NOT NULL DEFAULT 0,
                creativity INTEGER NOT NULL DEFAULT 0,
                fatigue INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create agent_stats table: {}", e),
        })?;

        if let Err(e) = sqlx::query("INSERT OR IGNORE INTO agent_stats (id, level, exp, resonance, creativity, fatigue) VALUES (1, 1, 0, 0, 0, 0);")
            .execute(pool)
            .await {
            warn!("⚠️ [DbInitializer] Failed to ensure default agent_stats record: {}", e);
        }

        // System State
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS system_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create system_state table: {}", e),
        })?;

        if let Err(e) = sqlx::query(
            "INSERT OR IGNORE INTO system_state (key, value) VALUES ('logical_clock', '0')",
        )
        .execute(pool)
        .await
        {
            warn!(
                "⚠️ [DbInitializer] Failed to ensure logical_clock in system_state: {}",
                e
            );
        }

        // Chat History & Memory
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS chat_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                channel_id TEXT NOT NULL,
                role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
                content TEXT NOT NULL,
                is_distilled INTEGER NOT NULL DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create chat_history: {}", e),
        })?;

        if let Err(e) = sqlx::query("CREATE INDEX IF NOT EXISTS idx_chat_history_channel ON chat_history(channel_id, created_at DESC);")
            .execute(pool).await {
            info!("💡 [DbInitializer] Index idx_chat_history_channel setup: {}", e);
        }
        if let Err(e) = sqlx::query("CREATE INDEX IF NOT EXISTS idx_chat_history_undistilled ON chat_history(is_distilled) WHERE is_distilled = 0;")
            .execute(pool).await {
            info!("💡 [DbInitializer] Index idx_chat_history_undistilled setup: {}", e);
        }

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS chat_memory_summaries (
                channel_id TEXT PRIMARY KEY,
                summary TEXT NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create chat_memory_summaries: {}", e),
        })?;

        // Soul Mutation History
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS soul_mutation_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                old_hash TEXT NOT NULL,
                new_hash TEXT NOT NULL,
                mutation_reason TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create soul_mutation_history: {}", e),
        })?;

        // Federation Peers
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS federation_peers (
                peer_url TEXT PRIMARY KEY,
                last_sync_at TEXT NOT NULL
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create federation_peers: {}", e),
        })?;

        // Immune Rules & Arena History
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS immune_rules (
                id TEXT PRIMARY KEY,
                pattern TEXT NOT NULL,
                severity INTEGER NOT NULL,
                action TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Active',
                is_federated INTEGER NOT NULL DEFAULT 0,
                lamport_clock INTEGER NOT NULL DEFAULT 0,
                node_id TEXT DEFAULT '',
                signature TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create immune_rules: {}", e),
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS arena_history (
                id TEXT PRIMARY KEY,
                skill_a TEXT NOT NULL,
                skill_b TEXT NOT NULL,
                topic TEXT NOT NULL,
                winner TEXT,
                reasoning TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create arena_history: {}", e),
        })?;

        // Federated Indices (Phase 15 Hardening)
        if let Err(e) = sqlx::query("CREATE INDEX IF NOT EXISTS idx_karma_logs_federated ON karma_logs(is_federated) WHERE is_federated = 0;").execute(pool).await {
            info!("💡 [DbInitializer] Index idx_karma_logs_federated setup: {}", e);
        }
        if let Err(e) = sqlx::query("CREATE INDEX IF NOT EXISTS idx_immune_rules_federated ON immune_rules(is_federated) WHERE is_federated = 0;").execute(pool).await {
            info!("💡 [DbInitializer] Index idx_immune_rules_federated setup: {}", e);
        }
        if let Err(e) = sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_karma_lamport ON karma_logs(lamport_clock, node_id);",
        )
        .execute(pool)
        .await
        {
            info!("💡 [DbInitializer] Index idx_karma_lamport setup: {}", e);
        }
        if let Err(e) = sqlx::query("CREATE INDEX IF NOT EXISTS idx_immune_lamport ON immune_rules(lamport_clock, node_id);").execute(pool).await {
            info!("💡 [DbInitializer] Index idx_immune_lamport setup: {}", e);
        }

        // Biome Protocol (Phase 20)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS biome_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sender_pubkey TEXT NOT NULL,
                recipient_pubkey TEXT NOT NULL,
                topic_id TEXT NOT NULL,
                content TEXT NOT NULL,
                karma_root_cid TEXT NOT NULL,
                signature TEXT NOT NULL,
                lamport_clock INTEGER NOT NULL,
                encryption TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create biome_messages: {}", e),
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS biome_peers (
                pubkey TEXT PRIMARY KEY,
                last_seen_at TEXT DEFAULT (datetime('now')),
                reputation_score INTEGER NOT NULL DEFAULT 100
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create biome_peers: {}", e),
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS biome_topics (
                topic_id TEXT PRIMARY KEY,
                peer_pubkey TEXT NOT NULL,
                summary TEXT,
                status TEXT NOT NULL CHECK(status IN ('Active', 'Archived', 'Blocked')),
                turn_count INTEGER NOT NULL DEFAULT 0,
                cooldown_until TEXT,
                updated_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create biome_topics: {}", e),
        })?;

        // Evolution Chronicle (The Record of Growth)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS evolution_chronicle (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                level_at INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                description TEXT NOT NULL,
                inspiration_source TEXT,
                karma_snapshot TEXT,
                prev_record_hash TEXT NOT NULL,
                record_hash TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create evolution_chronicle: {}", e),
        })?;

        if let Err(e) = sqlx::query("CREATE INDEX IF NOT EXISTS idx_biome_messages_recipient ON biome_messages(recipient_pubkey);").execute(pool).await {
            warn!("⚠️ [DbInitializer] Index idx_biome_messages_recipient setup failed: {}", e);
        }

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS timeline_checkpoints (
                id TEXT PRIMARY KEY,
                automerge_blob BLOB NOT NULL,
                last_seq INTEGER NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create timeline_checkpoints: {}", e),
        })?;

        // Soul Engine Storage (v4)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS agent_souls (
                id TEXT PRIMARY KEY,
                generation INTEGER NOT NULL,
                soul_hash TEXT NOT NULL,
                somatic_markers_json TEXT NOT NULL,
                defenses_json TEXT NOT NULL,
                predictive_model_json TEXT NOT NULL,
                attachment_json TEXT NOT NULL,
                instinct_json TEXT NOT NULL,
                anamnesis_json TEXT NOT NULL DEFAULT '{}',
                experience_buffer_json TEXT, -- L3 Context
                lora_adapter_path TEXT,      -- G-13 LoRA path
                lora_base_model TEXT,        -- G-13 base model
                lora_hash TEXT,              -- Phase 10.1b LoRA hash
                updated_at DATETIME
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create agent_souls table: {}", e),
        })?;

        // Migration for Step 6: Anamnesis Profile & LoRA Adaptation (NG-8)
        for migration in [
            "ALTER TABLE agent_souls ADD COLUMN anamnesis_json TEXT NOT NULL DEFAULT '{}';",
            "ALTER TABLE agent_souls ADD COLUMN lora_adapter_path TEXT;",
            "ALTER TABLE agent_souls ADD COLUMN lora_base_model TEXT;",
            "ALTER TABLE agent_souls ADD COLUMN lora_hash TEXT;",
            "ALTER TABLE agent_souls ADD COLUMN last_begging_at TEXT;",
            "ALTER TABLE gig_intents ADD COLUMN category TEXT NOT NULL DEFAULT 'Other';",
        ] {
            if let Err(e) = sqlx::query(migration).execute(pool).await {
                let msg = e.to_string();
                if !msg.contains("duplicate column name") && !msg.contains("already exists") {
                    warn!("⚠️ [DbInitializer] Migration failed ({}): {}", migration, e);
                }
            }
        }

        // Memory Evolution Sprint 2: Procedural Forgetting
        if let Err(e) =
            sqlx::query("ALTER TABLE karma_logs ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0;")
                .execute(pool)
                .await
        {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") && !msg.contains("already exists") {
                warn!("⚠️ [DbInitializer] Migration is_archived failed: {}", e);
            }
        }
        if let Err(e) = sqlx::query("CREATE INDEX IF NOT EXISTS idx_karma_logs_active ON karma_logs(is_archived) WHERE is_archived = 0;").execute(pool).await {
            warn!("⚠️ [DbInitializer] Index idx_karma_logs_active setup failed: {}", e);
        }

        // Sprint 3-A: FTS5 (High-speed Text Search Layer)
        if let Err(e) = sqlx::query("CREATE VIRTUAL TABLE IF NOT EXISTS karma_fts USING fts5(lesson, content=karma_logs, content_rowid=rowid);").execute(pool).await {
            let msg = e.to_string();
            if !msg.contains("already exists") {
                warn!("⚠️ [DbInitializer] FTS5 setup failed: {}", e);
            }
        }
        // Synchronization Triggers
        for trigger in [
            "CREATE TRIGGER IF NOT EXISTS karma_fts_ai AFTER INSERT ON karma_logs BEGIN INSERT INTO karma_fts(rowid, lesson) VALUES (new.rowid, new.lesson); END;",
            "CREATE TRIGGER IF NOT EXISTS karma_fts_ad AFTER DELETE ON karma_logs BEGIN INSERT INTO karma_fts(karma_fts, rowid, lesson) VALUES('delete', old.rowid, old.lesson); END;",
            "CREATE TRIGGER IF NOT EXISTS karma_fts_au AFTER UPDATE OF lesson ON karma_logs BEGIN INSERT INTO karma_fts(karma_fts, rowid, lesson) VALUES('delete', old.rowid, old.lesson); INSERT INTO karma_fts(rowid, lesson) VALUES(new.rowid, new.lesson); END;",
        ] {
            if let Err(e) = sqlx::query(trigger).execute(pool).await {
                let msg = e.to_string();
                if !msg.contains("already exists") {
                    warn!("⚠️ [DbInitializer] FTS5 trigger setup failed ({}): {}", trigger, e);
                }
            }
        }

        // Sprint 3-B: Taxonomy (Hierarchical Classification)
        if let Err(e) =
            sqlx::query("ALTER TABLE karma_logs ADD COLUMN domain TEXT DEFAULT 'general';")
                .execute(pool)
                .await
        {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") && !msg.contains("already exists") {
                warn!("⚠️ [DbInitializer] Migration domain setup failed: {}", e);
            }
        }
        if let Err(e) = sqlx::query("ALTER TABLE karma_logs ADD COLUMN subtopic TEXT;")
            .execute(pool)
            .await
        {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") && !msg.contains("already exists") {
                warn!("⚠️ [DbInitializer] Migration subtopic setup failed: {}", e);
            }
        }
        if let Err(e) = sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_karma_taxonomy ON karma_logs(domain, related_skill);",
        )
        .execute(pool)
        .await
        {
            warn!(
                "⚠️ [DbInitializer] Index idx_karma_taxonomy setup failed: {}",
                e
            );
        }

        // AI Artifacts Storage System
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS ai_artifacts (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                category TEXT NOT NULL,
                tags TEXT DEFAULT '[]',
                created_by TEXT NOT NULL,
                dir_path TEXT NOT NULL,
                file_manifest TEXT NOT NULL,
                karma_refs TEXT DEFAULT '[]',
                job_ref TEXT,
                soul_version_hash TEXT,
                signature TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create ai_artifacts table: {}", e),
        })?;

        // Phase 1: Artifact Evolution (Memory Crystal)
        if let Err(e) = sqlx::query("ALTER TABLE ai_artifacts ADD COLUMN embedding BLOB;")
            .execute(pool)
            .await
        {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") && !msg.contains("already exists") {
                warn!("⚠️ [DbInitializer] Migration embedding failed: {}", e);
            }
        }
        if let Err(e) = sqlx::query("ALTER TABLE ai_artifacts ADD COLUMN text_content TEXT;")
            .execute(pool)
            .await
        {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") && !msg.contains("already exists") {
                warn!("⚠️ [DbInitializer] Migration text_content failed: {}", e);
            }
        }

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS artifact_edges (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                source_type TEXT NOT NULL, -- 'Artifact' or 'Karma'
                relation TEXT NOT NULL,    -- 'DerivedFrom', 'AssociatedWith'
                metadata TEXT DEFAULT '{}',
                created_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create artifact_edges table: {}", e),
        })?;

        if let Err(e) =
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_edge_source ON artifact_edges(source_id);")
                .execute(pool)
                .await
        {
            warn!(
                "⚠️ [DbInitializer] Index idx_edge_source setup failed: {}",
                e
            );
        }
        if let Err(e) =
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_edge_target ON artifact_edges(target_id);")
                .execute(pool)
                .await
        {
            warn!(
                "⚠️ [DbInitializer] Index idx_edge_target setup failed: {}",
                e
            );
        }

        // Phase 5: System Settings (Dashboard Connectivity)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS system_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'system',
                is_secret INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create system_settings table: {}", e),
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS expressions (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                emotion TEXT NOT NULL,
                karma_refs TEXT DEFAULT '[]',
                audio_path TEXT,        -- DP-9: TTS音声ファイルパス
                duration_ms INTEGER,    -- DP-9: 音声の長さ(ms)
                avatar_params TEXT,     -- Phase 7: Inochi2D/VRM 感情パラメータ
                created_at TEXT DEFAULT (datetime('now')),
                tts_status TEXT NOT NULL DEFAULT 'NotRequested'
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create expressions table: {}", e),
        })?;

        // DP-10: Resource Usage & Cost Monitoring
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS resource_usage_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id TEXT,
                provider_name TEXT NOT NULL,
                model_name TEXT NOT NULL,
                usage_type TEXT NOT NULL,
                amount INTEGER NOT NULL,
                estimated_cost_usd REAL NOT NULL,
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE SET NULL
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create resource_usage_logs table: {}", e),
        })?;

        // 既存の expressions テーブルにカラムがない場合は追加する (手動マイグレーション)
        let columns = sqlx::query("PRAGMA table_info(expressions)")
            .fetch_all(pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to read expressions table info: {}", e),
            })?;

        let has_audio_path = columns
            .iter()
            .any(|c| c.get::<String, _>("name") == "audio_path");
        if !has_audio_path {
            sqlx::query("ALTER TABLE expressions ADD COLUMN audio_path TEXT")
                .execute(pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to alter expressions table (audio_path): {}", e),
                })?;
            sqlx::query("ALTER TABLE expressions ADD COLUMN duration_ms INTEGER")
                .execute(pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to alter expressions table (duration_ms): {}", e),
                })?;
        }

        // Add avatar_params if not exists
        let has_avatar_params = columns
            .iter()
            .any(|c| c.get::<String, _>("name") == "avatar_params");
        if !has_avatar_params {
            sqlx::query("ALTER TABLE expressions ADD COLUMN avatar_params TEXT")
                .execute(pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to alter expressions table (avatar_params): {}", e),
                })?;
        }

        // Phase 10.1a: tts_status column
        let has_tts_status = columns
            .iter()
            .any(|c| c.get::<String, _>("name") == "tts_status");
        if !has_tts_status {
            sqlx::query("ALTER TABLE expressions ADD COLUMN tts_status TEXT NOT NULL DEFAULT 'NotRequested'")
                .execute(pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to alter expressions table (tts_status): {}", e),
                })?;
        }

        // v5: AgentRx Diagnostics (Trajectory Tracking)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS trajectory_steps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id TEXT NOT NULL,
                step_id INTEGER NOT NULL,
                action TEXT NOT NULL,
                tool_name TEXT,
                input_json TEXT,
                output_json TEXT,
                timestamp TEXT NOT NULL,
                constraint_violations TEXT,  -- JSON array
                is_critical_failure INTEGER DEFAULT 0,
                failure_category TEXT,
                FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create trajectory_steps table: {}", e),
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS agent_diagnoses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id TEXT NOT NULL UNIQUE,
                critical_failure_step INTEGER,
                failure_category TEXT,
                root_cause TEXT,
                evidence TEXT,           -- JSON: 制約違反の証拠
                self_repair_hint TEXT,   -- 自己修復のためのヒント
                diagnosed_at TEXT NOT NULL,
                FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create agent_diagnoses table: {}", e),
        })?;

        // V6: Universal Immune Ledger (Solves R4-V1 - Immutable Hash Chain across all tables)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS audit_ledger_global (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                table_name TEXT NOT NULL,
                operation TEXT NOT NULL,
                record_id TEXT NOT NULL,
                new_data TEXT NOT NULL,
                prev_hash TEXT NOT NULL,
                current_hash TEXT NOT NULL,
                timestamp TEXT DEFAULT (datetime('now'))
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create audit_ledger_global table: {}", e),
        })?;

        // Add index on audit_ledger_global
        if let Err(e) = sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_audit_ledger_time ON audit_ledger_global(timestamp);",
        )
        .execute(pool)
        .await
        {
            warn!(
                "⚠️ [DbInitializer] Failed to create idx_audit_ledger_time: {}",
                e
            );
        }

        // Phase 10.2: Voice Commerce Webhook Idempotency (Gate 2)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS stripe_webhook_events (
                event_id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                metadata TEXT,
                processed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create stripe_webhook_events table: {}", e),
        })?;

        // Phase 10.2: Vault Key Persistence (§CISO-1)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS vault_keys (
                asset_id TEXT PRIMARY KEY,
                encrypted_key BLOB NOT NULL, -- Master key で暗号化されたアセット復号鍵
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create vault_keys table: {}", e),
        })?;

        // Migration to add metadata column if missing
        if let Err(e) = sqlx::query("ALTER TABLE stripe_webhook_events ADD COLUMN metadata TEXT;")
            .execute(pool)
            .await
        {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") && !msg.contains("already exists") {
                warn!(
                    "⚠️ [DbInitializer] Migration stripe_webhook_events metadata failed: {}",
                    e
                );
            }
        }

        // Phase 10: Asset Registry
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS asset_registry (
                id TEXT PRIMARY KEY,
                creator_id TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                price_coins INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create asset_registry table: {}", e),
        })?;

        // Phase 16.5: Revenue Splits
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS revenue_splits (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tx_id TEXT NOT NULL,
                recipient_id TEXT NOT NULL,
                role TEXT NOT NULL, -- 'creator', 'platform'
                amount INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create revenue_splits table: {}", e),
        })?;

        // Phase 11: Voice DRM & Economy Ledger (Gate 4 Patch)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS licenses (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                asset_id TEXT NOT NULL,
                original_event_id TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                granted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create licenses table: {}", e),
        })?;

        // Phase 14: eKYC Session Persistence
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS ekyc_sessions (
                user_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                status TEXT DEFAULT 'requires_input',
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create ekyc_sessions table: {}", e),
        })?;

        // Phase 22: Gig Engine (Autonomous Gig Economy)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS gig_intents (
                id TEXT PRIMARY KEY,
                requester_id TEXT NOT NULL,
                description TEXT NOT NULL,
                criteria TEXT NOT NULL,
                max_budget_coins INTEGER NOT NULL,
                category TEXT NOT NULL DEFAULT 'Other',
                deadline TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Open',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create gig_intents table: {}", e),
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS gig_bids (
                id TEXT PRIMARY KEY,
                intent_id TEXT NOT NULL REFERENCES gig_intents(id),
                bidder_id TEXT NOT NULL,
                price_coins INTEGER NOT NULL,
                est_duration_sec INTEGER NOT NULL,
                deposit_amount INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'Pending',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create gig_bids table: {}", e),
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS escrows (
                id TEXT PRIMARY KEY,
                payer_id TEXT NOT NULL,
                recipient_id TEXT,
                order_id TEXT NOT NULL REFERENCES gig_intents(id),
                amount INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'Locked',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create escrows table: {}", e),
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS gig_deliveries (
                order_id TEXT PRIMARY KEY REFERENCES gig_intents(id),
                deliverer_id TEXT NOT NULL,
                artifact_path TEXT NOT NULL,
                metadata TEXT NOT NULL,
                delivered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create gig_deliveries table: {}", e),
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS verification_logs (
                id TEXT PRIMARY KEY,
                order_id TEXT NOT NULL REFERENCES gig_intents(id),
                criteria_type TEXT NOT NULL,
                passed INTEGER NOT NULL,
                score REAL NOT NULL,
                detail TEXT,
                verified_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create verification_logs table: {}", e),
        })?;

        // Add key_version for Master Key rotation
        if let Err(e) =
            sqlx::query("ALTER TABLE vault_keys ADD COLUMN key_version INTEGER NOT NULL DEFAULT 1;")
                .execute(pool)
                .await
        {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") && !msg.contains("already exists") {
                warn!(
                    "⚠️ [DbInitializer] Migration vault_keys key_version failed: {}",
                    e
                );
            }
        }

        // --- Integrated Planning: Trend Fountain & Cost Control (Day 1) ---
        // 1. LLM Response Cache (Semantic Cache Table)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS llm_response_cache (
                prompt_hash TEXT PRIMARY KEY,
                response TEXT NOT NULL,
                provider_name TEXT NOT NULL,
                model_name TEXT NOT NULL,
                ttl_seconds INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create llm_response_cache table: {}", e),
        })?;

        // 2. Trend Fountain (L2 Cache Table)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS trend_cache (
                source_url TEXT PRIMARY KEY,
                content TEXT NOT NULL, -- JSON serialized TrendOutput
                expires_at DATETIME NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create trend_cache table: {}", e),
        })?;

        // 3. Job Priority (Day 5-6 Planning, but Migration here)
        if let Err(e) =
            sqlx::query("ALTER TABLE jobs ADD COLUMN priority INTEGER NOT NULL DEFAULT 100;")
                .execute(pool)
                .await
        {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") && !msg.contains("already exists") {
                warn!("⚠️ [DbInitializer] Migration jobs priority failed: {}", e);
            }
        }

        info!("✅ [SqliteJobQueue] Database and migrations initialized successfully.");
        Ok(())
    }
}
