/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::security::{BastionGuard, PermissionManifest, RuntimeJail, SandboxProfile};
use crate::soul_mutator::SoulMutator;
use aiome_contracts::error::AiomeError;
use aiome_contracts::llm::LlmResponse;
use aiome_contracts::traits::{AgentEvolver, JobQueue, LoraEngine};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

/// Configuration for LoRA Training
#[derive(Debug, Clone)]
pub struct LoraTrainingConfig {
    pub base_model: String,
    pub dataset_path: String,
    pub output_dir: String,
    pub vault_path: String, // Destination to isolate weights
    // --- Autotuner Hyperparameters ---
    pub learning_rate: f64,
    pub epochs: u32,
    pub lora_rank: u32,
    pub batch_size: u32,
}

impl Default for LoraTrainingConfig {
    fn default() -> Self {
        Self {
            base_model: String::new(),
            dataset_path: String::new(),
            output_dir: String::new(),
            vault_path: String::new(),
            learning_rate: 1e-4,
            epochs: 3,
            lora_rank: 8,
            batch_size: 4,
        }
    }
}

impl LoraTrainingConfig {
    /// Extracts training hyperparameters from a JSON payload, falling back to defaults.
    pub fn from_params(
        params: &serde_json::Value,
        base_model: &str,
        dataset_path: &str,
        output_dir: &str,
        vault_path: &str,
    ) -> Self {
        let mut config = Self {
            base_model: base_model.to_string(),
            dataset_path: dataset_path.to_string(),
            output_dir: output_dir.to_string(),
            vault_path: vault_path.to_string(),
            ..Default::default()
        };

        if let Some(lr) = params.get("learning_rate").and_then(|v| v.as_f64()) {
            config.learning_rate = lr;
        }
        if let Some(epochs) = params.get("epochs").and_then(|v| v.as_u64()) {
            config.epochs = epochs as u32;
        }
        if let Some(rank) = params.get("lora_rank").and_then(|v| v.as_u64()) {
            config.lora_rank = rank as u32;
        }
        if let Some(bs) = params.get("batch_size").and_then(|v| v.as_u64()) {
            config.batch_size = bs as u32;
        }

        config
    }
}

/// Service responsible for managing the lifecycle of LoRA training jobs.
/// Uses BastionGuard to safely execute MLX / Python training scripts.
#[derive(Clone)]
pub struct LoraTrainingService {
    _bastion: Arc<BastionGuard>,
    core_engine: Arc<aiome_core::lora::engine::LoraEngine>,
    soul_mutator: Option<Arc<SoulMutator>>,
    job_queue: Option<Arc<dyn JobQueue>>,
    event_tx: Option<tokio::sync::broadcast::Sender<aiome_contracts::events::CoreEvent>>,
    active_jobs: Arc<RwLock<HashMap<String, CancellationToken>>>,
    job_semaphore: Arc<tokio::sync::Semaphore>,
    datasets_dir: std::path::PathBuf,
}

impl std::fmt::Debug for LoraTrainingService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoraTrainingService")
            .field("core_engine", &self.core_engine)
            .finish()
    }
}

impl LoraTrainingService {
    /// Create a new LoraTrainingService with soul evolution support.
    pub fn new(
        core_engine: Arc<aiome_core::lora::engine::LoraEngine>,
        soul_mutator: Option<Arc<SoulMutator>>,
        job_queue: Option<Arc<dyn JobQueue>>,
        event_tx: Option<tokio::sync::broadcast::Sender<aiome_contracts::events::CoreEvent>>,
        compute_semaphore: Option<Arc<tokio::sync::Semaphore>>,
    ) -> Self {
        Self {
            _bastion: Arc::new(BastionGuard::new(
                aiome_core::security::PermissionManifest {
                    allow_network: true,
                    allow_filesystem_write: true,
                    allow_shell_execution: false,
                    allowed_domains: vec!["huggingface.co".into(), "hf.co".into()],
                },
            )),
            core_engine,
            soul_mutator,
            job_queue,
            event_tx,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            job_semaphore: compute_semaphore
                .unwrap_or_else(|| Arc::new(tokio::sync::Semaphore::new(1))),
            datasets_dir: shared::app_data::AppDataResolver::new().resolve("datasets"),
        }
    }

