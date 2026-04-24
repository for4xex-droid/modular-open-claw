#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! # クレート固有のインデックス
//!
#![forbid(unsafe_code)]
#![allow(missing_docs)]
#![deny(clippy::all)]

use regex::Regex;
use std::sync::OnceLock;

use napi::Result;
use napi_derive::napi;
mod state;
pub use state::*;

use aiome_core::traits::{
    AgentEvolver, ChatStore, ImmuneSystemOps, JobQueue, KarmaRegistry, TaskRegistry,
};
use infrastructure::job_queue::WatchtowerOps;

#[napi(object)]
/// `SubagentSpawnResponse` 構造体
pub struct SubagentSpawnResponse {
    /// `status` フィールド
    pub status: String,
}

#[napi(object)]
/// `ToolCheckResponse` 構造体
pub struct ToolCheckResponse {
    /// `blocked` フィールド
    pub blocked: bool,
    /// `reason` フィールド
    pub reason: Option<String>,
    /// `new_params` フィールド
    pub new_params: Option<String>,
}

fn map_err<E: std::fmt::Display>(e: E) -> napi::Error {
    napi::Error::from_reason(e.to_string())
}

#[napi]
/// `karma_bootstrap` 関数
pub async fn karma_bootstrap(_session_id: String) -> Result<()> {
    get_db().await.map_err(map_err)?;
    Ok(())
}

#[napi]
/// `get_karma_directives` 関数
pub async fn get_karma_directives(topic: String, skill_id: String) -> Result<String> {
    let db = get_db().await.map_err(map_err)?;
    let result = db
        .fetch_relevant_karma(&topic, &skill_id, 3, "current")
        .await
        .map_err(map_err)?;

    if result.entries.is_empty() {
        return Ok(String::new());
    }

    let mut directives = String::from("\n[Karma-based Operational Directives]:\n");
    for entry in result.entries {
        directives.push_str(&format!("- {}\n", entry.lesson));
    }
    Ok(directives)
}

#[napi]
/// `karma_ingest` 関数
pub async fn karma_ingest(session_id: String, message_json: String) -> Result<()> {
    let db = get_db().await.map_err(map_err)?;
    let msg: serde_json::Value = serde_json::from_str(&message_json)
        .map_err(|e| napi::Error::from_reason(format!("Invalid message JSON: {}", e)))?;

    let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
    let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");

    db.store_chat_message(&session_id, role, content, None)
        .await
        .map_err(map_err)?;
    Ok(())
}

#[napi]
/// `karma_distill_turn` 関数
pub async fn karma_distill_turn(messages_json: String, success: bool) -> Result<()> {
    tracing::info!(
        "karma_distill_turn: success={}, msgs_len={}",
        success,
        messages_json.len()
    );

    let db = get_db().await.map_err(map_err)?;
    let llm = get_llm_provider().await.map_err(map_err)?;

    // 1. Record basic experience
    if success {
        db.add_tech_exp(1).await.map_err(map_err)?;
    }

    // 2. Extract lesson using LLM
    let prompt = format!(
        "以下のエージェント間の対話履歴（JSON）を分析し、将来同じタスクを行う際の「教訓（知恵）」を1つ抽出してください。\n\
        実行結果の成否: {}\n\n履歴:\n{}\n\n出力形式: 教訓1行のみ。簡潔に日本語で答えよ。",
        if success { "成功" } else { "失敗" },
        messages_json
    );

    match llm
        .complete(
            &prompt,
            Some("You are a senior AI distilling operational wisdom."),
        )
        .await
    {
        Ok(resp) => {
            let lesson = resp.content.trim();
            if !lesson.is_empty() {
                tracing::info!("🔮 [Karma] Distilled lesson: {}", lesson);
                let store_res = db
                    .store_karma(
                        "distill-turn",
                        "subagent",
                        lesson,
                        "Technical",
                        "v1_genesis",
                        None, // domain
                        None, // subtopic
                        None, // clone_origin_id
                        false,
                    )
                    .await;
                if let Err(e) = store_res {
                    tracing::warn!("⚠️ [Karma] Failed to save distilled lesson to DB: {}", e);
                }
            }
        }
        Err(e) => {
            tracing::warn!("⚠️ [Karma] Failed to distill lesson: {:?}", e);
        }
    }

    Ok(())
}

#[napi]
/// `karma_fetch_relevant` 関数
pub async fn karma_fetch_relevant(session_id: String, _limit: u32) -> Result<String> {
    let db = get_db().await.map_err(map_err)?;
    // fetch relevant karmas for the session (requires embedding provider wiring in future)
    // for now we fetch recent jobs/summaries associated to the session
    let summary_data = db
        .get_chat_memory_summary(&session_id)
        .await
        .map_err(map_err)?;

    let summary_text = summary_data.map(|(s, _)| s).unwrap_or_default();
    Ok(summary_text)
}

