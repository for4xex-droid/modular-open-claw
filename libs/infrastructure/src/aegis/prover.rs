use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use std::sync::Arc;

use super::types::IncidentRecord;

/// ADR-040: Kani 検証リトライ上限。この回数を超えると IncidentStatus::WontFix に遷移。
pub const MAX_KANI_RETRIES: u32 = 3;

/// ADR-040: パッチコードの最大サイズ (1 MiB)。
/// LLM が生成した巨大なコードによるディスク攻撃を防止する。
const MAX_PATCH_CODE_BYTES: usize = 1024 * 1024;

pub struct AegisProver {
    llm: Arc<dyn LlmProvider>,
}

impl AegisProver {
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self { llm }
    }

    pub async fn generate_patch(&self, incident: &IncidentRecord) -> Result<String, AiomeError> {
        let workspace_dir = std::env::var("FORGE_WORKSPACE_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                crate::security::GLOBAL_SECURITY_CONFIG
                    .workspace_root
                    .join("forge_workspaces")
            });
        let src_path = workspace_dir
            .join(&incident.skill_name)
            .join("src")
            .join("lib.rs");

        let original_code = match tokio::fs::read_to_string(&src_path).await {
            Ok(code) => code,
            Err(e) => {
                return Err(AiomeError::Infrastructure {
                    reason: format!(
                        "Original source code not found for skill '{}' at {}: {}",
                        incident.skill_name,
                        src_path.display(),
                        e
                    ),
                });
            }
        };

        let prompt = format!(
            "Generate a Rust patch to fix the following panic in the Extism PDK skill '{}'.\n\
            \n\
            Original Source Code:\n```rust\n{}\n```\n\n\
            Stack Trace:\n{}\n\n\
            Payload:\n{}\n\n\
            Please return the COMPLETE, updated `src/lib.rs` file. It MUST be a valid Extism PDK plugin using `#[plugin_fn]`.",
            incident.skill_name, original_code, incident.stack_trace, incident.input_payload
        );
        let system_prompt = "You are an expert Rust systems programmer. Provide only the completely valid Rust code to fix the issue. Do NOT include markdown blocks like ```rust or any explanations. Return only the raw Rust code.";

        match self.llm.complete(&prompt, Some(system_prompt)).await {
            Ok(response) => {
                metrics::counter!("aegis_patch_generation_success").increment(1);
                Ok(response.content)
            }
            Err(e) => {
                metrics::counter!("aegis_patch_generation_failure").increment(1);
                Err(e)
            }
        }
    }

    pub async fn verify_with_kani(&self, patch_code: &str) -> Result<bool, AiomeError> {
        // 0. Patch code size validation ALWAYS runs (defense in depth, even in stub mode)
        if patch_code.len() > MAX_PATCH_CODE_BYTES {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Patch code exceeds maximum allowed size ({} bytes > {} bytes)",
                    patch_code.len(),
                    MAX_PATCH_CODE_BYTES
                ),
            });
        }

        // Phase A: Stubbed Kani verification
        if std::env::var("KANI_STUB_MODE").unwrap_or_default() == "true" {
            metrics::counter!("aegis_patch_verification_success").increment(1);
            return Ok(true);
        }

        use crate::security::{SafeCommandBuilder, SandboxProfile, GLOBAL_SECURITY_CONFIG};
        use aiome_core_contracts::security::PermissionManifest;
        use shared::sandbox::PathSandbox;

        // 1. Create temporary directory in workspace root
        let tmp_dir = tempfile::Builder::new()
            .prefix("kani-")
            .tempdir_in(&GLOBAL_SECURITY_CONFIG.workspace_root)
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to create kani tempdir: {}", e),
            })?;

        // 2. PathSandbox validation to prevent traversal
        let sandbox = PathSandbox::new(tmp_dir.path()).map_err(|e| AiomeError::Infrastructure {
            reason: format!("PathSandbox init failed: {}", e),
        })?;

        let patch_path = tmp_dir.path().join("src");
        tokio::fs::create_dir_all(&patch_path)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to create src dir: {}", e),
            })?;
        let lib_rs = patch_path.join("lib.rs");

        sandbox
            .validate_path(&lib_rs)
            .map_err(|_| AiomeError::SecurityViolation {
                reason: "Path traversal detected in Kani tempdir".to_string(),
            })?;

        // 3. Write patch code
        tokio::fs::write(&lib_rs, patch_code)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to write patch code: {}", e),
            })?;

        // 4. Validate Kani arguments (ADR-040 §3: ホワイトリスト制御)
        let kani_args = vec!["--harness".to_string(), "verify_patch".to_string()];
        validate_kani_args(&kani_args)?;

        // 5. Execute Podman rootless container via SafeCommandBuilder
        //    Least-privilege PermissionManifest: shell_execution のみ許可
        let volume_arg = format!("{}:/work:Z", tmp_dir.path().display());
        let manifest = PermissionManifest {
            allow_shell_execution: true,
            allow_filesystem_write: false,
            allow_network: false,
            ..Default::default()
        };
        let mut cmd = SafeCommandBuilder::new("podman")
            .arg("run")
            .arg("--rm")
            .arg("--network=none")
            .arg("--memory=2g")
            .arg("--cpus=1")
            .arg("--read-only")
            .arg("-v")
            .arg(&volume_arg)
            .arg("aiome/kani-verifier:latest")
            .arg("cargo")
            .arg("kani")
            .arg("--harness")
            .arg("verify_patch")
            .profile(SandboxProfile::Strict)
            .build(manifest)?;

        // 6. Apply 4-Layer Defense (L3: Timeout) using tokio::time::timeout
        let timeout = kani_timeout();

        // Spawn the process and use wait_with_output to prevent pipe deadlock
        let child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| AiomeError::SubprocessFailed {
                reason: format!("Podman spawn failed: {}", e),
            })?;

        match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => {
                let passed = output.status.success();
                if passed {
                    metrics::counter!("aegis_patch_verification_success").increment(1);
                } else {
                    metrics::counter!("aegis_patch_verification_failure").increment(1);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    tracing::warn!("Kani verification stderr: {}", stderr);
                }
                Ok(passed)
            }
            Ok(Err(e)) => Err(AiomeError::SubprocessFailed {
                reason: format!("Podman execution failed: {}", e),
            }),
            Err(_) => {
                metrics::counter!("aegis_patch_verification_timeout").increment(1);
                Err(AiomeError::RemoteServiceTimeout {
                    timeout_secs: timeout.as_secs(),
                })
            }
        }
    }
}

