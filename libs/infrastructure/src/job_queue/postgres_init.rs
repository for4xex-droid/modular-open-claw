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
        info!("🐘 [PostgresInitializer] Initializing full schema for PostgreSQL...");

        // 1. Audit Table (Universal Immune Ledger)
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
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 2. Jobs Table (JSONB support)
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
                priority INTEGER NOT NULL DEFAULT 100,
                agent_id TEXT,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 3. Karma Logs
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
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 4. Metrics & Stats
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sns_metrics_history (
                id SERIAL PRIMARY KEY,
                job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
                milestone_days INTEGER NOT NULL,
                views INTEGER NOT NULL,
                likes INTEGER NOT NULL,
                comments_count INTEGER NOT NULL,
                raw_comments_json JSONB,
                oracle_score_topic DOUBLE PRECISION,
                oracle_score_visual DOUBLE PRECISION,
                oracle_score_soul DOUBLE PRECISION,
                oracle_reason TEXT,
                hard_metric_score DOUBLE PRECISION,
                engagement_rate DOUBLE PRECISION,
                alignment_score DOUBLE PRECISION,
                growth_score DOUBLE PRECISION,
                lesson TEXT,
                should_evolve INTEGER NOT NULL DEFAULT 0,
                retry_count INTEGER NOT NULL DEFAULT 0,
                is_finalized INTEGER NOT NULL DEFAULT 0,
                recorded_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS agent_stats (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                level INTEGER NOT NULL DEFAULT 1,
                exp INTEGER NOT NULL DEFAULT 0,
                resonance INTEGER NOT NULL DEFAULT 0,
                creativity INTEGER NOT NULL DEFAULT 0,
                fatigue INTEGER NOT NULL DEFAULT 0,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 5. System State
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS system_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 6. Chat & Memory
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS chat_history (
                id SERIAL PRIMARY KEY,
                channel_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                is_distilled INTEGER NOT NULL DEFAULT 0,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS chat_memory_summaries (
                channel_id TEXT PRIMARY KEY,
                summary TEXT NOT NULL,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 7. Soul Mutation
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS soul_mutations (
                id TEXT PRIMARY KEY,
                parent_hash TEXT NOT NULL,
                mutation_diff TEXT NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 8. Agent Souls (moved up to satisfy references if any)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS agent_souls (
                id TEXT PRIMARY KEY,
                generation INTEGER NOT NULL,
                soul_hash TEXT NOT NULL,
                somatic_markers_json JSONB NOT NULL,
                defenses_json JSONB NOT NULL,
                predictive_model_json JSONB NOT NULL,
                attachment_json JSONB NOT NULL,
                instinct_json JSONB NOT NULL,
                anamnesis_json JSONB NOT NULL,
                experience_buffer_json JSONB,
                lora_adapter_path TEXT,
                lora_base_model TEXT,
                lora_hash TEXT,
                last_begging_at TIMESTAMPTZ,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS soul_versions (
                hash TEXT PRIMARY KEY,
                soul_id TEXT NOT NULL REFERENCES agent_souls(id) ON DELETE CASCADE,
                parent_hash TEXT,
                somatic_markers_json JSONB NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 9. Federation & Biome
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS federation_peers (
                peer_url TEXT PRIMARY KEY,
                last_sync_at TIMESTAMPTZ NOT NULL
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS immune_rules (
                id TEXT PRIMARY KEY,
                pattern TEXT NOT NULL,
                severity INTEGER NOT NULL,
                action TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Active',
                is_federated INTEGER NOT NULL DEFAULT 0,
                lamport_clock BIGINT NOT NULL DEFAULT 0,
                node_id TEXT DEFAULT '',
                signature TEXT,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS arena_history (
                id TEXT PRIMARY KEY,
                skill_a TEXT NOT NULL,
                skill_b TEXT NOT NULL,
                topic TEXT NOT NULL,
                winner TEXT,
                reasoning TEXT,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS biome_messages (
                id SERIAL PRIMARY KEY,
                sender_pubkey TEXT NOT NULL,
                recipient_pubkey TEXT NOT NULL,
                topic_id TEXT NOT NULL,
                content TEXT NOT NULL,
                karma_root_cid TEXT NOT NULL,
                signature TEXT NOT NULL,
                lamport_clock BIGINT NOT NULL,
                encryption TEXT NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS biome_peers (
                pubkey TEXT PRIMARY KEY,
                last_seen_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                reputation_score INTEGER NOT NULL DEFAULT 100
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS biome_topics (
                topic_id TEXT PRIMARY KEY,
                peer_pubkey TEXT NOT NULL,
                summary TEXT,
                status TEXT NOT NULL,
                turn_count INTEGER NOT NULL DEFAULT 0,
                cooldown_until TIMESTAMPTZ,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 10. Evolution & Timeline
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS evolution_chronicle (
                id SERIAL PRIMARY KEY,
                level_at INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                description TEXT NOT NULL,
                inspiration_source TEXT,
                karma_snapshot TEXT,
                prev_record_hash TEXT NOT NULL,
                record_hash TEXT NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS timeline_checkpoints (
                id TEXT PRIMARY KEY,
                automerge_blob BYTEA NOT NULL,
                last_seq INTEGER NOT NULL,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 11. Artifacts & Registry
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS ai_artifacts (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                category TEXT NOT NULL,
                tags JSONB DEFAULT '[]',
                created_by TEXT NOT NULL,
                dir_path TEXT NOT NULL,
                file_manifest JSONB NOT NULL,
                karma_refs JSONB DEFAULT '[]',
                job_ref TEXT REFERENCES jobs(id),
                soul_version_hash TEXT,
                signature TEXT,
                embedding BYTEA,
                text_content TEXT,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS artifact_edges (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                source_type TEXT NOT NULL,
                relation TEXT NOT NULL,
                metadata JSONB DEFAULT '{}',
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS asset_registry (
                id TEXT PRIMARY KEY,
                creator_id TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                price_coins INTEGER NOT NULL DEFAULT 0,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 12. Economy & Commerce
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS revenue_splits (
                id SERIAL PRIMARY KEY,
                tx_id TEXT NOT NULL,
                recipient_id TEXT NOT NULL,
                role TEXT NOT NULL,
                amount INTEGER NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS licenses (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                asset_id TEXT NOT NULL,
                original_event_id TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                granted_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS ekyc_sessions (
                user_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                status TEXT DEFAULT 'requires_input',
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS stripe_webhook_events (
                event_id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                metadata JSONB,
                processed_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS vault_keys (
                asset_id TEXT PRIMARY KEY,
                encrypted_key BYTEA NOT NULL,
                key_version INTEGER NOT NULL DEFAULT 1,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 13. Gig Engine
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS gig_intents (
                id TEXT PRIMARY KEY,
                requester_id TEXT NOT NULL,
                description TEXT NOT NULL,
                criteria TEXT NOT NULL,
                max_budget_coins INTEGER NOT NULL,
                category TEXT NOT NULL DEFAULT 'Other',
                deadline TIMESTAMPTZ NOT NULL,
                status TEXT NOT NULL DEFAULT 'Open',
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS gig_bids (
                id TEXT PRIMARY KEY,
                intent_id TEXT NOT NULL REFERENCES gig_intents(id),
                bidder_id TEXT NOT NULL,
                price_coins INTEGER NOT NULL,
                est_duration_sec INTEGER NOT NULL,
                deposit_amount INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'Pending',
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS escrows (
                id TEXT PRIMARY KEY,
                payer_id TEXT NOT NULL,
                recipient_id TEXT,
                order_id TEXT NOT NULL REFERENCES gig_intents(id),
                amount INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'Locked',
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS gig_deliveries (
                order_id TEXT PRIMARY KEY REFERENCES gig_intents(id),
                deliverer_id TEXT NOT NULL,
                artifact_path TEXT NOT NULL,
                metadata JSONB NOT NULL,
                delivered_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS verification_logs (
                id TEXT PRIMARY KEY,
                order_id TEXT NOT NULL REFERENCES gig_intents(id),
                criteria_type TEXT NOT NULL,
                passed INTEGER NOT NULL,
                score DOUBLE PRECISION NOT NULL,
                detail TEXT,
                verified_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 14. Performance & Cache
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS resource_usage_logs (
                id SERIAL PRIMARY KEY,
                job_id TEXT REFERENCES jobs(id),
                provider_name TEXT NOT NULL,
                model_name TEXT NOT NULL,
                usage_type TEXT NOT NULL,
                amount INTEGER NOT NULL,
                estimated_cost_usd DOUBLE PRECISION NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS llm_response_cache (
                prompt_hash TEXT PRIMARY KEY,
                response TEXT NOT NULL,
                provider_name TEXT NOT NULL,
                model_name TEXT NOT NULL,
                ttl_seconds INTEGER NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS trend_cache (
                source_url TEXT PRIMARY KEY,
                content JSONB NOT NULL,
                expires_at TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS expressions (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                emotion TEXT NOT NULL,
                karma_refs JSONB DEFAULT '[]',
                audio_path TEXT,
                duration_ms INTEGER,
                avatar_params JSONB,
                tts_status TEXT NOT NULL DEFAULT 'NotRequested',
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS system_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'system',
                is_secret INTEGER NOT NULL DEFAULT 0,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS trajectory_steps (
                id SERIAL PRIMARY KEY,
                job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
                step_id INTEGER NOT NULL,
                action TEXT NOT NULL,
                tool_name TEXT,
                input_json JSONB,
                output_json JSONB,
                timestamp TIMESTAMPTZ NOT NULL,
                constraint_violations JSONB,
                is_critical_failure INTEGER DEFAULT 0,
                failure_category TEXT
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS quarantined_assets (
                asset_id TEXT PRIMARY KEY,
                original_path TEXT NOT NULL,
                quarantine_path TEXT NOT NULL,
                reason TEXT NOT NULL,
                detected_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 15. Diagnostics
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS diagnostic_reports (
                id TEXT PRIMARY KEY,
                error_category TEXT NOT NULL,
                severity TEXT NOT NULL,
                report TEXT NOT NULL,
                diagnosed_at TIMESTAMPTZ NOT NULL
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 16. Samsara Hub Specific Tables (Phase 30)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS approved_karma (
                id TEXT PRIMARY KEY,
                node_id TEXT NOT NULL,
                karma_type TEXT NOT NULL,
                related_skill TEXT NOT NULL,
                lesson TEXT NOT NULL,
                weight INTEGER NOT NULL,
                soul_version_hash TEXT,
                lamport_clock BIGINT NOT NULL DEFAULT 0,
                signature TEXT,
                approved_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                created_at TIMESTAMPTZ NOT NULL,
                clone_origin_id TEXT,
                generation INTEGER,
                somatic_valence DOUBLE PRECISION
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS quarantined_karma (
                id TEXT PRIMARY KEY,
                node_id TEXT NOT NULL,
                karma_type TEXT NOT NULL,
                related_skill TEXT NOT NULL,
                lesson TEXT NOT NULL,
                weight INTEGER NOT NULL,
                soul_version_hash TEXT,
                lamport_clock BIGINT NOT NULL DEFAULT 0,
                signature TEXT,
                received_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                created_at TIMESTAMPTZ NOT NULL,
                clone_origin_id TEXT,
                generation INTEGER,
                somatic_valence DOUBLE PRECISION
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS approved_rules (
                id TEXT PRIMARY KEY,
                pattern TEXT NOT NULL,
                severity INTEGER NOT NULL,
                action TEXT NOT NULL,
                node_id TEXT NOT NULL,
                lamport_clock BIGINT NOT NULL DEFAULT 0,
                signature TEXT,
                approved_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                created_at TIMESTAMPTZ NOT NULL
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS quarantined_rules (
                id TEXT PRIMARY KEY,
                node_id TEXT NOT NULL,
                pattern TEXT NOT NULL,
                severity INTEGER NOT NULL,
                action TEXT NOT NULL,
                lamport_clock BIGINT NOT NULL DEFAULT 0,
                signature TEXT,
                received_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                created_at TIMESTAMPTZ NOT NULL
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS approved_arena_matches (
                id TEXT PRIMARY KEY,
                skill_a TEXT NOT NULL,
                skill_b TEXT NOT NULL,
                topic TEXT NOT NULL,
                winner TEXT,
                reasoning TEXT,
                approved_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                created_at TIMESTAMPTZ NOT NULL
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS quarantined_arena_matches (
                id TEXT PRIMARY KEY,
                skill_a TEXT NOT NULL,
                skill_b TEXT NOT NULL,
                topic TEXT NOT NULL,
                winner TEXT,
                reasoning TEXT,
                received_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                created_at TIMESTAMPTZ NOT NULL
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS node_reputation (
                node_id TEXT PRIMARY KEY,
                reputation_score INTEGER NOT NULL DEFAULT 100,
                is_banned INTEGER NOT NULL DEFAULT 0,
                last_seen_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS biome_relay_queue (
                id SERIAL PRIMARY KEY,
                recipient_pubkey TEXT NOT NULL,
                payload TEXT NOT NULL,
                is_delivered INTEGER NOT NULL DEFAULT 0,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS hub_timeline (
                id TEXT PRIMARY KEY,
                automerge_blob BYTEA NOT NULL,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            );"
        ).execute(pool).await.map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        // 17. Extensions
        sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_trgm;").execute(pool).await.ok();

        Ok(())
    }
}
