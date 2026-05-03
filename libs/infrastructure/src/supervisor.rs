/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// A supervised background task
pub trait SupervisedTask: Send + Sync {
    fn name(&self) -> &'static str;
    fn run(&self, cancel_token: CancellationToken) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

pub struct TaskSupervisor {
    max_restarts: usize,
    restart_window_secs: u64,
}

impl TaskSupervisor {
    pub fn new(max_restarts: usize, restart_window_secs: u64) -> Self {
        Self {
            max_restarts,
            restart_window_secs,
        }
    }

    /// Spawns a task and supervises its lifecycle.
    pub fn spawn_supervised<T: SupervisedTask + 'static>(
        &self,
        task: T,
        cancel_token: CancellationToken,
    ) {
        let max_restarts = self.max_restarts;
        let base_backoff_secs = self.restart_window_secs; // Used as base for exponential backoff or simple delay
        let task = Arc::new(task); // To share across restarts

        tokio::spawn(async move {
            let mut restart_count = 0;

            loop {
                if cancel_token.is_cancelled() {
                    break;
                }

                let task_name = task.name();
                tracing::info!(
                    "🚀 [Supervisor] Starting task '{}' (Restart: {}/{})",
                    task_name,
                    restart_count,
                    max_restarts
                );

                let run_future = task.run(cancel_token.clone());

                // Monitor the task
                let handle = tokio::spawn(run_future);

                match handle.await {
                    Ok(_) => {
                        // Task completed normally. If it wasn't cancelled, it's unexpected for a background loop, but we handle it.
                        if cancel_token.is_cancelled() {
                            tracing::info!("🛑 [Supervisor] Task '{}' completed gracefully due to cancellation.", task_name);
                            break;
                        } else {
                            tracing::warn!(
                                "⚠️ [Supervisor] Task '{}' exited unexpectedly but cleanly.",
                                task_name
                            );
                        }
                    }
                    Err(e) => {
                        if e.is_panic() {
                            tracing::error!("💥 [Supervisor] Task '{}' PANICKED!", task_name);
                        } else if e.is_cancelled() {
                            tracing::info!(
                                "🛑 [Supervisor] Task '{}' was cancelled externally.",
                                task_name
                            );
                            break;
                        } else {
                            tracing::error!("❌ [Supervisor] Task '{}' failed: {}", task_name, e);
                        }
                    }
                }

                if cancel_token.is_cancelled() {
                    break;
                }

                restart_count += 1;
                if restart_count > max_restarts {
                    tracing::error!("🚨 [Supervisor] Task '{}' exceeded max restarts ({}). Triggering global shutdown...", task_name, max_restarts);
                    // Fail-Closed: Trigger global cancellation if a critical background task dies permanently.
                    cancel_token.cancel();
                    break;
                }

                // Exponential backoff: min(base * 2^(restart_count - 1), 60 seconds max)
                let backoff_multiplier = 2_u64.pow((restart_count - 1) as u32);
                let delay_secs = std::cmp::min(base_backoff_secs * backoff_multiplier, 60);

                tracing::warn!(
                    "⏳ [Supervisor] Waiting {} seconds before restarting task '{}'...",
                    delay_secs,
                    task_name
                );

                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)) => {}
                    _ = cancel_token.cancelled() => {
                        break;
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::time::Duration;

    struct PanickingTask {
        run_count: Arc<AtomicUsize>,
    }

    impl SupervisedTask for PanickingTask {
        fn name(&self) -> &'static str {
            "PanickingTask"
        }

        fn run(
            &self,
            _cancel_token: CancellationToken,
        ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
            let count = self.run_count.clone();
            Box::pin(async move {
                count.fetch_add(1, Ordering::SeqCst);
                panic!("Intentional panic for testing");
            })
        }
    }

    #[tokio::test]
    async fn test_supervisor_restarts_panicking_task_up_to_max() {
        let run_count = Arc::new(AtomicUsize::new(0));
        let task = PanickingTask {
            run_count: run_count.clone(),
        };

        // Window 0 for instant restarts during testing
        let supervisor = TaskSupervisor::new(3, 0);
        let cancel_token = CancellationToken::new();

        supervisor.spawn_supervised(task, cancel_token.clone());

        // Wait until it restarts up to max_restarts (1 initial + 3 restarts = 4)
        for _ in 0..50 {
            if run_count.load(Ordering::SeqCst) >= 4 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        assert_eq!(run_count.load(Ordering::SeqCst), 4);

        cancel_token.cancel();
    }

    #[tokio::test]
    async fn test_fail_closed_triggers_cancellation() {
        let run_count = Arc::new(AtomicUsize::new(0));
        let task = PanickingTask {
            run_count: run_count.clone(),
        };

        let supervisor = TaskSupervisor::new(2, 0);
        let cancel_token = CancellationToken::new();

        supervisor.spawn_supervised(task, cancel_token.clone());

        // Wait for it to exceed max_restarts (1 initial + 2 restarts = 3)
        // This should trigger cancel_token.cancel() inside the supervisor
        for _ in 0..50 {
            if cancel_token.is_cancelled() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        assert!(cancel_token.is_cancelled(), "Supervisor MUST trigger global cancellation upon exceeding max_restarts (Fail-Closed design)");
        assert_eq!(run_count.load(Ordering::SeqCst), 3);
    }
}
