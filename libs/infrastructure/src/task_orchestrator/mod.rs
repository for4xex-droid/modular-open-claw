/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::traits::{Job, JobQueue, JobStatus};
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info};

pub mod planner;

/// Task orchestration event. Provides observability (like cmux read-screen).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TaskEvent {
    Spawned {
        job_id: String,
        conductor_id: String,
    },
    Progress {
        job_id: String,
        conductor_id: String,
        message: String,
        percent: Option<u8>,
    },
    Completed {
        job_id: String,
        result: String,
    },
    Failed {
        job_id: String,
        error: String,
    },
}

/// A conductor that executes a specific type of task autonomously.
#[async_trait]
pub trait TaskConductor: Send + Sync {
    /// Human-readable name of the conductor
    fn conductor_name(&self) -> &str;

    /// Categories of tasks this conductor can handle
    fn capable_categories(&self) -> Vec<String>;

    /// Execute the task
    async fn conduct(
        &self,
        job: Job,
        progress_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<String, AiomeError>;

    /// Cancel the execution of a specific job
    async fn cancel(&self, _job_id: &str) -> Result<(), AiomeError> {
        Ok(()) // Default implementation: do nothing
    }
}

/// The Task Dispatcher (Manager). Monitors the JobQueue and dispatches tasks to Conductors.
pub struct TaskDispatcher {
    conductors: Vec<Arc<dyn TaskConductor>>,
    job_queue: Arc<dyn JobQueue>,
    event_tx: broadcast::Sender<TaskEvent>,
    poll_interval: Duration,
    core_event_tx: Option<broadcast::Sender<aiome_contracts::events::CoreEvent>>,
    active_jobs: Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>,
    >,
    tool_discovery: Option<Arc<dyn aiome_contracts::traits::ToolDiscoveryEngine>>,
    planner: Option<Arc<dyn aiome_contracts::traits::StrategicPlanner>>,
    validator: Option<Arc<dyn aiome_contracts::traits::ConstitutionalValidator>>,
    soul_path: Option<std::path::PathBuf>,
}

impl TaskDispatcher {
    /// Create a new TaskDispatcher
    pub fn new(
        job_queue: Arc<dyn JobQueue>,
        poll_interval: Duration,
        core_event_tx: Option<broadcast::Sender<aiome_contracts::events::CoreEvent>>,
        tool_discovery: Option<Arc<dyn aiome_contracts::traits::ToolDiscoveryEngine>>,
        planner: Option<Arc<dyn aiome_contracts::traits::StrategicPlanner>>,
        validator: Option<Arc<dyn aiome_contracts::traits::ConstitutionalValidator>>,
        soul_path: Option<std::path::PathBuf>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(256);

        let mut rx = event_tx.subscribe();
        if let Some(ctx) = core_event_tx.clone() {
            tokio::spawn(async move {
                while let Ok(event) = rx.recv().await {
                    let core_ev = match event {
                        TaskEvent::Progress {
                            job_id,
                            conductor_id,
                            message,
                            percent,
                        } => aiome_contracts::events::CoreEvent::TaskProgress {
                            job_id,
                            conductor_id,
                            message,
                            percent,
                        },
                        TaskEvent::Completed { job_id, result } => {
                            aiome_contracts::events::CoreEvent::TaskCompleted {
                                job_id,
                                result,
                                topic: String::new(), // Optional fields for standard generation
                                style: String::new(),
                                preview_url: None,
                            }
                        }
                        TaskEvent::Failed { job_id, error } => {
                            aiome_contracts::events::CoreEvent::TaskFailed { job_id, error }
                        }
                        _ => continue,
                    };
                    let _ = ctx.send(core_ev);
                }
            });
        }

        Self {
            conductors: Vec::new(),
            job_queue,
            event_tx,
            poll_interval,
            core_event_tx,
            active_jobs: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            tool_discovery,
            planner,
            validator,
            soul_path,
        }
    }

