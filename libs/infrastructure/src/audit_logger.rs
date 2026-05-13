/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::db::DatabasePool;
use crate::sql_exec;
use aiome_core::error::AiomeError;
use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

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
        let rx_arc = Arc::new(tokio::sync::Mutex::new(rx));

        #[cfg(test)]
        let delay_secs = 0;
        #[cfg(not(test))]
        let delay_secs = 60;

        let supervisor = crate::supervisor::TaskSupervisor::new(10, delay_secs);
        let ct = tokio_util::sync::CancellationToken::new();

        struct AuditWorkerTask {
            pool: Arc<DatabasePool>,
            rx: Arc<tokio::sync::Mutex<mpsc::Receiver<AuditEntry>>>,
        }

        impl crate::supervisor::SupervisedTask for AuditWorkerTask {
            fn name(&self) -> &'static str {
                "AuditWorker"
            }
            fn run(
                &self,
                cancel_token: tokio_util::sync::CancellationToken,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
                let pool = self.pool.clone();
                let rx_clone = self.rx.clone();
                Box::pin(async move {
                    let mut rx_guard = rx_clone.lock().await;
                    AsyncAuditLogger::audit_worker_loop(pool, &mut rx_guard, cancel_token).await;
                })
            }
        }

        supervisor.spawn_supervised(
            AuditWorkerTask {
                pool: pool_clone,
                rx: rx_arc,
            },
            ct,
        );

        Self { sender: tx }
    }

    /// Background worker that receives elements from the channel and persists them.
    async fn audit_worker_loop(
        pool: Arc<DatabasePool>,
        rx: &mut mpsc::Receiver<AuditEntry>,
        cancel_token: tokio_util::sync::CancellationToken,
    ) {
        info!("🛡️ [AsyncAuditLogger] Background worker started.");

        use sha2::Digest;

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    info!("🛑 [AsyncAuditLogger] Cancellation requested.");
                    break;
                }
                msg = rx.recv() => {
                    match msg {
                        Some(entry) => {
                            // Write out the ledger item to the database pool.
                            let now_str = Utc::now().to_rfc3339();

                            // Merkle Chain: prev_hash || table_name || operation || record_id || new_data → SHA-256
                            let prev_q = "SELECT COALESCE((SELECT current_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1), 'GENESIS')";
                            let prev_hash: String = match &*pool {
                                DatabasePool::Sqlite(p) => sqlx::query_scalar(prev_q)
                                    .fetch_one(p)
                                    .await
                                    .unwrap_or_else(|_| "GENESIS".to_string()),
                                DatabasePool::Postgres(p) => sqlx::query_scalar(prev_q)
                                    .fetch_one(p)
                                    .await
                                    .unwrap_or_else(|_| "GENESIS".to_string()),
                            };

                            let entry_new_data_str = entry.new_data.to_string();
                            let hash_input = format!(
                                "{}|{}|{}|{}|{}",
                                prev_hash, entry.table_name, entry.operation, entry.record_id, entry_new_data_str
                            );
                            let current_hash = format!("{:x}", sha2::Sha256::digest(hash_input.as_bytes()));

                            // To ensure compatibility across both dialects (SQLite and PostgreSQL),
                            // we leverage `sql_exec!` which uses the positional placeholders underneath.
                            let q = format!(
                                "INSERT INTO audit_ledger_global (table_name, operation, record_id, new_data, prev_hash, current_hash, timestamp)
                                 VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6})",
                                pool.ph(0), pool.ph(1), pool.ph(2), pool.ph(3), pool.ph(4), pool.ph(5), pool.ph(6)
                            );

                            let result = sql_exec!(
                                &*pool,
                                &q,
                                &entry.table_name,
                                &entry.operation,
                                &entry.record_id,
                                &entry_new_data_str,
                                &prev_hash,
                                &current_hash,
                                &now_str
                            );

                            match result {
                                Ok(_) => debug!(
                                    "✅ [AsyncAuditLogger] Recorded {} on {}",
                                    entry.operation, entry.table_name
                                ),
                                Err(e) => {
                                    error!(
                                        "❌ [AsyncAuditLogger] Database insert failed for actor {}: {}. Payload: {}",
                                        entry.table_name, e, entry_new_data_str
                                    );
                                    // Fallback to DLQ file to avoid data loss and prevent panic
                                    let dlq_path = crate::security::GLOBAL_SECURITY_CONFIG.workspace_root.join("audit_dlq.jsonl");
                                    let dlq_json = serde_json::json!({
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                        "table": entry.table_name,
                                        "operation": entry.operation,
                                        "error": e.to_string(),
                                        "payload": entry.new_data,
                                    });
                                    let dlq_entry = format!("{}\n", dlq_json);
                                    use std::fs::OpenOptions;
                                    use std::io::Write;
                                    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&dlq_path) {
                                        if let Err(io_err) = file.write_all(dlq_entry.as_bytes()) {
                                            error!("CRITICAL: Failed to write to audit DLQ at {}: {}", dlq_path.display(), io_err);
                                        } else {
                                            warn!("Saved failed audit entry to DLQ: {}", dlq_path.display());
                                        }
                                    } else {
                                        error!("CRITICAL: Failed to open audit DLQ at {}", dlq_path.display());
                                    }
                                }
                            }
                        }
                        None => {
                            info!("🛑 [AsyncAuditLogger] Sequence completed; channel closed.");
                            break;
                        }
                    }
                }
            }
        }
    }
}

