/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::db::DatabasePool;
use crate::sql_exec;
use aiome_core::error::AiomeError;
use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Structure representing a single audit log event to be asynchronously written to the ledger.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub table_name: String,
    pub operation: String,
    pub record_id: String,
    pub new_data: Value,
}

/// The asynchronous Audit Logger backed by an MPSC queue.
///
/// Instead of relying heavily on database triggers (like SQLite's traditional
/// strict synchronous execution), this system queues audit events in memory
/// and bulk-processes them through a dedicated background task, dramatically
/// reducing write-lock contention.
pub struct AsyncAuditLogger {
    sender: mpsc::Sender<AuditEntry>,
}

impl AsyncAuditLogger {
    /// Creates a new AsyncAuditLogger and spawns the background worker task.
    /// `queue_capacity` defines the maximum number of in-flight audit events
    /// before `log_event_sync` calls block or start dropping.
    pub fn new(pool: Arc<DatabasePool>, queue_capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel(queue_capacity);

        let pool_clone = pool.clone();
        tokio::spawn(async move {
            Self::audit_worker(pool_clone, rx).await;
        });

        Self { sender: tx }
    }

    /// Background worker that receives elements from the channel and persists them.
    async fn audit_worker(pool: Arc<DatabasePool>, mut rx: mpsc::Receiver<AuditEntry>) {
        info!("🛡️ [AsyncAuditLogger] Background worker started.");

        while let Some(entry) = rx.recv().await {
            // Write out the ledger item to the database pool.
            let now_str = Utc::now().to_rfc3339();

            // To ensure compatibility across both dialects (SQLite and PostgreSQL),
            // we leverage `sql_exec!` which uses the positional placeholders underneath.
            let q = format!(
                "INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash, timestamp)
                 VALUES ({0}, {1}, {2}, {3}, COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS'), hex(randomblob(16)), {4})",
                pool.ph(0), pool.ph(1), pool.ph(2), pool.ph(3), pool.ph(4)
            );

            let result = sql_exec!(
                &*pool,
                &q,
                &entry.table_name,
                &entry.operation,
                &entry.record_id,
                entry.new_data.to_string(),
                &now_str
            );

            match result {
                Ok(_) => debug!(
                    "✅ [AsyncAuditLogger] Recorded {} on {}",
                    entry.operation, entry.table_name
                ),
                Err(e) => error!("❌ [AsyncAuditLogger] Database insert failed: {}", e),
            }
        }

        info!("🛑 [AsyncAuditLogger] Sequence completed; channel closed.");
    }
}

use aiome_contracts::audit::AuditLogger;
use async_trait::async_trait;

#[async_trait]
impl AuditLogger for AsyncAuditLogger {
    async fn log_event(
        &self,
        event_type: &str,
        actor: &str,
        details: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let entry = AuditEntry {
            table_name: actor.to_string(),
            operation: event_type.to_string(),
            record_id: details
                .get("record_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            new_data: details.clone(),
        };
        self.sender
            .send(entry)
            .await
            .map_err(|e| anyhow::anyhow!("Audit logger queue is full or closed: {}", e))
    }

    async fn log_violation(
        &self,
        violation_type: &str,
        description: &str,
        context: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let entry = AuditEntry {
            table_name: "SECURITY_VIOLATION".to_string(),
            operation: violation_type.to_string(),
            record_id: description.to_string(),
            new_data: context.clone(),
        };
        self.sender
            .send(entry)
            .await
            .map_err(|e| anyhow::anyhow!("Audit logger queue is full or closed: {}", e))
    }
}
