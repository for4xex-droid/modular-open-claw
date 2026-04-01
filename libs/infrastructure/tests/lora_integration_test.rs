/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![allow(unused_imports, unused_variables, dead_code, unused_mut)]

use aiome_core::lora::engine::LoraEngine;
use infrastructure::lora_autotuner::{LoraAutotuner, TrainingMetrics, TunedHyperparams};
use infrastructure::lora_training::{LoraTrainingConfig, LoraTrainingService};
use std::sync::Arc;
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

/// Integration test: LoraAutotuner → LoraTrainingService E2E pipeline.
///
/// Verifies that autotuner-suggested hyperparams flow correctly
/// into a LoraTrainingConfig that LoraTrainingService can execute.
#[tokio::test]
async fn test_autotuner_to_training_service_integration() {
    // --- Arrange ---
    let metrics = TrainingMetrics {
        loss_history: vec![2.0, 1.0, 0.1, 0.01, 0.001],
        previous_params: TunedHyperparams::default(),
    };
    let suggested = LoraAutotuner::suggest_hyperparams(&metrics);

    // Use tempdir to avoid race conditions with parallel test execution
    let tmp = tempdir().expect("Failed to create temp directory for test");
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("Failed to create workspace dir");

    let dataset_path = workspace.join("test_dataset_mock.jsonl");
    std::fs::write(&dataset_path, "{\"text\": \"hello\"}\n").expect("Failed to write mock dataset");

    // Act: Convert autotuner output → training config
    let config = LoraAutotuner::create_training_config(
        &suggested,
        "test_base_model".into(),
        dataset_path.to_string_lossy().to_string(),
        workspace.join("output").to_string_lossy().to_string(),
        workspace.join("vault").to_string_lossy().to_string(),
    );

    // Assert: Hyperparams are correctly mapped
    assert_eq!(config.base_model, "test_base_model");
    assert_eq!(config.learning_rate, suggested.learning_rate);
    assert_eq!(config.epochs, suggested.epochs);
    assert_eq!(config.lora_rank, suggested.lora_rank);
    assert_eq!(config.batch_size, suggested.batch_size);

    // --- Act: Service execution ---
    let core_engine = Arc::new(LoraEngine::new());
    let service = LoraTrainingService::new(
        core_engine,
        None, // soul_mutator
        None, // job_queue
        None, // event_tx
        None, // compute_semaphore
    );

    // Create mock MLX script that produces expected artifacts
    let scripts_dir = tmp.path().join("scripts");
    std::fs::create_dir_all(&scripts_dir).expect("Failed to create scripts dir");
    let script_content = r#"
import os, sys, argparse
parser = argparse.ArgumentParser()
parser.add_argument("--model")
parser.add_argument("--data")
parser.add_argument("--adapter-file")
args, _ = parser.parse_known_args()
if args.adapter_file:
    os.makedirs(os.path.dirname(args.adapter_file), exist_ok=True)
    with open(args.adapter_file, 'w') as f:
        f.write("mock weights")
print("mock MLX training complete")
"#;
    std::fs::write(scripts_dir.join("mlx_train.py"), script_content)
        .expect("Failed to write mock MLX script");

    let token = CancellationToken::new();
    let result = service.start_training("mock_job_id", config, token).await;

    // Assert: Training completes without error
    assert!(
        result.is_ok(),
        "LoraTrainingService::start_training failed: {:?}",
        result.err()
    );
}
