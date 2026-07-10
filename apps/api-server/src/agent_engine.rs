/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core::error::AiomeError;
use aiome_core::traits::{ChatStore, KarmaRegistry, SettingsOps};
use aiome_core_contracts::events::CoreEvent;
use shared::guardrails;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, warn};

// --- Backward compatibility exports ---
pub(crate) use crate::system_instructions::*;
pub(crate) use crate::tool_call_processor::*;

pub fn extract_thinking_process(content: &str) -> (String, Option<String>) {
    let mut cleaned_content = String::new();
    let mut thinking_parts = Vec::new();
    let mut remaining = content;

    while let Some(start_idx) = remaining.find("<thinking>") {
        cleaned_content.push_str(&remaining[..start_idx]);
        let after_start = &remaining[start_idx + 10..];

        if let Some(end_idx) = after_start.find("</thinking>") {
            thinking_parts.push(after_start[..end_idx].trim().to_string());
            remaining = &after_start[end_idx + 11..];
        } else {
            // Unclosed thinking tag
            thinking_parts.push(after_start.trim().to_string());
            remaining = "";
            break;
        }
    }
    cleaned_content.push_str(remaining);

    // Defense-in-depth: Ensure no stray tags leak to the UI.
    let cleaned_content = cleaned_content
        .replace("<thinking>", "")
        .replace("</thinking>", "")
        .trim()
        .to_string();

    let final_thinking = if thinking_parts.is_empty() {
        None
    } else {
        Some(thinking_parts.join("\n---\n"))
    };

    (cleaned_content, final_thinking)
}

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
            .store_chat_message(&actual_channel_id, "user", prompt, None)
            .await
        {
            error!("❌ [AgentEngine] Failed to store user message: {:?}", e);
        }

        // 3. Prepare Context
        let soul_hash = state.get_system_soul_hash().await;

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

        let mut economic_context = None;
        let system_id = state.system_agent_id;
        if let Some(engine) = state.commerce_engine.as_opt() {
            if let (Ok(balance), Ok(spent), Ok(limit), Ok(spent_month), Ok(month_limit)) = (
                engine.get_balance(system_id).await,
                engine.get_daily_spend(system_id).await,
                engine.get_daily_limit(system_id).await,
                engine.get_monthly_spend(system_id).await,
                engine.get_monthly_limit(system_id).await,
            ) {
                economic_context = Some(aiome_core::commerce::EconomicContext {
                    balance,
                    spent_today: spent,
                    daily_limit: limit,
                    spent_this_month: spent_month,
                    monthly_limit: month_limit,
                });
            }
        }

        let instructions = crate::system_instructions::build_system_instructions(
            state,
            &karma_str,
            summary.as_deref(),
            ai_name,
            None,
            economic_context,
            soul_snapshot,
            None,
        )
        .await;

        // 4. Chat Loop (Total 15 turns max)
        let mut turn = 0;
        let max_turns = 15;
        let mut final_reply = String::from("...");
        let mut total_token_in: i64 = 0;
        let mut total_token_out: i64 = 0;
        let mut has_token_metadata = false;
        let _chat_execution_id = format!("chat_exec_{}", uuid::Uuid::new_v4());
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
                    let mut reply = resp.content.trim().to_string();
                    if let Some(ref m) = resp.metadata {
                        if let Some(t_in) =
                            m.get("prompt_tokens").and_then(|v| v.parse::<i64>().ok())
                        {
                            total_token_in += t_in;
                            has_token_metadata = true;
                        }
                        if let Some(t_out) = m
                            .get("completion_tokens")
                            .and_then(|v| v.parse::<i64>().ok())
                        {
                            total_token_out += t_out;
                            has_token_metadata = true;
                        }
                    }
                    let (clean_reply, reasoning) = extract_thinking_process(&reply);
                    let mut meta = None;
                    if let Some(r) = reasoning {
                        meta = Some(serde_json::json!({ "reasoning": r }));
                    }
                    reply = clean_reply;
                    final_reply = reply.clone();
                    // Phase B-2 Enhancement: Store assistant response BEFORE tool execution
                    // This ensures the audit trail reflects the intent even if execution crashes.
                    if let Err(e) = state
                        .job_queue
                        .get_inner()
                        .store_chat_message(&actual_channel_id, "assistant", &reply, meta)
                        .await
                    {
                        error!("❌ Failed to store intent response: {:?}", e);
                    }

                    let skill_results = crate::tool_call_processor::process_generated_tool_calls(
                        &reply,
                        state,
                        &mut total_steps,
                        Some(&actual_channel_id),
                    )
                    .await;

                    if !skill_results.is_empty() {
                        // FIX: Intermediate results are already persisted in trajectory_steps.
                        // ChatStore remains a high-level summary of the final outcome.

                        let combined_results = skill_results.join("\n\n");
                        let sys_msg = format!("[Tool Output]\n{}", combined_results);
                        if let Err(e) = state
                            .job_queue
                            .get_inner()
                            .store_chat_message(&actual_channel_id, "system", &sys_msg, None)
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
        // Note: The final assistant response is already stored at line 204 to ensure audit trailing before tools.

        // P1 & P2: Provider-Aware Generation Cost Deduction for Autonomous Mode
        // Guard: Only bill if LLM actually generated a real reply (not the initial placeholder "...").
        if final_reply != "..." && !final_reply.is_empty() {
            if let Some(engine) = state.commerce_engine.as_opt().cloned() {
                let provider_name = provider.name().to_lowercase();
                if !provider_name.contains("ollama") && !provider_name.contains("local") {
                    let agent_id_for_billing = state.system_agent_id;
                    let state_clone = state.clone();
                    let final_reply_len = final_reply.len();
                    // フォールバック推定: system_instructions + 元のユーザープロンプトの合計長を使用
                    let full_input_len = instructions.len() + prompt.len();

                    tokio::spawn(async move {
                        // DBまたは環境変数からLLMモデル名を取得
                        let model_name = state_clone
                            .job_queue
                            .get_inner()
                            .get_setting_value("llm_model")
                            .await
                            .ok()
                            .flatten()
                            .or_else(|| std::env::var("LLM_MODEL").ok())
                            .unwrap_or_else(|| "gpt-4o".to_string());

                        // メタデータからトークン数が得られた場合はそれを使用、
                        // なければ文字数ベースの推定値にフォールバック
                        let token_in = if has_token_metadata && total_token_in > 0 {
                            total_token_in
                        } else {
                            (full_input_len as i64 / 3).max(1)
                        };
                        let token_out = if has_token_metadata && total_token_out > 0 {
                            total_token_out
                        } else {
                            (final_reply_len as i64 / 2).max(1)
                        };

                        let cost = infrastructure::llm::dynamic::calculate_cost_coins(
                            &model_name,
                            Some(token_in),
                            Some(token_out),
                        );

                        if cost > 0 {
                            if let Err(e) = engine
                                .deduct_generation_cost(
                                    agent_id_for_billing,
                                    None,
                                    cost,
                                    "autonomous_inference",
                                )
                                .await
                            {
                                error!(
                                    "🚨 [Billing] Failed to deduct generation cost from Agent {}: {:?}",
                                    agent_id_for_billing, e
                                );
                            } else {
                                tracing::info!(
                                    "💳 [Billing] Deducted {} coins for autonomous_inference (Model: {}, Tokens In/Out: {}/{})",
                                    cost, model_name, token_in, token_out
                                );
                            }
                        }
                    });
                } else {
                    tracing::debug!(
                        "🆓 [Billing Bypass] Local model ({}) used. No cost deducted.",
                        provider.name()
                    );
                }
            }
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
    use crate::routes::agent::should_trigger_diagnostics;
    use aiome_core::trajectory::TrajectoryStep;

    #[test]
    fn test_should_trigger_diagnostics() {
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
