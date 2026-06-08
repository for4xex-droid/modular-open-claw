/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::types::{TaskConductor, TaskEvent};
use aiome_core::error::AiomeError;
use aiome_core_contracts::traits::{Job, JobQueue};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn};

pub(crate) const MAX_GIG_BUDGET: u64 = 5000;

/// The Task Dispatcher (Manager). Monitors the JobQueue and dispatches tasks to Conductors.
pub struct TaskDispatcher {
    pub(crate) conductors: Vec<Arc<dyn TaskConductor>>,
    pub(crate) job_queue: Arc<dyn JobQueue>,
    pub(crate) event_tx: broadcast::Sender<TaskEvent>,
    pub(crate) poll_interval: Duration,
    pub(crate) core_event_tx: Option<broadcast::Sender<aiome_core_contracts::events::CoreEvent>>,
    pub(crate) active_jobs: Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>,
    >,
    pub(crate) tool_discovery: Option<Arc<dyn aiome_core_contracts::traits::ToolDiscoveryEngine>>,
    pub(crate) planner: Option<Arc<dyn aiome_core_contracts::traits::StrategicPlanner>>,
    pub(crate) validator: Option<Arc<dyn aiome_core_contracts::traits::ConstitutionalValidator>>,
    pub(crate) soul_path: Option<std::path::PathBuf>,
    pub oracle: Option<Arc<crate::oracle::Oracle>>,
    pub(crate) gig_engine: Option<Arc<dyn aiome_core_contracts::gig::GigEngine>>,
    pub(crate) diagnostics: Option<Arc<crate::diagnostics::AgentRxDiagnostics>>,
    pub(crate) immune_system: Option<Arc<crate::immune_system::AdaptiveImmuneSystem>>,
    pub(crate) quality_gate_store: Option<Arc<dyn crate::quality_gate_store::QualityGateStore>>,
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

    pub(crate) async fn maybe_publish_gig_intent(
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
                                let sanitized = shared::guardrails::sanitize_input(raw_description);
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
}