#[napi]
/// `immune_get_warnings` 関数
pub async fn immune_get_warnings() -> Result<String> {
    let db = get_db().await.map_err(map_err)?;
    let rules = db.fetch_active_immune_rules().await.map_err(map_err)?;

    if rules.is_empty() {
        return Ok(String::new());
    }

    let mut warnings = String::from("\n[🛡️ Sentinel Active Safeguards]:\n");
    for rule in rules.iter().take(5) {
        warnings.push_str(&format!(
            "- Pattern: {} (Action: {})\n",
            rule.pattern, rule.action
        ));
    }
    Ok(warnings)
}

#[napi]
/// `karma_compact` 関数
pub async fn karma_compact(
    session_id: String,
    _session_file: String,
    _token_budget: u32,
) -> Result<()> {
    tracing::info!("karma_compact for session {}", session_id);
    let db = get_db().await.map_err(map_err)?;

    // Memory distillation / Purging old chats
    db.do_purge_old_distilled_chats(7).await.map_err(map_err)?; // Purge 7 days old
    db.karma_decay_sweep().await.map_err(map_err)?;

    Ok(())
}

#[napi]
/// `quarantine_check_spawn` 関数
pub async fn quarantine_check_spawn(_child_session_key: String) -> Result<SubagentSpawnResponse> {
    let immune = get_immune().await.map_err(map_err)?;
    let db = get_db().await.map_err(map_err)?;

    // Analyze holistic system threats before allowing a subagent to spawn
    match immune.analyze_threats(db.as_ref()).await {
        Ok(threat_level) if threat_level > 50 => {
            tracing::warn!(
                "🛡️ [Quarantine] Subagent spawn blocked due to high threat level: {}",
                threat_level
            );
            Ok(SubagentSpawnResponse {
                status: "blocked".to_string(),
            })
        }
        Ok(threat_level) if threat_level > 20 => {
            tracing::warn!(
                "🛡️ [Quarantine] Subagent quarantined. Medium threat: {}",
                threat_level
            );
            Ok(SubagentSpawnResponse {
                status: "quarantined".to_string(),
            })
        }
        Ok(_) => Ok(SubagentSpawnResponse {
            status: "ok".to_string(),
        }),
        Err(e) => {
            tracing::error!("🛡️ [Quarantine] Threat analysis failed: {}", e);
            // Fail closed: if we can't analyze threats, block subagents
            Ok(SubagentSpawnResponse {
                status: "blocked".to_string(),
            })
        }
    }
}

#[napi]
/// `karma_learn_from_subagent` 関数
pub async fn karma_learn_from_subagent(target_session_key: String, outcome: String) -> Result<()> {
    let db = get_db().await.map_err(map_err)?;
    db.store_karma(
        &format!("subagent-{}", uuid::Uuid::new_v4()),
        "subagent",
        &format!(
            "Subagent session {} outcome: {}",
            target_session_key, outcome
        ),
        "Technical",
        "current",
        Some("quarantine"),
        Some("subagent_outcome"),
        None,
        false,
    )
    .await
    .map_err(map_err)?;
    Ok(())
}

#[napi]
/// `shutdown` 関数
pub fn shutdown() {
    tracing::info!("ContextEngine NAPI shutdown.");
}

