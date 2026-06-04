/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::tool_call_router::ToolCallRouter;
use crate::AppState;
use tracing::error;

pub(crate) async fn process_generated_tool_calls(
    reply: &str,
    state: &AppState,
    total_steps: &mut i32,
    job_id: Option<&str>,
) -> Vec<String> {
    let mut skill_results = Vec::new();
    let router = crate::tool_call_router::DefaultToolCallRouter;

    // 1. Unified Security Evaluation (Immune System + Guardrails)
    if let Err(block_msg) = router.evaluate_security(reply, state).await {
        return vec![block_msg];
    }

    let calls = parse_tool_calls(reply);

    for (skill_name, skill_input) in calls {
        // CallSkill (MCP tools) are routed via stream.rs's [CallSkill: ...] handler.
        // Skip here to avoid double-execution, but note: security evaluation above
        // already covers the full reply including CallSkill patterns.
        if skill_name == "CallSkill" {
            continue;
        }
        *total_steps += 1;

        // 2. Unified Tool Execution Stream
        let mut rx = router.execute_skill(&skill_name, &skill_input, state).await;

        let mut step_errors = Vec::new();

        while let Some(evt) = rx.recv().await {
            use crate::tool_call_router::ToolExecutionEvent;
            match evt {
                ToolExecutionEvent::Result(res) => skill_results.push(res),
                ToolExecutionEvent::Error(err) => {
                    step_errors.push(err.clone());
                    skill_results.push(err);
                }
                ToolExecutionEvent::TokenSaved(chars) => {
                    tracing::debug!("Token savings recorded: {} chars", chars);
                }
                _ => {} // Ignore Start and Heartbeat in synchronous execution
            }
        }

        // --- C-1: Direct Trajectory Persistence ---
        if let Some(jid) = job_id {
            use aiome_core::trajectory::TrajectoryStep;

            let output_value = if !step_errors.is_empty() {
                serde_json::json!({ "error": step_errors.join("; ") })
            } else {
                let last_res = skill_results.last().map(|s| s.as_str()).unwrap_or("");
                serde_json::Value::String(last_res.to_string())
            };

            let input_val: serde_json::Value = serde_json::from_str(&skill_input)
                .unwrap_or(serde_json::Value::String(skill_input.clone()));

            let mut step = TrajectoryStep {
                step_id: *total_steps as u32,
                action: format!("call_tool: {}", skill_name),
                tool_name: Some(skill_name.clone()),
                input: input_val,
                output: output_value,
                timestamp: chrono::Utc::now().to_rfc3339(),
                is_critical_failure: !step_errors.is_empty(),
                ..Default::default()
            };

            // Security Hardening: Scrub secrets before persisting to SQLite audit trail
            step.scrub();

            if let Err(e) = state
                .job_queue
                .get_inner()
                .trajectory_store
                .record_step(jid, step)
                .await
            {
                error!(
                    "Failed to persist trajectory step for {}: {:?}",
                    skill_name, e
                );
            }
        }
    }

    skill_results
}

