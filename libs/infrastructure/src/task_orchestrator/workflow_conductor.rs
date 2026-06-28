/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::task_orchestrator::{TaskConductor, TaskEvent};
use aiome_core::error::AiomeError;
use aiome_core_contracts::traits::Job;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::info;

pub struct WorkflowConductor;

impl WorkflowConductor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TaskConductor for WorkflowConductor {
    fn capable_categories(&self) -> Vec<String> {
        vec![
            "wf_llm".to_string(),
            "wf_http".to_string(),
            "wf_mcp".to_string(),
            "wf_loop".to_string(),
            "wf_parallel".to_string(),
            "wf_transform".to_string(),
            "wf_approval".to_string(),
            "wf_timer".to_string(),
            "wf_wasm".to_string(),
        ]
    }

    fn conductor_name(&self) -> &str {
        "WorkflowConductor"
    }

    async fn conduct(
        &self,
        job: Job,
        progress_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<(String, Option<String>), AiomeError> {
        info!(
            "🧬 [WorkflowConductor] Executing job: {} (Category: {})",
            job.id, job.category
        );

        let _ = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: format!("Executing workflow node: {}", job.id),
                percent: Some(10),
            })
            .await;

        let res = match job.category.as_str() {
            "wf_timer" => {
                let delay_sec = match serde_json::from_str::<serde_json::Value>(&job.topic) {
                    Ok(v) => v.get("delay_seconds").and_then(|d| d.as_u64()).unwrap_or_else(|| {
                        tracing::warn!("🧬 [WorkflowConductor] delay_seconds not found in timer job {}, defaulting to 1s", job.id);
                        1
                    }),
                    Err(e) => {
                        tracing::warn!("🧬 [WorkflowConductor] Invalid JSON topic for timer job {}: {}, defaulting to 1s", job.id, e);
                        1
                    }
                };
                let _ = progress_tx
                    .send(TaskEvent::Progress {
                        job_id: job.id.clone(),
                        conductor_id: self.conductor_name().to_string(),
                        message: format!("Waiting for {} seconds...", delay_sec),
                        percent: Some(50),
                    })
                    .await;
                tokio::time::sleep(tokio::time::Duration::from_secs(delay_sec)).await;
                ("Timer delay complete".to_string(), None)
            }
            "wf_wasm" => {
                let payload =
                    serde_json::from_str::<serde_json::Value>(&job.topic).map_err(|e| {
                        AiomeError::Validation {
                            reason: format!("Invalid WASM job configuration: {}", e),
                        }
                    })?;
                let code = payload.get("code").and_then(|c| c.as_str()).unwrap_or("");
                let language = payload
                    .get("language")
                    .and_then(|l| l.as_str())
                    .unwrap_or("");

                if code.trim().is_empty() {
                    return Err(AiomeError::Validation {
                        reason: "WASM code must not be empty".to_string(),
                    });
                }

                let _ = progress_tx
                    .send(TaskEvent::Progress {
                        job_id: job.id.clone(),
                        conductor_id: self.conductor_name().to_string(),
                        message: format!("Simulating WASM execution for language: {}", language),
                        percent: Some(50),
                    })
                    .await;

                ("WASM execution simulated successfully".to_string(), None)
            }
            _ => {
                let _ = progress_tx
                    .send(TaskEvent::Progress {
                        job_id: job.id.clone(),
                        conductor_id: self.conductor_name().to_string(),
                        message: "Generic workflow node executing...".to_string(),
                        percent: Some(50),
                    })
                    .await;
                ("Workflow node executed".to_string(), None)
            }
        };

        let _ = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: "Node execution complete.".to_string(),
                percent: Some(100),
            })
            .await;

        Ok(res)
    }
}
impl Default for WorkflowConductor {
    fn default() -> Self {
        Self::new()
    }
}
