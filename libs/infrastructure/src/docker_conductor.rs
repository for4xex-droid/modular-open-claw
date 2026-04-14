/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::grpc::a2a_grpc_client::{A2aGrpcClient, GrpcClientConfig};
use crate::security::{BastionGuard, PermissionManifest, RuntimeJail, SandboxProfile};
use crate::task_orchestrator::{TaskConductor, TaskEvent};
use aiome_core::error::AiomeError;
use aiome_core::traits::Job;
use aiome_core_contracts::a2a::{A2aClient, A2aTaskRequest};
use aiome_core_contracts::commerce::CommerceEngine;
use async_trait::async_trait;
use base64::Engine;
use dashmap::DashMap;
use futures::StreamExt;
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};
use tracing::{error, info, warn};
use uuid::Uuid;

/// 影分身 (Shadow Worker) を安全に実行するための Conductor
/// BastionGuard, Fork Bomb Protection (Semaphore), Guardrails Sterilization, CommerceEngine Billing を統合。
pub struct DockerConductor {
    bastion: BastionGuard,
    commerce_engine: Option<Arc<dyn CommerceEngine>>,
    concurrency_limit: Arc<Semaphore>,
    grpc_config: GrpcClientConfig,
    active_escrows: DashMap<String, String>,
    /// Shadow Worker コンテナに渡す Gemini API キー（パージ済み環境変数の代替）
    gemini_api_key: SecretString,
}

impl DockerConductor {
    pub fn new(
        commerce_engine: Option<Arc<dyn CommerceEngine>>,
        grpc_config: GrpcClientConfig,
        gemini_api_key: SecretString,
    ) -> Self {
        Self {
            bastion: BastionGuard::new_internal(PermissionManifest::default()),
            commerce_engine,
            concurrency_limit: Arc::new(Semaphore::new(3)), // MAX 3 concurrent shadow clones
            grpc_config,
            active_escrows: DashMap::new(),
            gemini_api_key,
        }
    }