use aiome_core_contracts::audit::AuditLogger;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DatabasePool;
    use crate::sql_fetch_all;
    use aiome_core_contracts::audit::AuditLogger;
    use serde_json::json;
    use sha2::Digest;

    #[tokio::test]
    async fn test_audit_merkle_chain() {
        let pool = DatabasePool::new_sqlite(":memory:").await.unwrap();

        let schema = "CREATE TABLE audit_ledger_global (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            table_name TEXT,
            operation TEXT,
            record_id TEXT,
            new_data TEXT,
            prev_hash TEXT,
            current_hash TEXT,
            timestamp TEXT
        )";
        sql_exec!(&pool, schema).unwrap();

        let pool_arc = Arc::new(pool.clone());
        let logger = AsyncAuditLogger::new(pool_arc.clone(), 100);

        // First event
        logger
            .log_event(
                "TEST_OP",
                "test_table",
                &json!({"record_id": "1", "data": "A"}),
            )
            .await
            .unwrap();
        // Second event
        logger
            .log_event(
                "TEST_OP",
                "test_table",
                &json!({"record_id": "2", "data": "B"}),
            )
            .await
            .unwrap();

        // Wait for worker to process
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let rows: Vec<(String, String, String, String)> = sql_fetch_all!(
            &*pool_arc,
            (String, String, String, String),
            "SELECT new_data, prev_hash, current_hash, record_id FROM audit_ledger_global ORDER BY id ASC"
        ).unwrap();

        assert_eq!(rows.len(), 2, "Should have 2 audit records");

        let (data1, prev1, curr1, id1) = &rows[0];
        let (data2, prev2, curr2, id2) = &rows[1];

        assert_eq!(prev1, "GENESIS");
        assert_eq!(
            prev2, curr1,
            "Second record's prev_hash should match first's current_hash"
        );

        // Validate hash algorithm (SHA-256) with pipe-delimited inputs
        let hash_input1 = format!(
            "{}|{}|{}|{}|{}",
            "GENESIS", "test_table", "TEST_OP", id1, data1
        );
        let expected_hash1 = format!("{:x}", sha2::Sha256::digest(hash_input1.as_bytes()));

        let hash_input2 = format!("{}|{}|{}|{}|{}", curr1, "test_table", "TEST_OP", id2, data2);
        let expected_hash2 = format!("{:x}", sha2::Sha256::digest(hash_input2.as_bytes()));

        assert_eq!(curr1, &expected_hash1, "Hash mismatch for first record");
        assert_eq!(curr2, &expected_hash2, "Hash mismatch for second record");
    }

    #[tokio::test]
    async fn test_audit_merkle_chain_integrity_after_db_failure() {
        let pool = DatabasePool::new_sqlite(":memory:").await.unwrap();

        let schema = "CREATE TABLE audit_ledger_global (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            table_name TEXT,
            operation TEXT,
            record_id TEXT,
            new_data TEXT,
            prev_hash TEXT,
            current_hash TEXT,
            timestamp TEXT
        )";
        sql_exec!(&pool, schema).unwrap();

        let pool_arc = Arc::new(pool.clone());
        let logger = AsyncAuditLogger::new(pool_arc.clone(), 100);

        // 1. Initial successful event
        logger
            .log_event("OP1", "test", &json!({"record_id": "1"}))
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 2. Cause a transient failure (rename table)
        sql_exec!(
            &*pool_arc,
            "ALTER TABLE audit_ledger_global RENAME TO audit_ledger_global_temp"
        )
        .unwrap();

        // This will cause a DLQ fallback (table not found) — worker continues without panic
        let _ = logger
            .log_event("OP2_FAIL", "test", &json!({"record_id": "2"}))
            .await;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 3. Recover the table
        sql_exec!(
            &*pool_arc,
            "ALTER TABLE audit_ledger_global_temp RENAME TO audit_ledger_global"
        )
        .unwrap();

        // Wait for supervisor to restart the worker (using exponential backoff or immediate restart)
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // 4. Log after recovery
        logger
            .log_event("OP3_RECOVER", "test", &json!({"record_id": "3"}))
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let rows: Vec<(String, String, String)> = sql_fetch_all!(
            &*pool_arc,
            (String, String, String),
            "SELECT operation, prev_hash, current_hash FROM audit_ledger_global ORDER BY id ASC"
        )
        .unwrap();

        assert_eq!(
            rows.len(),
            2,
            "Should have exactly 2 successful records (OP1 and OP3)"
        );
        assert_eq!(rows[0].0, "OP1");
        assert_eq!(rows[0].1, "GENESIS");
        assert_eq!(rows[1].0, "OP3_RECOVER");
        assert_eq!(
            rows[1].1, rows[0].2,
            "Recovered record MUST link to OP1's hash to preserve Merkle Chain integrity"
        );
    }
}