    /// Register a new Conductor
    pub fn register_conductor(&mut self, conductor: Arc<dyn TaskConductor>) {
        info!(
            "🔌 Registered TaskConductor: {}",
            conductor.conductor_name()
        );
        self.conductors.push(conductor);
    }

    /// Get a receiver subscribing to TaskEvents
    pub fn subscribe_events(&self) -> broadcast::Receiver<TaskEvent> {
        self.event_tx.subscribe()
    }

    /// Cancel a running job
    pub async fn cancel_job(&self, job_id: &str) -> Result<(), AiomeError> {
        info!("🛑 Attempting to cancel job: {}", job_id);

        // 1. Mark as cancelled in DB first to prevent re-pickup if it fails later
        self.job_queue.cancel_job(job_id).await?;

        // 2. Trigger task-level cancellation
        let mut active = self.active_jobs.write().await;
        if let Some(token) = active.remove(job_id) {
            token.cancel();

            // 3. Trigger conductor-level cleanup
            // We need to find which conductor was running it.
            // For now, we notify ALL conductors to cancel it if they have it.
            for conductor in &self.conductors {
                let _ = conductor.cancel(job_id).await;
            }
            Ok(())
        } else {
            // Not running locally, but maybe Pending?
            // In that case, the DB mark was enough.
            Ok(())
        }
    }

    /// Run the dispatch loop. This should be spawned as a background task.
    pub async fn run_dispatch_loop(&self) {
        info!("🚀 Starting TaskDispatcher loop...");
        loop {
            // Collect categories we can handle
            let categories: Vec<String> = self
                .conductors
                .iter()
                .flat_map(|c| c.capable_categories())
                .collect();

            let categories_refs: Vec<&str> = categories.iter().map(|s| s.as_str()).collect();

            match self.job_queue.dequeue(&categories_refs).await {
                Ok(Some(job)) => {
                    info!("📥 Dequeued job: {} (category: {})", job.id, job.category);

                    // --- Phase 13: Strategic Planning ---
                    if job.category == "Goal" {
                        if let Some(planner) = &self.planner {
                            info!(
                                "🎯 Goal detected for job {}. Starting Strategic Planning...",
                                job.id
                            );
                            if let Err(e) = self.process_goal_job(job.clone()).await {
                                error!("❌ Planning failed for job {}: {:?}", job.id, e);
                                let _ = self.job_queue.fail_job(&job.id, &e.to_string()).await;
                            }
                            continue; // Skip normal conduction for Goal
                        }
                    }

                    // Find a suitable conductor
                    if let Some(conductor) = self
                        .conductors
                        .iter()
                        .find(|c| c.capable_categories().contains(&job.category))
                    {
                        let job_id = job.id.clone();
                        let conductor_id = conductor.conductor_name().to_string();

                        // Send Spawned event
                        let _ = self.event_tx.send(TaskEvent::Spawned {
                            job_id: job_id.clone(),
                            conductor_id: conductor_id.clone(),
                        });

                        // Set up progress channel
                        let (progress_tx, mut progress_rx) = mpsc::channel(32);
                        let event_tx_clone = self.event_tx.clone();
                        let job_queue_clone = self.job_queue.clone();

                        // Spawn a listener for progress updates
                        tokio::spawn(async move {
                            while let Some(event) = progress_rx.recv().await {
                                let _ = event_tx_clone.send(event);
                            }
                        });

                        // Create cancellation token for this job
                        let cancellation_token = tokio_util::sync::CancellationToken::new();
                        let job_token = cancellation_token.clone();

                        {
                            let mut active = self.active_jobs.write().await;
                            active.insert(job_id.clone(), cancellation_token);
                        }

                        // Spawn the actual task conductor
                        let conductor_clone = conductor.clone();
                        let active_jobs_clone = self.active_jobs.clone();
                        tokio::spawn(async move {
                            tokio::select! {
                                _ = job_token.cancelled() => {
                                    info!("⏹️ Job {} was cancelled.", job_id);
                                    let _ = progress_tx.send(TaskEvent::Failed {
                                        job_id: job_id.clone(),
                                        error: "Cancelled by user".to_string(),
                                    }).await;
                                }
                                result = conductor_clone.conduct(job, progress_tx.clone()) => {
                                    match result {
                                        Ok(res) => {
                                            let _ = job_queue_clone.complete_job(&job_id, Some(&res)).await;
                                            let _ = progress_tx
                                                .send(TaskEvent::Completed {
                                                    job_id: job_id.clone(),
                                                    result: res,
                                                })
                                                .await;
                                        }
                                        Err(e) => {
                                            error!("Task {} failed: {:?}", job_id, e);
                                            let _ = job_queue_clone.fail_job(&job_id, &e.to_string()).await;
                                            let _ = progress_tx
                                                .send(TaskEvent::Failed {
                                                    job_id: job_id.clone(),
                                                    error: e.to_string(),
                                                })
                                                .await;
                                        }
                                    }
                                }
                            }

                            // Cleanup active job entry
                            let mut active = active_jobs_clone.write().await;
                            active.remove(&job_id);
                        });

                        // Validation Fix ③: Adaptive polling. Check again immediately if we found a job.
                        continue;
                    } else {
                        error!("Dequeued job {}, but no capable conductor found. This shouldn't happen due to dequeue filter.", job.id);
                        let _ = self
                            .job_queue
                            .fail_job(&job.id, "No capable conductor found")
                            .await;
                    }
                }
                Ok(None) => {
                    // No job found, sleep.
                    debug!("No jobs found. Sleeping for {:?}", self.poll_interval);
                }
                Err(e) => {
                    error!("Error dequeueing job: {:?}", e);
                }
            }

            tokio::time::sleep(self.poll_interval).await;
        }
    }

