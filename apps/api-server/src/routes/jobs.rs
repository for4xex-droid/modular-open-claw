/*
 * Aiome - Job Management API Handlers
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core::traits::*;
use aiome_core::trajectory::{AgentDiagnosis, TrajectoryStep, TrajectoryStore};
use axum::{
    extract::{Path, State},
    response::Json,
};
use serde_json::{json, Value};

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
    state
        .task_dispatcher
        .cancel_job(&job_id)
        .await
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
    let job = state
        .job_queue
        .fetch_job(&job_id)
        .await
        .map_err(AppError::from)?;

    match job {
        Some(job) => Ok(Json(json!({
            "job_id": job.id,
            "status": job.status,
            "logs": job.execution_log,
            "error": job.error_message,
        }))),
        None => Err(aiome_contracts::error::AiomeError::ArtifactNotFound {
            path: format!("job:{}", job_id),
        }
        .into()),
    }
}

/// GET /api/v1/trajectory/:job_id
#[utoipa::path(
    get,
    path = "/api/v1/trajectory/{id}",
    responses(
        (status = 200, description = "Execution trajectory for the job", body = Vec<TrajectoryStep>),
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
pub async fn get_trajectory_handler(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Path(job_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let steps: Vec<TrajectoryStep> = state
        .job_queue
        .trajectory_store
        .fetch_trajectory(&job_id)
        .await
        .map_err(AppError::from)?;

    Ok(Json(json!({
        "job_id": job_id,
        "steps": steps,
    })))
}

/// GET /api/v1/trajectory/:job_id/diagnosis
#[utoipa::path(
    get,
    path = "/api/v1/trajectory/{id}/diagnosis",
    responses(
        (status = 200, description = "Self-diagnosis report for a failed job", body = AgentDiagnosis),
        (status = 404, description = "No diagnosis found for this job"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = String, Path, description = "Job ID")
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn get_diagnosis_handler(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Path(job_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let diagnosis: Option<AgentDiagnosis> = state
        .job_queue
        .trajectory_store
        .fetch_diagnosis(&job_id)
        .await
        .map_err(AppError::from)?;

    match diagnosis {
        Some(d) => Ok(Json(json!(d))),
        None => Err(aiome_contracts::error::AiomeError::ArtifactNotFound {
            path: format!("diagnosis:{}", job_id),
        }
        .into()),
    }
}
