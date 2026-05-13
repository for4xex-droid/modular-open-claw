/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::auth::Authenticated;
use crate::error::AppError;
use aiome_core_contracts::traits::{JobStatus, TaskRegistry};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct A2uiActionRequest {
    pub surface_id: String,
    pub action: String,
    /// オプションのペイロード（フォーム入力値など将来用）
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct A2uiActionResponse {
    pub success: bool,
    pub message: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/a2ui/action",
    request_body = A2uiActionRequest,
    responses(
        (status = 200, description = "Action executed successfully", body = A2uiActionResponse),
        (status = 400, description = "Invalid action or parameters"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (BOLA violation)"),
        (status = 429, description = "Too many requests")
    ),
    security(("api_key" = []))
)]
pub async fn submit_a2ui_action(
    State(state): State<crate::AppState>,
    _auth: Authenticated,
    Json(req): Json<A2uiActionRequest>,
) -> Result<Json<A2uiActionResponse>, AppError> {
    // 1. 入力長制限 (DoS 防止)
    if req.surface_id.len() > 256 || req.action.len() > 512 {
        return Err(AppError::bad_request("Input exceeds maximum length"));
    }

    // 1b. payload サイズ制限 (メモリ枯渇防止: 4KB)
    if let Some(ref payload) = req.payload {
        let payload_size = serde_json::to_string(payload).map(|s| s.len()).unwrap_or(0);
        if payload_size > 4096 {
            return Err(AppError::bad_request("Payload exceeds 4KB size limit"));
        }
    }

    // 2. action のホワイトリスト検証
    let valid_prefixes = ["approve_job:", "run_skill:", "cancel_job:"];
    if !valid_prefixes
        .iter()
        .any(|prefix| req.action.starts_with(prefix))
    {
        return Err(AppError::bad_request("Unauthorized action prefix"));
    }

    // 3. Extracted Target ID validation (P-8 パッチ)
    let parts: Vec<&str> = req.action.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(AppError::bad_request("Malformed action string"));
    }
    let action_type = parts[0];
    let target_id = parts[1];

    match action_type {
        "approve_job" | "cancel_job" => {
            // Validate UUID format for jobs
            if uuid::Uuid::parse_str(target_id).is_err() {
                return Err(AppError::bad_request("Invalid target UUID format"));
            }
        }
        "run_skill" => {
            if target_id.is_empty()
                || target_id.len() > 64
                || !target_id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Err(AppError::bad_request("Invalid skill name format"));
            }
        }
        _ => return Err(AppError::bad_request("Unknown action type")),
    }

    // 4. Action Dispatching
    match action_type {
        "approve_job" => {
            state
                .job_queue
                .get_inner()
                .update_job_status(target_id, JobStatus::Pending)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to approve job {}: {}", target_id, e);
                    AppError::internal("Failed to update job status")
                })?;
        }
        "cancel_job" => {
            state
                .job_queue
                .get_inner()
                .update_job_status(target_id, JobStatus::Cancelled)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to cancel job {}: {}", target_id, e);
                    AppError::internal("Failed to update job status")
                })?;
        }
        "run_skill" => {
            let skill_name = target_id;
            let payload_str = serde_json::to_string(&req.payload.unwrap_or_default())
                .unwrap_or_else(|_| "{}".to_string());
            state
                .wasm_skill_manager
                .get_inner()
                .dry_run_skill(skill_name, &payload_str)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to run skill {}: {}", skill_name, e);
                    AppError::internal(&format!("Skill execution failed: {}", e))
                })?;
        }
        _ => {
            return Err(AppError::bad_request("Unknown action type"));
        }
    }

    Ok(Json(A2uiActionResponse {
        success: true,
        message: format!("Action {} dispatched successfully", action_type),
    }))
}
