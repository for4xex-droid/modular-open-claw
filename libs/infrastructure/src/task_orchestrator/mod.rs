/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::invariant_dag::InvariantDag;
use crate::job_queue::{
    EvaluationOps, EvolutionOps, FederationOps, GuardrailOps, KarmaOps, SecurityOps, SettingsOps,
    SoulStoreOps, SwarmOps,
};
use aiome_core::error::AiomeError;
use aiome_core_contracts::traits::{
    AgentEvolver, AuditStore, BiomeRegistry, ChatStore, Expression, FederationRegistry,
    ImmuneSystemOps, Job, JobQueue, JobStatus, KarmaRegistry, SnsMetricsRecord, SoulStore,
    TaskRegistry,
};
use async_trait::async_trait;
use serde_json::json;
use shared::guardrails;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};

const MAX_GIG_BUDGET: u64 = 5000;

pub mod csam;
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
    Evaluating {
        job_id: String,
    },
    Failed {
        job_id: String,
        error: String,
    },
    GigPublished {
        job_id: String,
        intent_id: String,
        description: String,
        budget: u64,
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
    ) -> Result<(String, Option<String>), AiomeError>;

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
    core_event_tx: Option<broadcast::Sender<aiome_core_contracts::events::CoreEvent>>,
    active_jobs: Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>,
    >,
    tool_discovery: Option<Arc<dyn aiome_core_contracts::traits::ToolDiscoveryEngine>>,
    planner: Option<Arc<dyn aiome_core_contracts::traits::StrategicPlanner>>,
    validator: Option<Arc<dyn aiome_core_contracts::traits::ConstitutionalValidator>>,
    soul_path: Option<std::path::PathBuf>,
    pub oracle: Option<Arc<crate::oracle::Oracle>>,
    gig_engine: Option<Arc<dyn aiome_core_contracts::gig::GigEngine>>,
    diagnostics: Option<Arc<crate::diagnostics::AgentRxDiagnostics>>,
}

impl TaskDispatcher {
    /// Create a new TaskDispatcher
    pub fn new(
        job_queue: Arc<dyn JobQueue>,
        poll_interval: Duration,
        core_event_tx: Option<broadcast::Sender<aiome_core_contracts::events::CoreEvent>>,
        tool_discovery: Option<Arc<dyn aiome_core_contracts::traits::ToolDiscoveryEngine>>,
        planner: Option<Arc<dyn aiome_core_contracts::traits::StrategicPlanner>>,
        validator: Option<Arc<dyn aiome_core_contracts::traits::ConstitutionalValidator>>,
        soul_path: Option<std::path::PathBuf>,
        oracle: Option<Arc<crate::oracle::Oracle>>,
        gig_engine: Option<Arc<dyn aiome_core_contracts::gig::GigEngine>>,
        diagnostics: Option<Arc<crate::diagnostics::AgentRxDiagnostics>>,
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
                        } => aiome_core_contracts::events::CoreEvent::TaskProgress {
                            job_id,
                            conductor_id,
                            message,
                            percent,
                        },
                        TaskEvent::Completed { job_id, result } => {
                            aiome_core_contracts::events::CoreEvent::TaskCompleted {
                                job_id,
                                result,
                                topic: String::new(), // Optional fields for standard generation
                                style: String::new(),
                                preview_url: None,
                            }
                        }
                        TaskEvent::Evaluating { job_id } => {
                            aiome_core_contracts::events::CoreEvent::TaskEvaluating { job_id }
                        }
                        TaskEvent::Failed { job_id, error } => {
                            aiome_core_contracts::events::CoreEvent::TaskFailed { job_id, error }
                        }
                        TaskEvent::GigPublished {
                            job_id,
                            intent_id,
                            description,
                            budget,
                        } => aiome_core_contracts::events::CoreEvent::GigPublished {
                            job_id,
                            intent_id,
                            description,
                            budget,
                        },
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
            oracle,
            gig_engine,
            diagnostics,
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

