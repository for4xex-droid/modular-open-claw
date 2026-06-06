/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

//! MCP Tool: Sandbox Code Execution.
//! Allows agents to run Python in a restricted environment.

use commerce_protocol::error::NurtureError;
use commerce_protocol::mcp_commerce::{SandboxExecRequest, SandboxExecResponse};
use nurture_infra::sandbox::executor::PythonExecutor;
use std::sync::Arc;

pub async fn handle_sandbox_exec(
    executor: Arc<PythonExecutor>,
    req: SandboxExecRequest,
) -> Result<SandboxExecResponse, NurtureError> {
    let input_data = req.input_data.unwrap_or_else(|| serde_json::json!({}));
    let output_data = executor.execute(&req.code, input_data).await?;

    Ok(SandboxExecResponse { output_data })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nurture_infra::sandbox::executor::{PythonExecutor, ResourceLimits};
    use std::sync::Arc;

    #[tokio::test]
    #[ignore = "Requires Podman/Docker"]
    async fn test_sandbox_exec_success() -> Result<(), Box<dyn std::error::Error>> {
        let limits = ResourceLimits::default();
        let executor = Arc::new(PythonExecutor::new(limits));

        let req = SandboxExecRequest {
            code: "output_data = {'result': 'success', 'echo': input_data.get('msg')}".to_string(),
            input_data: Some(serde_json::json!({"msg": "hello from test"})),
        };

        let response = handle_sandbox_exec(executor, req).await?;
        assert_eq!(response.output_data["result"], "success");
        assert_eq!(response.output_data["echo"], "hello from test");
        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requires Podman/Docker"]
    async fn test_sandbox_exec_syntax_error() -> Result<(), Box<dyn std::error::Error>> {
        let limits = ResourceLimits::default();
        let executor = Arc::new(PythonExecutor::new(limits));

        let req = SandboxExecRequest {
            code: "this is not python".to_string(),
            input_data: None,
        };

        let result = handle_sandbox_exec(executor, req).await;
        assert!(result.is_err(), "Sandbox exec should fail on syntax error");
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("Python script failed") || err_str.contains("SyntaxError"));
        Ok(())
    }
}