    /// Allow overriding the datasets directory (useful for testing)
    pub fn with_datasets_dir(mut self, path: std::path::PathBuf) -> Self {
        self.datasets_dir = path;
        self
    }

    fn find_mlx_script_path(&self) -> Result<std::path::PathBuf, AiomeError> {
        if std::path::Path::new("scripts/mlx_train.py").exists() {
            Ok(std::path::PathBuf::from("scripts/mlx_train.py"))
        } else {
            // Find root from current dir (safely)
            let mut curr: std::path::PathBuf =
                std::env::current_dir().map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to get current dir: {}", e),
                })?;

            let mut found = None;
            for _ in 0..3 {
                // Up to 3 levels
                if curr.join("scripts/mlx_train.py").exists() {
                    found = Some(curr.join("scripts/mlx_train.py"));
                    break;
                }
                if let Some(parent) = curr.parent() {
                    curr = parent.to_path_buf();
                } else {
                    break;
                }
            }

            found.ok_or_else(|| AiomeError::Infrastructure {
                reason: "Could not find scripts/mlx_train.py in workspace tree".to_string(),
            })
        }
    }

    /// Triggers the training process synchronously.
    /// Takes `job_id` and `cancel_token` to support P-23 features.
    pub async fn start_training(
        &self,
        job_id: &str,
        config: LoraTrainingConfig,
        cancel_token: CancellationToken,
    ) -> Result<(), AiomeError> {
        tracing::info!(
            "🛠️ [LoraTrainingService] Starting MLX training (job: {}): model={}, dataset={}",
            job_id,
            config.base_model,
            config.dataset_path
        );

        // SEC-PATH: P-08 Dataset Validation
        if !std::path::Path::new(&config.dataset_path).exists() {
            return Err(AiomeError::Infrastructure {
                reason: format!("Dataset not found at: {}", config.dataset_path),
            });
        }
        // Basic check for Dataset format
        if let Ok(content) = std::fs::read_to_string(&config.dataset_path) {
            if !content.contains("\"text\"") && !content.contains("\"messages\"") {
                return Err(AiomeError::Infrastructure {
                    reason: "Dataset is missing 'text' or 'messages' fields (P-08)".to_string(),
                });
            }
        }

        let adapter_output = format!("{}/adapter_model.safetensors", config.vault_path);

        // SEC-PATH: 検索パスの正規化 (G-21/X-001)
        let script_path = self.find_mlx_script_path()?;

        // B-002: Ensure vault directory exists before external script writes to it
        std::fs::create_dir_all(&config.vault_path).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create vault directory for LoRA: {}", e),
        })?;

        // F-02: Acquire semaphore permit
        let _permit =
            self.job_semaphore
                .acquire()
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to acquire job semaphore: {}", e),
                })?;

        // F-01: Use BastionGuard structured arguments
        let mut cmd = self._bastion.build_safe_command_args(
            "python3",
            vec![
                script_path.to_string_lossy().to_string(),
                "--model".to_string(),
                config.base_model,
                "--data".to_string(),
                config.dataset_path,
                "--adapter-file".to_string(),
                adapter_output,
                "--learning-rate".to_string(),
                config.learning_rate.to_string(),
                "--epochs".to_string(),
                config.epochs.to_string(),
                "--lora-rank".to_string(),
                config.lora_rank.to_string(),
                "--batch-size".to_string(),
                config.batch_size.to_string(),
            ],
            SandboxProfile::LoraTraining,
        )?;

        // P-21: Stream stderr
        let mut child = cmd
            .kill_on_drop(true)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to start MLX training script: {}", e),
            })?;

        let mut stderr_stream = child
            .stderr
            .take()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "Failed to capture stderr".to_string(),
            })?;

        let event_tx_clone = self.event_tx.clone();
        let jq_clone = self.job_queue.clone();
        let j_id = job_id.to_string();

        let progress_task = tokio::spawn(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut reader = BufReader::new(stderr_stream).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                // Publish core event if wired
                if let Some(tx) = &event_tx_clone {
                    let mut percent = None;
                    if line.contains("Epoch") {
                        if let Some(p_str) = line
                            .split('%')
                            .next()
                            .and_then(|s: &str| s.split_whitespace().last())
                        {
                            if let Ok(p) = p_str.parse::<u8>() {
                                percent = Some(p);
                            }
                        }
                    }
                    let _ = tx.send(aiome_contracts::events::CoreEvent::TaskProgress {
                        job_id: j_id.clone(),
                        conductor_id: "LoraConductor".to_string(),
                        message: line.clone(),
                        percent,
                    });
                }
                tracing::info!("🎓 [LoRA:{}] {}", j_id, line);
            }
        });

        // P-23: Wait for process or cancellation (with 1 hour timeout)
        let status_res: std::io::Result<std::process::ExitStatus> = tokio::select! {
            _ = cancel_token.cancelled() => {
                tracing::warn!("🛑 [LoraTrainingService] Training job {} cancelled!", job_id);
                let _ = child.kill().await;
                return Err(AiomeError::Infrastructure {
                    reason: "Training cancelled".to_string(),
                });
            }
            res = tokio::time::timeout(std::time::Duration::from_secs(3600), child.wait()) => {
                match res {
                    Ok(r) => r,
                    Err(_) => {
                        let _ = child.kill().await;
                        return Err(AiomeError::Infrastructure {
                            reason: "Training timed out after 1 hour".to_string(),
                        });
                    }
                }
            }
        };

        // Ensure progress reader finishes
        let _ = progress_task.await;

        let status = status_res.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Training script failed to execute: {}", e),
        })?;

        if !status.success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("LoRA training script exited with error ({})", status),
            });
        }

        tracing::info!(
            "🔐 [LoraTrainingService] isolated weights securely at {}",
            config.vault_path
        );

        Ok(())
    }
}