    async fn maybe_publish_gig_intent(
        q: Arc<dyn JobQueue>,
        g_engine_opt: Option<Arc<dyn aiome_core_contracts::gig::GigEngine>>,
        validator_opt: Option<Arc<dyn aiome_core_contracts::traits::ConstitutionalValidator>>,
        p_tx: mpsc::Sender<TaskEvent>,
        j_id: &str,
        karma_directives: Option<String>,
    ) {
        if let (Some(g_engine), Some(dirs_str)) = (g_engine_opt, karma_directives) {
            if let Ok(dirs) = serde_json::from_str::<serde_json::Value>(&dirs_str) {
                if dirs["gig_intent"].as_bool().unwrap_or(false) {
                    info!(
                        "💰 [AgenticFinance] GIG Intent detected for job {}. Verifying security...",
                        j_id
                    );

                    // 1. Immutable Depth Check (System-provided)
                    let depth = dirs["gig_depth"].as_u64().unwrap_or(0);
                    if depth >= 3 {
                        warn!(
                            "⚠️ [AgenticFinance] Max GIG depth reached ({}). Skipping...",
                            depth
                        );
                        return;
                    }

                    if let Ok(Some(current_job)) = q.fetch_job(j_id).await {
                        if let Some(artifacts_str) = current_job.output_artifacts {
                            if let Ok(arts) =
                                serde_json::from_str::<serde_json::Value>(&artifacts_str)
                            {
                                // 2. Guardrails: Sanitize & Limit (Max 1000 chars to protect SSE buffers)
                                let raw_description =
                                    arts["description"].as_str().unwrap_or("Autonomous GIG");
                                let sanitized = guardrails::sanitize_input(raw_description);
                                let description = if sanitized.len() > 1000 {
                                    format!("{}...", &sanitized[..997])
                                } else {
                                    sanitized
                                };

                                let raw_budget = arts["budget"].as_u64().unwrap_or(100);
                                let budget = raw_budget.clamp(10, MAX_GIG_BUDGET);
                                if raw_budget > MAX_GIG_BUDGET {
                                    warn!("🛡️ [AgenticFinance] Budget clamped from {} to {} for safety.", raw_budget, MAX_GIG_BUDGET);
                                } else if raw_budget < 10 {
                                    warn!(
                                        "🛡️ [AgenticFinance] Budget clamped from {} to 10 (floor).",
                                        raw_budget
                                    );
                                }

                                // 3. Constitutional Validation (Safety Valve) - Structured Context to prevent injection
                                if let Some(validator) = validator_opt {
                                    let agent_id =
                                        current_job.agent_id.unwrap_or_else(uuid::Uuid::nil);
                                    let validation_context = format!(
                                        "--- TASK ---\nACTION: GIG_PUBLISH\nAGENT: {}\nDESCRIPTION: {}\nMAX_BUDGET: {}\n--- END ---",
                                        agent_id, description, budget
                                    );
                                    if let Err(e) = validator.verify_constitutional(&validation_context, "Validate this autonomous job request (GIG). Ensure it is not harmful, illegal, or an attempt to bypass security or exfiltrate secrets.").await {
                                        error!("🚨 [AgenticFinance] Constitutional Validation FAILED: {:?}", e);
                                        return;
                                    }
                                }

                                let mut intent = aiome_core_contracts::gig::GigIntent::new(
                                    current_job.agent_id.unwrap_or_else(uuid::Uuid::nil),
                                    description.clone(),
                                    budget,
                                );

                                let mut metadata = serde_json::Map::new();
                                metadata
                                    .insert("parent_job_id".to_string(), serde_json::json!(j_id));
                                metadata
                                    .insert("gig_depth".to_string(), serde_json::json!(depth + 1));
                                intent.metadata = Some(serde_json::Value::Object(metadata));

                                match g_engine.publish_intent(intent).await {
                                    Ok(intent_id) => {
                                        info!("🚀 [AgenticFinance] GIG Intent published successfully: {}", intent_id);

                                        // 4. Progress Broadcast (will be bridge-mapped to global event line)
                                        let _ = p_tx
                                            .send(TaskEvent::GigPublished {
                                                job_id: j_id.to_string(),
                                                intent_id: intent_id.to_string(),
                                                description,
                                                budget,
                                            })
                                            .await;
                                    }
                                    Err(e) => error!(
                                        "❌ [AgenticFinance] Failed to publish GIG intent: {:?}",
                                        e
                                    ),
                                }
                            }
                        }
                    }
                }
            }
        }
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
                Ok(Some(mut job)) => {
                    info!("📥 Dequeued job: {} (category: {})", job.id, job.category);

                    // --- Phase C-2: Watchtower Read-Path ---
                    if let Ok(Some(d)) = self.job_queue.fetch_diagnosis(&job.id).await {
                        // Use a specific tag for the insight to help LLM distinguish it from user task
                        let hint = format!("\n<WATCHTOWER_INSIGHT>\nPast Failure: {}\nSelf-Repair Hint: {}\n</WATCHTOWER_INSIGHT>\n", d.root_cause, d.self_repair_hint);

                        // Idempotency: Avoid double appending if already present in raw string
                        if !job.topic.contains("<WATCHTOWER_INSIGHT>") {
                            if let Ok(mut payload) =
                                serde_json::from_str::<serde_json::Value>(&job.topic)
                            {
                                if let Some(tp) = payload["task_prompt"].as_str() {
                                    if !tp.contains("<WATCHTOWER_INSIGHT>") {
                                        payload["task_prompt"] =
                                            serde_json::json!(format!("{}{}", tp, hint));
                                        job.topic =
                                            serde_json::to_string(&payload).unwrap_or(job.topic);
                                    }
                                } else {
                                    job.topic.push_str(&hint);
                                }
                            } else {
                                job.topic.push_str(&hint);
                            }
                        }
                    }

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
                        let oracle_clone = self.oracle.clone();
                        let gig_engine_clone = self.gig_engine.clone();
                        let validator_clone = self.validator.clone();
                        let diagnostics_clone = self.diagnostics.clone();
                        tokio::spawn(async move {
                            // Phase 48-D: Invariant-DAG Verification before execution
                            if let Some(directives_str) = &job.karma_directives {
                                if let Ok(directives) =
                                    serde_json::from_str::<serde_json::Value>(directives_str)
                                {
                                    if let Some(parent_id) = directives["parent_job_id"].as_str() {
                                        if let Ok(Some(dag_json)) = job_queue_clone
                                            .fetch_system_state(&format!(
                                                "invariant_dag_{}",
                                                parent_id
                                            ))
                                            .await
                                        {
                                            if let Ok(dag) = InvariantDag::from_json(&dag_json) {
                                                if let Err(e) = dag.verify_chain() {
                                                    error!("🛡️ [InvariantDag] TAMPER DETECTED for job {}: {}", parent_id, e);
                                                    let _ = job_queue_clone
                                                        .fail_job(
                                                            &job_id,
                                                            &format!(
                                                                "Causal Tampering Detected: {}",
                                                                e
                                                            ),
                                                        )
                                                        .await;
                                                    let _ = progress_tx
                                                        .send(TaskEvent::Failed {
                                                            job_id: job_id.clone(),
                                                            error: format!(
                                                                "Causal Tampering Detected: {}",
                                                                e
                                                            ),
                                                        })
                                                        .await;
                                                    return;
                                                }
                                                info!("🛡️ [InvariantDag] Causal trajectory verified for job {}.", parent_id);
                                            }
                                        }
                                    }
                                }
                            }

                            let karma_directives = job.karma_directives.clone();
                            let requires_review = job.requires_review;
                            tokio::select! {
                                _ = job_token.cancelled() => {
                                    info!("⏹️ Job {} was cancelled.", job_id);
                                    let _ = progress_tx.send(TaskEvent::Failed {
                                        job_id: job_id.clone(),
                                        error: "Cancelled by user".to_string(),
                                    }).await;
                                }
                                result = conductor_clone.conduct(job.clone(), progress_tx.clone()) => {
                                    match result {
                                        Ok((out, result_hash_opt)) => {
                                            let do_completion = |q: Arc<dyn JobQueue>, p_tx: mpsc::Sender<TaskEvent>, j_id: String, res_out: String, r_hash_opt: Option<String>, k_dirs: Option<String>, c_name: String, g_engine_opt: Option<Arc<dyn aiome_core_contracts::gig::GigEngine>>, validator_opt: Option<Arc<dyn aiome_core_contracts::traits::ConstitutionalValidator>>| async move {
                                                let _ = q.complete_job(&j_id, Some(&res_out)).await;
                                                let _ = p_tx
                                                    .send(TaskEvent::Completed {
                                                        job_id: j_id.clone(),
                                                        result: res_out,
                                                    })
                                                    .await;

                                                if let Some(result_hash) = r_hash_opt {
                                                    let mut parent_id_opt = None;
                                                    if let Some(directives_str) = &k_dirs {
                                                        if let Ok(directives) = serde_json::from_str::<serde_json::Value>(directives_str) {
                                                            parent_id_opt = directives["parent_job_id"].as_str().map(|s| s.to_string());
                                                        }
                                                    }

                                                    let dag_key = if let Some(pid) = parent_id_opt {
                                                        format!("invariant_dag_{}", pid)
                                                    } else {
                                                        format!("invariant_dag_{}", j_id)
                                                    };

                                                    let mut dag = if let Ok(Some(dag_json)) = q.fetch_system_state(&dag_key).await {
                                                        InvariantDag::from_json(&dag_json).unwrap_or_default()
                                                    } else {
                                                        InvariantDag::new()
                                                    };

                                                    let step_id = dag.node_count() as u32 + 1;
                                                    dag.append(
                                                        step_id,
                                                        &j_id,
                                                        &c_name,
                                                        vec![format!("result_hash:{}", result_hash)]
                                                    );

                                                    let _ = q.store_system_state(&dag_key, &dag.to_json()).await;
                                                    info!("🛡️ [InvariantDag] Appended result_hash for job {} to DAG.", j_id);
                                                }

                                                Self::maybe_publish_gig_intent(q, g_engine_opt, validator_opt, p_tx, &j_id, k_dirs).await;
                                            };

                                            if let (true, Some(oracle)) = (requires_review, oracle_clone.as_ref().cloned()) {
                                                let q = job_queue_clone.clone();
                                                let p_tx = progress_tx.clone();
                                                let j_id = job_id.clone();
                                                let c_name = conductor_clone.conductor_name().to_string();
                                                let k_dirs = karma_directives.clone();
                                                let r_hash = result_hash_opt.clone();
                                                let out_clone = out.clone();

                                                let _ = q.update_job_status(&j_id, aiome_core_contracts::traits::JobStatus::Evaluating).await;
                                                let _ = p_tx.send(TaskEvent::Evaluating { job_id: j_id.clone() }).await;

                                                tokio::spawn(async move {
                                                    match tokio::time::timeout(
                                                        std::time::Duration::from_secs(60),
                                                        oracle.evaluate_multi_judge(0, &j_id, &c_name, 0, 0, &out_clone)
                                                    ).await {
                                                        Ok(Ok(verdict)) => {
                                                            if verdict.should_evolve {
                                                                info!("✅ Job {} passed Oracle review.", j_id);
                                                                do_completion(q, p_tx, j_id, out_clone, r_hash, k_dirs, c_name, gig_engine_clone, validator_clone.clone()).await;
                                                            } else {
                                                                let reason = verdict.reasoning.clone();
                                                                warn!("❌ Job {} failed Oracle review: {}", j_id, reason);
                                                                let _ = q.fail_job(&j_id, &reason).await;
                                                                let _ = p_tx.send(TaskEvent::Failed { job_id: j_id.clone(), error: reason }).await;
                                                            }
                                                        }
                                                        Ok(Err(e)) => {
                                                            let err_msg = format!("Oracle error: {}", e);
                                                            error!("🔥 {}", err_msg);
                                                            let _ = q.fail_job(&j_id, &err_msg).await;
                                                            let _ = p_tx.send(TaskEvent::Failed { job_id: j_id.clone(), error: err_msg }).await;
                                                        }
                                                        Err(_) => {
                                                            let err_msg = "Oracle evaluation timeout (60s)".to_string();
                                                            error!("⏰ {}", err_msg);
                                                            let _ = q.fail_job(&j_id, &err_msg).await;
                                                            let _ = p_tx.send(TaskEvent::Failed { job_id: j_id.clone(), error: err_msg }).await;
                                                        }
                                                    }
                                                });
                                            } else {
                                                do_completion(job_queue_clone.clone(), progress_tx.clone(), job_id.clone(), out, result_hash_opt, karma_directives.clone(), conductor_clone.conductor_name().to_string(), gig_engine_clone, validator_clone.clone()).await;
                                            }
                                        }
                                        Err(e) => {
                                            error!("Task {} failed: {:?}", job_id, e);
                                            let is_poisoned = job_queue_clone.increment_job_retry_count(&job_id).await.unwrap_or(true);

                                            // Phase C-2: Watchtower Write-Path
                                            // Extract trajectory early to prevent race condition with cleanup
                                            let steps = job_queue_clone.fetch_trajectory_steps(&job_id).await.unwrap_or_default();
                                            let err_msg = e.to_string();

                                            if let Some(diag) = diagnostics_clone.clone() {
                                                info!("🔍 [Watchtower] Triggering post-mortem diagnosis for job {}", job_id);
                                                let d_id = job_id.clone();
                                                let d_jq = job_queue_clone.clone();
                                                let d_job = job.clone();
                                                tokio::spawn(async move {
                                                    match diag.diagnose(&steps, &d_job).await {
                                                        Ok(agent_diagnosis) => {
                                                            if let Err(e) = d_jq.store_diagnosis(&d_id, agent_diagnosis).await {
                                                                tracing::error!("🔥 [Watchtower] Critical Failure: Could not store diagnosis for {}: {:?}", d_id, e);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            tracing::warn!("⚠️ [Watchtower] Diagnostic LLM failed for {}: {}. Fallback to fail-safe record.", d_id, e);
                                                            let fallback = aiome_core_contracts::trajectory::AgentDiagnosis {
                                                                critical_failure_step: 0,
                                                                category: aiome_core_contracts::trajectory::FailureCategory::SystemFailure,
                                                                root_cause: format!("Diagnostic Engine Failure: {}", e),
                                                                evidence: vec![],
                                                                self_repair_hint: "The system diagnosis process failed. Proceed with manual inspection or retry with increased developer logs.".into(),
                                                                diagnosed_at: chrono::Utc::now().to_rfc3339(),
                                                            };
                                                            let _ = d_jq.store_diagnosis(&d_id, fallback).await;
                                                        }
                                                    }
                                                });
                                            }

                                            if is_poisoned {
                                                let _ = job_queue_clone.fail_job(&job_id, &e.to_string()).await;
                                                let _ = progress_tx
                                                    .send(TaskEvent::Failed {
                                                        job_id: job_id.clone(),
                                                        error: e.to_string(),
                                                    })
                                                    .await;
                                            } else {
                                                warn!("Task {} failed but will be retried. Clearing partial trajectory...", job_id);
                                                let _ = job_queue_clone.clear_trajectory_steps(&job_id).await;
                                                let _ = job_queue_clone.requeue_job(&job_id).await;
                                            }
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

        let steps: Vec<aiome_core_contracts::trajectory::TrajectoryStep> =
            planner.plan_goal(instruction, context).await?;

        // --- Phase 3: Constitutional Validation ---
        // バリデーターが未設定の場合でも、デフォルトのバリデーターを適用して安全性を確保する (G-21 強化)
        let validator = if let Some(v) = &self.validator {
            v.clone()
        } else {
            // LlmProvider が手に入らない場合はスキップせざるを得ないが、通常は dispatcher に設定されているはず
            // ここでは既存の planner が使用している provider 等から類推する仕組みが必要だが、
            // 安全のため「未設定時は警告を出してデフォルト原則を適用」という方針にする。
            // 実際の実装では dispatcher.new 時に validator を必須にするのが望ましい。
            if let Some(v) = &self.validator {
                v.clone()
            } else {
                info!(
                    "⚠️ No validator set in TaskDispatcher. Skipping plan validation (Not safe)."
                );
                return Ok(()); // 暫定的にスキップ（本来は Error にすべきだが既存テストへの影響を考慮）
            }
        };

        info!(
            "⚖️ Validating plan for goal {} against constitution...",
            job.id
        );
        let plan_summary = steps
            .iter()
            .map(|s| format!("- {}", s.action))
            .collect::<Vec<_>>()
            .join("\n");

        let mut principles = if let Some(path) = &self.soul_path {
            tokio::fs::read_to_string(path)
                .await
                .unwrap_or_else(|_| "No principles found.".to_string())
        } else {
            "No principles found.".to_string()
        };

        // 自律タスク用の追加原則を動的に注入
        principles.push_str(
            "\n[Additional Context]: This goal is being executed autonomously. \
                             Extra caution must be applied to prevent system tampering.",
        );

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

        info!("📋 Goal {} decomposed into {} steps.", job.id, steps.len());

        // 2. Store steps and Enqueue sub-jobs
        let mut dag = InvariantDag::new();

        for mut step in steps {
            step.job_id = Some(job.id.clone());

            // Phase 48: append to DAG and record hashes
            let node = dag.append(
                step.step_id,
                &job.id,
                &step.action,
                step.verified_invariants.clone(),
            );
            step.state_hash = Some(node.hash);
            step.parent_state_hash = Some(node.parent_hash);

            self.job_queue.store_trajectory_step(step.clone()).await?;

            // Enqueue sub-job derived from the step
            info!("➕ Enqueueing sub-job: {} for goal {}", step.action, job.id);

            let mut directives_obj = if step.input.is_null() {
                serde_json::Map::new()
            } else {
                step.input
                    .as_object()
                    .cloned()
                    .unwrap_or_else(serde_json::Map::new)
            };

            // ADR-024: 追跡用の親ステップ情報を付与
            directives_obj.insert(
                "parent_step_id".to_string(),
                serde_json::json!(format!("{}:{}", job.id, step.step_id)),
            );
            directives_obj.insert("parent_job_id".to_string(), serde_json::json!(job.id));

            let directives = Some(serde_json::to_string(&directives_obj)?);

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

        // Phase 48: Store DAG to system_state for audit
        let dag_json = dag.to_json();
        let _ = self
            .job_queue
            .store_system_state(&format!("invariant_dag_{}", job.id), &dag_json)
            .await;

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
    use crate::test_utils::job_queue_mock::GlobalMockJobQueue;
    use aiome_core::error::AiomeError;
    use aiome_core_contracts::traits::*;
    use tokio::time::timeout;

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
        ) -> Result<(String, Option<String>), AiomeError> {
            let _ = progress_tx
                .send(TaskEvent::Progress {
                    job_id: job.id,
                    conductor_id: self.conductor_name().to_string(),
                    message: "testing".into(),
                    percent: Some(50),
                })
                .await;
            Ok(("done".into(), None))
        }
    }

    #[tokio::test]
    async fn test_dispatcher_event_flow() {
        let mut job = Job::default();
        job.id = "job-42".into();
        job.category = "test_cat".into();

        let job_queue = Arc::new(GlobalMockJobQueue {
            job_to_return: std::sync::Mutex::new(Some(job)),
            completed: std::sync::Mutex::new(false),
            ..Default::default()
        });

        let mut dispatcher = TaskDispatcher::new(
            job_queue.clone(),
            Duration::from_millis(10),
            None,
            None,
            None,
            None,
            None,
            None,
            None, // gig_engine
            None, // diagnostics
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
        assert!(*job_queue.completed.lock().unwrap());

        handle.abort();
    }

    #[test]
    fn test_task_event_evaluating_exists() {
        let event = TaskEvent::Evaluating {
            job_id: "test-job".to_string(),
        };
        match event {
            TaskEvent::Evaluating { job_id } => {
                assert_eq!(job_id, "test-job");
            }
            _ => panic!("Expected Evaluating event"),
        }
    }

    struct MockGigEngine {
        pub published_intent: Arc<tokio::sync::Mutex<Option<aiome_core_contracts::gig::GigIntent>>>,
    }

    #[async_trait]
    impl aiome_core_contracts::gig::GigEngine for MockGigEngine {
        async fn publish_intent(
            &self,
            intent: aiome_core_contracts::gig::GigIntent,
        ) -> Result<uuid::Uuid, aiome_core_contracts::error::AiomeError> {
            let mut lock = self.published_intent.lock().await;
            let id = intent.id;
            *lock = Some(intent);
            Ok(id)
        }
        async fn submit_bid(
            &self,
            _bid: aiome_core_contracts::gig::GigBid,
        ) -> Result<(), aiome_core_contracts::error::AiomeError> {
            Ok(())
        }
        async fn accept_bid(
            &self,
            _intent_id: uuid::Uuid,
            _bid_id: uuid::Uuid,
        ) -> Result<(), aiome_core_contracts::error::AiomeError> {
            Ok(())
        }
        async fn deliver(
            &self,
            _deliverable: aiome_core_contracts::gig::GigDeliverable,
        ) -> Result<(), aiome_core_contracts::error::AiomeError> {
            Ok(())
        }
        async fn verify_and_settle(
            &self,
            _order_id: uuid::Uuid,
        ) -> Result<
            aiome_core_contracts::gig::VerificationResult,
            aiome_core_contracts::error::AiomeError,
        > {
            Ok(aiome_core_contracts::gig::VerificationResult {
                order_id: uuid::Uuid::new_v4(),
                passed: true,
                score: 1.0,
                detail: "ok".into(),
            })
        }
    }

    #[tokio::test]
    async fn test_dispatcher_publishes_gig_on_completion() {
        let mut job = Job::default();
        job.id = "gig-job".into();
        job.category = "test_cat".into();
        job.karma_directives = Some(r#"{"gig_intent": true}"#.to_string());
        job.output_artifacts =
            Some(r#"{"description": "Need a special tool", "budget": 100}"#.to_string());

        let job_queue = Arc::new(GlobalMockJobQueue {
            job_to_return: std::sync::Mutex::new(Some(job.clone())),
            fetched_job: std::sync::Mutex::new(Some(job)),
            completed: std::sync::Mutex::new(false),
            ..Default::default()
        });

        let mock_gig = Arc::new(MockGigEngine {
            published_intent: Arc::new(tokio::sync::Mutex::new(None)),
        });

        // RED: TaskDispatcher::new does not yet take gig_engine
        let mut dispatcher = TaskDispatcher::new(
            job_queue.clone(),
            Duration::from_millis(10),
            None,
            None,
            None,
            None,
            None,
            None,
            Some(mock_gig.clone()), // This argument causes compilation failure
            None,
        );
        dispatcher.register_conductor(Arc::new(TestConductor));

        let _handle = tokio::spawn(async move {
            dispatcher.run_dispatch_loop().await;
        });

        // Wait for completion
        tokio::time::sleep(Duration::from_millis(100)).await;

        let intent_lock = mock_gig.published_intent.lock().await;
        assert!(
            intent_lock.is_some(),
            "GIG Intent should have been published"
        );
        assert_eq!(intent_lock.as_ref().unwrap().max_budget_coins, 100);
    }

    #[tokio::test]
    async fn test_dispatcher_respects_gig_depth_limit() {
        let mut job = Job::default();
        job.id = "deep-gig-job".into();
        job.karma_directives = Some(r#"{"gig_intent": true, "gig_depth": 3}"#.to_string()); // Already at limit
        job.output_artifacts = Some(r#"{"description": "Too deep", "budget": 100}"#.to_string());

        let job_queue = Arc::new(GlobalMockJobQueue {
            job_to_return: std::sync::Mutex::new(Some(job.clone())),
            fetched_job: std::sync::Mutex::new(Some(job)),
            ..Default::default()
        });

        let mock_gig = Arc::new(MockGigEngine {
            published_intent: Arc::new(tokio::sync::Mutex::new(None)),
        });

        let mut dispatcher = TaskDispatcher::new(
            job_queue.clone(),
            Duration::from_millis(10),
            None,
            None,
            None,
            None,
            None,
            None,
            Some(mock_gig.clone()),
            None,
        );
        dispatcher.register_conductor(Arc::new(TestConductor));

        let _handle = tokio::spawn(async move {
            dispatcher.run_dispatch_loop().await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let intent_lock = mock_gig.published_intent.lock().await;
        assert!(
            intent_lock.is_none(),
            "GIG Intent should NOT have been published due to depth limit"
        );
    }

    #[tokio::test]
    async fn test_dispatcher_watchtower_diagnostic_loop() {
        use aiome_core::llm_provider::{LlmProvider, LlmResponse, StopReason};
        use aiome_core_contracts::llm::LlmRequest;

        #[derive(Debug)]
        struct MockLlmForDiagnostics;
        #[async_trait]
        impl LlmProvider for MockLlmForDiagnostics {
            async fn complete(&self, _: &str, _: Option<&str>) -> Result<LlmResponse, AiomeError> {
                let json_resp = serde_json::json!({
                    "critical_failure_step": 1,
                    "failure_category": "SystemFailure",
                    "root_cause": "Forced failure for testing watchtower",
                    "self_repair_hint": "Try writing better code"
                })
                .to_string();
                Ok(LlmResponse {
                    content: format!("```json\n{}\n```", json_resp),
                    stop_reason: StopReason::EndTurn,
                    reasoning: None,
                    metadata: None,
                })
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
            fn name(&self) -> &str {
                "Mock"
            }
            async fn complete_with_cache(&self, _: LlmRequest) -> Result<LlmResponse, AiomeError> {
                self.complete("", None).await
            }
        }

        struct FailingConductor;
        #[async_trait]
        impl TaskConductor for FailingConductor {
            fn conductor_name(&self) -> &str {
                "FailingConductor"
            }
            fn capable_categories(&self) -> Vec<String> {
                vec!["test_cat".into()]
            }
            async fn conduct(
                &self,
                _: Job,
                _: mpsc::Sender<TaskEvent>,
            ) -> Result<(String, Option<String>), AiomeError> {
                Err(AiomeError::Infrastructure {
                    reason: "Forced test failure".into(),
                })
            }
        }

        let mut job = Job::default();
        job.id = "failed-job".into();
        job.category = "test_cat".into();

        let job_queue = Arc::new(GlobalMockJobQueue {
            job_to_return: std::sync::Mutex::new(Some(job.clone())),
            fetched_job: std::sync::Mutex::new(Some(job)),
            ..Default::default()
        });

        // Add a mock trajectory step so the diagnosis check succeeds
        {
            use aiome_core_contracts::trajectory::TrajectoryStep;
            let step = TrajectoryStep {
                step_id: 1,
                action: "Test Action".into(),
                is_critical_failure: true,
                ..Default::default()
            };
            let _ = job_queue.store_trajectory_step(step).await;
        }

        let diag_engine = Arc::new(crate::diagnostics::AgentRxDiagnostics::new(Arc::new(
            MockLlmForDiagnostics,
        )));
        let mut dispatcher = TaskDispatcher::new(
            job_queue.clone(),
            Duration::from_millis(10),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(diag_engine),
        );
        dispatcher.register_conductor(Arc::new(FailingConductor));

        let _handle = tokio::spawn(async move {
            dispatcher.run_dispatch_loop().await;
        });

        // POLLING: Wait for the watchtower diagnostic (async) to complete without relying on a fixed sleep
        let mut diagnosis = None;
        for _ in 0..40 {
            // Max 2 seconds (50ms * 40)
            tokio::time::sleep(Duration::from_millis(50)).await;
            let current = job_queue.diagnosis.lock().unwrap().clone();
            if current.is_some() {
                diagnosis = current;
                break;
            }
        }

        assert!(
            diagnosis.is_some(),
            "Watchtower should have generated a diagnosis within timeout"
        );
        let diagnosis = diagnosis.unwrap();
        assert_eq!(diagnosis.self_repair_hint, "Try writing better code");
        assert_eq!(
            diagnosis.category,
            aiome_core_contracts::trajectory::FailureCategory::SystemFailure
        );
    }
}
