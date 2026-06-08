/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::dispatcher::{TaskDispatcher, MAX_GIG_BUDGET};
use super::types::{compute_soul_hash, TaskEvent};
use crate::invariant_dag::InvariantDag;
use aiome_core_contracts::traits::JobStatus;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

impl TaskDispatcher {
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
                        if let Some(_planner) = &self.planner {
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
                                            let do_completion = |q: Arc<dyn aiome_core_contracts::traits::JobQueue>, p_tx: mpsc::Sender<TaskEvent>, j_id: String, res_out: String, r_hash_opt: Option<String>, k_dirs: Option<String>, c_name: String, g_engine_opt: Option<Arc<dyn aiome_core_contracts::gig::GigEngine>>, validator_opt: Option<Arc<dyn aiome_core_contracts::traits::ConstitutionalValidator>>, hooks: Option<Arc<crate::security::hook_manager::HookManager>>| async move {
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

                                                TaskDispatcher::maybe_publish_gig_intent(q, g_engine_opt, validator_opt, p_tx, &j_id, k_dirs).await;
                                            };

                                            if let (true, Some(oracle)) = (requires_review, oracle_clone.as_ref().cloned()) {
                                                let q = job_queue_clone.clone();
                                                let p_tx = progress_tx.clone();
                                                let j_id = job_id.clone();
                                                let c_name = conductor_clone.conductor_name().to_string();
                                                let k_dirs = karma_directives.clone();
                                                let r_hash = result_hash_opt.clone();
                                                let out_clone = out.clone();

                                                if let Err(e) = q.update_job_status(&j_id, JobStatus::Evaluating).await {
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
}