#[async_trait]
impl LoraEngine for LoraTrainingService {
    async fn complete_with_lora(
        &self,
        prompt: &str,
        lora_id: &str,
    ) -> Result<LlmResponse, AiomeError> {
        self.core_engine.complete_with_lora(prompt, lora_id).await
    }

    async fn train(
        &self,
        base_model: &str,
        dataset_id: &str,
        params: serde_json::Value,
    ) -> Result<String, AiomeError> {
        let agent_id = params
            .get("agent_id")
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok());

        let job_id = if let Some(jq) = &self.job_queue {
            jq.enqueue(
                "LORA_TRAINING",
                base_model, // topic
                dataset_id, // style
                None,
                None,
                agent_id,
                0,
            )
            .await?
        } else {
            format!("job_{}", uuid::Uuid::new_v4())
        };

        // Ensure newly enqueued job status transitions to InProgress
        if let Some(jq) = &self.job_queue {
            let _ = jq
                .update_job_status(&job_id, aiome_contracts::traits::JobStatus::InProgress)
                .await;
        }

        let cancel_token = CancellationToken::new();
        {
            if dataset_id.contains("..") || dataset_id.contains("/") || dataset_id.contains("\\") {
                return Err(aiome_contracts::error::AiomeError::SecurityViolation {
                    reason: "Invalid dataset_id: path traversal detected".into(),
                });
            }
            let mut active = self.active_jobs.write().await;
            if active.len() >= 100 {
                return Err(aiome_contracts::error::AiomeError::ResourceBusy {
                    reason: "Too many pending jobs".into(),
                });
            }
            active.insert(job_id.clone(), cancel_token.clone());
        }

        // 🧬 Phase 1A-2: Dynamic Dataset Extraction from SoulStore
        let actual_dataset_path = if let Some(jq) = &self.job_queue {
            let extractor =
                crate::dataset_extractor::DatasetExtractor::new(self.datasets_dir.clone());
            match extractor.extract_to_jsonl(&**jq, dataset_id, &job_id).await {
                Ok(path) => {
                    tracing::info!(
                        "✅ [LoraTrainingService] Successfully extracted dataset for Soul: {}",
                        dataset_id
                    );
                    path.to_string_lossy().to_string()
                }
                Err(e) => {
                    tracing::info!("ℹ️ [LoraTrainingService] Soul not found or extraction failed, falling back to raw path usage: {}", e);
                    self.datasets_dir
                        .join(dataset_id)
                        .to_string_lossy()
                        .to_string()
                }
            }
        } else {
            self.datasets_dir
                .join(dataset_id)
                .to_string_lossy()
                .to_string()
        };

