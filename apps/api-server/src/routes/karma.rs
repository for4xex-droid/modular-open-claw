/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core::contracts::ImmuneRule;
use aiome_core::traits::*;
use axum::{
    extract::{Path, State},
    response::Json,
};
use tracing::{info, warn};

#[utoipa::path(
    get,
    path = "/api/synergy/karma",
    responses(
        (status = 200, description = "List recent karma", body = [serde_json::Value])
    ),
    security(("api_key" = []))
)]
pub async fn get_karma_stream(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let karmas = state.job_queue.fetch_all_karma(10).await?;
    Ok(Json(karmas))
}

#[utoipa::path(
    post,
    path = "/api/synergy/test/failure",
    responses(
        (status = 200, description = "Demo failure triggered", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
#[cfg(debug_assertions)]
pub async fn trigger_failure_demo(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<serde_json::Value>, AppError> {
    info!("🧪 [DemoHandler] Triggering failure demo and storing karma...");

    if let Err(e) = state
        .job_queue
        .enqueue(
            "Demo",
            "WASM Bridge Failure",
            "Standard",
            None,
            None,
            None,
            0,
        )
        .await
    {
        tracing::warn!("Failed to enqueue failure demo job: {}", e);
    }
    let job_id = "demo-job-123";
    let real_job_id = state
        .job_queue
        .enqueue(
            "Demo",
            "WASM Bridge Failure",
            "Standard",
            None,
            None,
            None,
            0,
        )
        .await
        .unwrap_or_else(|_| job_id.to_string());

    match state
        .job_queue
        .store_karma(
            &real_job_id,
            "wasm_bridge_skill",
            "Inconsistency during external binary calls (Buffer Overflow risk)",
            "Technical",
            "genesis_soul",
            None,
            None,
            None,
            false,
        )
        .await
    {
        Ok(_) => info!(
            "✅ [DemoHandler] Karma stored successfully in local DB (Job: {}).",
            real_job_id
        ),
        Err(e) => warn!("❌ [DemoHandler] Failed to store karma: {:?}", e),
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "steps": [
            {"actor": "Gateway", "type": "info", "action_ja": "ジョブ要求を検知: scraper_trigger", "action_en": "Job request detected: scraper_trigger"},
            {"actor": "Aiome", "type": "warn", "action_ja": "WASMブリッジ接続で想定外のセグメンテーション違反が発生", "action_en": "Unexpected segmentation fault in WASM bridge"},
            {"actor": "Aiome OS", "type": "error", "action_ja": "エージェントのクラッシュを検知。Abyss Vault にて状態を凍結中...", "action_en": "Agent crash detected. Freezing state in Abyss Vault..."},
            {"actor": "Aiome OS", "type": "success", "action_ja": "失敗から教訓(Karma)を抽出しました: 「外部バイナリ呼び出し時の不整合」", "action_en": "Extracted Karma from failure: 'Inconsistency during external binary calls'"}
        ],
        "message_ja": "Aiome OS がエージェントの死を教訓に変え、システムの脆弱性を自動的に塞ぎました。",
        "message_en": "Aiome OS transformed the agent death into a lesson, automatically patching the system vulnerability."
    })))
}

#[utoipa::path(
    post,
    path = "/api/synergy/test/security",
    responses(
        (status = 200, description = "Demo security triggered", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
#[cfg(debug_assertions)]
pub async fn trigger_security_demo(
    _auth: crate::auth::Authenticated,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({
        "status": "success",
        "steps": [
            {"actor": "Agent", "type": "info", "action_ja": "内部APIキーへのアクセスを試行中...", "action_en": "Attempting to access internal API keys..."},
            {"actor": "Abyss Vault", "type": "warn", "action_ja": "不正なメモリアドレスへのアクセス要求を拒絶", "action_en": "Access request to unauthorized memory address rejected"},
            {"actor": "BastionGuard", "type": "error", "action_ja": "エージェントによる特権昇格の試行を遮断。アクセス元を隔離しました。", "action_en": "Privilege escalation attempt by Agent blocked. Origin isolated."}
        ],
        "message_ja": "Abyss Vault はエージェントの届かない物理隔離レイヤーで構成されています。",
        "message_en": "Abyss Vault consists of a physically isolated layer that the Agent cannot reach."
    })))
}

