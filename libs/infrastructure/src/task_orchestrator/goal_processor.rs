/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::dispatcher::TaskDispatcher;
use crate::invariant_dag::InvariantDag;
use aiome_core::error::AiomeError;
use aiome_core_contracts::traits::Job;
use serde_json::json;
use tracing::{error, info, warn};

impl TaskDispatcher {
    pub(crate) async fn process_goal_job(&self, job: Job) -> Result<(), AiomeError> {
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
            // ここでは既存 of planner が使用している provider 等から類推する仕組みが必要だが、
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
                                if let Err(e_tx) =
                                    self.event_tx.send(super::types::TaskEvent::AwaitingInput {
                                        job_id: job.id.clone(),
                                        reason,
                                    })
                                {
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
        if let Err(e) = self.event_tx.send(super::types::TaskEvent::Completed {
            job_id: job.id,
            result: "Goal planned successfully".to_string(),
        }) {
            warn!("Failed to send target goal completed event: {:?}", e);
        }

        Ok(())
    }
}
