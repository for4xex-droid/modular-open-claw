/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use commerce_protocol::error::NurtureError;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;

/// Resource constraints applied to sandboxed Python execution.
#[derive(Debug, Clone, Copy)]
pub struct ResourceLimits {
    pub memory_bytes: u64,
    pub cpu_seconds: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_bytes: 256 * 1024 * 1024, // 256MB
            cpu_seconds: 2,                  // 2s
        }
    }
}

/// Executes Python code in an isolated Podman/Docker container.
///
/// The executor pipes a Python script via stdin to `python3 -` running inside
/// a network-isolated, memory-limited container. All host filesystem access is
/// denied by design (no volume mounts, rootless Podman).
pub struct PythonExecutor {
    limits: ResourceLimits,
    runtime: String,
}

impl PythonExecutor {
    pub fn new(limits: ResourceLimits) -> Self {
        let runtime = nurture_bridge::container_runtime::detect_runtime().to_string();

        // Warm-up: pre-pull image in background
        let rt = runtime.clone();
        tokio::spawn(async move {
            if let Ok(mut cmd) = nurture_bridge::security::SafeCommandBuilder::new(&rt)
                .arg("pull")
                .arg("python:3.11-alpine")
                .build_internal()
            {
                let _ = cmd.output().await;
            }
        });

        Self { limits, runtime }
    }

    /// Execute Python code out-of-process via Podman/Docker.
    ///
    /// The user-supplied `code` must define an `output_data` variable which
    /// will be serialised to JSON and returned. `input` is injected as `input_data`.
    pub async fn execute(
        &self,
        code: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, NurtureError> {
        let mem_bytes = self.limits.memory_bytes.to_string();

        let mut cmd = nurture_bridge::security::SafeCommandBuilder::new(&self.runtime)
            .arg("run")
            .arg("--rm")
            .arg("-i")
            .arg("--network=none")
            .arg(format!("--memory={}", mem_bytes))
            .arg("--cpus=1") // Limit to 1 CPU core for sandbox isolation
            .arg("python:3.11-alpine")
            .arg("python3")
            .arg("-")
            .build_internal()
            .map_err(|e| NurtureError::Infrastructure(format!("Command build failed: {:?}", e)))?;

        cmd.kill_on_drop(true);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            NurtureError::Infrastructure(format!(
                "Container runtime '{}' not found or failed to start: {}. \
                 Ensure podman or docker is installed.",
                self.runtime, e
            ))
        })?;

        // Build the wrapper script with embedded input data
        let input_json = input.to_string().replace("'''", "\\'\\'\\'");
        let wrapper_script = format!(
            "import json\ninput_data = json.loads(r'''{}''')\n{}\nprint(json.dumps(output_data))",
            input_json, code
        );

        // Write script to stdin and explicitly close it so Python starts execution
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(wrapper_script.as_bytes())
                .await
                .map_err(|e| {
                    NurtureError::Infrastructure(format!("Failed to write script to stdin: {}", e))
                })?;
            drop(stdin); // Explicitly close stdin to signal EOF to Python
        }

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.limits.cpu_seconds + 2),
            child.wait_with_output(),
        )
        .await
        .map_err(|_| NurtureError::Infrastructure("Python execution timed out".into()))?
        .map_err(|e| NurtureError::Infrastructure(format!("Process error: {}", e)))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(NurtureError::Infrastructure(format!(
                "Python script failed: {}",
                err_msg
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(stdout.trim())
            .map_err(|e| NurtureError::Infrastructure(format!("Output parse error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requires Podman/Docker"]
    async fn test_python_executor_success() {
        let limits = ResourceLimits::default();
        let executor = PythonExecutor::new(limits);
        let code = "output_data = {'result': 'ok', 'echo': input_data.get('msg')}";
        let input = serde_json::json!({"msg": "hello"});

        let output = executor.execute(code, input).await.unwrap();
        assert_eq!(output["result"], "ok");
        assert_eq!(output["echo"], "hello");
    }

    #[tokio::test]
    #[ignore = "Requires Podman/Docker"]
    async fn test_python_executor_timeout() {
        let limits = ResourceLimits {
            memory_bytes: 256 * 1024 * 1024,
            cpu_seconds: 1, // Short timeout for test
        };
        let executor = PythonExecutor::new(limits);
        let code = "while True: pass";
        let input = serde_json::json!({});

        let result = executor.execute(code, input).await;
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(
            err_str.contains("timed out") || err_str.contains("Process error"),
            "Got: {}",
            err_str
        );
    }
}
