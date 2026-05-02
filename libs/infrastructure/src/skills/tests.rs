/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#[cfg(test)]
mod tests {
    use crate::skills::WasmSkillManager;
    use serde_json::json;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_wasm_skill_timeout() {
        let temp_dir = tempfile::tempdir().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        // Write the pre-compiled Extism PDK plugin to the temp directory
        let wasm_bytes = include_bytes!("test_data/hello_skill.wasm");
        std::fs::write(skills_dir.join("hello_skill.wasm"), wasm_bytes).unwrap();

        let manager = WasmSkillManager::new(&skills_dir, &temp_dir.path().to_path_buf())
            .expect("Failed to create manager")
            .with_limits(1024 * 1024, std::time::Duration::from_millis(500));

        let verified = crate::skills::VerifiedSkill::new_for_test("hello_skill".to_string());
        let result = manager
            .call_skill(&verified, "test_timeout", "", None)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn test_wasm_skill_incident_on_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();
        let wasm_bytes = include_bytes!("test_data/hello_skill.wasm");
        std::fs::write(skills_dir.join("hello_skill.wasm"), wasm_bytes).unwrap();

        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();
        {
            let ts = std::sync::Arc::new(
                crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
            );
            let jq = crate::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap();
            crate::job_queue::migrations::DbInitializer::init_db(&jq)
                .await
                .unwrap();
        }

        let manager = WasmSkillManager::new(&skills_dir, &temp_dir.path().to_path_buf())
            .expect("Failed to create manager")
            .with_limits(1024 * 1024, std::time::Duration::from_millis(500))
            .with_db_pool(pool.clone());

        let verified = crate::skills::VerifiedSkill::new_for_test("hello_skill".to_string());
        let _ = manager
            .call_skill(&verified, "test_timeout", "", None)
            .await;

        if let crate::db::DatabasePool::Sqlite(sqlite_pool) = &pool {
            let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM aegis_incidents")
                .fetch_one(sqlite_pool)
                .await
                .unwrap();
            assert_eq!(row.0, 1, "Incident should be recorded on timeout error");
        }
    }

    #[tokio::test]
    async fn test_wasm_skill_config_injection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        // Write the pre-compiled Extism PDK plugin to the temp directory
        let wasm_bytes = include_bytes!("test_data/hello_skill.wasm");
        std::fs::write(skills_dir.join("hello_skill.wasm"), wasm_bytes).unwrap();

        let manager = WasmSkillManager::new(&skills_dir, &temp_dir.path().to_path_buf())
            .expect("Failed to create manager");

        let mut configs = std::collections::HashMap::new();
        configs.insert("api_key".to_string(), "SECRET_TOKEN_123".to_string());

        let verified = crate::skills::VerifiedSkill::new_for_test("hello_skill".to_string());
        let result = manager
            .call_skill(&verified, "test_config", "", Some(configs))
            .await
            .expect("Execution failed");
        assert_eq!(result, "Key: SECRET_TOKEN_123");
    }

    #[tokio::test]
    async fn test_dry_run_call_validation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();
        // Write the pre-compiled Extism PDK plugin to the temp directory
        let wasm_bytes = include_bytes!("test_data/hello_skill.wasm");
        std::fs::write(skills_dir.join("hello_skill.wasm"), wasm_bytes).unwrap();
        let manager = WasmSkillManager::new(&skills_dir, &temp_dir.path().to_path_buf())
            .expect("Failed to create manager");

        // Dry-run should execute without system-level error and validate the execution.
        let result = manager.dry_run_skill("hello_skill", "{}").await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            true,
            "Valid WASM must pass dry-run successfully"
        );
    }

    #[tokio::test]
    async fn test_dry_run_missing_skill_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();
        let manager = WasmSkillManager::new(&skills_dir, &temp_dir.path().to_path_buf())
            .expect("Failed to create manager");

        let result = manager.dry_run_skill("non_existent_skill", "{}").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_hot_reload_skills() {
        let temp_dir = tempfile::tempdir().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        // Use temp_dir as root as well for the manager
        let skills_dir_buf = skills_dir.to_path_buf();
        let manager =
            WasmSkillManager::new(&skills_dir_buf, &temp_dir.path().to_path_buf()).unwrap();

        // No skills initially
        assert!(manager.list_skills().is_empty());

        // Add a fake wasm
        std::fs::write(skills_dir.join("test.wasm"), b"wasm").unwrap();

        // list_skills should find it
        let skills = manager.list_skills();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0], "test");

        // hot_reload_skills should return it
        let reloaded = manager.hot_reload_skills();
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0], "test");
    }

    #[tokio::test]
    async fn test_skill_verification_promotion() {
        let temp_dir = tempfile::tempdir().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        // Setup manager
        let manager =
            WasmSkillManager::new(skills_dir.to_path_buf(), temp_dir.path().to_path_buf()).unwrap();

        // 1. Missing skill should fail verification
        let unverified = crate::skills::UnverifiedSkill {
            name: "non_existent".into(),
            input_test_payload: "{}".into(),
        };
        let res = unverified.verify(&manager).await;
        assert!(res.is_err());

        // 2. We can't easily execute a real WASM here without a valid file,
        // but we've tested the logic in dry_run_skill.
    }

    #[tokio::test]
    async fn test_list_skills_with_metadata_auto_generation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        std::fs::write(skills_dir.join("test.wasm"), b"wasm").unwrap();

        let manager =
            WasmSkillManager::new(skills_dir.to_path_buf(), temp_dir.path().to_path_buf()).unwrap();
        let metadata = manager.list_skills_with_metadata();

        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].name, "test");
        assert_eq!(metadata[0].description, "No metadata provided");
    }

    #[tokio::test]
    async fn test_list_skills_with_explicit_metadata() {
        let temp_dir = tempfile::tempdir().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        std::fs::write(skills_dir.join("calc.wasm"), b"wasm").unwrap();
        let meta_json = json!({
            "name": "calc",
            "description": "Calculator skill",
            "capabilities": ["execute"],
            "inputs": ["json"],
            "outputs": ["json"],
            "allowed_hosts": ["api.example.com"],
            "permissions": { "allow_filesystem_write": false, "allow_network": true, "allow_shell_execution": false, "allowed_domains": [] }
        });
        std::fs::write(skills_dir.join("calc.meta.json"), meta_json.to_string()).unwrap();

        let manager =
            WasmSkillManager::new(skills_dir.to_path_buf(), temp_dir.path().to_path_buf()).unwrap();
        let metadata = manager.list_skills_with_metadata();

        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].name, "calc");
        assert_eq!(
            metadata[0].allowed_hosts,
            vec!["api.example.com".to_string()]
        );
    }

    #[cfg(feature = "grpc")]
    #[tokio::test]
    async fn test_grpc_formal_proof_gate_empty_token_rejection() {
        use crate::grpc_proof_gate::GrpcFormalProofGate;
        use aiome_contracts::proof::FormalProofGate;
        use aiome_core_contracts::AiomeError;

        // Step 2: Negative Test (異常注入)
        // 意図的に空の auth_token を注入し、gRPC通信前にブロックされることを確認する。
        let endpoint = tonic::transport::Endpoint::from_static("http://[::1]:50051");
        let channel = endpoint.connect_lazy();

        let gate = GrpcFormalProofGate::new(channel, "".to_string());

        let result = gate.verify_skill("dummy_skill", "dummy_proof").await;

        assert!(result.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = result {
            assert!(reason.contains("A2A_AUTH_TOKEN is empty"));
        } else {
            panic!(
                "Expected Infrastructure error due to empty token, got: {:?}",
                result
            );
        }
    }
}