/// Kani 検証に許可される引数のバリデーション (ADR-040 §3)
pub fn validate_kani_args(args: &[String]) -> Result<(), AiomeError> {
    let allowed_flags = [
        "--harness",
        "--unwind",
        "--restrict-vtable-size",
        "--output-format",
    ];
    let mut expect_harness_value = false;
    for arg in args {
        if expect_harness_value {
            // ADR-040 §3: ハーネス名は英数字 + アンダースコアのみ許可
            if !arg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(AiomeError::SecurityViolation {
                    reason: format!("Kani harness name '{}' contains disallowed characters (only [a-zA-Z0-9_] allowed)", arg),
                });
            }
            expect_harness_value = false;
            continue;
        }
        if arg.starts_with("--") {
            let flag = arg.split('=').next().unwrap_or(arg);
            if !allowed_flags.contains(&flag) {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Kani argument '{}' is not whitelisted", flag),
                });
            }
            if flag == "--harness" && !arg.contains('=') {
                expect_harness_value = true;
            }
            // Validate inline --harness=<value>
            if flag == "--harness" {
                if let Some(value) = arg.strip_prefix("--harness=") {
                    if !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                        return Err(AiomeError::SecurityViolation {
                            reason: format!(
                                "Kani harness name '{}' contains disallowed characters",
                                value
                            ),
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

/// Kani 検証のタイムアウト秒数 (ADR-040)
const DEFAULT_KANI_TIMEOUT_SECS: u64 = 300;

pub fn kani_timeout() -> std::time::Duration {
    let secs = std::env::var("KANI_PROOF_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_KANI_TIMEOUT_SECS);
    std::time::Duration::from_secs(secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aegis::types::IncidentStatus;
    use aiome_core::llm_provider::{LlmRequest, LlmResponse, StopReason};
    use async_trait::async_trait;
    use chrono::Utc;

    #[derive(Debug)]
    struct MockLlm;
    #[async_trait]
    impl LlmProvider for MockLlm {
        fn name(&self) -> &str {
            "mock"
        }
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            Ok(LlmResponse {
                content: "fn patched() {}".into(),
                stop_reason: StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            })
        }
        async fn complete_with_cache(
            &self,
            _req: aiome_core_contracts::llm::LlmRequest,
        ) -> Result<LlmResponse, AiomeError> {
            self.complete("", None).await
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    fn dummy_incident() -> IncidentRecord {
        IncidentRecord {
            id: "inc_123".to_string(),
            skill_name: "test_skill".to_string(),
            wasm_hash: "hash_abc".to_string(),
            input_payload: "{}".to_string(),
            stack_trace: "panic at line 1".to_string(),
            status: crate::aegis::types::IncidentStatus::Open,
            retry_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_generate_patch() {
        let tmp_dir = tempfile::Builder::new()
            .prefix("aegis-test-")
            .tempdir()
            .unwrap();
        
        let skill_dir = tmp_dir.path().join("test_skill").join("src");
        tokio::fs::create_dir_all(&skill_dir).await.unwrap();
        tokio::fs::write(skill_dir.join("lib.rs"), "fn original() {}").await.unwrap();
        
        std::env::set_var("FORGE_WORKSPACE_DIR", tmp_dir.path());

        let prover = AegisProver::new(Arc::new(MockLlm));
        let incident = dummy_incident();

        let patch = prover.generate_patch(&incident).await.expect("Failed to generate patch");
        assert_eq!(patch, "fn patched() {}");
        
        std::env::remove_var("FORGE_WORKSPACE_DIR");
    }

    #[tokio::test]
    async fn test_verify_with_kani() {
        std::env::set_var("KANI_STUB_MODE", "true");
        let prover = AegisProver::new(Arc::new(MockLlm));

        let result = prover.verify_with_kani("fn patched() {}").await.unwrap();
        assert!(result);
    }

    #[test]
    fn test_validate_kani_args_allowed() {
        let args = vec![
            "--harness".to_string(),
            "verify_patch".to_string(),
            "--unwind".to_string(),
            "10".to_string(),
            "--restrict-vtable-size".to_string(),
            "--output-format=terse".to_string(),
        ];
        assert!(super::validate_kani_args(&args).is_ok());
    }

    #[test]
    fn test_validate_kani_args_rejected() {
        let args = vec![
            "--harness".to_string(),
            "test".to_string(),
            "--tests".to_string(),
        ];
        let err = super::validate_kani_args(&args).unwrap_err();
        assert!(err.to_string().contains("is not whitelisted"));

        let args2 = vec!["--features".to_string(), "malicious".to_string()];
        assert!(super::validate_kani_args(&args2).is_err());
    }

    #[test]
    fn test_kani_timeout_parsing() {
        std::env::set_var("KANI_PROOF_TIMEOUT_SECS", "120");
        assert_eq!(super::kani_timeout().as_secs(), 120);
        std::env::remove_var("KANI_PROOF_TIMEOUT_SECS");
        assert_eq!(super::kani_timeout().as_secs(), 300); // Default
    }

    #[tokio::test]
    async fn test_verify_rejects_oversized_patch() {
        // Size check runs before KANI_STUB_MODE check, so env var state doesn't matter
        let prover = AegisProver::new(Arc::new(MockLlm));
        // Generate a patch that exceeds MAX_PATCH_CODE_BYTES (1 MiB + 1)
        let oversized = "x".repeat(super::MAX_PATCH_CODE_BYTES + 1);
        let result = prover.verify_with_kani(&oversized).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("exceeds maximum allowed size"),
            "Expected size rejection, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_max_kani_retries_constant() {
        assert_eq!(super::MAX_KANI_RETRIES, 3);
    }

    #[test]
    fn test_validate_kani_args_harness_name_injection() {
        // ADR-040 §3: harness name must be [a-zA-Z0-9_] only
        // Valid harness name
        let valid = vec!["--harness".to_string(), "verify_patch".to_string()];
        assert!(super::validate_kani_args(&valid).is_ok());

        // Valid inline format
        let valid_inline = vec!["--harness=verify_patch".to_string()];
        assert!(super::validate_kani_args(&valid_inline).is_ok());

        // Shell injection attempt via harness name (positional)
        let inject = vec![
            "--harness".to_string(),
            "verify_patch; rm -rf /".to_string(),
        ];
        let err = super::validate_kani_args(&inject).unwrap_err();
        assert!(
            err.to_string().contains("disallowed characters"),
            "Expected injection rejection, got: {}",
            err
        );

        // Shell injection attempt via harness name (inline)
        let inject_inline = vec!["--harness=$(whoami)".to_string()];
        let err2 = super::validate_kani_args(&inject_inline).unwrap_err();
        assert!(
            err2.to_string().contains("disallowed characters"),
            "Expected injection rejection, got: {}",
            err2
        );

        // Path traversal via harness name
        let traversal = vec!["--harness".to_string(), "../../../etc/passwd".to_string()];
        assert!(super::validate_kani_args(&traversal).is_err());
    }
}
