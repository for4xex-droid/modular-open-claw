use crate::security::{BastionGuard, PermissionManifest, RuntimeJail, SandboxProfile};
use crate::task_orchestrator::{TaskConductor, TaskEvent};
use aiome_contracts::commerce::CommerceEngine;
use aiome_core::error::AiomeError;
use aiome_core::traits::Job;
use async_trait::async_trait;
use base64::Engine;
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tracing::{error, info, warn};
use uuid::Uuid;

/// 影分身 (Shadow Worker) を安全に実行するための Conductor
/// BastionGuard, Fork Bomb Protection (Semaphore), Guardrails Sterilization, CommerceEngine Billing を統合。
pub struct DockerConductor {
    bastion: BastionGuard,
    commerce_engine: Option<Arc<dyn CommerceEngine>>,
    concurrency_limit: Arc<Semaphore>,
}

impl DockerConductor {
    pub fn new(commerce_engine: Option<Arc<dyn CommerceEngine>>) -> Self {
        Self {
            bastion: BastionGuard::new_internal(PermissionManifest::default()),
            commerce_engine,
            concurrency_limit: Arc::new(Semaphore::new(3)), // MAX 3 concurrent shadow clones
        }
    }
}

#[async_trait]
impl TaskConductor for DockerConductor {
    fn capable_categories(&self) -> Vec<String> {
        vec!["docker_shadow_worker".to_string()]
    }

    fn conductor_name(&self) -> &str {
        "DockerConductor"
    }

    async fn conduct(
        &self,
        job: Job,
        progress_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<String, AiomeError> {
        let _ = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: "Acquiring capacity (Concurrency Limit)...".to_string(),
                percent: Some(5),
            })
            .await;

        // Layer 1: Fork Bomb Protection
        // Wait for a permit. If we exceed 3, we wait here.
        let _permit =
            self.concurrency_limit
                .acquire()
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Semaphore closed: {}", e),
                })?;

        let _ = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: "Capacity acquired. Validating authorization...".to_string(),
                percent: Some(10),
            })
            .await;

        // Extract payload from `topic` string
        let payload: serde_json::Value =
            serde_json::from_str(&job.topic).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to parse payload: {}", e),
            })?;
        let agent_id_str = payload["agent_id"].as_str().unwrap_or_default();
        let agent_id = Uuid::parse_str(agent_id_str).unwrap_or(Uuid::nil());
        let agent_yaml = payload["agent_yaml_content"].as_str().unwrap_or_default();
        let task_prompt = payload["task_prompt"].as_str().unwrap_or_default();

        // Layer 3: Economy Check (Billing)
        if let Some(engine) = &self.commerce_engine {
            if !agent_id.is_nil() {
                // Check if allowed (validate_activity) - requires inference tag or custom activity
                engine
                    .validate_activity(agent_id, "docker_invocation", 1)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Budget Exhausted or Invalid: {:?}", e),
                    })?;

                // Deduct cost
                let item_id = Uuid::new_v4();
                let metadata = serde_json::json!({ "reason": "Shadow Worker Invocation" });
                engine
                    .execute_autonomous_purchase(agent_id, item_id, metadata)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to execute purchase: {:?}", e),
                    })?;

                info!(
                    "💳 [DockerConductor] Billed agent {} for Shadow Clone usage.",
                    agent_id
                );
            }
        }

        let session_id = Uuid::new_v4().to_string();
        let temp_dir = std::env::temp_dir().join(format!("aiome-delegation-{}", session_id));

        if let Err(e) = std::fs::create_dir_all(&temp_dir) {
            return Err(AiomeError::Infrastructure {
                reason: format!("Failed to create sandbox: {}", e),
            });
        }

        let yaml_path = temp_dir.join("agent.yaml");
        if let Err(e) = std::fs::write(&yaml_path, agent_yaml) {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(AiomeError::Infrastructure {
                reason: format!("Failed to write agent config: {}", e),
            });
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&yaml_path, std::fs::Permissions::from_mode(0o600));
        }

        let _ = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: "Executing inside BastionGuard...".to_string(),
                percent: Some(30),
            })
            .await;

        // Encoding prompt
        let task_prompt_b64 = base64::engine::general_purpose::STANDARD.encode(task_prompt);

        // Capability Check: Verify Docker is installed
        match self
            .bastion
            .safe_exec_with_profile("docker --version", SandboxProfile::Default)
            .await
        {
            Ok(_) => {}
            Err(_) => {
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err(AiomeError::Infrastructure {
                    reason: "Docker capability check failed. Docker is required for Shadow Clones."
                        .to_string(),
                });
            }
        }

        // Execution
        let cmd = format!(
            "docker agent run --exec --json {} --prompt-b64 {}",
            yaml_path.display(),
            task_prompt_b64
        );

        let start = std::time::Instant::now();

        // Use tokio timeout for overall execution to avoid infinitely hanging containers
        let exec_result = tokio::time::timeout(
            std::time::Duration::from_secs(300), // 5 minute max execution
            self.bastion
                .safe_exec_with_profile(&cmd, SandboxProfile::Strict),
        )
        .await;

        let duration_ms = start.elapsed().as_millis() as u64;
        let _ = std::fs::remove_dir_all(&temp_dir);

        let _ = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: "Execution completed. Sterilizing output...".to_string(),
                percent: Some(90),
            })
            .await;

        let raw_output = match exec_result {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Execution error: {}", e),
                })
            }
            Err(_) => {
                return Err(AiomeError::Infrastructure {
                    reason: "Execution timed out after 300s".to_string(),
                })
            }
        };

        // Layer 5: Sterilization
        // Validate against XSS/malicious prompts via Unified Response Purger
        if let shared::guardrails::ValidationResult::Blocked(reason) =
            shared::guardrails::validate_input(&raw_output)
        {
            warn!(
                "🚨 [DockerConductor] Shadow Clone output BLOCKED by Guardrails: {}",
                reason
            );
            return Err(AiomeError::SecurityViolation {
                reason: format!(
                    "Shadow Clone output was blocked by security guardrails: {}",
                    reason
                ),
            });
        }

        // Purge sensitive entities
        let clean_output = aiome_core::security_impl::purge_entities(&raw_output);

        info!(
            "✅ [DockerConductor] Shadow Clone session {} completed cleanly in {}ms.",
            session_id, duration_ms
        );

        Ok(clean_output)
    }
}