    async fn process_goal_job(&self, job: Job) -> Result<(), AiomeError> {
        let planner = self
            .planner
            .as_ref()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "StrategicPlanner is not configured".to_string(),
            })?;

        // 1. Plan the goal
        let context = json!({
            "job_id": job.id,
            "topic": job.topic,
        });

        // Use topic or karma_directives as the instruction
        let instruction = if job.topic.is_empty() {
            job.karma_directives.as_deref().unwrap_or("No instruction")
        } else {
            &job.topic
        };

        let steps: Vec<aiome_contracts::trajectory::TrajectoryStep> =
            planner.plan_goal(instruction, context).await?;

        // --- Phase 3: Constitutional Validation ---
        if let Some(validator) = &self.validator {
            info!(
                "⚖️ Validating plan for goal {} against constitution...",
                job.id
            );
            let plan_summary = steps
                .iter()
                .map(|s| format!("- {}", s.action))
                .collect::<Vec<_>>()
                .join("\n");

            let principles = if let Some(path) = &self.soul_path {
                tokio::fs::read_to_string(path)
                    .await
                    .unwrap_or_else(|_| "No principles found.".to_string())
            } else {
                "No principles found.".to_string()
            };

            if let Err(e) = validator
                .verify_constitutional(&plan_summary, &principles)
                .await
            {
                error!(
                    "🚨 Plan for goal {} REJECTED by ConstitutionalValidator: {:?}",
                    job.id, e
                );
                let _ = self
                    .job_queue
                    .fail_job(&job.id, &format!("Constitutional Violation: {}", e))
                    .await;
                return Err(e);
            }
            info!(
                "✅ Plan for goal {} PASSED constitutional validation.",
                job.id
            );
        }

        info!("📋 Goal {} decomposed into {} steps.", job.id, steps.len());

        // 2. Store steps and Enqueue sub-jobs
        for mut step in steps {
            step.job_id = Some(job.id.clone());
            self.job_queue.store_trajectory_step(step.clone()).await?;

            // Enqueue sub-job derived from the step
            info!("➕ Enqueueing sub-job: {} for goal {}", step.action, job.id);

            let directives = if step.input.is_null() {
                None
            } else {
                Some(step.input.to_string())
            };

            self.job_queue
                .enqueue(
                    "Task", // Execute as a normal task
                    &step.action,
                    "auto",
                    directives.as_deref(),
                    None, // Inherit permissions or use default
                    None, // agent_id
                    0,    // priority
                )
                .await?;
        }

