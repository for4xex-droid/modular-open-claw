/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::app_state::AppState;
use crate::auth::Authenticated;
use crate::error::AppError;
use aiome_core_contracts::TaskRegistry;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct LoraTrainRequest {
    pub base_model: String,
    pub dataset_id: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct LoraTrainResponse {
    pub job_id: String,
}

/// [POST] /api/v1/lora/train
/// Triggers a LoRA fine-tuning job
#[utoipa::path(
    post,
    path = "/api/v1/lora/train",
    request_body = LoraTrainRequest,
    responses(
        (status = 202, description = "Training job started", body = LoraTrainResponse)
    ),
    security(("api_key" = []))
)]
pub async fn train_lora_handler(
    _auth: Authenticated,
    State(state): State<AppState>,
    Json(req): Json<LoraTrainRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        "🛠️ [LoRA] Triggering training for model: {}, dataset: {}",
        req.base_model, req.dataset_id
    );

    let mut params = req.params;
    params["agent_id"] = serde_json::json!(_auth.agent_id.to_string());

    let job_id = state
        .lora_engine
        .train(&req.base_model, &req.dataset_id, params)
        .await
        .map_err(|e| AppError::internal(format!("Failed to start LoRA training: {}", e)))?;

    Ok((StatusCode::ACCEPTED, Json(LoraTrainResponse { job_id })))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct LoraJobStatusResponse {
    pub job_id: String,
    pub status: String,
}

/// [GET] /api/v1/lora/status/{job_id}
/// Checks the status of a LoRA training job
#[utoipa::path(
    get,
    path = "/api/v1/lora/status/{job_id}",
    responses(
        (status = 200, description = "Returns job status", body = LoraJobStatusResponse),
        (status = 404, description = "Job not found")
    ),
    security(("api_key" = []))
)]
pub async fn status_lora_handler(
    _auth: Authenticated,
    State(state): State<AppState>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, AppError> {
    info!("🛠️ [LoRA] Checking training status for job: {}", job_id);

    let job = state
        .job_queue
        .fetch_job(&job_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch job {}: {}", job_id, e)))?;

    if let Some(j) = job {
        // SECURITY BOLA CHECK (Broken Object Level Authorization)
        // Ensure that if the job is owned by an agent, it matches the authenticated user.
        // Returning 404 instead of 403 prevents job listing/enumeration attacks.
        if let Some(owner_id) = j.agent_id {
            if owner_id != _auth.agent_id {
                tracing::warn!(
                    "🔒 BOLA attempt: agent {} tried to access job {}",
                    _auth.agent_id,
                    job_id
                );
                return Err(AppError::not_found("Job not found"));
            }
        }

        Ok((
            StatusCode::OK,
            Json(LoraJobStatusResponse {
                job_id,
                status: j.status.as_str().to_string(),
            }),
        ))
    } else {
        Err(AppError::not_found(format!("Job {} not found", job_id)))
    }
}