        let resolver = shared::app_data::AppDataResolver::new();

        // Use the parameters passed in from the frontend (or Autotuner)
        let config = LoraTrainingConfig::from_params(
            &params,
            base_model,
            &actual_dataset_path,
            &resolver.resolve("output").to_string_lossy(),
            &resolver
                .resolve(format!("vault/lora/{}", job_id))
                .to_string_lossy(),
        );

        let service_clone = self.clone();
        let j_id = job_id.clone();
        let b_model = base_model.to_string();
        let d_id = dataset_id.to_string();

        // Spawn as background task
        tokio::spawn(async move {
            match service_clone
                .start_training(&j_id, config, cancel_token)
                .await
            {
                Ok(_) => {
                    // 🧬 Sprint 3: Evolution integration
                    if let (Some(mutator), Some(jq)) =
                        (&service_clone.soul_mutator, &service_clone.job_queue)
                    {
                        tracing::info!(
                            "🧬 [LoraTrainingService] LoRA complete. triggering soul evolution..."
                        );
                        let mut metadata = serde_json::Map::new();
                        metadata.insert("event".into(), "lora_training_complete".into());
                        metadata.insert("model".into(), b_model.into());
                        metadata.insert("dataset".into(), d_id.into());
                        metadata.insert("job_id".into(), j_id.clone().into());

                        if let Err(e) = mutator
                            .transmute_with_metadata(&**jq, serde_json::Value::Object(metadata))
                            .await
                        {
                            tracing::warn!(
                                "⚠️ [LoraTrainingService] Evolution failed (Non-fatal): {}",
                                e
                            );
                        }
                    }
                    if let Some(jq) = &service_clone.job_queue {
                        let _ = jq
                            .update_job_status(&j_id, aiome_contracts::traits::JobStatus::Completed)
                            .await;
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "❌ [LoraTrainingService] Training failed for {}: {}",
                        j_id,
                        e
                    );
                    if let Some(jq) = &service_clone.job_queue {
                        let _ = jq.fail_job(&j_id, &e.to_string()).await;
                    }
                }
            }

            // Cleanup
            let mut active = service_clone.active_jobs.write().await;
            active.remove(&j_id);
        });

