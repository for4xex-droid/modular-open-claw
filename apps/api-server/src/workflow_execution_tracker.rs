/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use infrastructure::db::DatabasePool;
use infrastructure::workflow::store::WorkflowStore;
use shared::watchtower::CoreEvent;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug)]
struct PendingExecution {
    workflow_id: Uuid,
    remaining: HashSet<String>,
    failed: bool,
}

#[derive(Clone, Default)]
pub struct WorkflowExecutionTracker {
    inner: Arc<Mutex<HashMap<Uuid, PendingExecution>>>,
}

impl WorkflowExecutionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register(&self, execution_id: Uuid, workflow_id: Uuid, job_ids: Vec<String>) {
        let mut guard = self.inner.lock().await;
        guard.insert(
            execution_id,
            PendingExecution {
                workflow_id,
                remaining: job_ids.into_iter().collect(),
                failed: false,
            },
        );
    }

    async fn on_job_terminal(&self, job_id: &str, failed: bool) -> Option<(Uuid, bool)> {
        let mut guard = self.inner.lock().await;
        for (exec_id, pending) in guard.iter_mut() {
            if pending.remaining.remove(job_id) {
                if failed {
                    pending.failed = true;
                }
                let done = pending.remaining.is_empty();
                let exec_id = *exec_id;
                let workflow_id = pending.workflow_id;
                let failed = pending.failed;
                if done {
                    guard.remove(&exec_id);
                    return Some((exec_id, failed));
                }
                let _ = workflow_id;
                return None;
            }
        }
        None
    }

    pub fn spawn_listener(
        self: Arc<Self>,
        mut event_rx: broadcast::Receiver<CoreEvent>,
        db_pool: DatabasePool,
    ) {
        tokio::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(CoreEvent::TaskCompleted { job_id, .. }) => {
                        if let Some((exec_id, failed)) = self.on_job_terminal(&job_id, false).await
                        {
                            if let Err(e) =
                                Self::finalize_execution(&db_pool, exec_id, failed).await
                            {
                                error!("Failed to finalize workflow execution {}: {}", exec_id, e);
                            }
                        }
                    }
                    Ok(CoreEvent::TaskFailed { job_id, .. }) => {
                        if let Some((exec_id, failed)) = self.on_job_terminal(&job_id, true).await {
                            if let Err(e) =
                                Self::finalize_execution(&db_pool, exec_id, failed).await
                            {
                                error!("Failed to finalize workflow execution {}: {}", exec_id, e);
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WorkflowExecutionTracker lagged by {} events", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    async fn finalize_execution(
        db_pool: &DatabasePool,
        execution_id: Uuid,
        failed: bool,
    ) -> Result<(), aiome_core::error::AiomeError> {
        let store = WorkflowStore::new(db_pool.clone());
        let status = if failed { "Failed" } else { "Completed" };
        store
            .update_execution_status(execution_id, status, None)
            .await?;
        info!(
            "🧬 Workflow execution {} marked as {}",
            execution_id, status
        );
        Ok(())
    }
}