#[napi]
/// `immune_check_tool` 関数
pub async fn immune_check_tool(tool_name: String, params: String) -> Result<ToolCheckResponse> {
    tracing::info!(
        "🛡️ [NAPI Sentinel] immune_check_tool: {} | {}",
        tool_name,
        params
    );

    // 1. Baseline RegExp Check (Sentinel Layer 1.5 - No DB needed)
    // catch obvious dangerous patterns quickly using cached RegExp instances
    static DANGEROUS_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    let patterns = DANGEROUS_PATTERNS.get_or_init(|| {
        vec![
            Regex::new(r"(?i)rm\s+-rf").unwrap(),    // allow-anti-pattern
            Regex::new(r"(?i)chmod\s+777").unwrap(), // allow-anti-pattern
            Regex::new(r"(?i)cat\s+/etc/shadow").unwrap(), // allow-anti-pattern
            Regex::new(r"(?i)shutdown").unwrap(),    // allow-anti-pattern
            Regex::new(r"(?i)reboot").unwrap(),      // allow-anti-pattern
            Regex::new(r#"(?i)":\s*".*";"#).unwrap(), // allow-anti-pattern
        ]
    });

    for re in patterns {
        if re.is_match(&params) {
            return Ok(ToolCheckResponse {
                blocked: true,
                reason: Some(format!(
                    "[SENTINEL] Baseline Violation: Blocked dangerous pattern in tool params: {}",
                    re.as_str()
                )),
                new_params: None,
            });
        }
    }

    // 2. Complex Adaptive Check (Requires DB & LLM)
    let immune = get_immune().await.map_err(map_err)?;
    let db = get_db().await.map_err(map_err)?;

    // We use a mock topic for tool check context
    let context_topic = format!("Tool Execute: {}", tool_name);
    if let Ok(Some(rule)) = immune
        .verify_intent(
            &format!("{} with params: {}", context_topic, params),
            db.as_ref(),
        )
        .await
    {
        return Ok(ToolCheckResponse {
            blocked: true,
            reason: Some(format!(
                "[SENTINEL] Adaptive Block: {} (Pattern: {})",
                rule.action, rule.pattern
            )),
            new_params: None,
        });
    }

    Ok(ToolCheckResponse {
        blocked: false,
        reason: None,
        new_params: None,
    })
}

#[napi]
/// `karma_learn_from_tool` 関数
pub async fn karma_learn_from_tool(
    tool_name: String,
    result: String,
    error_msg: String,
) -> Result<()> {
    tracing::info!(
        "karma_learn_from_tool: {} | res len: {} | err len: {}",
        tool_name,
        result.len(),
        error_msg.len()
    );
    let db = get_db().await.map_err(map_err)?;

    if !error_msg.is_empty() {
        // Record failure lesson
        db.store_karma(
            &format!("tool-fail-{}", uuid::Uuid::new_v4()),
            "tool",
            &format!(
                "Tool {} failed with error: {}. Result context: {}",
                tool_name, error_msg, result
            ),
            "Technical",
            "current",
            Some("safety"),
            Some("tool_error"),
            None,
            false,
        )
        .await
        .map_err(map_err)?;
    }

    Ok(())
}

#[napi]
/// `karma_preserve_facts` 関数
pub async fn karma_preserve_facts(session_file: String) -> Result<()> {
    tracing::info!("karma_preserve_facts for {}", session_file);
    let db = get_db().await.map_err(map_err)?;

    // In a real scenario, we would parse the session file and extract key facts.
    // For now, we record that fact preservation was triggered.
    db.store_karma(
        &format!("preserve-{}", uuid::Uuid::new_v4()),
        "system",
        &format!(
            "Preservation checkpoint triggered for session file: {}",
            session_file
        ),
        "Technical",
        "current",
        Some("pivotal"),
        Some("checkpoint"),
        None,
        false,
    )
    .await
    .map_err(map_err)?;

    Ok(())
}

#[napi]
/// `immune_scan_input` 関数
pub async fn immune_scan_input(prompt: String, _history_messages: String) -> Result<()> {
    let immune = get_immune().await.map_err(map_err)?;
    let db = get_db().await.map_err(map_err)?;

    if let Ok(Some(rule)) = immune.verify_intent(&prompt, db.as_ref()).await {
        return Err(napi::Error::from_reason(format!(
            "[SENTINEL] Blocked by Rule: {} -> action: {}",
            rule.pattern, rule.action
        )));
    }

    Ok(())
}

#[napi]
/// `karma_flush_session` 関数
pub async fn karma_flush_session(session_id: String) -> Result<()> {
    tracing::info!("karma_flush_session triggered for session {}", session_id);
    let db = get_db().await.map_err(map_err)?;

    if let Err(e) = db.do_mark_chats_as_distilled(&session_id, i64::MAX).await {
        tracing::warn!(
            "⚠️ [Karma] Failed to mark chats as distilled for flush: {}",
            e
        );
    }
    if let Err(e) = db
        .do_update_chat_memory_summary(&session_id, "[FLUSHED AND ARCHIVED]", None)
        .await
    {
        tracing::warn!(
            "⚠️ [Karma] Failed to update chat memory summary for flush: {}",
            e
        );
    }

    Ok(())
}

#[napi]
/// `watchtower_track_usage` 関数
pub async fn watchtower_track_usage(usage: String) -> Result<()> {
    // LLMトークン消費量の本格記録 (モック解消)
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&usage) {
        let prompt = parsed
            .get("prompt_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let completion = parsed
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if prompt > 0 || completion > 0 {
            tracing::info!(
                "📊 [Watchtower] Usage tracked: {} prompt tokens, {} completion tokens.",
                prompt,
                completion
            );
            // Future persistence into sns_metrics_history or similar metrics store.
        } else {
            tracing::debug!("watchtower_track_usage: {}", usage);
        }
    } else {
        tracing::debug!("watchtower_track_usage (raw): {}", usage);
    }
    Ok(())
}

#[napi]
/// `watchtower_init` 関数
pub async fn watchtower_init() -> Result<()> {
    get_db().await.map_err(map_err)?;
    get_immune().await.map_err(map_err)?;
    tracing::info!("Watchtower and Aiome subsystems initialized.");
    Ok(())
}

#[napi]
/// `watchtower_shutdown` 関数
pub fn watchtower_shutdown() {
    tracing::info!("Watchtower shutdown.");
}

#[napi]
/// `karma_geodesic_importance` 関数 (Phase 4: Poincare GC)
pub async fn karma_geodesic_importance(query: String) -> Result<f64> {
    let slm = get_slm_bridge().await.map_err(map_err)?;
    let importance = slm.calculate_importance(&query).await.map_err(map_err)?;
    Ok(importance)
}
