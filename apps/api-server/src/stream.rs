/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::JobQueue;
use axum::{
    extract::{Json, State},
    response::sse::{Event, KeepAlive, Sse},
    response::IntoResponse,
};
use core::convert::Infallible;
use futures::stream::Stream;
// use infrastructure::skills::UnverifiedSkill;
use crate::skill_handler;
use futures::StreamExt;
use std::sync::Arc;
use tokio::time::{interval, timeout, Duration};
use tracing::info;

use crate::routes::agent::{
    build_system_instructions, parse_tool_calls, read_workspace_file, AgentChatRequest,
};
use crate::AppState;

pub async fn trigger_agent_chat_stream(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(payload): Json<AgentChatRequest>,
) -> impl axum::response::IntoResponse {
    let provider = (*state.provider).clone();

    let stream = async_stream::stream! {
        // Discovery H: Guardrails check (Security Layer 0)
        if let shared::guardrails::ValidationResult::Blocked(reason) = shared::guardrails::validate_input(&payload.prompt) {
            yield Ok::<Event, Infallible>(Event::default().event("security_block").data(format!("🚨 [GUARDRAIL BLOCK] {}", reason)));
            return;
        }

        // Discovery B: Immune System check (Security Layer 1)
        let immune_system = infrastructure::immune_system::AdaptiveImmuneSystem::new(provider.clone());
        if let Ok(Some(rule)) = immune_system.verify_intent(&payload.prompt, state.job_queue.as_ref()).await {
            let stats = state.job_queue.get_agent_stats().await.unwrap_or_default();
            let _ = state.job_queue.record_evolution_event(
                stats.level,
                "ImmuneAlert",
                &format!("Block: {} (Pattern: {})", rule.action, rule.pattern),
                None,
                None
            ).await;
            yield Ok::<Event, Infallible>(Event::default().event("security_block").data(format!("🚨 [SENTINEL BLOCK] {}\nPattern: {}", rule.action, rule.pattern)));
            return;
        }

        let soul_hash = {
            use std::hash::{Hash, Hasher};
            let soul = read_workspace_file("SOUL.md");
            let evolving_soul = read_workspace_file("EVOLVING_SOUL.md");
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            format!("{}{}", soul, evolving_soul).hash(&mut hasher);
            format!("{:x}", hasher.finish())
        };

        // Sprint 1-A: Fetch relevant karma using proper search
        let karma_result = state.job_queue.fetch_relevant_karma(&payload.prompt, "global", 5, &soul_hash).await.unwrap_or_else(|_| aiome_core::traits::KarmaSearchResult::empty());

        let mut karma_str = karma_result.entries.iter()
            .map(|e| format!("- {}", e.lesson))
            .collect::<Vec<_>>()
            .join("\n");

        if karma_result.is_ood {
            karma_str.push_str("\n[NOTICE: 関連する過去の教訓は見つかりませんでした。このリクエストは新しいコンテキストである可能性があります。]");
        }

        // Notify client about relevant karma being used (Sprint 4-A)
        yield Ok::<Event, Infallible>(Event::default().event("karma").data(&karma_str));

        // Also send structured data for feedback mechanisms
        let karma_json = serde_json::json!({
            "is_ood": karma_result.is_ood,
            "entries": karma_result.entries
        });
        yield Ok::<Event, Infallible>(Event::default().event("karma_data").data(karma_json.to_string()));

        // God Mode (Phase 21): Fetch relevant project knowledge
        let knowledge_result = state.artifact_store.search_artifacts_semantic(
            &payload.prompt,
            Some(aiome_core::traits::ArtifactCategory::Knowledge),
            2
        ).await.unwrap_or_default();
        let knowledge_str = if knowledge_result.is_empty() {
            None
        } else {
            Some(knowledge_result.iter()
                .map(|a| format!("--- {} ---\n{}", a.title, a.text_content.as_deref().unwrap_or("（内容なし）")))
                .collect::<Vec<_>>()
                .join("\n\n"))
        };

        if let Some(ref _ks) = knowledge_str {
            // Notify client about relevant knowledge being used
            let titles = knowledge_result.iter().map(|a| a.title.as_str()).collect::<Vec<_>>().join(", ");
            yield Ok::<Event, Infallible>(Event::default().event("knowledge").data(&titles));
        }

        let channel_id = payload.channel_id.unwrap_or_else(|| "default_console".to_string());

        // Phase 3-B: Persist user message
        let _ = state.job_queue.store_chat_message(&channel_id, "user", &payload.prompt).await;

        // Phase 3-C: Fetch intelligent context
        let (summary, db_history) = state.context_engine.get_intelligent_history(&channel_id, 10).await.unwrap_or((None, Vec::new()));

        let mut current_history = Vec::new();

        // Combine DB history and current request history
        for msg in db_history {
            let role = msg["role"].as_str().unwrap_or("user");
            let content = msg["content"].as_str().unwrap_or("");
            let prefix = if role == "user" { "USER: " } else { "AI: " };
            current_history.push(format!("{}{}", prefix, content));
        }

        let ai_name = state.job_queue.get_setting_value("ai_name").await.ok().flatten();

        let mut economic_context = None;
        if let Some(engine) = state.commerce_engine.as_opt() {
            let agent_id = state.job_queue.get_system_agent_id().await.unwrap_or_else(|err| {
                tracing::error!("Failed to get system agent ID, falling back to nil: {:?}", err);
                uuid::Uuid::nil()
            });
            if let (Ok(balance), Ok(spent_today), Ok(daily_limit)) = (
                engine.get_balance(agent_id).await,
                engine.get_daily_spend(agent_id).await,
                engine.get_daily_limit(agent_id).await,
            ) {
                economic_context = Some(aiome_core::commerce::EconomicContext {
                    balance,
                    spent_today,
                    daily_limit,
                });
            }
        }

        let soul_snapshot = state.soul_store.get_snapshot().await;
        // Load actual soul from store to get last_begging_at memory
        let mut soul = state.soul_store.load_soul("system-soul").await.unwrap_or_default().unwrap_or_else(|| soul::AgentSoul::new("system-soul".to_string()));

        let system_instructions = build_system_instructions(
            &state,
            &karma_str,
            summary.as_deref(),
            ai_name,
            knowledge_str.as_deref(),
            economic_context,
            soul_snapshot,
        ).await;

        let mut turn = 0;
        let max_turns = 15;
        let original_prompt = payload.prompt.clone();
        let mut full_reply_for_storage = String::new();

        while turn < max_turns {
            let full_prompt = format!(
                "{}\n{}\nUSER: {}\nAI: ",
                system_instructions,
                current_history.join("\n"),
                original_prompt
            );

            yield Ok(Event::default().event("turn_start").data(turn.to_string()));

            // Security Layer: Enforce semaphore even for SSE to prevent GPU OOM
            // Phase 15.1: 10s timeout for VRAM arbitration
            let _permit = match timeout(Duration::from_secs(10), state.llm_semaphore.acquire()).await {
                Ok(Ok(p)) => p,
                _ => {
                    yield Ok(Event::default().event("error").data("System busy: VRAM resource contention. Please try again in 10-30 seconds."));
                    return;
                }
            };

            if let Ok(Ok(mut llm_stream)) = timeout(Duration::from_secs(300), provider.stream_complete(&full_prompt, None)).await {

                let mut buffer = String::new();
                let mut full_reply = String::new();
                let mut is_tool_call_mode = false;

                while let Some(chunk_res) = llm_stream.next().await {
                    let chunk: String = chunk_res.unwrap_or_default();
                    full_reply.push_str(&chunk);
                    full_reply_for_storage.push_str(&chunk);

                    if !is_tool_call_mode {
                        buffer.push_str(&chunk);
                        if let Some(idx) = buffer.find("[CallSkill") {
                            let text = buffer[..idx].to_string();
                            if !text.is_empty() {
                                yield Ok(Event::default().event("text").data(&text));
                            }
                            is_tool_call_mode = true;
                            yield Ok(Event::default().event("tool_detect").data("tool detected"));
                        } else if let Some(idx) = buffer.rfind('[') {
                            let text = buffer[..idx].to_string();
                            if !text.is_empty() {
                                yield Ok(Event::default().event("text").data(&text));
                            }
                            buffer = buffer[idx..].to_string();
                        } else if !buffer.is_empty() {
                            yield Ok(Event::default().event("text").data(&buffer));
                            buffer.clear();
                        }
                    }
                }

                if !is_tool_call_mode && !buffer.is_empty() {
                    yield Ok(Event::default().event("text").data(&buffer));
                }

                // Phase 15.1: Begging Control with Memory
                use shared::guardrails::{BeggingSupervisor, ValidationResult};
                let now = chrono::Utc::now();
                match BeggingSupervisor::validate_output_with_memory(&full_reply, soul.last_begging_at, now) {
                    ValidationResult::Blocked(reason) => {
                        tracing::warn!("🛡️ [Guardrail] Begging blocked: {}", reason);
                        yield Ok(Event::default().event("error").data(format!("🚨 [SECURITY BLOCK] {}", reason)));
                        break; // Stop turns
                    }
                    ValidationResult::Valid => {
                        // Check if it WAS a begging attempt that we allowed (implied by validate_output check)
                        if let ValidationResult::Blocked(_) = BeggingSupervisor::validate_output(&full_reply) {
                            tracing::info!("🎁 [Guardrail] Begging allowed (First time or outside window). Updating memory.");
                            soul.last_begging_at = Some(now);
                            if let Err(e) = state.soul_store.save_soul(&soul).await {
                                tracing::error!("🚨 Failed to update soul last_begging_at memory: {:?}", e);
                            }
                        }
                    }
                }

                let calls = parse_tool_calls(&full_reply);

                if calls.is_empty() {
                    yield Ok(Event::default().event("turn_end").data("done"));
                    break;
                } else {
                    let mut skill_results = Vec::new();
                    for (skill_name, skill_input) in calls {
                        info!("🛠️ [AgentStreamLoop] Executing skill: {}", skill_name);
                        yield Ok(Event::default().event("tool_exec").data(format!("Executing {}", skill_name)));

                        if skill_name.starts_with("forge_") {
                            // Phase 12-C: SSE Heartbeat implementation for long-running forge tasks
                            let mut heartbeat_ticker = interval(Duration::from_secs(5));
                            let forge_future = skill_handler::execute_forge_command(&skill_name, &skill_input, &state);
                            tokio::pin!(forge_future);

                            loop {
                                tokio::select! {
                                    _ = heartbeat_ticker.tick() => {
                                        yield Ok(Event::default().event("heartbeat").data("build in progress..."));
                                    }
                                    res = &mut forge_future => {
                                        match res {
                                            Ok(out) => {
                                                skill_results.push(out.clone());
                                                yield Ok(Event::default().event("tool_result").data(out));
                                            }
                                            Err(e) => {
                                                skill_results.push(format!("[{} Error: {}]", skill_name, e));
                                                yield Ok(Event::default().event("tool_result").data(format!("Error: {}", e)));
                                            }
                                        }
                                        break;
                                    }
                                }
                            }
                        } else if skill_name == "describe_skill" {
                            let out = skill_handler::describe_skill(&skill_input, &state).await;
                            skill_results.push(out.clone());
                            yield Ok(Event::default().event("tool_result").data(format!("{}: metadata returned", skill_name)));
                        } else {
                            let out = skill_handler::execute_wasm_skill(&skill_name, &skill_input, &state, None, 0).await;

                            // Phase 2B: Record skill execution
                            let stats = state.job_queue.get_agent_stats().await.unwrap_or_default();
                            let status = if out.contains("Error:") { "failed" } else { "success" };
                            let _ = state.job_queue.record_evolution_event(
                                stats.level,
                                "SkillExecution",
                                &format!("Exec: {} -> {}", skill_name, status),
                                Some(&skill_name),
                                None
                            ).await;

                            skill_results.push(out.clone());
                            yield Ok(Event::default().event("tool_result").data(format!("{}: {}", skill_name, status)));
                        }
                    }

                    current_history.push(format!("AI: {}", full_reply));
                    current_history.push(format!("SYSTEM: [Results: {}]", skill_results.join("\n")));
                    turn += 1;
                }
            } else {
                yield Ok(Event::default().event("error").data("LLM timeout or error"));
                break;
            }
        }
        // Phase 3-D: Persist assistant message and maintain context
        let _ = state.job_queue.store_chat_message(&channel_id, "assistant", &full_reply_for_storage).await;
        let ce = state.context_engine.clone();
        let cid = channel_id.clone();
        tokio::spawn(async move {
            let _ = ce.maintain_context(&cid, 8000).await; // 文字数基準 (≒4000トークン)
        });

        yield Ok(Event::default().event("done").data("stream finished"));
    };

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

pub async fn trigger_system_vitality_stream(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let mut interval = interval(Duration::from_secs(5));
        let mut last_karma_count = 0;
        let mut last_evolution_count = 0;
        let mut last_level = 0;
        let mut last_is_thinking = false;
        let mut last_stats: Option<shared::watchtower::AgentStats> = None;
        let mut last_expression_id: Option<String> = None;

        // Initialize state
        if let Ok(stats) = state.job_queue.get_agent_stats().await {
            last_level = stats.level;
            last_karma_count = state.job_queue.fetch_all_karma(100).await.unwrap_or_default().len();
            last_evolution_count = state.job_queue.fetch_evolution_history(100).await.unwrap_or_default().len();
            last_is_thinking = state.job_queue.get_pending_job_count().await.unwrap_or(0) > 0;
            last_stats = Some(stats);
            if let Ok(exprs) = state.job_queue.fetch_expressions(1).await {
                if let Some(first) = exprs.first() {
                    last_expression_id = Some(first.id.clone());
                }
            }
        }

        let mut rx = state.event_sender.subscribe();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Connection quality: send server timestamp for RTT calculation
                    let now = chrono::Utc::now().to_rfc3339();
                    yield Ok(Event::default().event("ping").data(now));

                    // 4. Thinking Check
                    if let Ok(pending_count) = state.job_queue.get_pending_job_count().await {
                        let is_thinking = pending_count > 0;
                        if is_thinking && !last_is_thinking {
                            yield Ok(Event::default().event("job_started").data("thinking"));
                        } else if !is_thinking && last_is_thinking {
                            yield Ok(Event::default().event("job_completed").data("idle"));
                        }
                        last_is_thinking = is_thinking;
                    }

                    if let Ok(stats) = state.job_queue.get_agent_stats().await {
                        // 0. Continuous Stats Update
                        let stats_changed = if let Some(ref last) = last_stats {
                            last.level != stats.level ||
                            last.exp != stats.exp ||
                            last.resonance != stats.resonance ||
                            last.creativity != stats.creativity ||
                            last.fatigue != stats.fatigue
                        } else {
                            true
                        };

                        if stats_changed {
                            yield Ok(Event::default().event("agent_stats").data(serde_json::to_string(&stats).unwrap_or_default()));
                            last_stats = Some(stats.clone());
                        }

                        // 1. Level Up Check
                        if stats.level > last_level {
                            yield Ok(Event::default().event("level_up").data(serde_json::to_string(&stats).unwrap_or_default()));
                            last_level = stats.level;
                        }

                        // 2. Karma Check
                        let current_karmas = state.job_queue.fetch_all_karma(100).await.unwrap_or_default();
                        if current_karmas.len() > last_karma_count {
                            // Send newest karma
                            if let Some(new_karma) = current_karmas.first() {
                                    yield Ok(Event::default().event("karma_update").data(serde_json::to_string(new_karma).unwrap_or_default()));
                            }
                            last_karma_count = current_karmas.len();
                        }

                        // 3. Evolution Check (Inspiration / Soul Mutation / Alerts / SkillExec)
                        let current_evos = state.job_queue.fetch_evolution_history(100).await.unwrap_or_default();
                        if current_evos.len() > last_evolution_count {
                            if let Some(new_evo) = current_evos.first() {
                                let event_type = new_evo["event_type"].as_str().unwrap_or("inspiration");
                                let sse_event = match event_type {
                                    "ImmuneAlert" => "immune_alert",
                                    "SkillExecution" => "skill_execution",
                                    _ => "inspiration",
                                };
                                yield Ok(Event::default().event(sse_event).data(serde_json::to_string(new_evo).unwrap_or_default()));
                            }
                            last_evolution_count = current_evos.len();
                        }

                        // 4. Expression Check (Avatar Parameters)
                        if let Ok(exprs) = state.job_queue.fetch_expressions(1).await {
                            if let Some(latest) = exprs.first() {
                                if Some(latest.id.clone()) != last_expression_id {
                                    let resonance = stats.resonance as f32 / 100.0; // 0.0 - 1.0
                                    let mut physics_override = if latest.emotion == "excited" || latest.emotion == "aggressive" {
                                        1.2 + resonance * 0.3
                                    } else if latest.emotion == "sad" || latest.emotion == "tired" {
                                        0.5 + resonance * 0.2
                                    } else {
                                        1.0 + resonance * 0.1
                                    };

                                    // Phase 14c: Resonance Boost (Level 80+)
                                    if stats.resonance >= 80 {
                                        physics_override *= 1.5;
                                        info!("🔥 [Inochi2D] Resonance Boost active! Physics x1.5");
                                    }

                                    let mut payload = serde_json::json!({
                                        "id": latest.id,
                                        "emotion": latest.emotion,
                                        "content": latest.content,
                                        "physics_override": Some(physics_override),
                                    });
                                    if let Some(ref params) = latest.avatar_params {
                                        payload["avatar_params"] = params.clone();
                                    }
                                    yield Ok(Event::default().event("avatar_expression").data(serde_json::to_string(&payload).unwrap_or_default()));
                                    last_expression_id = Some(latest.id.clone());
                                }
                            }
                        }
                    }
                },
                Ok(event) = rx.recv() => {
                    match event {
                        shared::watchtower::CoreEvent::ProactiveTalk { message, .. } => {
                            yield Ok(Event::default().event("proactive_talk").data(message));
                        },
                        shared::watchtower::CoreEvent::PluginEvent { plugin_name, event_type, payload } => {
                            let data = serde_json::json!({
                                "plugin_name": plugin_name,
                                "event_type": event_type,
                                "payload": payload,
                            });
                            yield Ok(Event::default().event("plugin_event").data(data.to_string()));
                        },
                        _ => {} // Other events handled by polling above
                    }
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
