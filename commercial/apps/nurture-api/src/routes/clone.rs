/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use crate::state::SharedState;
use axum::http::StatusCode;
use axum::{
    extract::{Json, Path},
    routing::{get, post},
    Extension, Router,
};
use chrono::Duration;
use commerce_protocol::identity::ActorId;
use nurture_infra::sidecar::clone_manager::{CloneSpec, ResourceBudget};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ForkRequest {
    pub parent_actor_id: ActorId,
    pub specialization: String,
    pub vram_mb: u32,
    pub max_duration_minutes: i64,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ForkResponse {
    pub clone_id: Uuid,
}

pub fn clone_routes() -> Router {
    Router::new()
        .route("/fork", post(fork_clone))
        .route("/terminate/:id", post(terminate_clone))
        .route("/list", get(list_clones))
}

async fn fork_clone(
    auth: crate::auth::McpAuth,
    Extension(state): Extension<SharedState>,
    Json(req): Json<ForkRequest>,
) -> Result<Json<ForkResponse>, (StatusCode, String)> {
    // 🔒 冪等性チェック (二重課金 & 重複プロセス起動防止)
    let ikey = req.idempotency_key.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "Missing idempotency_key".to_string(),
        )
    })?;

    // 既存のレスポンスがあるか確認
    if let Ok(Some(Some(prev_res))) = state.idempotency.get_response(ikey).await {
        let resp: ForkResponse = serde_json::from_str(&prev_res.body).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to parse cached response".to_string(),
            )
        })?;
        return Ok(Json(resp));
    }

    // 予約
    state
        .idempotency
        .reserve_key(ikey, Duration::hours(24))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 🚧 M-3: ActorId 一致検証
    if auth.0.sub != req.parent_actor_id.0.to_string() {
        return Err((
            StatusCode::FORBIDDEN,
            "Authentication mismatch: parent_actor_id must match token sub".into(),
        ));
    }

    // 🚧 入力バリデーション & サニタイズ
    let safe_duration = req.max_duration_minutes.clamp(1, 1440);
    if req.max_duration_minutes != safe_duration {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid max_duration_minutes (1-1440 allowed)".into(),
        ));
    }

    let safe_vram = req.vram_mb.clamp(1, 24000);
    if req.vram_mb != safe_vram {
        return Err((
            StatusCode::BAD_REQUEST,
            "VRAM request exceeds safety limit (max 24000MB)".into(),
        ));
    }

    if req.specialization.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Specialization cannot be empty".into(),
        ));
    }

    let spec = CloneSpec {
        clone_id: Uuid::new_v4(),
        parent_actor_id: req.parent_actor_id,
        specialization: req.specialization,
        resource_budget: ResourceBudget {
            vram_mb: req.vram_mb as u64,
            max_cpu_percent: 100, // フィールド名を修正
            max_memory_mb: 2048,  // フィールド名を修正
        },
        max_duration: chrono::Duration::minutes(req.max_duration_minutes),
        karma_snapshot: Vec::new(),
    };

    let clone_id = state
        .clone_manager
        .fork(spec)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let resp = ForkResponse { clone_id };

    // レスポンスを保存
    if let Ok(resp_json) = serde_json::to_string(&resp) {
        if let Err(e) = state.idempotency.save_response(ikey, 200, resp_json).await {
            tracing::warn!("Failed to save idempotency response for clone fork: {}", e);
        }
    }

    Ok(Json(resp))
}

async fn terminate_clone(
    auth: crate::auth::McpAuth,
    Extension(state): Extension<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    // 🚧 M-3: 権限チェック (所有者確認)
    // 本来は DB を引いて ActorId を確認すべきだが、CloneManager 内で実施するように設計
    state
        .clone_manager
        .terminate(id, &auth.0.sub)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

async fn list_clones(
    auth: crate::auth::McpAuth,
    Extension(state): Extension<SharedState>,
) -> Json<Vec<Uuid>> {
    let clones = state
        .clone_manager
        .list_active_clones_for_actor(&auth.0.sub);
    Json(clones)
}
