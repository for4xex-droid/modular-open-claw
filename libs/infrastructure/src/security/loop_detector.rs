/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{LlmRequest, LlmResponse};
use aiome_core_contracts::security::AgentHook;
use async_trait::async_trait;

#[derive(Debug, Default)]
pub struct LoopDetectorHook;

fn get_tool_classification(skill_name: &str) -> &'static str {
    match skill_name {
        "run_command"
        | "write_to_file"
        | "replace_file_content"
        | "multi_replace_file_content"
        | "delete_file"
        | "git_commit"
        | "git_push" => "Mutating",
        "view_file" | "list_dir" | "grep_search" | "read_url_content" | "search_web"
        | "command_status" => "Readonly",
        _ => "Idempotent",
    }
}

fn get_threshold(skill_name: &str) -> usize {
    match get_tool_classification(skill_name) {
        "Mutating" => 3,
        "Readonly" => 5,
        _ => 10,
    }
}

fn parse_tool_calls(text: &str) -> Vec<(String, String)> {
    let mut calls = Vec::new();
    let mut start_idx = 0;

    while let Some(brace_start) = text[start_idx..].find('{') {
        let abs_brace = start_idx + brace_start;
        let before_brace = text[..abs_brace].trim();
        if before_brace.is_empty() {
            start_idx = abs_brace + 1;
            continue;
        }

        let skill_name = before_brace
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .rfind(|s| !s.is_empty())
            .unwrap_or("")
            .to_string();

        if !skill_name.is_empty() && skill_name != "CallSkill" {
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

#[async_trait]
impl AgentHook for LoopDetectorHook {
    async fn on_pre_execute(&self, request: &LlmRequest) -> Result<(), AiomeError> {
        let mut last_calls_hash = None;
        let mut consecutive_count = 0;
        let mut target_threshold = 10;

        for msg in request.messages.iter().rev() {
            if msg.role != "assistant" {
                continue;
            }

            let calls = parse_tool_calls(&msg.content);
            if calls.is_empty() {
                break; // If assistant didn't call a tool, chain is broken
            }

            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            for (name, input) in &calls {
                name.hash(&mut hasher);
                input.hash(&mut hasher);
            }
            let h = hasher.finish();

            if let Some(last_h) = last_calls_hash {
                if h == last_h {
                    consecutive_count += 1;
                } else {
                    break;
                }
            } else {
                last_calls_hash = Some(h);
                consecutive_count = 1;
                target_threshold = calls
                    .iter()
                    .map(|(n, _)| get_threshold(n))
                    .min()
                    .unwrap_or(10);
            }

            if consecutive_count >= target_threshold {
                tracing::warn!(
                    "🛡️ [LoopDetectorHook] Loop detected. Blocked after {} identical calls.",
                    consecutive_count
                );
                return Err(AiomeError::SecurityViolation {
                    reason: format!(
                        "Loop detected: Tool called identically {} times.",
                        consecutive_count
                    ),
                });
            }
        }

        Ok(())
    }

    async fn on_post_execute(
        &self,
        _request: &LlmRequest,
        _response: &LlmResponse,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn on_job_completed(&self, _job_id: &str, _status: &str) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn on_proof_completed(
        &self,
        _skill_name: &str,
        _is_valid: bool,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn on_transaction_completed(
        &self,
        _source: &str,
        _amount_cents: i64,
        _actor_id: &str,
        _transaction_id: &str,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn on_permission_request(&self, _tool: &str, _reason: &str) -> Result<bool, AiomeError> {
        Ok(true)
    }

    async fn on_session_start(&self) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn on_stop(&self, _reason: &str) -> Result<(), AiomeError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core_contracts::llm::LlmMessage;

    #[tokio::test]
    async fn test_mutating_tool_loop_blocked() {
        let hook = LoopDetectorHook::default();
        let mut request = LlmRequest::default();

        let tool_content = r#"Let me try writing the file again.
write_to_file { "TargetFile": "test.rs", "CodeContent": "fn main() {}" }"#;

        for _ in 0..4 {
            request.messages.push(LlmMessage {
                role: "assistant".into(),
                content: tool_content.into(),
                cache: false,
            });
            request.messages.push(LlmMessage {
                role: "tool".into(),
                content: "Error: file is locked".into(),
                cache: false,
            });
        }

        // 4th attempt should be blocked since Mutating limit is 3.
        let result = hook.on_pre_execute(&request).await;
        assert!(
            result.is_err(),
            "Mutating tool loop should be blocked after 3 identical calls"
        );
        if let Err(AiomeError::SecurityViolation { reason }) = result {
            assert!(reason.contains("Loop detected"));
        } else {
            panic!("Expected SecurityViolation error");
        }
    }

    #[tokio::test]
    async fn test_readonly_tool_loop_blocked() {
        let hook = LoopDetectorHook::default();
        let mut request = LlmRequest::default();

        let tool_content = r#"Let me view the file again.
view_file { "AbsolutePath": "/tmp/test.rs" }"#;

        for _ in 0..6 {
            request.messages.push(LlmMessage {
                role: "assistant".into(),
                content: tool_content.into(),
                cache: false,
            });
            request.messages.push(LlmMessage {
                role: "tool".into(),
                content: "Error: No such file".into(),
                cache: false,
            });
        }

        // 6th attempt should be blocked since Readonly limit is 5.
        let result = hook.on_pre_execute(&request).await;
        assert!(
            result.is_err(),
            "Readonly tool loop should be blocked after 5 identical calls"
        );
    }
}
