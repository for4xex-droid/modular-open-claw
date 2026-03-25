/*
 * Aiome - Job Management API Handlers
 */

use axum::{
    extract::{Path, State},
    response::Json,
};
use serde_json::{json, Value};
use crate::AppState;
use crate::error::AppError;
use aiome_core::traits::JobQueue;

/// POST /api/v1/jobs/:id/cancel
#[utoipa::path(
    post,
    path = "/api/v1/jobs/{id}/cancel",
    responses(
        (status = 200, description = "Job cancelled successfully", body = Value),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = String, Path, description = "Job ID")
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn cancel_job_handler(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Path(job_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    // Component implements Deref, so we can use it directly. 
    // It will panic if not initialized, but that's standard for critical components in this repo.
    state.task_dispatcher.cancel_job(&job_id).await
        .map_err(AppError::from)?;

    Ok(Json(json!({"success": true})))
}

/// GET /api/v1/jobs/:id/logs
#[utoipa::path(
    get,
    path = "/api/v1/jobs/{id}/logs",
    responses(
        (status = 200, description = "Job status and logs", body = Value),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = String, Path, description = "Job ID")
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn get_job_logs_handler(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Path(job_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let job = state.job_queue.fetch_job(&job_id).await
        .map_err(AppError::from)?;

    match job {
        Some(job) => {
            Ok(Json(json!({
                "job_id": job.id,
                "status": job.status,
                "logs": job.execution_log,
                "error": job.error_message,
            })))
        },
        None => Err(aiome_contracts::error::AiomeError::ArtifactNotFound {
            path: format!("job:{}", job_id),
        }.into()),
    }
}
