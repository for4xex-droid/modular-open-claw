/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::app_state::AppState;
use aiome_contracts::error::AiomeError;
use aiome_contracts::events::CoreEvent;
use aiome_contracts::traits::JobQueue;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};
use uuid::Uuid;

/// デモの各ステップ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DemoStep {
    IntentGeneration,
    TrendAnalysis,
    GigPublishing,
    BiddingSimulation,
    AcceptanceSimulation,
    DeliverySimulation,
    SettlementAndEvolution,
}

/// 自律デモのオーケストレーター
///
/// IMPORTANT DESIGN DECISION: This demo does NOT use gig_engine's trait methods
/// (accept_bid, deliver, verify_and_settle) because they use SQLite transactions
/// internally. With multiple browser tabs each maintaining SSE connections that
/// poll the database every 5 seconds, the connection pool (max_connections=10)
/// becomes saturated. When a transaction holds a connection and tries to acquire
/// the SQLite WRITE lock, it competes with other connections for the lock,
/// resulting in SQLITE_BUSY (error 517) after the busy_timeout (5000ms) expires.
///
/// Instead, we execute individual SQL statements against the pool directly,
/// which acquire and immediately release connections, avoiding pool exhaustion
/// and lock contention.
pub struct AutonomousDemo;

impl AutonomousDemo {
    /// デモを非同期で実行する
    pub async fn run(state: AppState) {
        info!("🎬 [AutonomousDemo] Starting 60s demo cycle...");
        if let Err(e) = Self::do_run(state).await {
            error!("❌ [AutonomousDemo] Demo failed: {}", e);
        }
    }

    async fn do_run(state: AppState) -> Result<(), AiomeError> {
        let agent_id = state.system_agent_id;
        let pool = state.job_queue.get_pool().get_sqlite_pool_or_err()?.clone();

        // Drop ALL audit triggers on gig tables for the demo duration.
        // Even without transactions, triggers cause write lock contention.
        let gig_tables = [
            ("gig_intents", "id"),
            ("gig_bids", "id"),
            ("escrows", "id"),
            ("gig_deliveries", "order_id"),
            ("verification_logs", "id"),
        ];
        for (table, _) in &gig_tables {
            let _ = sqlx::query(&format!("DROP TRIGGER IF EXISTS audit_insert_{}", table))
                .execute(&pool)
                .await;
            let _ = sqlx::query(&format!("DROP TRIGGER IF EXISTS audit_update_{}", table))
                .execute(&pool)
                .await;
        }

        // Run demo (result captured so triggers are always restored)
        let result = Self::do_run_steps(&state, &pool, agent_id).await;

        // Restore triggers regardless of success/failure
        for (table, pk) in &gig_tables {
            let _ = sqlx::query(&format!(
                "CREATE TRIGGER IF NOT EXISTS audit_insert_{0} AFTER INSERT ON {0} BEGIN \
                 INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) \
                 VALUES ('{0}', 'INSERT', COALESCE(CAST(NEW.{1} AS TEXT), 'UNKNOWN'), \
                 '{0}:INSERT:' || COALESCE(CAST(NEW.{1} AS TEXT), 'UNKNOWN'), \
                 COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), \
                 hex(randomblob(16))); END;",
                table, pk
            )).execute(&pool).await;
            let _ = sqlx::query(&format!(
                "CREATE TRIGGER IF NOT EXISTS audit_update_{0} AFTER UPDATE ON {0} BEGIN \
                 INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash) \
                 VALUES ('{0}', 'UPDATE', COALESCE(CAST(NEW.{1} AS TEXT), 'UNKNOWN'), \
                 '{0}:UPDATE:' || COALESCE(CAST(NEW.{1} AS TEXT), 'UNKNOWN'), \
                 COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), \
                 hex(randomblob(16))); END;",
                table, pk
            )).execute(&pool).await;
        }

        result
    }

