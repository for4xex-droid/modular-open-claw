/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use aiome_core_contracts::traits::JobStatus;
use infrastructure::db::DatabasePool;
use infrastructure::workflow::store::WorkflowStore;
use shared::watchtower::CoreEvent;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
enum ExecutionRecoveryAction {
    Finalize { failed: bool },
    Resume { remaining_job_ids: Vec<String> },
}

fn classify_execution_jobs(jobs: &[(String, JobStatus)]) -> ExecutionRecoveryAction {
    if jobs.is_empty() {
        return ExecutionRecoveryAction::Finalize { failed: true };
    }

    let mut any_failed = false;
    let mut remaining = Vec::new();
    for (job_id, status) in jobs {
        match status {
            JobStatus::Completed | JobStatus::Archived => {}
            JobStatus::Failed | JobStatus::Cancelled | JobStatus::Quarantined => {
                any_failed = true;
            }
            _ => remaining.push(job_id.clone()),
        }
    }

    if remaining.is_empty() {
        ExecutionRecoveryAction::Finalize { failed: any_failed }
    } else {
        ExecutionRecoveryAction::Resume {
            remaining_job_ids: remaining,
        }
    }
}

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

    /// api-server 再起動後、DB 上 Running のまま残った execution を orphan 検知して復旧する。
    pub async fn recover_orphan_executions(
        &self,
        db_pool: &DatabasePool,
    ) -> Result<(), aiome_core::error::AiomeError> {
        let store = WorkflowStore::new(db_pool.clone());
        let running = store.list_running_executions().await?;
        if running.is_empty() {
            return Ok(());
        }

        warn!(
            "♻️ Recovering {} workflow execution(s) left in Running state after restart",
            running.len()
        );

        for exec in running {
            let execution_id = match Uuid::parse_str(&exec.id) {
                Ok(id) => id,
                Err(e) => {
                    error!("Invalid workflow execution id '{}': {}", exec.id, e);
                    continue;
                }
            };
            let workflow_id = match Uuid::parse_str(&exec.workflow_id) {
                Ok(id) => id,
                Err(e) => {
                    error!(
                        "Invalid workflow_id '{}' for execution {}: {}",
                        exec.workflow_id, execution_id, e
                    );
                    continue;
                }
            };

            let raw_jobs = store.list_jobs_for_execution(execution_id).await?;
            let jobs: Vec<(String, JobStatus)> = raw_jobs
                .into_iter()
                .map(|(id, status)| (id, JobStatus::from_string(status)))
                .collect();

            match classify_execution_jobs(&jobs) {
                ExecutionRecoveryAction::Finalize { failed } => {
                    if let Err(e) = Self::finalize_execution(db_pool, execution_id, failed).await {
                        error!(
                            "Failed to finalize orphan workflow execution {}: {}",
                            execution_id, e
                        );
                    }
                }
                ExecutionRecoveryAction::Resume { remaining_job_ids } => {
                    info!(
                        "♻️ Re-registering workflow execution {} with {} pending job(s)",
                        execution_id,
                        remaining_job_ids.len()
                    );
                    self.register(execution_id, workflow_id, remaining_job_ids)
                        .await;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_execution_jobs_marks_empty_as_failed() {
        assert_eq!(
            classify_execution_jobs(&[]),
            ExecutionRecoveryAction::Finalize { failed: true }
        );
    }

    #[test]
    fn classify_execution_jobs_finalizes_when_all_terminal() {
        let jobs = vec![
            ("j1".to_string(), JobStatus::Completed),
            ("j2".to_string(), JobStatus::Completed),
        ];
        assert_eq!(
            classify_execution_jobs(&jobs),
            ExecutionRecoveryAction::Finalize { failed: false }
        );
    }

    #[test]
    fn classify_execution_jobs_resumes_pending_jobs() {
        let jobs = vec![
            ("j1".to_string(), JobStatus::Completed),
            ("j2".to_string(), JobStatus::InProgress),
        ];
        assert_eq!(
            classify_execution_jobs(&jobs),
            ExecutionRecoveryAction::Resume {
                remaining_job_ids: vec!["j2".to_string()],
            }
        );
    }

    #[test]
    fn classify_execution_jobs_marks_failed_when_any_job_failed() {
        let jobs = vec![
            ("j1".to_string(), JobStatus::Failed),
            ("j2".to_string(), JobStatus::Completed),
        ];
        assert_eq!(
            classify_execution_jobs(&jobs),
            ExecutionRecoveryAction::Finalize { failed: true }
        );
    }
}
