/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::skill_handler;
use crate::AppState;
use aiome_contracts::events::CoreEvent;
use aiome_core::error::AiomeError;
use aiome_core::traits::{ChatStore, KarmaRegistry};
use shared::guardrails;
use std::time::Duration;
use tokio::fs;
use tokio::time::timeout;
use tracing::{error, info, warn};

// --- Backward compatibility exports ---
pub(crate) use crate::system_instructions::*;
pub(crate) use crate::tool_call_processor::*;

/// エージェントとの対話ロジックを管理するエンジン。
/// HTTPハンドラと内部サービスの双方から利用される。
pub struct AgentEngine;

impl AgentEngine {
    /// エージェントにチャットメッセージを送信し、応答を生成する（フル機能版）。
    pub async fn chat(
        state: &AppState,
        prompt: &str,
        channel_id: Option<String>,
        user_agent_id: uuid::Uuid,
    ) -> Result<String, AppError> {
        // 1. Guardrails & Security
        if let guardrails::ValidationResult::Blocked(reason) = guardrails::validate_input(prompt) {
            return Ok(format!("🚨 [GUARDRAIL BLOCK] {}", reason));
        }

        let provider = state.provider.get_inner().clone();
        let immune_system =
            infrastructure::immune_system::AdaptiveImmuneSystem::new(provider.clone());
        match immune_system
            .verify_intent(prompt, &**state.job_queue.get_inner())
            .await
        {
            Ok(Some(rule)) => {
                warn!(
                    "Sentinel Block activated in AgentEngine: pattern `{}`",
                    rule.pattern
                );
                return Ok(format!(
                    "🚨 [SENTINEL BLOCK] Security violation detected. Pattern: {}",
                    rule.pattern
                ));
            }
            Err(e) => {
                error!(
                    "Adaptive Immune System evaluation failed in AgentEngine: {:?}",
                    e
                );
                // fail-open: proceed with caution
            }
            _ => {}
        }

        let actual_channel_id = channel_id.unwrap_or_else(|| "default_console".to_string());

        // 2. Persist user message
        if let Err(e) = state
            .job_queue
            .get_inner()
            .store_chat_message(&actual_channel_id, "user", prompt)
            .await
        {
            error!("❌ [AgentEngine] Failed to store user message: {:?}", e);
        }

        // 3. Prepare Context
        let soul_hash = {
            use std::hash::{Hash, Hasher};
            let resolver = &state.config.get_inner().resolver;
            let soul = crate::system_instructions::read_app_data_file(resolver, "SOUL.md").await;
            let evolving_soul =
                crate::system_instructions::read_app_data_file(resolver, "EVOLVING_SOUL.md").await;
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            format!("{}{}", soul, evolving_soul).hash(&mut hasher);
            format!("{:x}", hasher.finish())
        };

        let karma_result = state
            .job_queue
            .get_inner()
            .fetch_relevant_karma(prompt, "global", 5, &soul_hash)
            .await
            .unwrap_or_else(|_| aiome_core::traits::KarmaSearchResult::empty());
        let karma_str = karma_result
            .entries
            .iter()
            .map(|e| format!("- {}", e.lesson))
            .collect::<Vec<_>>()
            .join("\n");

        let (summary, _) = state
            .context_engine
            .get_inner()
            .get_intelligent_history(&actual_channel_id, 10)
            .await
            .unwrap_or((None, Vec::new()));

        let ai_name = state
            .job_queue
            .get_inner()
            .get_setting_value("ai_name")
            .await
            .ok()
            .flatten();
        let soul_snapshot = state.soul_store.get_inner().get_snapshot().await;

        let instructions = crate::system_instructions::build_system_instructions(
            state,
            &karma_str,
            summary.as_deref(),
            ai_name,
            None,
            None,
            soul_snapshot,
        )
        .await;

        // 4. Chat Loop (Total 15 turns max)
        let mut turn = 0;
        let max_turns = 15;
        let mut final_reply = String::from("...");
        let chat_execution_id = format!("chat_exec_{}", uuid::Uuid::new_v4());
        let mut total_steps = 0;

        let mut current_prompt = prompt.to_string();

        while turn < max_turns {
            let request = state
                .context_engine
                .get_inner()
                .prepare_hybrid_request(&actual_channel_id, &current_prompt, Some(&instructions))
                .await?;

            // Economy Validation
            if let Some(engine) = state.commerce_engine.as_opt() {
                if let Err(e) = engine
                    .validate_activity(user_agent_id, "inference", 1)
                    .await
                {
                    return Err(AppError(e));
                }
            }

            let _permit = state
                .llm_semaphore
                .get_inner()
                .acquire()
                .await
                .map_err(|e| {
                    error!("Failed to acquire LLM permit: {}", e);
                    AiomeError::Infrastructure {
                        reason: "Service busy".into(),
                    }
                })?;

            match timeout(
                Duration::from_secs(300),
                provider.complete_with_cache(request),
            )
            .await
            {
                Ok(Ok(resp)) => {
                    let reply = resp.content.trim().to_string();
                    final_reply = reply.clone();
                    let skill_results = crate::tool_call_processor::process_generated_tool_calls(
                        &reply,
                        state,
                        &mut total_steps,
                    )
                    .await;

                    if !skill_results.is_empty() {
                        // FIX: Store intermediate tool invocations to feed the context engine on the next loop iteration
                        if let Err(e) = state
                            .job_queue
                            .get_inner()
                            .store_chat_message(&actual_channel_id, "assistant", &reply)
                            .await
                        {
                            error!(
                                "❌ Failed to store intermediate assistant response: {:?}",
                                e
                            );
                        }

                        let combined_results = skill_results.join("\n\n");
                        let sys_msg = format!("[Tool Output]\n{}", combined_results);
                        if let Err(e) = state
                            .job_queue
                            .get_inner()
                            .store_chat_message(&actual_channel_id, "system", &sys_msg)
                            .await
                        {
                            error!("❌ Failed to store intermediate tool outputs: {:?}", e);
                        }

                        // For the next turn, we instruct the LLM to continue based on history, instead of repeating the user prompt
                        current_prompt = "[System] Tool execution completed. Please evaluate the tool output and continue.".to_string();

                        turn += 1;
                        continue;
                    }
                    break;
                }
                Ok(Err(e)) => return Err(AppError(e)),
                Err(_) => {
                    return Err(AiomeError::Infrastructure {
                        reason: "LLM Timeout".into(),
                    }
                    .into())
                }
            }
        }

        // 5. Finalize
        if let Err(e) = state
            .job_queue
            .get_inner()
            .store_chat_message(&actual_channel_id, "assistant", &final_reply)
            .await
        {
            error!(
                "❌ [AgentEngine] Failed to store final assistant response: {:?}",
                e
            );
        }

        // Notify Watchtower bridges
        let _ = state
            .event_sender
            .get_inner()
            .send(CoreEvent::ChatResponse {
                response: final_reply.clone(),
                channel_id: actual_channel_id.parse().unwrap_or(0),
                resource_path: None,
            });

        Ok(final_reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::Component;
    use infrastructure::job_queue::UniversalJobQueue;
    use infrastructure::registry::{AssetManifest, AssetType, RegistryManager};
    use infrastructure::skills::WasmSkillManager;
    use std::sync::Arc;

    async fn setup_test_state() -> (crate::AppState, tempfile::TempDir) {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test_agent.db");
        let pool_url = format!("sqlite://{}", db_path.to_str().unwrap());

        let ts = std::sync::Arc::new(
            infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(
                infrastructure::db::DatabasePool::new_sqlite(&pool_url)
                    .await
                    .unwrap(),
            ),
        );
        let jq = Arc::new(UniversalJobQueue::new(&pool_url, None, ts).await.unwrap());
        let registry = Arc::new(RegistryManager::new(jq.get_pool().clone()));

        let skills_dir = tmp_dir.path().join("skills");
        let sandbox_dir = tmp_dir.path().join("sandbox");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::create_dir_all(&sandbox_dir).unwrap();

        let wsm = Arc::new(
            WasmSkillManager::new(skills_dir.to_str().unwrap(), sandbox_dir.to_str().unwrap())
                .unwrap(),
        );

        let mut config = shared::config::AiomeConfig::default();
        config.resolver = shared::app_data::AppDataResolver::new();

        let state = crate::AppState {
            registry: Component::new(registry),
            wasm_skill_manager: Component::new(wsm),
            job_queue: Component::new(jq),
            config: Component::new(Arc::new(config)),
            ..Default::default()
        };

        (state, tmp_dir)
    }

    use crate::routes::agent::should_trigger_diagnostics;

    #[test]
    fn test_should_trigger_diagnostics() {
        use aiome_core::trajectory::{ConstraintViolation, TrajectoryStep};

        let mut step1 = TrajectoryStep::default();
        step1.is_critical_failure = false;

        assert!(
            !should_trigger_diagnostics(&[step1.clone()]),
            "No failure should not trigger"
        );

        let mut step2 = TrajectoryStep::default();
        step2.is_critical_failure = true;
        assert!(
            should_trigger_diagnostics(&[step2.clone()]),
            "Critical failure should trigger"
        );
    }
}
