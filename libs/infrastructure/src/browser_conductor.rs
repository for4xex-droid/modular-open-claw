/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::task_orchestrator::{TaskConductor, TaskEvent};
use aiome_contracts::commerce::CommerceEngine;
use aiome_core::error::AiomeError;
use aiome_core::traits::Job;
use async_trait::async_trait;
use secrecy::SecretString;
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use uuid::Uuid;

/// BrowserConductor: browser-use 統合のための TaskConductor
pub struct BrowserConductor {
    pub commerce_engine: Option<Arc<dyn CommerceEngine>>,
    pub gemini_api_key: SecretString,
    pub concurrency_limit: Arc<Semaphore>,
}

impl BrowserConductor {
    pub fn new(
        commerce_engine: Option<Arc<dyn CommerceEngine>>,
        gemini_api_key: SecretString,
    ) -> Self {
        Self {
            commerce_engine,
            gemini_api_key,
            concurrency_limit: Arc::new(Semaphore::new(1)), // OOM回避のための直列実行
        }
    }

    pub fn sanitize_payload(&self, topic: &str) -> Result<String, AiomeError> {
        let mut parsed: serde_json::Value =
            serde_json::from_str(topic).unwrap_or_else(|_| serde_json::json!({}));

        // ハードコードされた防壁 (Red Team対応)
        if let Some(obj) = parsed.as_object_mut() {
            obj.insert("max_steps".into(), serde_json::json!(20));
            obj.insert("max_actions_per_step".into(), serde_json::json!(3));
        } else {
            parsed = serde_json::json!({
                "max_steps": 20,
                "max_actions_per_step": 3
            });
        }

        Ok(parsed.to_string())
    }
}

#[async_trait]
impl TaskConductor for BrowserConductor {
    fn conductor_name(&self) -> &str {
        "BrowserConductor"
    }

    fn capable_categories(&self) -> Vec<String> {
        vec!["browser_automation".into()]
    }

    async fn conduct(
        &self,
        job: Job,
        progress_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<(String, Option<String>), AiomeError> {
        let topic = self.sanitize_payload(&job.topic)?;
        let parsed: serde_json::Value =
            serde_json::from_str(&topic).unwrap_or_else(|_| serde_json::json!({}));
        let provider = parsed
            .get("llm_provider")
            .and_then(|v| v.as_str())
            .unwrap_or("gemini");

        let cost = if provider == "ollama" { 0 } else { 100 };
        let agent_uuid = job.agent_id.unwrap_or_default();
        let mut escrow_id = String::new();

        if cost > 0 {
            if let Some(engine) = &self.commerce_engine {
                escrow_id = engine.escrow_create(agent_uuid, cost).await?;
            }
        }

        let _permit =
            self.concurrency_limit
                .acquire()
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to acquire semaphore: {}", e),
                })?;

        // Write payload to a temporary file
        let session_id = Uuid::new_v4().to_string();
        let temp_dir = std::env::temp_dir().join(format!("aiome-browser-{}", session_id));
        std::fs::create_dir_all(&temp_dir).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create sandbox: {}", e),
        })?;

        let payload_path = temp_dir.join("payload.json");
        std::fs::write(&payload_path, &topic).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to write payload: {}", e),
        })?;

        let env_file_path = temp_dir.join(".env.shadow");
        let env_content = if provider == "ollama" {
            "OLLAMA_BASE_URL=http://host.docker.internal:11434\n".to_string()
        } else {
            use secrecy::ExposeSecret;
            format!("GEMINI_API_KEY={}\n", self.gemini_api_key.expose_secret())
        };

        std::fs::write(&env_file_path, env_content).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to write env file: {}", e),
        })?;

        use crate::security::SafeCommandBuilder;
        use aiome_contracts::security::SandboxProfile;

        let runtime = shared::container_runtime::detect_runtime();

        let mut cmd = SafeCommandBuilder::new(runtime)
            .args(vec![
                "run".to_string(),
                "--rm".to_string(),
                "-i".to_string(),
                "--memory=1g".to_string(),
                "--cpus=1.0".to_string(),
                "--cap-drop=ALL".to_string(),
                "--security-opt=no-new-privileges".to_string(),
                "--network=aiome-internal".to_string(),
                format!("--env-file={}", env_file_path.display()),
                "aiome-browser-use".to_string(),
            ])
            .profile(SandboxProfile::BrowserAgent)
            .build_internal()?;

        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to spawn docker: {}", e),
        })?;

        // Write payload to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let result: Result<(), std::io::Error> = stdin.write_all(topic.as_bytes()).await;
            if result.is_ok() {
                let _ = stdin.shutdown().await;
            }
        }

        // Read stdout
        let mut final_result = String::new();
        if let Some(stdout) = child.stdout.take() {
            use tokio::io::AsyncBufReadExt;
            let mut reader = tokio::io::BufReader::new(stdout).lines();

            while let Ok(Some(line)) = reader.next_line().await {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(event) = parsed.get("event").and_then(|v| v.as_str()) {
                        match event {
                            "progress" => {
                                let msg = parsed
                                    .get("data")
                                    .and_then(|d| d.get("message"))
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("");
                                let _ = progress_tx
                                    .send(TaskEvent::Progress {
                                        job_id: job.id.clone(),
                                        conductor_id: "browser-use".into(),
                                        percent: Some(50),
                                        message: msg.to_string(),
                                    })
                                    .await;
                            }
                            "completed" => {
                                final_result = parsed
                                    .get("data")
                                    .and_then(|d| d.get("result"))
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("")
                                    .to_string();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let status: std::process::ExitStatus =
            child.wait().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Docker wait error: {}", e),
            })?;

        let _ = std::fs::remove_dir_all(&temp_dir);

        if status.success() {
            if cost > 0 && !escrow_id.is_empty() {
                if let Some(engine) = &self.commerce_engine {
                    let _ = engine.escrow_release(&escrow_id, uuid::Uuid::nil()).await;
                }
            }
            Ok((final_result, None))
        } else {
            if cost > 0 && !escrow_id.is_empty() {
                if let Some(engine) = &self.commerce_engine {
                    let _ = engine.escrow_refund(&escrow_id).await;
                }
            }
            Err(AiomeError::Infrastructure {
                reason: "Browser container failed".into(),
            })
        }
    }
}