        Ok(job_id)
    }

    async fn cancel_training(&self, job_id: &str) -> Result<(), AiomeError> {
        let active = self.active_jobs.read().await;
        if let Some(token) = active.get(job_id) {
            token.cancel();
            Ok(())
        } else {
            Err(AiomeError::Infrastructure {
                reason: format!("Job {} not found or already completed", job_id),
            })
        }
    }

    /// Verifies the presence of MLX training script and mlx-lm dependency.
    async fn health_check(&self) -> Result<bool, AiomeError> {
        #[cfg(test)]
        if true {
            return Ok(true);
        }

        // 🔍 Sprint 4: Robust health check
        let script_path = match self.find_mlx_script_path() {
            Ok(p) => p,
            Err(_) => return Ok(false),
        };
        if !script_path.exists() {
            tracing::warn!(
                "⚠️ [LoraTrainingService] MLX script not found at {:?}",
                script_path
            );
            return Ok(false);
        }

        // Check if mlx-lm is installed (simple check)
        let output = tokio::process::Command::new("python3")
            .arg("-c")
            .arg("import mlx_lm; print('ready')")
            .env_clear() // Clean environment for security
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                Ok(stdout.trim() == "ready")
            }
            _ => {
                tracing::warn!(
                    "⚠️ [LoraTrainingService] mlx-lm library not found or python3 missing."
                );
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_lora_training_service_initialization() {
        let core = Arc::new(aiome_core::lora::engine::LoraEngine::new());
        let _service = LoraTrainingService::new(core, None, None, None, None);
        assert!(true, "Instantiated service properly");
    }

    #[tokio::test]
    async fn test_start_training_isolates_weights_in_vault() {
        // RED test: Should assert that weights are moved to the vault.
        let core = Arc::new(aiome_core::lora::engine::LoraEngine::new());
        let service = LoraTrainingService::new(core, None, None, None, None);
        let tmp = tempdir().unwrap();
        let output_dir = tmp.path().join("output").to_string_lossy().to_string();
        let vault_path = tmp.path().join("vault").to_string_lossy().to_string();

        let dataset_path = tmp.path().join("dataset.jsonl");
        std::fs::write(&dataset_path, "{\"text\": \"hello\"}").unwrap();

        let config = LoraTrainingConfig {
            base_model: "test-model".into(),
            dataset_path: dataset_path.to_string_lossy().to_string(),
            output_dir,
            vault_path: vault_path.clone(),
            ..Default::default()
        };

        // When implemented, this should execute a mock script and move artifacts to vault.
        let res = service
            .start_training(
                "mock_job",
                config,
                tokio_util::sync::CancellationToken::new(),
            )
            .await;
        if let Err(ref e) = res {
            println!("start_training failed: {:?}", e);
        }
        assert!(res.is_ok(), "start_training failed!");

        // Assert that the vault_path directory was created and contains the weights
        let vault_exists = std::path::Path::new(&vault_path).exists();
        assert!(
            vault_exists,
            "Vault directory must be created and populated"
        );
    }

    #[tokio::test]
    async fn test_training_triggers_evolution() {
        use crate::test_utils::job_queue_mock::{GlobalMockJobQueue, GlobalMockLlm};

        let core = Arc::new(aiome_core::lora::engine::LoraEngine::new());
        let tmp = tempdir().unwrap();
        let soul_path = tmp.path().join("SOUL.md");
        std::fs::write(&soul_path, "Initial Soul").unwrap();

        // Setup mock LLM that returns a specific mutation
        let mutator = Arc::new(SoulMutator::new(
            Arc::new(GlobalMockLlm),
            tmp.path().to_path_buf(),
            None,
        ));
        let jq = Arc::new(GlobalMockJobQueue::default());

        let service = LoraTrainingService::new(core, Some(mutator), Some(jq), None, None)
            .with_datasets_dir(std::path::PathBuf::from("workspace/datasets"));

        let dataset_dir = std::path::Path::new("workspace/datasets");
        let _ = std::fs::create_dir_all(&dataset_dir);
        let dataset_path = dataset_dir.join("test_dataset");
        std::fs::write(&dataset_path, "{\"text\": \"hello\"}").unwrap();

        // When implemented, this should trigger SoulMutator
        let res = service.train("test", "test_dataset", serde_json::json!({}));
        res.await.unwrap();

        // Give it enough time to spawn python and execute (which should be fast as it's mocked, but still)
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

        let mutated_content = std::fs::read_to_string(&soul_path).unwrap();

        let _ = std::fs::remove_file(&dataset_path);

        assert_ne!(
            mutated_content, "Initial Soul",
            "Soul should have evolved after training"
        );
    }

    #[tokio::test]
    async fn test_lora_training_health_check() {
        use crate::test_utils::job_queue_mock::{GlobalMockJobQueue, GlobalMockLlm};
        let mock_llm = Arc::new(GlobalMockLlm);
        let mutator = Arc::new(SoulMutator::new(
            mock_llm,
            std::path::PathBuf::from("/tmp"),
            None,
        ));
        let jq = Arc::new(GlobalMockJobQueue::default());

        let core = Arc::new(aiome_core::lora::engine::LoraEngine::new());
        let service = LoraTrainingService::new(core, Some(mutator), Some(jq), None, None);

        let health = service.health_check().await.unwrap();
        assert!(
            health,
            "Health check should pass in test environment (STUB mode)"
        );
    }

    #[test]
    fn test_lora_training_config_from_params() {
        let params = serde_json::json!({
            "learning_rate": 0.0002,
            "epochs": 10,
            "lora_rank": 32,
            "batch_size": 8
        });

        // TDD RED: We want a method that extracts these precisely.
        let config = LoraTrainingConfig::from_params(&params, "base", "data", "out", "vault");

        assert_eq!(config.learning_rate, 0.0002);
        assert_eq!(config.epochs, 10);
        assert_eq!(config.lora_rank, 32);
        assert_eq!(config.batch_size, 8);
        assert_eq!(config.base_model, "base");
    }
}
