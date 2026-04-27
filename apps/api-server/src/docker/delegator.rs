/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::biome::DelegationResult;
use std::fs;
use std::time::Instant;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Phase 17-C: Docker Agent "Shadow Worker" Delegation
/// This function executes untrusted or dependency-heavy tasks inside a disposable
/// docker-agent container to isolate the host Aiome system.
pub async fn delegate_docker_worker(
    agent_yaml_content: &str,
    task_prompt: &str,
) -> DelegationResult {
    let session_id = Uuid::new_v4().to_string();
    let temp_dir = std::env::temp_dir().join(format!("aiome-delegation-{}", session_id));
    let start = Instant::now();

    if let Err(e) = fs::create_dir_all(&temp_dir) {
        error!("❌ [DockerDelegator] Failed to create sandbox: {}", e);
        return DelegationResult {
            stdout: "".to_string(),
            stderr: format!("Failed to create delegation sandbox: {}", e),
            exit_code: -1,
            duration_ms: 0,
        };
    }

    let yaml_path = temp_dir.join("agent.yaml");
    if let Err(e) = fs::write(&yaml_path, agent_yaml_content) {
        let _ = fs::remove_dir_all(&temp_dir);
        return DelegationResult {
            stdout: "".to_string(),
            stderr: format!("Failed to write agent config: {}", e),
            exit_code: -1,
            duration_ms: 0,
        };
    }

    // N10: Secure file permissions (600)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&yaml_path, fs::Permissions::from_mode(0o600));
    }

    info!(
        "🐳 [DockerDelegator] Spinning up Shadow Worker for session: {}",
        session_id
    );

    // Call docker-agent (expected to be in PATH or symlinked)
    // Using --exec and --json for stable parsing (scanned from docker-agent repo)
    let yaml_path_str = yaml_path.to_string_lossy().to_string();
    let task_prompt_b64 = {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(task_prompt)
    };
    let runtime = shared::container_runtime::detect_runtime().to_string();

    let cmd_res = infrastructure::security::SafeCommandBuilder::new(&runtime)
        .arg("agent")
        .arg("run")
        .arg("--exec")
        .arg("--json")
        .arg(&yaml_path_str)
        .arg("--prompt-b64")
        .arg(&task_prompt_b64)
        .env_passthrough("DOCKER_HOST")
        .env_passthrough("PATH")
        .env_passthrough("USER")
        .build_internal();

    let output = match cmd_res {
        Ok(mut cmd) => cmd
            .output()
            .await
            .map_err(|e| format!("Command execution failed: {}", e)),
        Err(e) => Err(format!("Command build failed: {}", e)),
    };

    // Clean up temp dir
    let _ = fs::remove_dir_all(&temp_dir);
    let duration_ms = start.elapsed().as_millis() as u64;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let exit_code = out.status.code().unwrap_or(-2);

            if out.status.success() {
                info!(
                    "✅ [DockerDelegator] Worker session {} completed in {}ms",
                    session_id, duration_ms
                );
            } else {
                warn!(
                    "🚨 [DockerDelegator] Worker session {} failed with code {}",
                    session_id, exit_code
                );
            }

            DelegationResult {
                stdout,
                stderr,
                exit_code,
                duration_ms,
            }
        }
        Err(e) => {
            error!("❌ [DockerDelegator] Execution error: {}", e);
            DelegationResult {
                stdout: "".to_string(),
                stderr: e,
                exit_code: -3,
                duration_ms,
            }
        }
    }
}
