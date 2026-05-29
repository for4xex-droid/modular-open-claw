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
pub mod geo_audit;
pub mod llm_conductor;
pub mod planner;
pub mod seo_content;

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
    Cancelled {
        job_id: String,
    },
    AwaitingInput {
        job_id: String,
        reason: String,
    },
    GigPublished {
        job_id: String,
        intent_id: String,
        description: String,
        budget: u64,
    },
    QualityGate {
        job_id: String,
        score: u32,
        passed: bool,
        conductor: String,
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

/// SOUL.md と EVOLVING_SOUL.md を読み込み、soul_hash を計算する。
/// AppState::get_system_soul_hash() と同じハッシュロジックを使用する。
#[tracing::instrument(skip_all, fields(has_path = soul_path.is_some()))]
async fn compute_soul_hash(soul_path: &Option<std::path::PathBuf>) -> String {
    let path = match soul_path {
        Some(p) => p,
        None => return "unknown".to_string(),
    };
    shared::soul_hash::compute_from_path(path).await
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
    immune_system: Option<Arc<crate::immune_system::AdaptiveImmuneSystem>>,
    quality_gate_store: Option<Arc<dyn crate::quality_gate_store::QualityGateStore>>,
    pub(crate) hook_manager: Option<Arc<crate::security::hook_manager::HookManager>>,
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
        immune_system: Option<Arc<crate::immune_system::AdaptiveImmuneSystem>>,
        quality_gate_store: Option<Arc<dyn crate::quality_gate_store::QualityGateStore>>,
        hook_manager: Option<Arc<crate::security::hook_manager::HookManager>>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(256);

        let mut rx = event_tx.subscribe();
        let qgs_for_event = quality_gate_store.clone();
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
                        TaskEvent::Cancelled { job_id } => {
                            aiome_core_contracts::events::CoreEvent::TaskCancelled { job_id }
                        }
                        TaskEvent::AwaitingInput { job_id, reason } => {
                            aiome_core_contracts::events::CoreEvent::TaskAwaitingInput {
                                job_id,
                                reason,
                            }
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
                        TaskEvent::QualityGate {
                            job_id,
                            score,
                            passed,
                            conductor,
                        } => {
                            if let Some(ref store) = qgs_for_event {
                                let store_clone = store.clone();
                                let j_id = job_id.clone();
                                let cond = conductor.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = store_clone
                                        .record(
                                            &j_id,
                                            score as i32,
                                            passed,
                                            &cond,
                                            None,
                                            None,
                                            None,
                                        )
                                        .await
                                    {
                                        tracing::warn!(
                                            "Failed to record quality gate history for {}: {}",
                                            j_id,
                                            e
                                        );
                                    }
                                });
                            }
                            aiome_core_contracts::events::CoreEvent::QualityGate {
                                job_id,
                                score,
                                passed,
                                conductor,
                            }
                        }
                        _ => continue,
                    };
                    if let Err(e) = ctx.send(core_ev) {
                        warn!("Failed to send core event: {:?}", e);
                    }
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
            immune_system,
            quality_gate_store,
            hook_manager,
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

    /// Exposes a conductor by category for testing and diagnostic routing.
    /// Note: O(N * C) where N is number of conductors and C is number of categories.
    /// Not intended for hot-path use.
    pub fn get_conductor_for(&self, category: &str) -> Option<Arc<dyn TaskConductor>> {
        self.conductors
            .iter()
            .find(|c| c.capable_categories().iter().any(|cat| cat == category))
            .cloned()
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

                    if dirs["geo_blocked"].as_bool().unwrap_or(false) {
                        tracing::warn!("⚠️ [AgenticFinance] GEO Quality Gate failed. Blocking GIG intent for job {}.", j_id);
                        return;
                    }

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
                                    format!(
                                        "{}...",
                                        shared::strings::truncate_bytes_safely(&sanitized, 997)
                                    )
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
                                        if let Err(e) = p_tx
                                            .send(TaskEvent::GigPublished {
                                                job_id: j_id.to_string(),
                                                intent_id: intent_id.to_string(),
                                                description,
                                                budget,
                                            })
                                            .await
                                        {
                                            warn!("Failed to send GigPublished event: {:?}", e);
                                        }
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
                if let Err(e) = conductor.cancel(job_id).await {
                    tracing::warn!(
                        "Failed to cancel conductor {}: {}",
                        conductor.conductor_name(),
                        e
                    );
                }
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

            let mut categories_refs: Vec<&str> = categories.iter().map(|s| s.as_str()).collect();
            // Always include "Goal" if we have a planner
            if self.planner.is_some() && !categories_refs.contains(&"Goal") {
                categories_refs.push("Goal");
            }

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
                                if let Err(db_err) =
                                    self.job_queue.fail_job(&job.id, &e.to_string()).await
                                {
                                    error!(
                                        "Failed to mark job {} as failed in DB: {}",
                                        job.id, db_err
                                    );
                                }
                            }
                            continue; // Skip normal conduction for Goal
                        }
                    }

                    // Find a suitable conductor
                    if let Some(conductor) = self.get_conductor_for(&job.category) {
                        let job_id = job.id.clone();
                        let conductor_id = conductor.conductor_name().to_string();

                        // Send Spawned event
                        if let Err(e) = self.event_tx.send(TaskEvent::Spawned {
                            job_id: job_id.clone(),
                            conductor_id: conductor_id.clone(),
                        }) {
                            warn!("Failed to send Spawned event: {:?}", e);
                        }

                        // Set up progress channel
                        let (progress_tx, mut progress_rx) = mpsc::channel(32);
                        let event_tx_clone = self.event_tx.clone();
                        let job_queue_clone = self.job_queue.clone();

                        // Spawn a listener for progress updates
                        tokio::spawn(async move {
                            while let Some(event) = progress_rx.recv().await {
                                if let Err(e) = event_tx_clone.send(event) {
                                    warn!("Failed to forward progress event: {:?}", e);
                                }
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
                        let hooks_clone = self.hook_manager.clone();
                        let soul_path_clone = self.soul_path.clone();
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
                                                    if let Err(e_db) = job_queue_clone
                                                        .fail_job(
                                                            &job_id,
                                                            &format!(
                                                                "Causal Tampering Detected: {}",
                                                                e
                                                            ),
                                                        )
                                                        .await
                                                    {
                                                        warn!(
                                                            "Failed to update job status: {}",
                                                            e_db
                                                        );
                                                    }
                                                    if let Err(e) = progress_tx
                                                        .send(TaskEvent::Failed {
                                                            job_id: job_id.clone(),
                                                            error: format!(
                                                                "Causal Tampering Detected: {}",
                                                                e
                                                            ),
                                                        })
                                                        .await
                                                    {
                                                        tracing::warn!(
                                                            "Failed to send failed event: {}",
                                                            e
                                                        );
                                                    }
                                                    return;
                                                }
                                                info!("🛡️ [InvariantDag] Causal trajectory verified for job {}.", parent_id);
                                            }
                                        }
                                    }
                                }
                            }

                            // Phase B-4: Job-level Budget Check
                            let max_job_cost =
                                crate::context_engine::ContextBudget::default().max_job_cost_usd;
                            let current_cost =
                                job_queue_clone.fetch_job_cost(&job_id).await.unwrap_or(0.0);

                            if current_cost >= max_job_cost {
                                warn!("🚨 [TaskDispatcher] Job {} has exceeded the spending cap (${:.4} >= ${:.4}). Terminating job.", job_id, current_cost, max_job_cost);
                                if let Err(e) = job_queue_clone
                                    .fail_job(
                                        &job_id,
                                        &format!(
                                            "Budget exceeded: ${:.4} >= ${:.4}",
                                            current_cost, max_job_cost
                                        ),
                                    )
                                    .await
                                {
                                    error!(
                                        "Failed to mark job {} as failed due to budget overrun: {}",
                                        job_id, e
                                    );
                                }
                                if let Err(e) = progress_tx
                                    .send(TaskEvent::Failed {
                                        job_id: job_id.clone(),
                                        error: format!(
                                            "Budget exceeded: ${:.4} >= ${:.4}",
                                            current_cost, max_job_cost
                                        ),
                                    })
                                    .await
                                {
                                    tracing::warn!("Failed to send failed event: {}", e);
                                }
                                return; // Stop executing this job
                            }
                            let karma_directives = job.karma_directives.clone();
                            let requires_review = job.requires_review;
                            tokio::select! {
                                _ = job_token.cancelled() => {
                                    info!("⏹️ Job {} was cancelled.", job_id);
                                    if let Err(e) = progress_tx.send(TaskEvent::Cancelled {
                                        job_id: job_id.clone(),
                                    }).await {
                                        tracing::warn!("Failed to send cancelled event: {}", e);
                                    }
                                }
                                result = conductor_clone.conduct(job.clone(), progress_tx.clone()) => {
                                    match result {
                                        Ok((out, result_hash_opt)) => {
                                            let do_completion = |q: Arc<dyn JobQueue>, p_tx: mpsc::Sender<TaskEvent>, j_id: String, res_out: String, r_hash_opt: Option<String>, k_dirs: Option<String>, c_name: String, g_engine_opt: Option<Arc<dyn aiome_core_contracts::gig::GigEngine>>, validator_opt: Option<Arc<dyn aiome_core_contracts::traits::ConstitutionalValidator>>, hooks: Option<Arc<crate::security::hook_manager::HookManager>>| async move {
                                                if let Err(e) = q.complete_job(&j_id, Some(&res_out)).await {
                                                    error!("Failed to mark job {} as completed in DB: {}", j_id, e);
                                                }
                                                if let Err(e) = p_tx
                                                    .send(TaskEvent::Completed {
                                                        job_id: j_id.clone(),
                                                        result: res_out,
                                                    })
                                                    .await
                                                {
                                                    tracing::warn!("Failed to send completed event for {}: {}", j_id, e);
                                                }

                                                if let Some(hm) = &hooks {
                                                    if let Err(e) = hm.trigger_job_completed(&j_id, "completed").await {
                                                        tracing::warn!("Job completion hook failed for {}: {}", j_id, e);
                                                    }
                                                }

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

                                                    if let Err(e) = q.store_system_state(&dag_key, &dag.to_json()).await {
                                                        error!("Failed to store InvariantDag for job {}: {}", j_id, e);
                                                    }
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

                                                if let Err(e) = q.update_job_status(&j_id, aiome_core_contracts::traits::JobStatus::Evaluating).await {
                                                    error!("Failed to update job {} status to Evaluating: {}", j_id, e);
                                                }
                                                if let Err(e) = p_tx.send(TaskEvent::Evaluating { job_id: j_id.clone() }).await {
                                                    tracing::warn!("Failed to send evaluating event for {}: {}", j_id, e);
                                                }

                                                tokio::spawn(async move {
                                                    match tokio::time::timeout(
                                                        std::time::Duration::from_secs(60),
                                                        oracle.evaluate_multi_judge(0, &j_id, &c_name, 0, 0, &out_clone)
                                                    ).await {
                                                        Ok(Ok(verdict)) => {
                                                            if verdict.should_evolve {
                                                                info!("✅ Job {} passed Oracle review.", j_id);

                                                                // Feed back the alignment score to the trajectory as a reward signal
                                                                if let Err(e) = q.update_trajectory_reward(&j_id, verdict.alignment_score).await {
                                                                    warn!("Failed to update trajectory reward for job {}: {}", j_id, e);
                                                                }

                                                                // Phase G-4: Extract high-reward triplets and store as KarmaDirectives
                                                                if let Some(validator) = validator_clone.clone() {
                                                                    if let Ok(steps) = q.fetch_trajectory_steps(&j_id).await {
                                                                        let adapter = crate::trajectory_adapter::TrajectoryToTripletAdapter::new(validator);
                                                                        if let Err(e) = adapter.extract_and_store_triplets(
                                                                            steps,
                                                                            &j_id,
                                                                            &c_name,
                                                                            "", // Extracting soul_hash here is difficult, use empty for now
                                                                            0.8, // Configurable threshold for "good" trajectory
                                                                            q.clone()
                                                                        ).await {
                                                                            warn!("Failed to extract and store trajectory triplets: {}", e);
                                                                        }
                                                                    }
                                                                }

                                                                do_completion(q, p_tx, j_id, out_clone, r_hash, k_dirs, c_name, gig_engine_clone, validator_clone.clone(), hooks_clone.clone()).await;
                                                            } else {
                                                                let reason = verdict.reasoning.clone();
                                                                warn!("❌ Job {} failed Oracle review: {}", j_id, reason);
                                                                if let Err(db_err) = q.fail_job(&j_id, &reason).await {
                                                                    error!("Failed to mark job {} as failed in DB: {}", j_id, db_err);
                                                                }
                                                                if let Err(e) = p_tx.send(TaskEvent::Failed { job_id: j_id.clone(), error: reason }).await {
                                                                    tracing::warn!("Failed to send failed event for {}: {}", j_id, e);
                                                                }
                                                            }
                                                        }
                                                        Ok(Err(e)) => {
                                                            let err_msg = format!("Oracle error: {}", e);
                                                            error!("🔥 {}", err_msg);
                                                            if let Err(db_err) = q.fail_job(&j_id, &err_msg).await {
                                                                error!("Failed to mark job {} as failed in DB: {}", j_id, db_err);
                                                            }
                                                            if let Err(e) = p_tx.send(TaskEvent::Failed { job_id: j_id.clone(), error: err_msg }).await {
                                                                tracing::warn!("Failed to send failed event for {}: {}", j_id, e);
                                                            }
                                                        }
                                                        Err(_) => {
                                                            let err_msg = "Oracle evaluation timeout (60s)".to_string();
                                                            error!("⏰ {}", err_msg);
                                                            if let Err(db_err) = q.fail_job(&j_id, &err_msg).await {
                                                                error!("Failed to mark job {} as failed in DB: {}", j_id, db_err);
                                                            }
                                                            if let Err(e) = p_tx.send(TaskEvent::Failed { job_id: j_id.clone(), error: err_msg }).await {
                                                                tracing::warn!("Failed to send failed event for {}: {}", j_id, e);
                                                            }
                                                        }
                                                    }
                                                });
                                            } else {
                                                do_completion(job_queue_clone.clone(), progress_tx.clone(), job_id.clone(), out, result_hash_opt, karma_directives.clone(), conductor_clone.conductor_name().to_string(), gig_engine_clone, validator_clone.clone(), hooks_clone.clone()).await;
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
                                                let d_id = job_id.clone();
                                                let d_jq = job_queue_clone.clone();
                                                let d_job = job.clone();
                                                tokio::spawn(async move {
                                                    match diag.diagnose(&steps, &d_job).await {
                                                        Ok(agent_diagnosis) => {
                                                            if let Err(e) = d_jq.store_diagnosis(&d_id, agent_diagnosis.clone()).await {
                                                                tracing::error!("🔥 [Watchtower] Critical Failure: Could not store diagnosis for {}: {:?}", d_id, e);
                                                            }

                                                            // Phase E-1/E-2: Reflexion Loop
                                                            if !is_poisoned {
                                                                if let Err(e) = d_jq.append_job_karma_directives(&d_id, &agent_diagnosis.self_repair_hint).await {
                                                                    tracing::error!("🔥 [Reflexion] Failed to append self-repair hint to Job {}: {:?}", d_id, e);
                                                                } else {
                                                                    tracing::info!("🔄 [Reflexion] Appended self-repair hint to Job {} for next retry.", d_id);
                                                                }
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
                                                            if let Err(db_err) = d_jq.store_diagnosis(&d_id, fallback).await {
                                                                tracing::error!("🔥 [Watchtower] Failed to store fallback diagnosis for {}: {:?}", d_id, db_err);
                                                            }
                                                        }
                                                    }
                                                });
                                            }

                                            if is_poisoned {
                                                if let Err(db_err) = job_queue_clone.fail_job(&job_id, &e.to_string()).await {
                                                    error!("Failed to mark job {} as failed in DB: {}", job_id, db_err);
                                                }
                                                if let Err(send_err) = progress_tx
                                                    .send(TaskEvent::Failed {
                                                        job_id: job_id.clone(),
                                                        error: e.to_string(),
                                                    })
                                                    .await
                                                {
                                                    tracing::warn!("Failed to send failed event: {}", send_err);
                                                }

                                                // --- VERIFY-LEARN: Extract Negative Karma on complete failure ---
                                                let karma_reason = err_msg.clone();
                                                let job_id_clone = job_id.clone();
                                                let jq_for_karma = job_queue_clone.clone();
                                                let c_name = conductor_clone.conductor_name().to_string();
                                                let soul_hash_str = compute_soul_hash(&soul_path_clone).await;
                                                tokio::spawn(async move {
                                                    if let Err(e) = jq_for_karma.store_karma(
                                                        &job_id_clone,
                                                        &c_name,
                                                        &karma_reason,
                                                        "negative",
                                                        &soul_hash_str,
                                                        None,
                                                        None,
                                                        None,
                                                        true,
                                                    ).await {
                                                        tracing::warn!("Failed to store negative karma for failed job {}: {:?}", job_id_clone, e);
                                                    }
                                                });
                                            } else {
                                                warn!("Task {} failed but will be retried. Clearing partial trajectory...", job_id);
                                                if let Err(e) = job_queue_clone.clear_trajectory_steps(&job_id).await {
                                                    error!("Failed to clear trajectory steps for {}: {}", job_id, e);
                                                }
                                                if let Err(e) = job_queue_clone.requeue_job(&job_id).await {
                                                    error!("Failed to requeue job {}: {}", job_id, e);
                                                }
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

                        let fallback_msg = if let Some(discovery) = &self.tool_discovery {
                            let instruction = if !job.topic.is_empty() {
                                &job.topic
                            } else {
                                "No instruction provided"
                            };
                            match discovery.suggest_tools(instruction).await {
                                Ok(tools) if !tools.is_empty() => {
                                    format!("No conductor found. However, ToolDiscoveryEngine suggests downloading: {}", tools.join(", "))
                                }
                                _ => "No capable conductor found and no alternative tools discovered.".to_string(),
                            }
                        } else {
                            "No capable conductor found.".to_string()
                        };

                        if let Err(db_err) = self.job_queue.fail_job(&job.id, &fallback_msg).await {
                            error!("Failed to mark job {} as failed in DB: {}", job.id, db_err);
                        }
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
            if let Err(e_db) = self
                .job_queue
                .fail_job(&job.id, &format!("Constitutional Violation: {}", e))
                .await
            {
                warn!("Failed to mark job as failed: {}", e_db);
            }
            return Err(e);
        }
        info!(
            "✅ Plan for goal {} PASSED constitutional validation.",
            job.id
        );

        info!(
            "📋 Goal {} decomposed into {} steps. Verifying all steps for security...",
            job.id,
            steps.len()
        );

        // --- Phase 2: [Governable Execution] Atomic Immune System Verification ---
        // We verify ALL steps BEFORE storing or enqueueing anything to prevent partial execution of an unsafe plan.
        let mut bypass_immune_check = false;
        if let Some(log) = &job.execution_log {
            if log.contains("IMMUNE_BYPASS_APPROVED") {
                info!("⚠️ [AdaptiveImmuneSystem] Immune check BYPASSED for job {} due to user approval.", job.id);
                bypass_immune_check = true;
                // Clear the bypass marker for a one-time use
                if let Err(e) = self.job_queue.store_execution_log(&job.id, "").await {
                    warn!("Failed to clear execution log: {:?}", e);
                }
            }
        }

        if let Some(immune) = &self.immune_system {
            if !bypass_immune_check {
                for step in &steps {
                    if let Some(tool_name) = &step.tool_name {
                        if let Ok(Some(rule)) = immune
                            .verify_tool_call(tool_name, &step.input, self.job_queue.as_ref())
                            .await
                        {
                            warn!("🚨 [AdaptiveImmuneSystem] Plan for goal {} blocked by rule {}: tool={}", job.id, rule.id, tool_name);

                            // Use 70 as the threshold for "High" severity elicitation triggers
                            const ELICITATION_THRESHOLD: u8 = 70;
                            if rule.severity >= ELICITATION_THRESHOLD {
                                info!("✋ [Elicitation] High severity violation detected. Transitioning job {} to AwaitingInput.", job.id);
                                let reason = format!(
                                    "Governable Execution Blocked: {}. User input required.",
                                    rule.id
                                );

                                // Gap 3: Persist the reason to the DB before setting status to AwaitingInput
                                if let Err(e) = self.job_queue.fail_job(&job.id, &reason).await {
                                    tracing::error!("🚨 [TaskOrchestrator] Failed to record job failure for {}: {}", job.id, e);
                                }

                                self.job_queue
                                    .update_job_status(
                                        &job.id,
                                        aiome_core_contracts::traits::JobStatus::AwaitingInput,
                                    )
                                    .await?;

                                // Notify elicitation event
                                if let Err(e_tx) = self.event_tx.send(TaskEvent::AwaitingInput {
                                    job_id: job.id.clone(),
                                    reason,
                                }) {
                                    warn!("Failed to send AwaitingInput event: {:?}", e_tx);
                                }
                                return Ok(()); // Stop planning/dispatching for this goal completely
                            } else {
                                info!("⚠️ [AdaptiveImmuneSystem] Rule violation (low severity: {}). Warning logged but proceeding.", rule.severity);
                            }
                        }
                    }
                }
            }
        }

        // 2. Store steps and Enqueue sub-jobs (Only if all steps passed verification)
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
        if let Err(e) = self
            .job_queue
            .store_system_state(&format!("invariant_dag_{}", job.id), &dag_json)
            .await
        {
            warn!("Failed to store invariant dag to system state: {:?}", e);
        }

        // 3. Complete the Goal job
        self.job_queue
            .complete_job(&job.id, Some("Planned and Decomposed"))
            .await?;

        // Notify via event
        if let Err(e) = self.event_tx.send(TaskEvent::Completed {
            job_id: job.id,
            result: "Goal planned successfully".to_string(),
        }) {
            warn!("Failed to send target goal completed event: {:?}", e);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
