/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::skills::WasmSkillManager;
use crate::task_orchestrator::workflow_runtime::{
    self, evaluate_condition_expression, evaluate_transform, execute_http_request,
    render_workflow_template, SKIP_MARKER,
};
use crate::task_orchestrator::{TaskConductor, TaskEvent};
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use aiome_core_contracts::security::PermissionManifest;
use aiome_core_contracts::traits::{Job, JobQueue, McpToolInvoker};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

/// ワークフロー Conductor が利用する外部依存
pub struct WorkflowConductorDeps {
    pub llm: Arc<dyn LlmProvider>,
    pub job_queue: Arc<dyn JobQueue>,
    pub wasm_manager: Option<Arc<WasmSkillManager>>,
    pub mcp_invoker: Option<Arc<dyn McpToolInvoker>>,
    pub http_client: reqwest::Client,
}

pub struct WorkflowConductor {
    deps: Option<Arc<WorkflowConductorDeps>>,
}

impl WorkflowConductor {
    /// テスト用: 依存なしスタブ Conductor
    pub fn new() -> Self {
        Self { deps: None }
    }

    /// 本番用: DI された依存で Conductor を構築
    pub fn with_deps(deps: WorkflowConductorDeps) -> Self {
        Self {
            deps: Some(Arc::new(deps)),
        }
    }

    fn deps(&self) -> Option<&WorkflowConductorDeps> {
        self.deps.as_deref()
    }

    fn parse_topic(job: &Job) -> Result<Value, AiomeError> {
        serde_json::from_str(&job.topic).map_err(|e| AiomeError::Validation {
            reason: format!("Invalid job topic JSON for {}: {}", job.id, e),
        })
    }
}

#[async_trait]
impl TaskConductor for WorkflowConductor {
    fn capable_categories(&self) -> Vec<String> {
        vec![
            "wf_llm".to_string(),
            "wf_http".to_string(),
            "wf_mcp".to_string(),
            "wf_loop".to_string(),
            "wf_parallel".to_string(),
            "wf_transform".to_string(),
            "wf_approval".to_string(),
            "wf_timer".to_string(),
            "wf_wasm".to_string(),
            "wf_condition".to_string(),
            "wf_generic".to_string(),
        ]
    }

    fn conductor_name(&self) -> &str {
        "WorkflowConductor"
    }

    async fn conduct(
        &self,
        job: Job,
        progress_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<(String, Option<String>), AiomeError> {
        info!(
            "🧬 [WorkflowConductor] Executing job: {} (Category: {})",
            job.id, job.category
        );

        let _ = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: format!("Executing workflow node: {}", job.id),
                percent: Some(10),
            })
            .await;

        let res = if job.category == "wf_timer" {
            Self::execute_timer(&job, &progress_tx, self.conductor_name()).await?
        } else if let Some(deps) = self.deps() {
            let outcome = workflow_runtime::await_parents(&job, deps.job_queue.as_ref()).await?;
            if outcome.skipped {
                (SKIP_MARKER.to_string(), None)
            } else {
                self.execute_with_deps(deps, &job, &progress_tx, &outcome.input)
                    .await?
            }
        } else {
            self.execute_stub(&job, &progress_tx).await?
        };

        let _ = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: "Node execution complete.".to_string(),
                percent: Some(100),
            })
            .await;

        Ok(res)
    }
}