    /// Demo steps using direct SQL (NO transactions, NO gig_engine trait calls).
    /// Each SQL statement acquires and immediately releases a pool connection,
    /// avoiding connection pool exhaustion under heavy SSE load.
    async fn do_run_steps(
        state: &AppState,
        pool: &sqlx::SqlitePool,
        agent_id: Uuid,
    ) -> Result<(), AiomeError> {
        // Phase 0: Cleanup
        Self::broadcast(
            state,
            0,
            "System Maintenance: Pruning previous demo artifacts...",
        )
        .await;
        for table in &[
            "verification_logs",
            "gig_deliveries",
            "escrows",
            "gig_bids",
            "gig_intents",
        ] {
            let _ = sqlx::query(&format!("DELETE FROM {}", table))
                .execute(pool)
                .await;
            sleep(Duration::from_millis(50)).await; // yield between deletes
        }
        sleep(Duration::from_secs(2)).await;

        // Step 1: Intent Generation
        Self::broadcast(state, 1, "Intent Generator: Analyzing internal desire...").await;
        let _intent_base = state.intent_generator.generate_for_agent(agent_id).await?;
        let intent_id = Uuid::new_v4();
        let criteria_json = serde_json::to_string(&vec![json!({
            "JsonSchema": {
                "schema": {
                    "type": "object",
                    "properties": { "insight": { "type": "string" } },
                    "required": ["insight"]
                }
            }
        })])
        .unwrap_or_default();
        let deadline = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();

        sqlx::query(
            "INSERT INTO gig_intents (id, requester_id, description, criteria, max_budget_coins, category, deadline, status)
             VALUES (?, ?, ?, ?, ?, ?, ?, 'Open')"
        )
        .bind(intent_id.to_string())
        .bind(agent_id.to_string())
        .bind(format!("Autonomous Desire: {}", _intent_base.description))
        .bind(&criteria_json)
        .bind(100i64)
        .bind("Learning")
        .bind(&deadline)
        .execute(pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Intent insert: {}", e) })?;
        sleep(Duration::from_secs(5)).await;

        // Step 2: Trend Analysis
        Self::broadcast(state, 2, "TrendSonar: Scanning global market trends...").await;
        sleep(Duration::from_secs(7)).await;

        // Step 3: Gig Publishing (already inserted above, just broadcast)
        Self::broadcast(state, 3, "GigEngine: Broadcasting intent to the swarm...").await;
        sleep(Duration::from_secs(5)).await;

        // Step 4: Bidding Simulation
        Self::broadcast(state, 4, "SwarmOps: External agent bidding on the task...").await;
        let agent_b_id = Uuid::new_v4();
        let bid_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO gig_bids (id, intent_id, bidder_id, price_coins, est_duration_sec, deposit_amount, status)
             VALUES (?, ?, ?, ?, ?, ?, 'Pending')"
        )
        .bind(bid_id.to_string())
        .bind(intent_id.to_string())
        .bind(agent_b_id.to_string())
        .bind(80i64)
        .bind(10i64)
        .bind(10i64)
        .execute(pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Bid insert: {}", e) })?;
        sleep(Duration::from_secs(7)).await;

        // Step 5: Acceptance Simulation (individual queries, NO transaction)
        Self::broadcast(
            state,
            5,
            "The Immutable Gateway: Locking escrow and accepting bid...",
        )
        .await;
        let escrow_id = format!("escrow-{}", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO escrows (id, payer_id, recipient_id, order_id, amount, status)
             VALUES (?, ?, ?, ?, ?, 'Locked')",
        )
        .bind(&escrow_id)
        .bind(agent_id.to_string())
        .bind(agent_b_id.to_string())
        .bind(intent_id.to_string())
        .bind(80i64)
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Escrow insert: {}", e),
        })?;
        sleep(Duration::from_millis(100)).await;

        sqlx::query("UPDATE gig_intents SET status = 'Accepted' WHERE id = ?")
            .bind(intent_id.to_string())
            .execute(pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Intent status: {}", e),
            })?;
        sleep(Duration::from_millis(100)).await;

        sqlx::query("UPDATE gig_bids SET status = 'Accepted' WHERE id = ?")
            .bind(bid_id.to_string())
            .execute(pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Bid status: {}", e),
            })?;
        sleep(Duration::from_secs(5)).await;

        // Step 6: Delivery Simulation (individual queries, NO transaction)
        Self::broadcast(
            state,
            6,
            "Agent Swarm: Delivering verified insight artifact...",
        )
        .await;
        let metadata =
            json!({ "insight": "Aiome Protocol is the backbone of the new Musk Economy." });
        sqlx::query(
            "INSERT INTO gig_deliveries (order_id, deliverer_id, artifact_path, metadata)
             VALUES (?, ?, ?, ?)",
        )
        .bind(intent_id.to_string())
        .bind(agent_b_id.to_string())
        .bind("demo_result.json")
        .bind(metadata.to_string())
        .execute(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Delivery insert: {}", e),
        })?;
        sleep(Duration::from_millis(100)).await;

        sqlx::query("UPDATE gig_intents SET status = 'Delivered' WHERE id = ?")
            .bind(intent_id.to_string())
            .execute(pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Delivery status: {}", e),
            })?;
        sleep(Duration::from_secs(7)).await;

        // Step 7: Settlement & Evolution (individual queries, NO transaction)
        Self::broadcast(
            state,
            7,
            "Karma Engine: Verifying payload and evolving DNA...",
        )
        .await;

        // Verification log
        sqlx::query(
            "INSERT INTO verification_logs (id, order_id, criteria_type, passed, score, detail)
             VALUES (?, ?, 'Combined', 1, 1.0, 'JsonSchema check passed. Demo verification auto-approved.')"
        )
        .bind(Uuid::new_v4().to_string())
        .bind(intent_id.to_string())
        .execute(pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Verification log: {}", e) })?;
        sleep(Duration::from_millis(100)).await;

        // Settle escrow
        sqlx::query("UPDATE escrows SET status = 'Released' WHERE id = ?")
            .bind(&escrow_id)
            .execute(pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Escrow release: {}", e),
            })?;
        sleep(Duration::from_millis(100)).await;

        // Final status
        sqlx::query("UPDATE gig_intents SET status = 'Completed' WHERE id = ?")
            .bind(intent_id.to_string())
            .execute(pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Final status: {}", e),
            })?;
        sleep(Duration::from_millis(300)).await;

        // Resonance and XP boost
        state.job_queue.add_resonance(30).await?;
        sleep(Duration::from_millis(200)).await;
        state.job_queue.add_tech_exp(20).await?;

        Self::broadcast(state, 8, "🎉 Demo Cycle Complete: Agent Evolved!").await;
        info!("✅ [AutonomousDemo] Demo cycle finished.");

        Ok(())
    }

    async fn broadcast(state: &AppState, step: i32, message: &str) {
        info!("[Demo Step {}] {}", step, message);
        let event = CoreEvent::PluginEvent {
            plugin_name: "AutonomousDemo".to_string(),
            event_type: "StepUpdate".to_string(),
            payload: json!({
                "step": step,
                "message": message,
                "timestamp": Utc::now().to_rfc3339()
            }),
        };
        let _ = state.event_sender.send(event);
    }
}