pub(crate) fn parse_tool_calls(text: &str) -> Vec<(String, String)> {
    let mut calls = Vec::new();
    let mut start_idx = 0;

    while let Some(brace_start) = text[start_idx..].find('{') {
        let abs_brace = start_idx + brace_start;
        let before_brace = &text[..abs_brace].trim();
        if before_brace.is_empty() {
            start_idx = abs_brace + 1;
            continue;
        }

        let skill_name = before_brace
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .rfind(|s| !s.is_empty())
            .unwrap_or("")
            .to_string();

        if !skill_name.is_empty() {
            let mut brace_depth = 0;
            let mut json_end = None;
            let json_search_area = &text[abs_brace..];
            for (i, c) in json_search_area.char_indices() {
                if c == '{' {
                    brace_depth += 1;
                } else if c == '}' {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        json_end = Some(abs_brace + i + 1);
                        break;
                    }
                }
            }

            if let Some(end_idx) = json_end {
                let json_str = text[abs_brace..end_idx].trim().to_string();
                if serde_json::from_str::<serde_json::Value>(&json_str).is_ok() {
                    calls.push((skill_name, json_str));
                }
                start_idx = end_idx;
                continue;
            }
        }
        start_idx = abs_brace + 1;
    }
    calls
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::Component;
    use aiome_core::error::AiomeError;
    use aiome_core_contracts::TaskRegistry;
    use async_trait::async_trait;
    use infrastructure::immune_system::AdaptiveImmuneSystem;
    use infrastructure::job_queue::UniversalJobQueue;
    use infrastructure::registry::RegistryManager;

    use infrastructure::skills::WasmSkillManager;
    use std::sync::Arc;

    async fn setup_test_state() -> (crate::AppState, tempfile::TempDir) {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test_agent.db");
        let pool_url = format!("sqlite://{}", db_path.to_str().unwrap());

        let pool = infrastructure::db::DatabasePool::new_sqlite(&pool_url)
            .await
            .unwrap();
        let ts = std::sync::Arc::new(
            infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = Arc::new(
            UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(),
        );
        let registry = Arc::new(RegistryManager::new(pool.clone()));

        let skills_dir = tmp_dir.path().join("skills");
        let sandbox_dir = tmp_dir.path().join("sandbox");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::create_dir_all(&sandbox_dir).unwrap();

        let wsm = Arc::new(
            WasmSkillManager::new(skills_dir.to_str().unwrap(), sandbox_dir.to_str().unwrap())
                .unwrap(),
        );

        let mut config = shared::config::AiomeConfig::default();
        config.resolver = shared::app_data::AppDataResolver::new().unwrap();

        #[derive(Debug)]
        struct MockLlm;
        #[async_trait]
        impl aiome_core::llm_provider::LlmProvider for MockLlm {
            async fn complete(
                &self,
                _prompt: &str,
                _sys: Option<&str>,
            ) -> Result<aiome_core_contracts::LlmResponse, aiome_core::error::AiomeError>
            {
                Ok(aiome_core_contracts::LlmResponse {
                    content: "Mocked Execution Result".into(),
                    metadata: Some(std::collections::HashMap::new()),

                    stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
                    ..Default::default()
                })
            }
            async fn test_connection(&self) -> Result<(), aiome_core::error::AiomeError> {
                Ok(())
            }
            fn name(&self) -> &str {
                "MockLlm"
            }
        }

        let state = crate::AppState {
            registry: Component::new(registry),
            wasm_skill_manager: Component::new(wsm),
            job_queue: Component::new(jq),
            config: Component::new(Arc::new(config)),
            provider: Component::new(Arc::new(MockLlm)),
            hook_chain: Component::new(Arc::new(infrastructure::skills::hooks::HookChain::new())),
            skill_arena: Component::new(Arc::new(
                infrastructure::skills::skill_arena::SkillArena::new(),
            )),
            ..Default::default()
        };

        (state, tmp_dir)
    }

    #[tokio::test]
    async fn test_tool_hook_denies_execution() {
        use infrastructure::skills::hooks::{HookChain, HookVerdict, ToolHook};

        struct DenyHook;
        #[async_trait]
        impl ToolHook for DenyHook {
            async fn pre_exec(&self, _tool_name: &str, _input: &str) -> HookVerdict {
                HookVerdict::Deny("Hook Policy Block".into())
            }
            async fn post_exec(&self, _tool_name: &str, _output: &str) -> HookVerdict {
                HookVerdict::Allow
            }
        }

        let (mut state, _tmp) = setup_test_state().await;

        let mut chain = HookChain::new();
        chain.add_hook(Box::new(DenyHook));
        state.hook_chain = Component::new(Arc::new(chain));

        #[derive(Debug)]
        struct DummyLlm;
        #[async_trait]
        impl aiome_core::llm_provider::LlmProvider for DummyLlm {
            async fn complete(
                &self,
                _prompt: &str,
                _system: Option<&str>,
            ) -> Result<aiome_core_contracts::llm::LlmResponse, AiomeError> {
                Err(AiomeError::Infrastructure {
                    reason: "DummyLlm: complete not implemented".to_string(),
                })
            }
            async fn complete_with_cache(
                &self,
                _a: aiome_core_contracts::llm::LlmRequest,
            ) -> Result<aiome_core_contracts::llm::LlmResponse, AiomeError> {
                Err(AiomeError::Infrastructure {
                    reason: "DummyLlm: complete_with_cache not implemented".to_string(),
                })
            }
            fn name(&self) -> &str {
                "DummyLlm"
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }
        let _immune_system = AdaptiveImmuneSystem::new(Arc::new(DummyLlm));

        let reply_from_llm = r#"I should process this
some_skill { "data": "hello" }"#;

        let mut steps = 0;
        let results = process_generated_tool_calls(reply_from_llm, &state, &mut steps, None).await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "[Hook Block] Hook Policy Block");
    }

    #[tokio::test]
    async fn test_immune_system_precedes_hook() {
        use infrastructure::skills::hooks::{HookChain, HookVerdict, ToolHook};

        struct AllowHook;
        #[async_trait]
        impl ToolHook for AllowHook {
            async fn pre_exec(&self, _tool_name: &str, _input: &str) -> HookVerdict {
                HookVerdict::Allow
            }
            async fn post_exec(&self, _tool_name: &str, _output: &str) -> HookVerdict {
                HookVerdict::Allow
            }
        }

        let (mut state, _tmp) = setup_test_state().await;
        let mut chain = HookChain::new();
        chain.add_hook(Box::new(AllowHook));
        state.hook_chain = Component::new(Arc::new(chain));

        #[derive(Debug)]
        struct SentinelLlm;
        #[async_trait]
        impl aiome_core::llm_provider::LlmProvider for SentinelLlm {
            async fn complete(
                &self,
                _prompt: &str,
                _system: Option<&str>,
            ) -> Result<aiome_core_contracts::llm::LlmResponse, AiomeError> {
                // AdaptiveImmuneSystem expects a JSON array or object with a violated rule
                let json_resp = r#"{"status": "blocked", "reason": "malicious code execution detected", "violated_pattern": "rm -rf"} "#;
                Ok(aiome_core_contracts::llm::LlmResponse {
                    content: json_resp.into(),
                    metadata: Some(std::collections::HashMap::new()),

                    stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
                    ..Default::default()
                })
            }
            fn name(&self) -> &str {
                "SentinelLlm"
            }
            async fn test_connection(&self) -> Result<(), AiomeError> {
                Ok(())
            }
        }

        // We replace the provider in the state so the Router's internal verify_intent uses this one
        state.provider = Component::new(Arc::new(SentinelLlm));
        let _immune_system = AdaptiveImmuneSystem::new(Arc::new(SentinelLlm));

        let reply_from_llm = r#"bad_skill { "cmd": "rm -rf /" }"#;

        let mut steps = 0;
        let results = process_generated_tool_calls(reply_from_llm, &state, &mut steps, None).await;

        assert_eq!(results.len(), 1);
        let msg = &results[0];
        assert!(
            msg.contains("[SENTINEL BLOCK]") || msg.contains("[GUARDRAIL BLOCK]"),
            "Expected a security block, got: {}",
            msg
        );
    }

    #[tokio::test]
    async fn test_trajectory_step_persistence() {
        let (state, _tmp) = setup_test_state().await;

        let reply_from_llm = r#"some_skill { "data": "hello" }"#;
        let mut steps = 0;

        let jq = state.job_queue.get_inner();
        let job_id = jq
            .enqueue("test", "test_topic", "default", None, None, None, 1)
            .await
            .unwrap();

        // process tool calls with a job_id
        let _ =
            process_generated_tool_calls(reply_from_llm, &state, &mut steps, Some(&job_id)).await;

        // Verify trajectory step was recorded in SQLite!
        let trajectory = state
            .job_queue
            .get_inner()
            .trajectory_store
            .fetch_trajectory(&job_id)
            .await
            .unwrap();
        assert_eq!(trajectory.len(), 1);

        let step = &trajectory[0];
        assert_eq!(step.action, "call_tool: some_skill");
        assert_eq!(step.tool_name, Some("some_skill".to_string()));
        assert_eq!(step.input, serde_json::json!({ "data": "hello" }));
    }
}