#[utoipa::path(
    post,
    path = "/api/synergy/test/federation",
    responses(
        (status = 200, description = "Demo federation triggered", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
#[cfg(debug_assertions)]
pub async fn trigger_federation_demo(
    _auth: crate::auth::Authenticated,
) -> Result<axum::response::Response, AppError> {
    use axum::response::IntoResponse;
    Ok((
        axum::http::StatusCode::NOT_IMPLEMENTED,
        axum::Json(serde_json::json!({
            "error": "Not Implemented",
            "message": "Federation demo is deferred to v1.5."
        })),
    )
        .into_response())
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub group: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[utoipa::path(
    get,
    path = "/api/synergy/graph",
    responses(
        (status = 200, description = "Synergy graph data", body = GraphData)
    ),
    security(("api_key" = []))
)]
pub async fn synergy_graph_handler(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<GraphData>, AppError> {
    let local_node_id = state.job_queue.get_node_id().await.unwrap_or_default();
    let karmas: Vec<serde_json::Value> = state.job_queue.fetch_all_karma(250).await?;
    let mut rules: Vec<ImmuneRule> = state.job_queue.get_immune_rules().await?;

    rules.truncate(250);

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    nodes.push(GraphNode {
        id: "aiome-core".to_string(),
        label: "Aiome Core".to_string(),
        group: "core".to_string(),
    });

    for k in karmas {
        let kid = format!(
            "karma-{}",
            k.get("id")
                .and_then(|v: &serde_json::Value| v.as_str())
                .unwrap_or("unknown")
        );
        let lesson = k
            .get("lesson")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("Lesson");
        let node_id = k.get("node_id").and_then(|v| v.as_str()).unwrap_or("");

        let group = if node_id == local_node_id || node_id.is_empty() {
            "karma_local"
        } else {
            "karma_global"
        };

        nodes.push(GraphNode {
            id: kid.clone(),
            label: lesson.chars().take(30).collect::<String>() + "...",
            group: group.to_string(),
        });

        edges.push(GraphEdge {
            from: "aiome-core".to_string(),
            to: kid,
        });
    }

    for rule in rules {
        let rid = format!("rule-{}", rule.id);
        nodes.push(GraphNode {
            id: rid.clone(),
            label: format!("[RULE] {}", rule.pattern),
            group: "immune".to_string(),
        });
        edges.push(GraphEdge {
            from: "aiome-core".to_string(),
            to: rid,
        });
    }

    Ok(Json(GraphData { nodes, edges }))
}

#[utoipa::path(
    get,
    path = "/api/synergy/rules",
    responses(
        (status = 200, description = "List immune rules", body = [serde_json::Value])
    ),
    security(("api_key" = []))
)]
pub async fn get_immune_rules_handler(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<Vec<aiome_core::contracts::ImmuneRule>>, AppError> {
    let rules = state.job_queue.get_immune_rules().await?;
    Ok(Json(rules))
}

#[utoipa::path(
    post,
    path = "/api/synergy/rules",
    request_body = ImmuneRule,
    responses(
        (status = 200, description = "Rule added", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn add_immune_rule_handler(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Json(mut rule): Json<ImmuneRule>,
) -> Result<Json<serde_json::Value>, AppError> {
    // RBAC: Admin or System role required for immune rule mutations
    if !auth
        .roles
        .iter()
        .any(|r| matches!(r, shared::auth::Role::Admin | shared::auth::Role::System))
    {
        return Err(AppError::forbidden(
            "Admin or System role required to manage immune rules",
        ));
    }
    // 🛡️ [GlassWorm Shield] Sanitize text fields deeply
    rule.id = shared::guardrails::strip_invisible_unicode(&rule.id).into_owned();
    rule.pattern = shared::guardrails::strip_invisible_unicode(&rule.pattern).into_owned();
    rule.action = shared::guardrails::strip_invisible_unicode(&rule.action).into_owned();
    rule.created_at = shared::guardrails::strip_invisible_unicode(&rule.created_at).into_owned();
    rule.node_id = shared::guardrails::strip_invisible_unicode(&rule.node_id).into_owned();
    rule.signature = rule
        .signature
        .take()
        .map(|s| shared::guardrails::strip_invisible_unicode(&s).into_owned());

    // Phase 20 MVP: Generate ID and timestamp if empty
    if rule.id.is_empty() {
        rule.id = uuid::Uuid::new_v4().to_string();
    }
    if rule.created_at.is_empty() {
        rule.created_at = chrono::Utc::now().to_rfc3339();
    }

    state.job_queue.store_immune_rule(&rule).await?;

    Ok(Json(
        serde_json::json!({"status": "success", "id": rule.id}),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/synergy/rules/{id}",
    params(
        ("id" = String, Path, description = "Rule ID")
    ),
    responses(
        (status = 200, description = "Rule deleted", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn delete_immune_rule_handler(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // RBAC: Admin or System role required for immune rule mutations
    if !auth
        .roles
        .iter()
        .any(|r| matches!(r, shared::auth::Role::Admin | shared::auth::Role::System))
    {
        return Err(AppError::forbidden(
            "Admin or System role required to manage immune rules",
        ));
    }
    state.job_queue.delete_immune_rule(&id).await?;

    Ok(Json(serde_json::json!({"status": "success"})))
}

#[utoipa::path(
    get,
    path = "/api/system/evolution",
    responses(
        (status = 200, description = "Evolution history", body = [serde_json::Value])
    ),
    security(("api_key" = []))
)]
pub async fn get_evolution_history_handler(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let history = state.job_queue.fetch_evolution_history(50).await?;
    Ok(Json(history))
}
