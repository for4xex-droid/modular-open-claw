/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::app_state::AppState;
use crate::auth::Authenticated;
use crate::error::AppError;
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

    let job_id = state
        .lora_engine
        .train(&req.base_model, &req.dataset_id, req.params)
        .await
        .map_err(|e| AppError::internal(format!("Failed to start LoRA training: {}", e)))?;

    Ok((StatusCode::ACCEPTED, Json(LoraTrainResponse { job_id })))
}