impl WorkflowConductor {
    async fn execute_timer(
        job: &Job,
        progress_tx: &mpsc::Sender<TaskEvent>,
        conductor_name: &str,
    ) -> Result<(String, Option<String>), AiomeError> {
        let delay_sec = match serde_json::from_str::<Value>(&job.topic) {
            Ok(v) => v.get("delay_seconds").and_then(|d| d.as_u64()).unwrap_or_else(|| {
                tracing::warn!(
                    "🧬 [WorkflowConductor] delay_seconds not found in timer job {}, defaulting to 1s",
                    job.id
                );
                1
            }),
            Err(e) => {
                tracing::warn!(
                    "🧬 [WorkflowConductor] Invalid JSON topic for timer job {}: {}, defaulting to 1s",
                    job.id, e
                );
                1
            }
        };
        let _ = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: conductor_name.to_string(),
                message: format!("Waiting for {} seconds...", delay_sec),
                percent: Some(50),
            })
            .await;
        tokio::time::sleep(tokio::time::Duration::from_secs(delay_sec)).await;
        Ok(("Timer delay complete".to_string(), None))
    }

    async fn execute_stub(
        &self,
        job: &Job,
        progress_tx: &mpsc::Sender<TaskEvent>,
    ) -> Result<(String, Option<String>), AiomeError> {
        if job.category == "wf_wasm" {
            let payload = Self::parse_topic(job)?;
            let code = payload.get("code").and_then(|c| c.as_str()).unwrap_or("");
            if code.trim().is_empty() {
                return Err(AiomeError::Validation {
                    reason: "WASM code must not be empty".to_string(),
                });
            }
        }
        let _ = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: "Generic workflow node executing (no deps)...".to_string(),
                percent: Some(50),
            })
            .await;
        Ok(("Workflow node executed".to_string(), None))
    }

    async fn execute_with_deps(
        &self,
        deps: &WorkflowConductorDeps,
        job: &Job,
        progress_tx: &mpsc::Sender<TaskEvent>,
        input: &str,
    ) -> Result<(String, Option<String>), AiomeError> {
        let topic = Self::parse_topic(job)?;
        let _ = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: format!("Running {} node...", job.category),
                percent: Some(50),
            })
            .await;

        match job.category.as_str() {
            "wf_llm" => {
                let prompt_tpl = topic
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AiomeError::Validation {
                        reason: "LlmPrompt node requires config.prompt".to_string(),
                    })?;
                if prompt_tpl.trim().is_empty() {
                    return Err(AiomeError::Validation {
                        reason: "LlmPrompt prompt must not be empty".to_string(),
                    });
                }
                let prompt = render_workflow_template(prompt_tpl, input)?;
                let response = deps.llm.complete(&prompt, None).await?;
                Ok((response.content, None))
            }
            "wf_http" => {
                let method = topic
                    .get("method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("GET");
                let url = topic
                    .get("url_template")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AiomeError::Validation {
                        reason: "HttpRequest requires url_template".to_string(),
                    })?;
                let body = execute_http_request(&deps.http_client, method, url, input).await?;
                Ok((body, None))
            }
            "wf_mcp" => {
                let server = topic
                    .get("server_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AiomeError::Validation {
                        reason: "McpToolCall requires server_name".to_string(),
                    })?;
                let tool = topic
                    .get("tool_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AiomeError::Validation {
                        reason: "McpToolCall requires tool_name".to_string(),
                    })?;
                let args = topic
                    .get("config")
                    .cloned()
                    .unwrap_or(Value::Object(serde_json::Map::new()));
                let invoker =
                    deps.mcp_invoker
                        .as_ref()
                        .ok_or_else(|| AiomeError::Infrastructure {
                            reason: "MCP invoker not configured".to_string(),
                        })?;
                let out = invoker.invoke_tool(server, tool, args).await?;
                Ok((out, None))
            }
            "wf_transform" => {
                let expr = topic
                    .get("expression")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let out = evaluate_transform(expr, input)?;
                Ok((out, None))
            }
            "wf_condition" => {
                let expr = topic
                    .get("expression")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let mode = topic
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Expression");
                let result = if mode == "LlmJudge" {
                    let prompt = format!(
                        "Evaluate the following condition and respond with ONLY 'true' or 'false'.\nCondition: {}\nInput: {}",
                        expr, input
                    );
                    let response = deps.llm.complete(&prompt, None).await?;
                    response.content.trim().eq_ignore_ascii_case("true")
                } else {
                    evaluate_condition_expression(expr, input)
                };
                Ok((result.to_string(), None))
            }
            "wf_approval" => {
                let approved = job
                    .execution_log
                    .as_ref()
                    .map(|l| l.contains("IMMUNE_BYPASS_APPROVED"))
                    .unwrap_or(false);
                if approved {
                    let msg = topic
                        .get("prompt_message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Approved");
                    Ok((msg.to_string(), None))
                } else {
                    let msg = topic
                        .get("prompt_message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Human approval required");
                    Err(AiomeError::AwaitingApproval {
                        reason: msg.to_string(),
                    })
                }
            }
            "wf_wasm" => {
                let code = topic.get("code").and_then(|c| c.as_str()).unwrap_or("");
                let language = topic
                    .get("language")
                    .and_then(|l| l.as_str())
                    .unwrap_or("javascript");
                if code.trim().is_empty() {
                    return Err(AiomeError::Validation {
                        reason: "WASM code must not be empty".to_string(),
                    });
                }
                let lang = language.to_lowercase();
                if lang == "rust" {
                    return Err(AiomeError::Validation {
                        reason: "Rust runtime compile is not supported in workflow nodes"
                            .to_string(),
                    });
                }
                if lang != "javascript" && lang != "typescript" {
                    return Err(AiomeError::Validation {
                        reason: format!("Unsupported WASM language: {}", language),
                    });
                }
                let mgr = deps
                    .wasm_manager
                    .as_ref()
                    .ok_or_else(|| AiomeError::Infrastructure {
                        reason: "WasmSkillManager not configured".to_string(),
                    })?;
                let manifest = PermissionManifest {
                    allow_network: false,
                    allow_filesystem_write: false,
                    allow_shell_execution: false,
                    allowed_domains: vec![],
                };
                let out = mgr.run_code_mode_js(code, &manifest).await.map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: format!("WASM execution failed: {}", e),
                    }
                })?;
                Ok((out, None))
            }
            "wf_loop" | "wf_parallel" | "wf_generic" => {
                Ok((format!("{} pass-through complete", job.category), None))
            }
            _ => Ok(("Workflow node executed".to_string(), None)),
        }
    }
}

impl Default for WorkflowConductor {
    fn default() -> Self {
        Self::new()
    }
}