        // 3. Complete the Goal job
        self.job_queue
            .complete_job(&job.id, Some("Planned and Decomposed"))
            .await?;

        // Notify via event
        let _ = self.event_tx.send(TaskEvent::Completed {
            job_id: job.id,
            result: "Goal planned successfully".to_string(),
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::error::AiomeError;
    use tokio::time::timeout;

    // A simple mock job queue for testing
    #[derive(Debug)]
    struct TestJobQueue {
        job_to_return: std::sync::Mutex<Option<Job>>,
        completed: std::sync::Mutex<bool>,
    }

    #[async_trait]
    impl JobQueue for TestJobQueue {
        async fn enqueue(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: Option<&str>,
            _: Option<aiome_contracts::security::PermissionManifest>,
            _: Option<uuid::Uuid>,
            _: i32,
        ) -> Result<String, AiomeError> {
            unimplemented!()
        }
        async fn dequeue(&self, categories: &[&str]) -> Result<Option<Job>, AiomeError> {
            if categories.contains(&"test_cat") {
                Ok(self.job_to_return.lock().unwrap().take())
            } else {
                Ok(None)
            }
        }
        async fn fetch_job(&self, _: &str) -> Result<Option<Job>, AiomeError> {
            unimplemented!()
        }
        async fn complete_job(&self, _: &str, _: Option<&str>) -> Result<(), AiomeError> {
            *self.completed.lock().unwrap() = true;
            Ok(())
        }
        async fn fail_job(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn cancel_job(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn reclaim_zombie_jobs(&self, _: i64) -> Result<u64, AiomeError> {
            unimplemented!()
        }
        async fn fetch_chat_history(
            &self,
            _: &str,
            _: i64,
        ) -> Result<Vec<serde_json::Value>, AiomeError> {
            unimplemented!()
        }
        async fn store_chat_message(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn get_chat_memory_summary(
            &self,
            _: &str,
        ) -> Result<Option<(String, Option<String>)>, AiomeError> {
            unimplemented!()
        }
        async fn update_chat_memory_summary(
            &self,
            _: &str,
            _: &str,
            _: Option<&str>,
        ) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn mark_chats_as_distilled(&self, _: &str, _: i64) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn set_creative_rating(&self, _: &str, _: i32) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn heartbeat_pulse(&self, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn store_execution_log(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn fetch_relevant_karma(
            &self,
            _: &str,
            _: &str,
            _: i64,
            _: &str,
        ) -> Result<aiome_contracts::KarmaSearchResult, AiomeError> {
            unimplemented!()
        }
        async fn store_karma(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn adjust_karma_weight(&self, _: &str, _: i32) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn karma_decay_sweep(&self) -> Result<u64, AiomeError> {
            unimplemented!()
        }
        async fn fetch_undistilled_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            unimplemented!()
        }
        async fn mark_karma_extracted(&self, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn purge_old_jobs(&self, _: i64) -> Result<u64, AiomeError> {
            unimplemented!()
        }
        async fn link_sns_data(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn fetch_jobs_for_evaluation(&self, _: i64, _: i64) -> Result<Vec<Job>, AiomeError> {
            unimplemented!()
        }
        async fn record_sns_metrics(
            &self,
            _: &str,
            _: i64,
            _: i64,
            _: i64,
            _: i64,
            _: Option<&str>,
        ) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn fetch_pending_evaluations(
            &self,
            _: i64,
        ) -> Result<Vec<aiome_contracts::traits::SnsMetricsRecord>, AiomeError> {
            unimplemented!()
        }
        async fn apply_final_verdict(
            &self,
            _: i64,
            _: aiome_contracts::contracts::OracleVerdict,
            _: &str,
        ) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn fetch_recent_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            unimplemented!()
        }
        async fn get_agent_stats(&self) -> Result<aiome_contracts::AgentStats, AiomeError> {
            unimplemented!()
        }
        async fn add_resonance(&self, _: i32) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn add_tech_exp(&self, _: i32) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn add_creativity(&self, _: i32) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn sync_samsara_level(
            &self,
        ) -> Result<Option<aiome_contracts::contracts::SamsaraEvent>, AiomeError> {
            unimplemented!()
        }
        async fn record_evolution_event(
            &self,
            _: i32,
            _: &str,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn fetch_evolution_history(
            &self,
            _: i64,
        ) -> Result<Vec<serde_json::Value>, AiomeError> {
            unimplemented!()
        }
        async fn get_pending_job_count(&self) -> Result<i64, AiomeError> {
            unimplemented!()
        }
        async fn get_job_count_since(
            &self,
            _: chrono::DateTime<chrono::Utc>,
        ) -> Result<i64, AiomeError> {
            unimplemented!()
        }
        async fn fetch_all_karma(&self, _: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
            unimplemented!()
        }
        async fn fetch_top_performing_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            unimplemented!()
        }
        async fn record_soul_mutation(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn fetch_job_retry_count(&self, _: &str) -> Result<i64, AiomeError> {
            unimplemented!()
        }
        async fn reset_job_retry_count(&self, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn increment_job_retry_count(&self, _: &str) -> Result<bool, AiomeError> {
            unimplemented!()
        }
        async fn fetch_unincorporated_karma(
            &self,
            _: i64,
            _: &str,
        ) -> Result<Vec<serde_json::Value>, AiomeError> {
            unimplemented!()
        }
        async fn mark_karma_as_incorporated(
            &self,
            _: Vec<String>,
            _: &str,
        ) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn store_immune_rule(
            &self,
            _: &aiome_contracts::contracts::ImmuneRule,
        ) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn delete_immune_rule(&self, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn fetch_active_immune_rules(
            &self,
        ) -> Result<Vec<aiome_contracts::contracts::ImmuneRule>, AiomeError> {
            unimplemented!()
        }
        async fn record_arena_match(
            &self,
            _: &aiome_contracts::contracts::ArenaMatch,
        ) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn get_immune_rules(
            &self,
        ) -> Result<Vec<aiome_contracts::contracts::ImmuneRule>, AiomeError> {
            unimplemented!()
        }
        async fn export_federated_data(
            &self,
            _: Option<&str>,
        ) -> Result<
            (
                Vec<aiome_contracts::traits::KarmaEntry>,
                Vec<aiome_contracts::contracts::ImmuneRule>,
                Vec<aiome_contracts::contracts::ArenaMatch>,
            ),
            AiomeError,
        > {
            unimplemented!()
        }
        async fn import_federated_data(
            &self,
            _: Vec<aiome_contracts::traits::KarmaEntry>,
            _: Vec<aiome_contracts::contracts::ImmuneRule>,
            _: Vec<aiome_contracts::contracts::ArenaMatch>,
        ) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn get_peer_sync_time(&self, _: &str) -> Result<Option<String>, AiomeError> {
            unimplemented!()
        }
        async fn update_peer_sync_time(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn get_node_id(&self) -> Result<String, AiomeError> {
            unimplemented!()
        }
        async fn fetch_unfederated_data(
            &self,
        ) -> Result<
            (
                Vec<aiome_contracts::traits::KarmaEntry>,
                Vec<aiome_contracts::contracts::ImmuneRule>,
            ),
            AiomeError,
        > {
            unimplemented!()
        }
        async fn mark_as_federated(
            &self,
            _: Vec<String>,
            _: Vec<String>,
        ) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn fetch_federated_metrics(
            &self,
        ) -> Result<aiome_contracts::contracts::FederatedMetrics, AiomeError> {
            unimplemented!()
        }
        async fn sign_swarm_payload(&self, _: &str) -> Result<String, AiomeError> {
            unimplemented!()
        }
        async fn sync_local_clock(&self, _: u64) -> Result<u64, AiomeError> {
            unimplemented!()
        }
        async fn tick_local_clock(&self) -> Result<u64, AiomeError> {
            unimplemented!()
        }
        async fn storage_gc(&self, _: f64) -> Result<u64, AiomeError> {
            unimplemented!()
        }
        async fn get_biome_topic_status(
            &self,
            _: &str,
        ) -> Result<Option<(i32, Option<String>)>, AiomeError> {
            unimplemented!()
        }
        async fn advance_biome_turn(&self, _: &str, _: i64) -> Result<i32, AiomeError> {
            unimplemented!()
        }
        async fn fetch_biome_messages(
            &self,
            _: &str,
            _: i64,
        ) -> Result<Vec<serde_json::Value>, AiomeError> {
            unimplemented!()
        }
        async fn store_biome_message(
            &self,
            _: &aiome_contracts::biome::BiomeMessage,
        ) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn update_biome_reputation(&self, _: &str, _: f64) -> Result<f64, AiomeError> {
            unimplemented!()
        }
        async fn archive_biome_topic(&self, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn fetch_relevant_karma_by_category(
            &self,
            _: &str,
            _: &str,
            _: i64,
        ) -> Result<aiome_contracts::KarmaSearchResult, AiomeError> {
            unimplemented!()
        }
        async fn get_system_agent_id(&self) -> Result<uuid::Uuid, AiomeError> {
            unimplemented!()
        }
        async fn store_expression(
            &self,
            _: &aiome_contracts::traits::Expression,
        ) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn fetch_expressions(
            &self,
            _: i64,
        ) -> Result<Vec<aiome_contracts::traits::Expression>, AiomeError> {
            unimplemented!()
        }
        async fn store_soul_fragment(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError> {
            unimplemented!()
        }
        async fn get_security_request_count(
            &self,
            _: Option<uuid::Uuid>,
        ) -> Result<u32, AiomeError> {
            unimplemented!()
        }
        async fn increment_security_request_count(
            &self,
            _: Option<uuid::Uuid>,
        ) -> Result<u32, AiomeError> {
            Ok(0)
        }
        async fn store_trajectory_step(
            &self,
            _: aiome_contracts::trajectory::TrajectoryStep,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_trajectory_steps(
            &self,
            _: &str,
        ) -> Result<Vec<aiome_contracts::trajectory::TrajectoryStep>, AiomeError> {
            Ok(Vec::new())
        }
    }

    struct TestConductor;
    #[async_trait]
    impl TaskConductor for TestConductor {
        fn conductor_name(&self) -> &str {
            "TestConductor"
        }
        fn capable_categories(&self) -> Vec<String> {
            vec!["test_cat".into()]
        }
        async fn conduct(
            &self,
            job: Job,
            progress_tx: mpsc::Sender<TaskEvent>,
        ) -> Result<String, AiomeError> {
            progress_tx
                .send(TaskEvent::Progress {
                    job_id: job.id.clone(),
                    conductor_id: "TestConductor".to_string(),
                    message: "Working...".into(),
                    percent: Some(50),
                })
                .await
                .unwrap();
            Ok("done".into())
        }
    }

    #[tokio::test]
    async fn test_dispatcher_event_flow() {
        let mut job = Job::default();
        job.id = "job-42".into();
        job.category = "test_cat".into();

        let queue = Arc::new(TestJobQueue {
            job_to_return: std::sync::Mutex::new(Some(job)),
            completed: std::sync::Mutex::new(false),
        });

        let mut dispatcher = TaskDispatcher::new(
            queue.clone(),
            Duration::from_millis(10),
            None,
            None,
            None,
            None,
            None,
        );
        dispatcher.register_conductor(Arc::new(TestConductor));

        let mut rx = dispatcher.subscribe_events();

        let handle = tokio::spawn(async move {
            dispatcher.run_dispatch_loop().await;
        });

        // Use timeout to assert events
        let event1 = timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(event1, TaskEvent::Spawned { .. }));

        let event2 = timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(event2, TaskEvent::Progress { .. }));

        let event3 = timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(event3, TaskEvent::Completed { .. }));

        // Check if queue complete function was called (this uses async spawned code, so give it a tiny delay)
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(*queue.completed.lock().unwrap());

        handle.abort();
    }
}
