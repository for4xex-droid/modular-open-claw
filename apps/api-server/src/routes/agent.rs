/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::docker;
use crate::error::AppError;
use crate::skill_handler;
use crate::AppState;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::*;
use axum::{
    extract::State, http::HeaderMap, http::StatusCode, response::IntoResponse, response::Json,
};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use serial_test::serial;
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

// [AIOME-REFACTOR] core logic moved to agent_engine.rs for better performance (async I/O) and modularity.

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
    let reply = crate::agent_engine::AgentEngine::chat(
        &state,
        &payload.prompt,
        payload.channel_id.clone(),
        auth.agent_id,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "reply": reply
    })))
}

pub(crate) fn should_trigger_diagnostics(
    trajectory: &[aiome_core::trajectory::TrajectoryStep],
) -> bool {
    trajectory
        .iter()
        .any(|s| s.is_critical_failure || s.constraint_violations.iter().any(|v| v.severity >= 80))
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
    use crate::agent_engine::build_system_instructions;
    use crate::app_state::Component;
    use infrastructure::job_queue::UniversalJobQueue;
    use infrastructure::registry::{AssetManifest, AssetType, RegistryManager};
    use infrastructure::skills::WasmSkillManager;
    use serial_test::serial;
    use std::sync::Arc;

    async fn setup_test_state() -> (crate::AppState, tempfile::TempDir) {
        let tmp_dir = tempfile::TempDir::new().unwrap(); // allow-anti-pattern
        let db_path = tmp_dir.path().join("test_agent.db");
        let pool_url = format!("sqlite://{}", db_path.to_str().unwrap()); // allow-anti-pattern

        let pool = infrastructure::db::DatabasePool::new_sqlite(&pool_url).await.unwrap();
        let ts = std::sync::Arc::new(
            infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone())
        );
        let jq = Arc::new(UniversalJobQueue::new(pool.clone(), None, ts).await.unwrap()); // allow-anti-pattern
        let registry = Arc::new(RegistryManager::new(pool.clone()));

        // Setup WASM Skill Manager in a tmp dir
        let skills_dir = tmp_dir.path().join("skills");
        let sandbox_dir = tmp_dir.path().join("sandbox");
        std::fs::create_dir_all(&skills_dir).unwrap(); // allow-anti-pattern
        std::fs::create_dir_all(&sandbox_dir).unwrap(); // allow-anti-pattern

        let wsm = Arc::new(
            WasmSkillManager::new(skills_dir.to_str().unwrap(), sandbox_dir.to_str().unwrap()) // allow-anti-pattern
                .unwrap(), // allow-anti-pattern
        );

        let state = crate::AppState {
            registry: Component::new(registry),
            wasm_skill_manager: Component::new(wsm),
            job_queue: Component::new(jq),
            config: Component::new(Arc::new(shared::config::AiomeConfig::default())),
            project_rules_cache: Component::new(Arc::new(
                moka::future::Cache::builder()
                    .time_to_live(std::time::Duration::from_secs(30))
                    .build(),
            )),
            ..Default::default()
        };

        (state, tmp_dir)
    }

    #[serial]
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
            safety_level: aiome_core_contracts::contracts::ToolSafetyLevel::Safe,
            metadata: Some(serde_json::json!({
                "command": "node",
                "args": ["weather.js"]
            })),
        };
        state.registry.register_asset(mcp_manifest).await.unwrap(); // allow-anti-pattern

        // 2. Build instructions
        let instructions = build_system_instructions(
            &state,
            "no karma",
            None,
            Some("Aiome".to_string()),
            None,
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

    #[serial]
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
            safety_level: aiome_core_contracts::contracts::ToolSafetyLevel::Safe,
            metadata: Some(serde_json::json!({
                "command": "python",
                "args": ["search.py"]
            })),
        };
        state.registry.register_asset(mcp_manifest).await.unwrap(); // allow-anti-pattern

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

        let mut step3 = TrajectoryStep::default();
        step3.constraint_violations.push(ConstraintViolation {
            constraint_name: "ShadowModeHarness".into(),
            expected: "No violation".into(),
            actual: "Violation".into(),
            severity: 50, // Shadow mode severity (<80)
        });

        // This test will fail under the RED phase because currently ANY violation triggers diagnostics
        assert!(
            !should_trigger_diagnostics(&[step3]),
            "Shadow mode violation (severity 50) MUST NOT trigger diagnostics (VULN-03)"
        );
    }
}
