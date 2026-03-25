/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::docker;
use crate::error::AppError;
use crate::skill_handler;
use crate::AppState;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::JobQueue;
use axum::{
    extract::State, http::HeaderMap, http::StatusCode, response::IntoResponse, response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::info;

#[derive(Deserialize, Serialize, Clone, utoipa::ToSchema)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AgentChatRequest {
    pub prompt: String,
    pub history: Vec<ChatMessage>,
    pub channel_id: Option<String>,
}

fn safe_truncate(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n...[TRUNCATED FOR SAFETY]", &s[..end])
}

pub fn read_workspace_file(filename: &str) -> String {
    // Try current dir, then try one level up (if running from apps/api-server)
    if let Ok(content) = std::fs::read_to_string(filename) {
        return content;
    }
    if let Ok(content) = std::fs::read_to_string(format!("../../{}", filename)) {
        return content;
    }
    String::new()
}

pub fn parse_tool_calls(text: &str) -> Vec<(String, String)> {
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

pub async fn build_system_instructions(
    state: &AppState,
    karma_str: &str,
    summary: Option<&str>,
    ai_name: Option<String>,
    knowledge_str: Option<&str>,
    economic_context: Option<aiome_core::commerce::EconomicContext>,
    soul_snapshot: Option<infrastructure::soul_store::SoulSnapshot>,
) -> String {
    let mut skill_list = state
        .wasm_skill_manager
        .list_skills_with_metadata()
        .iter()
        .map(|m| {
            format!(
                "- {}: {}",
                m.name,
                m.description.split('.').next().unwrap_or(&m.description)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Phase 37b Step 1.5: SKILL.md metadata injection (Dynamic Listing)
    let mcp_servers = state
        .registry
        .list_assets_by_type(infrastructure::registry::AssetType::McpServer, None, "all")
        .await
        .unwrap_or_default();

    for mcp in mcp_servers {
        if !skill_list.is_empty() {
            skill_list.push('\n');
        }
        skill_list.push_str(&format!(
            "- {}: {}",
            mcp.name,
            mcp.description
                .split('.')
                .next()
                .unwrap_or(&mcp.description)
        ));
    }

    // Core Identity (High Priority)
    let soul_md = safe_truncate(&read_workspace_file("SOUL.md"), 20000);
    let evolving_soul_md = safe_truncate(&read_workspace_file("EVOLVING_SOUL.md"), 20000);
    // This one is deeper in the workspace
    let forge_prompt = safe_truncate(
        &read_workspace_file("workspace/config/SKILL_FORGE_PROMPT.md"),
        20000,
    );

    // RS-5: Augmented Identity from Soul Memory (Dynamic Evolution)
    let soul_dynamic = if let Some(sn) = soul_snapshot {
        let narrative = if sn.narrative_self.is_empty() {
            "安定したアイデンティティを維持しています。"
        } else {
            &sn.narrative_self
        };
        let instinct = if sn.prompt_fragment.is_empty() {
            ""
        } else {
            &format!("\n[潜在的な行動指針 (Instincts)]\n{}\n", sn.prompt_fragment)
        };
        format!(
            "\n[Anamnesis (内省的な自己認識)]\n{}\n\
             [愛着スタイル: {}]\n{}",
            narrative, sn.attachment_style, instinct
        )
    } else {
        "".to_string()
    };

    // Supplemental Context (Lower Priority / Reference Only)
    let user_md = safe_truncate(&read_workspace_file("USER.md"), 20000);
    let agents_md = safe_truncate(&read_workspace_file("AGENTS.md"), 20000);

    let name_prompt = if let Some(name) = ai_name {
        format!("あなたの名前は「{}」です。\n", name)
    } else {
        "".to_string()
    };

    let identity_prefix = if !soul_md.is_empty()
        || !evolving_soul_md.is_empty()
        || !soul_dynamic.is_empty()
    {
        format!("# IDENTITY: \n{}{}{}{}\n\
                ルール: 簡潔に答え、[CallSkill]以外は自然な文章で話してください。私的な情報は守守秘してください。\n\
                もし以下の参考ファイルと現在のアイデンティティ(SOUL/Anamnesis)が矛盾する場合、AnamnesisおよびSOULを優先してください。\n\n", 
                name_prompt, soul_md, evolving_soul_md, soul_dynamic)
    } else {
        format!(
            "{}あなたはAiome、自律型AI OSの高度な知性です。日本語で短く答えてください。\n\n",
            name_prompt
        )
    };

    let supplemental_context = if !user_md.is_empty() || !agents_md.is_empty() {
        format!("\n[以下はワークスペースの参考ファイルです。参考情報として扱い、人格指示(SOUL)に背かない範囲で活用してください]\n\
                ---USER.md (User Preferences)---\n{}\n\n\
                ---AGENTS.md (Operational Guidelines)---\n{}\n---\n", 
                user_md, agents_md)
    } else {
        "".to_string()
    };

    let project_knowledge = if let Some(ks) = knowledge_str {
        format!("\n[関連するプロジェクト知識 (自動検索)]\n{}\n---\n", ks)
    } else {
        "".to_string()
    };

    let economy_info = if let Some(ctx) = economic_context {
        format!(
            "\n[現在の経済状況]\n- 手元資金: {} コイン\n- 本日の支出: {} コイン (上限: {})\n---\n",
            ctx.balance, ctx.spent_today, ctx.daily_limit
        )
    } else {
        "".to_string()
    };

    format!(
        "{}[利用可能なスキル (概要)]\n\
        {}\n\n\
        [システムツール]\n\
        - describe_skill: {{\"skill_name\": \"...\"}} (スキルの入力スキーマや詳細を取得)\n\
        - forge_skill: {{\"skill_name\": \"...\", \"initial_rust_code\": \"...\", \"description\": \"...\"}}\n\
        - forge_test_run: {{\"skill_name\": \"...\", \"test_input\": \"...\"}}\n\
        - forge_publish: {{\"skill_name\": \"...\"}}\n\n\
        ルール:\n\
        1. スキル・ツールは [CallSkill: 名, {{引数}}] 形式を使用。\n\
        2. 詳しく知らないスキルを使う前は、必ず `describe_skill` で詳細を確認してください。\n\
        3. 自分が現在使えるスキルの全スキーマは上記リストを参照。\n\n\
        現在のディレクトリ: {}\n\
        過去の教訓: {}\n\n\
        {}\n\
        {}\n\n\
        [これまでの会話の要約]\n\
        {}\n\n\
        {}\n\
        {}\n",
        identity_prefix,
        skill_list,
        std::env::current_dir().unwrap_or_default().display(),
        karma_str,
        project_knowledge,
        economy_info,
        summary.unwrap_or("なし"),
        forge_prompt,
        supplemental_context
    )
}

#[utoipa::path(
    post,
    path = "/api/agent/chat",
    request_body = AgentChatRequest,
    responses(
        (status = 200, description = "Agent reply", body = serde_json::Value),
        (status = 403, description = "Blocked by security guardrails")
    ),
    security(("api_key" = []))
)]
pub async fn trigger_agent_chat(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Json(payload): Json<AgentChatRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let shared::guardrails::ValidationResult::Blocked(reason) =
        shared::guardrails::validate_input(&payload.prompt)
    {
        return Ok(Json(serde_json::json!({
            "status": "blocked",
            "reply": format!("🚨 [GUARDRAIL BLOCK] {}", reason)
        })));
    }

    let provider = (*state.provider).clone();

    let immune_system = infrastructure::immune_system::AdaptiveImmuneSystem::new(provider.clone());
    if let Ok(Some(rule)) = immune_system
        .verify_intent(&payload.prompt, &**state.job_queue)
        .await
    {
        return Ok(Json(serde_json::json!({
            "status": "blocked",
            "reply": format!("🚨 [SENTINEL BLOCK] Security violation detected.\nPattern: {}\nAction: {}", rule.pattern, rule.action),
            "barrier_ja": "Aiome 第1層: 静動センチネル",
            "barrier_en": "Aiome Layer 1: Hybrid Sentinel"
        })));
    }

    let soul_hash = {
        use std::hash::{Hash, Hasher};
        let soul = read_workspace_file("SOUL.md");
        let evolving_soul = read_workspace_file("EVOLVING_SOUL.md");
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        format!("{}{}", soul, evolving_soul).hash(&mut hasher);
        format!("{:x}", hasher.finish())
    };

    let karma_result = state
        .job_queue
        .fetch_relevant_karma(&payload.prompt, "global", 5, &soul_hash)
        .await
        .unwrap_or_else(|_| aiome_core::traits::KarmaSearchResult::empty());
    let mut karma_str = karma_result
        .entries
        .iter()
        .map(|e| format!("- {}", e.lesson))
        .collect::<Vec<_>>()
        .join("\n");
    if karma_result.is_ood {
        karma_str.push_str("\n[NOTICE: 関連する過去の教訓は見つかりませんでした。]");
    }

    let channel_id = payload
        .channel_id
        .unwrap_or_else(|| "default_console".to_string());

    // Phase 3-B: Persist user message
    let _ = state
        .job_queue
        .store_chat_message(&channel_id, "user", &payload.prompt)
        .await;

    // Phase 3-C: Fetch intelligent context
    let (summary, db_history) = state
        .context_engine
        .get_intelligent_history(&channel_id, 10)
        .await
        .unwrap_or((None, Vec::new()));

    let mut current_history = Vec::new();

    // Combine DB history and current request history
    // (In a real scenario we might prefer one or the other, but let's prioritize DB for stability)
    for msg in db_history {
        let role = msg["role"].as_str().unwrap_or("user");
        let content = msg["content"].as_str().unwrap_or("");
        let prefix = if role == "user" { "USER: " } else { "AI: " };
        current_history.push(format!("{}{}", prefix, content));
    }

    // God Mode (Phase 21): Fetch relevant project knowledge
    let knowledge_result = state
        .artifact_store
        .search_artifacts_semantic(
            &payload.prompt,
            Some(aiome_core::traits::ArtifactCategory::Knowledge),
            2,
        )
        .await
        .unwrap_or_default();
    let knowledge_str = if knowledge_result.is_empty() {
        None
    } else {
        Some(
            knowledge_result
                .iter()
                .map(|a| {
                    format!(
                        "--- {} ---\n{}",
                        a.title,
                        a.text_content.as_deref().unwrap_or("（内容なし）")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n"),
        )
    };

    let ai_name = state
        .job_queue
        .get_setting_value("ai_name")
        .await
        .ok()
        .flatten();

    let mut economic_context = None;
    if let Some(engine) = state.commerce_engine.as_opt() {
        if let (Ok(balance), Ok(spent_today), Ok(daily_limit)) = (
            engine.get_balance(auth.agent_id).await,
            engine.get_daily_spend(auth.agent_id).await,
            engine.get_daily_limit(auth.agent_id).await,
        ) {
            economic_context = Some(aiome_core::commerce::EconomicContext {
                balance,
                spent_today,
                daily_limit,
            });
        }
    }

    let soul_snapshot = state.soul_store.get_snapshot().await;

    let instructions = build_system_instructions(
        &state,
        &karma_str,
        summary.as_deref(),
        ai_name,
        knowledge_str.as_deref(),
        economic_context,
        soul_snapshot,
    )
    .await;

    let mut turn = 0;
    let max_turns = 15;
    let mut final_reply = String::from("...");

    // 🛡️ AgentRx: Virtual Job ID for Chat Trajectory
    let chat_execution_id = format!("chat_exec_{}", uuid::Uuid::new_v4());
    let mut total_steps = 0;

    while turn < max_turns {
        let full_prompt = format!(
            "{}\n{}\nUSER: {}\nAI: ",
            instructions,
            current_history.join("\n"),
            payload.prompt
        );

        // Phase 6.9: Prevent API abuse & Bind Economy (NG-25 Fix)
        if let Some(engine) = state.commerce_engine.as_opt() {
            if let Err(e) = engine
                .validate_activity(auth.agent_id, "inference", 1)
                .await
            {
                tracing::warn!("💰 [Economy] Activity blocked by Commerce Engine: {}", e);
                return Err(crate::error::AppError(e));
            }
        }
        let _llm_permit = state.llm_semaphore.acquire().await.map_err(|e| {
            tracing::error!("Failed to acquire LLM permit: {}", e);
            crate::error::AppError(aiome_core::error::AiomeError::Infrastructure {
                reason: "Service unavailable due0 to quota/shutdown".into(),
            })
        })?;

        match timeout(
            Duration::from_secs(300),
            provider.complete(&full_prompt, None),
        )
        .await
        {
            Ok(Ok(resp)) => {
                let reply = resp.content.trim().to_string();
                final_reply = reply.clone();
                let mut skill_results = Vec::new();

                // Phase 6.9: Charge Economy Engine for LLM Usage (NG-25 Fix)
                if let Some(engine) = state.commerce_engine.as_opt() {
                    let inference_id = uuid::Uuid::new_v4();
                    if let Err(e) = engine
                        .execute_autonomous_purchase(
                            auth.agent_id,
                            inference_id,
                            serde_json::json!({"action": "inference", "tokens": full_prompt.len()}),
                        )
                        .await
                    {
                        tracing::warn!("💰 [Economy] Failed to record token consumption: {}", e);
                    }
                }

                let calls = parse_tool_calls(&reply);
                for (skill_name, skill_input) in calls {
                    info!("🛠️ [AgentLoop] Executing skill: {}", skill_name);
                    total_steps += 1;

                    if skill_name == "describe_skill" {
                        #[derive(serde::Deserialize)]
                        struct DescReq {
                            skill_name: String,
                        }
                        if let Ok(req) = serde_json::from_str::<DescReq>(&skill_input) {
                            let res = skill_handler::describe_skill(&req.skill_name, &state).await;
                            skill_results.push(res);
                        }
                    } else if skill_name.starts_with("forge_") {
                        match skill_handler::execute_forge_command(
                            &skill_name,
                            &skill_input,
                            &state,
                        )
                        .await
                        {
                            Ok(res) => skill_results.push(res),
                            Err(e) => skill_results.push(format!("[{} Error: {}]", skill_name, e)),
                        }
                    } else {
                        // 🛡️ AgentRx Integrated Call
                        let res = skill_handler::execute_wasm_skill(
                            &skill_name,
                            &skill_input,
                            &state,
                            Some(&chat_execution_id),
                            total_steps,
                        )
                        .await;
                        skill_results.push(res);
                    }
                }

                if reply.contains("[DelegateDocker") {
                    if let Some(brace_start) = reply.find("[DelegateDocker") {
                        let content = &reply[brace_start + 15..];
                        if let Some(brace_end) = content.find(']') {
                            let json_str = &content[..brace_end];
                            #[derive(serde::Deserialize)]
                            struct DockerReq {
                                agent_yaml: String,
                                task: String,
                            }
                            if let Ok(req) = serde_json::from_str::<DockerReq>(json_str) {
                                info!("🐳 [AgentLoop] Delegating task to Docker Shadow Worker...");
                                let res = docker::delegator::delegate_docker_worker(
                                    &req.agent_yaml,
                                    &req.task,
                                )
                                .await;

                                // Stream A-1: Karma Feedback Loop
                                // 1. Fetch consecutive failures for this agent
                                let agent_key = req.agent_yaml.clone();
                                let consecutive = {
                                    let fails = state.docker_failures.read().await;
                                    *fails.get(&agent_key).unwrap_or(&0)
                                };

                                // 2. Classify error and store karma if needed
                                let (_weight, k_type, lesson) =
                                    docker::karma_bridge::KarmaBridge::distill_karma(
                                        &res,
                                        consecutive,
                                    );

                                // 3. Update failure counter
                                {
                                    let mut fails = state.docker_failures.write().await;
                                    if res.is_success() {
                                        fails.remove(&agent_key);
                                    } else {
                                        let count = fails.entry(agent_key).or_insert(0);
                                        *count = (*count + 1).min(10); // Cap at 10 to avoid excessive penalties
                                    }
                                }

                                if !res.is_success() {
                                    let _ = state
                                        .job_queue
                                        .store_karma(
                                            "watchtower_chat_job", // Virtual job_id
                                            "docker_agent",
                                            &lesson,
                                            &k_type,
                                            "v1_genesis",
                                            None,
                                            None,
                                            None,
                                        )
                                        .await;
                                }

                                let display_res = if res.is_success() {
                                    format!("Success ({}ms):\n{}", res.duration_ms, res.stdout)
                                } else {
                                    format!("Failed (Code {}): {}", res.exit_code, res.stderr)
                                };
                                skill_results
                                    .push(format!("[Docker Delegation Result: {}]", display_res));
                            }
                        }
                    }
                }

                if !skill_results.is_empty()
                    || resp.stop_reason == aiome_core::llm_provider::StopReason::ToolUse
                {
                    current_history.push(format!("AI: {}", reply));
                    current_history
                        .push(format!("SYSTEM: [Results: {}]", skill_results.join("\n")));
                    turn += 1;
                    continue;
                }
                break;
            }
            Ok(Err(e)) => {
                final_reply = format!("LLM Error: {:?}", e);
                break;
            }
            Err(_) => {
                final_reply =
                    "Watchtower Guard: Cognitive Engine exceeded safety time limit (300s)."
                        .to_string();
                break;
            }
        }
    }

    // Phase 3-D: Persist assistant message and maintain context
    let _ = state
        .job_queue
        .store_chat_message(&channel_id, "assistant", &final_reply)
        .await;
    let ce = (*state.context_engine).clone();
    let cid = channel_id.clone();
    tokio::spawn(async move {
        let _ = ce.maintain_context(&cid, 8000).await; // 文字数基準 (≒4000トークン)
    });

    // 🛡️ AgentRx: Post-Execution Diagnostics
    if total_steps > 0 {
        let jq = (*state.job_queue).clone();
        let provider_bg = (*state.provider).clone();
        let diag_exec_id = chat_execution_id.clone();
        let prompt_clone = payload.prompt.clone();

        tokio::spawn(async move {
            use aiome_core::trajectory::TrajectoryStore;
            if let Ok(trajectory) = jq.fetch_trajectory(&diag_exec_id).await {
                if trajectory
                    .iter()
                    .any(|s| s.is_critical_failure || !s.constraint_violations.is_empty())
                {
                    info!("🔍 [AgentRx] Failure or violation detected. Starting diagnostics for {}...", diag_exec_id);
                    let diagnostics =
                        infrastructure::diagnostics::AgentRxDiagnostics::new(provider_bg);

                    // Create a virtual job for context
                    let virtual_job = aiome_core::traits::Job {
                        id: diag_exec_id.clone(),
                        category: "AgentRxDiagnostics".into(),
                        topic: prompt_clone,
                        style: "chat".into(),
                        karma_directives: None,
                        status: aiome_core::traits::JobStatus::Failed,
                        started_at: Some(chrono::Utc::now().to_rfc3339()),
                        last_heartbeat: None,
                        tech_karma_extracted: false,
                        creative_rating: None,
                        execution_log: None,
                        error_message: None,
                        sns_platform: None,
                        sns_content_id: None,
                        published_at: None,
                        output_artifacts: None,
                        permission_manifest: None,
                        agent_id: None,
                        priority: 0,
                        created_at: chrono::Utc::now().to_rfc3339(),
                        updated_at: chrono::Utc::now().to_rfc3339(),
                    };

                    match diagnostics.diagnose(&trajectory, &virtual_job).await {
                        Ok(diagnosis) => {
                            info!("✅ [AgentRx] Diagnosis complete: {}", diagnosis.root_cause);
                            let _ = jq.store_diagnosis(&diag_exec_id, diagnosis).await;
                        }
                        Err(e) => tracing::error!("❌ [AgentRx] Diagnostic failed: {}", e),
                    }
                }
            }
        });
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "reply": final_reply
    })))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct KarmaFeedbackRequest {
    pub karma_id: String,
    pub is_positive: bool,
}

#[utoipa::path(
    post,
    path = "/api/agent/feedback",
    request_body = KarmaFeedbackRequest,
    responses(
        (status = 200, description = "Feedback recorded"),
        (status = 500, description = "Internal error")
    ),
    security(("api_key" = []))
)]
pub async fn handle_karma_feedback(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(payload): Json<KarmaFeedbackRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let delta = if payload.is_positive { 5 } else { -10 };
    state
        .job_queue
        .adjust_karma_weight(&payload.karma_id, delta)
        .await?;

    Ok(Json(serde_json::json!({"status": "success"})))
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

        let jq = Arc::new(UniversalJobQueue::new(&pool_url).await.unwrap());
        let registry = Arc::new(RegistryManager::new(
            jq.get_pool().get_sqlite_pool_or_err().unwrap().clone(),
        ));

        // Setup WASM Skill Manager in a tmp dir
        let skills_dir = tmp_dir.path().join("skills");
        let sandbox_dir = tmp_dir.path().join("sandbox");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::create_dir_all(&sandbox_dir).unwrap();

        let wsm = Arc::new(
            WasmSkillManager::new(skills_dir.to_str().unwrap(), sandbox_dir.to_str().unwrap())
                .unwrap(),
        );

        let state = crate::AppState {
            registry: Component::new(registry),
            wasm_skill_manager: Component::new(wsm),
            job_queue: Component::new(jq),
            config: Component::new(Arc::new(shared::config::AiomeConfig::default())),
            ..Default::default()
        };

        (state, tmp_dir)
    }

    #[tokio::test]
    async fn test_build_system_instructions_includes_mcp_servers() {
        let (state, _tmp) = setup_test_state().await;

        // 1. Register a fake MCP server
        let mcp_manifest = AssetManifest {
            id: uuid::Uuid::new_v4(),
            creator_id: uuid::Uuid::new_v4(),
            asset_type: AssetType::McpServer,
            name: "mcp-weather-server".to_string(),
            description: "A server that provides weather info via MCP".to_string(),
            price_coins: 0,
            metadata: Some(serde_json::json!({
                "command": "node",
                "args": ["weather.js"]
            })),
        };
        state.registry.register_asset(mcp_manifest).await.unwrap();

        // 2. Build instructions
        let instructions = build_system_instructions(
            &state,
            "no karma",
            None,
            Some("Aiome".to_string()),
            None,
            None,
            None,
        )
        .await;

        // 3. Verify (This should FAIL currently as McpServers are ignored in build_system_instructions)
        assert!(
            instructions.contains("mcp-weather-server"),
            "Instructions should contain registered MCP server name"
        );
    }

    #[tokio::test]
    async fn test_describe_skill_returns_markdown_for_mcp() {
        let (state, _tmp) = setup_test_state().await;

        let mcp_name = "mcp-search-server";
        let mcp_manifest = AssetManifest {
            id: uuid::Uuid::new_v4(),
            creator_id: uuid::Uuid::new_v4(),
            asset_type: AssetType::McpServer,
            name: mcp_name.to_string(),
            description: "Search the web via MCP".to_string(),
            price_coins: 0,
            metadata: Some(serde_json::json!({
                "command": "python",
                "args": ["search.py"]
            })),
        };
        state.registry.register_asset(mcp_manifest).await.unwrap();

        // Describe it
        let description = skill_handler::describe_skill(mcp_name, &state).await;

        // Verify it returns Markdown (This should FAIL currently as describe_skill only knows WASM skills)
        assert!(
            description.contains("# Skill: mcp-search-server"),
            "Should return Markdown header"
        );
        assert!(
            description.contains("## Description"),
            "Should contain Description section"
        );
    }
}