    async fn _do_conduct(
        &self,
        job: Job,
        progress_tx: tokio::sync::mpsc::Sender<TaskEvent>,
    ) -> Result<(String, Option<String>), AiomeError> {
        if let Err(e) = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: "Acquiring capacity (Concurrency Limit)...".to_string(),
                percent: Some(5),
            })
            .await
        {
            tracing::warn!("Failed to send progress event: {}", e);
        }

        // Layer 1: Fork Bomb Protection
        // Wait for a permit. If we exceed 3, we wait here.
        let _permit =
            self.concurrency_limit
                .acquire()
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Semaphore closed: {}", e),
                })?;

        if let Err(e) = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: "Capacity acquired. Validating authorization...".to_string(),
                percent: Some(10),
            })
            .await
        {
            tracing::warn!("Failed to send progress event: {}", e);
        }

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

                // Deduct cost via Escrow
                let escrow_id = engine
                    .escrow_create(agent_id, 1) // 1 token per invocation
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to create escrow: {:?}", e),
                    })?;

                if let Some(previous_escrow_id) = self
                    .active_escrows
                    .insert(job.id.clone(), escrow_id.clone())
                {
                    // Self-Healing mechanism: Catch dangling escrows from previous failed rollbacks.
                    tracing::warn!("⚠️ [DockerConductor] Found dangling escrow {} for job {} during retry. Triggering background refund payload.", previous_escrow_id, job.id);
                    let engine_clone = engine.clone();
                    tokio::spawn(async move {
                        let mut success = false;
                        for attempt in 1..=5 {
                            match engine_clone.escrow_refund(&previous_escrow_id).await {
                                Ok(_) => {
                                    tracing::info!("✅ [DockerConductor] Successfully refunded dangling escrow {}.", previous_escrow_id);
                                    success = true;
                                    break;
                                }
                                Err(e) => {
                                    tokio::time::sleep(tokio::time::Duration::from_secs(attempt))
                                        .await;
                                }
                            }
                        }
                        if !success {
                            tracing::error!("🚨 CRITICAL: Dangling escrow {} permanently failed to refund. Requires manual reconciliation.", previous_escrow_id);
                        }
                    });
                }

                info!(
                    "💳 [DockerConductor] Locked 1 token in escrow ({}) for Shadow Clone usage.",
                    escrow_id
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

        // One-time Token generation for gRPC Authorization
        let auth_token = Uuid::new_v4().to_string();

        if let Err(e) = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: "Executing worker container via BastionGuard...".to_string(),
                percent: Some(30),
            })
            .await
        {
            tracing::warn!("Failed to send progress event: {}", e);
        }

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
                    reason: "Docker capability check failed.".to_string(),
                });
            }
        }

        let container_name = format!("aiome-job-{}", job.id);

        // Gap S: Ensure aiome-internal network exists (idempotent)
        // Note: BastionGuard executes binaries directly without a shell, so we cannot use `|| true` or redirections.
        let _ = self
            .bastion
            .safe_exec_with_profile(
                "docker network create --driver bridge aiome-internal",
                SandboxProfile::Strict,
            )
            .await;

        // Gap R: Write secrets to ephemeral env-file instead of CLI args (Threat #39 mitigation)
        // This prevents API keys from being visible via `ps aux` on the host.
        let env_file_path = temp_dir.join(".env.shadow");
        let env_file_content = format!(
            "A2A_AUTH_TOKEN={}\nGEMINI_API_KEY={}\n",
            auth_token,
            self.gemini_api_key.expose_secret()
        );
        if let Err(e) = std::fs::write(&env_file_path, &env_file_content) {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(AiomeError::Infrastructure {
                reason: format!("Failed to write env-file for shadow worker: {}", e),
            });
        }
        // Restrict env-file permissions to owner-only (0600)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ =
                std::fs::set_permissions(&env_file_path, std::fs::Permissions::from_mode(0o600));
        }

        // Detached Docker Execution (gRPC Server) - Gap I, J, L, N, R, S
        let cmd = format!(
            "docker run -d --name {} --network aiome-internal -p 127.0.0.1:0:50051 -v {}:/app/config/agent.yaml:ro --env-file {} aiome-shadow-worker",
            container_name,
            yaml_path.display(),
            env_file_path.display()
        );

        let start = std::time::Instant::now();

        // 1. Start container
        let container_start_result = self
            .bastion
            .safe_exec_with_profile(&cmd, SandboxProfile::Strict)
            .await;

        // Immediately wipe env-file after container start (secrets no longer needed on host)
        let _ = std::fs::remove_file(&env_file_path);

        if let Err(e) = container_start_result {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(AiomeError::Infrastructure {
                reason: format!("Failed to start shadow worker container: {:?}", e),
            });
        }

        // 2. Get dynamic port mapped to 50051
        let port_cmd = format!("docker port {} 50051", container_name);

        // Simple retry loop to wait for container to bind port
        let mut mapped_port = String::new();
        for _ in 0..10 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if let Ok(out) = self
                .bastion
                .safe_exec_with_profile(&port_cmd, SandboxProfile::Strict)
                .await
            {
                // Expected output format: 127.0.0.1:32768
                if let Some(port) = out.trim().split(':').last() {
                    mapped_port = port.to_string();
                    break;
                }
            }
        }

        if mapped_port.is_empty() {
            if let Err(e) = self.cancel(&job.id).await {
                tracing::warn!("Best-effort container cleanup failed for {}: {}", job.id, e);
            }
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(AiomeError::Infrastructure {
                reason: "Failed to resolve worker mapped port".to_string(),
            });
        }

        // 3. Connect via gRPC
        let mut client_config = self.grpc_config.clone();
        client_config.endpoint_url = format!("http://127.0.0.1:{}", mapped_port);
        client_config.auth_token = auth_token.clone();

        let grpc_client = A2aGrpcClient::new(client_config);
        let req = A2aTaskRequest {
            job_id: job.id.clone(),
            prompt_b64: task_prompt_b64,
            artifact_path: None,
            agent_yaml_b64: base64::engine::general_purpose::STANDARD.encode(agent_yaml),
            auth_token: auth_token.clone(),
            proof_of_intent: None,
            sender_did: None,
        };

        // 3.5 Gap C: gRPC Health Check
        let mut health_ok = false;
        for _ in 0..15 {
            let health_res: Result<(), AiomeError> = grpc_client.check_health().await;
            if health_res.is_ok() {
                health_ok = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        if !health_ok {
            if let Err(e) = self.cancel(&job.id).await {
                tracing::warn!("Best-effort container cleanup failed for {}: {}", job.id, e);
            }
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(AiomeError::Infrastructure {
                reason: "Shadow Worker gRPC health check failed after startup".to_string(),
            });
        }

        // 4. Stream Results
        let stream_result: Result<Result<_, AiomeError>, tokio::time::error::Elapsed> =
            tokio::time::timeout(Duration::from_secs(300), async {
                grpc_client.execute_task(req).await
            })
            .await;

        let mut raw_output = String::new();
        let mut final_result_hash: Option<String> = None;
        let mut is_completed = false; // Gap G

        match stream_result {
            Ok(Ok(mut stream)) => {
                let mut stream = stream;
                while let Some(progress_item) = tokio_stream::StreamExt::next(&mut stream).await {
                    match progress_item {
                        Ok(p) => {
                            if let Err(e) = progress_tx
                                .send(TaskEvent::Progress {
                                    job_id: job.id.clone(),
                                    conductor_id: self.conductor_name().to_string(),
                                    message: p.message.clone(),
                                    percent: Some(p.percent.min(100) as u8),
                                })
                                .await
                            {
                                tracing::warn!("Failed to send stream progress event: {}", e);
                            }

                            if p.is_failed {
                                if let Err(e) = self.cancel(&job.id).await {
                                    tracing::warn!(
                                        "Best-effort container cleanup failed for {}: {}",
                                        job.id,
                                        e
                                    );
                                }
                                let _ = std::fs::remove_dir_all(&temp_dir);
                                return Err(AiomeError::Infrastructure {
                                    reason: format!(
                                        "Shadow Worker failed: {}",
                                        p.error.unwrap_or_default()
                                    ),
                                });
                            } else if p.is_completed {
                                raw_output = p.result.unwrap_or_default();
                                final_result_hash = p.result_hash;
                                is_completed = true;
                                break;
                            }
                        }
                        Err(e) => {
                            if let Err(e) = self.cancel(&job.id).await {
                                tracing::warn!(
                                    "Best-effort container cleanup failed for {}: {}",
                                    job.id,
                                    e
                                );
                            }
                            let _ = std::fs::remove_dir_all(&temp_dir);
                            return Err(e);
                        }
                    }
                }

                if !is_completed {
                    if let Err(e) = self.cancel(&job.id).await {
                        tracing::warn!(
                            "Best-effort container cleanup failed for {}: {}",
                            job.id,
                            e
                        );
                    }
                    let _ = std::fs::remove_dir_all(&temp_dir);
                    return Err(AiomeError::Infrastructure {
                        reason:
                            "Shadow Worker stream terminated unexpectedly before completion (Gap G)"
                                .to_string(),
                    });
                }
            }
            Ok(Err(e)) => {
                if let Err(e) = self.cancel(&job.id).await {
                    tracing::warn!("Best-effort container cleanup failed for {}: {}", job.id, e);
                }
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err(e);
            }
            Err(_) => {
                if let Err(e) = self.cancel(&job.id).await {
                    tracing::warn!("Best-effort container cleanup failed for {}: {}", job.id, e);
                }
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err(AiomeError::Infrastructure {
                    reason: "gRPC Stream Execution timed out after 300s".to_string(),
                });
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let _ = std::fs::remove_dir_all(&temp_dir);

        // Gap K: Clean up container after success
        if let Err(e) = self.stop_and_remove_container(&job.id).await {
            tracing::warn!("Best-effort container cleanup failed for {}: {}", job.id, e);
        }

        if let Err(e) = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: "Execution completed. Sterilizing output...".to_string(),
                percent: Some(90),
            })
            .await
        {
            tracing::warn!("Failed to send sterilization progress event: {}", e);
        }

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

        // Success: Release escrow (pay to system)
        if let Some((_, escrow_id)) = self.active_escrows.remove(&job.id) {
            if let Some(engine) = &self.commerce_engine {
                tracing::info!(
                    "💳 [DockerConductor] Shadow Clone job {} succeeded. Releasing escrow {}.",
                    job.id,
                    escrow_id
                );

                let mut success = false;
                for attempt in 1..=3 {
                    if let Err(e) = engine.escrow_release(&escrow_id, uuid::Uuid::nil()).await {
                        tracing::warn!(
                            "Release attempt {} failed for escrow {}: {:?}",
                            attempt,
                            escrow_id,
                            e
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(500 * attempt)).await;
                    } else {
                        success = true;
                        break;
                    }
                }

                if !success {
                    tracing::error!("🚨 CRITICAL: Failed to release escrow {} after 3 attempts. Platform revenue may be lost. Rolling back to active escrows.", escrow_id);
                    self.active_escrows.insert(job.id.clone(), escrow_id);
                }
            }
        }

        info!(
            "✅ [DockerConductor] Shadow Clone session {} completed cleanly in {}ms.",
            session_id, duration_ms
        );

        Ok((clean_output, final_result_hash))
    }

    async fn stop_and_remove_container(&self, job_id: &str) -> Result<(), AiomeError> {
        let container_name = format!("aiome-job-{}", job_id);
        info!(
            "🐳 [DockerConductor] Stopping container: {}",
            container_name
        );

        // Attempt to stop and remove the container
        let cmd = format!("docker stop {}", container_name);
        match self
            .bastion
            .safe_exec_with_profile(&cmd, SandboxProfile::Default)
            .await
        {
            Ok(_) => {
                info!(
                    "✅ [DockerConductor] Container {} stopped successfully.",
                    container_name
                );
                // Also try to remove it
                let _ = self
                    .bastion
                    .safe_exec_with_profile(
                        &format!("docker rm {}", container_name),
                        SandboxProfile::Default,
                    )
                    .await;
            }
            Err(e) => {
                warn!(
                    "⚠️ [DockerConductor] Failed to stop container {}: {:?}",
                    container_name, e
                );
            }
        }
        Ok(())
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
        progress_tx: tokio::sync::mpsc::Sender<TaskEvent>,
    ) -> Result<(String, Option<String>), AiomeError> {
        let result = self._do_conduct(job.clone(), progress_tx).await;
        if result.is_err() {
            tracing::warn!(
                "⚠️ [DockerConductor] Job {} failed. Triggering failsafe cancellation & refund.",
                job.id
            );
            if let Err(e) = self.cancel(&job.id).await {
                tracing::error!(
                    "🚨 [DockerConductor] Critical failure in failsafe cleanup for {}: {}",
                    job.id,
                    e
                );
            }
        }
        result
    }

    async fn cancel(&self, job_id: &str) -> Result<(), AiomeError> {
        info!(
            "🛑 [DockerConductor] Processing cancellation for job: {}",
            job_id
        );

        // Refund escrow if exists (meaning it wasn't successfully released)
        if let Some((_, escrow_id)) = self.active_escrows.remove(job_id) {
            if let Some(engine) = &self.commerce_engine {
                tracing::info!(
                    "💳 [DockerConductor] Refunding unused escrow {} for job {}",
                    escrow_id,
                    job_id
                );

                let mut success = false;
                for attempt in 1..=3 {
                    match engine.escrow_refund(&escrow_id).await {
                        Ok(_) => {
                            success = true;
                            break;
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Refund attempt {} failed for escrow {}: {:?}",
                                attempt,
                                escrow_id,
                                e
                            );
                            tokio::time::sleep(std::time::Duration::from_millis(500 * attempt))
                                .await;
                        }
                    }
                }

                if !success {
                    tracing::error!("🚨 CRITICAL: Failed to refund escrow {} after 3 attempts. Tokens may be locked. Rolling back to active escrows.", escrow_id);
                    // Rollback to prevent token leakage, so a subsequent GC or manual retry can catch it.
                    self.active_escrows.insert(job_id.to_string(), escrow_id);
                }
            }
        }

        self.stop_and_remove_container(job_id).await
    }
}
