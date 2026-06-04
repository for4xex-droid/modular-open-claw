/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
/*
 * Aiome - Job Management API Handlers
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core::traits::*;
use aiome_core::trajectory::{AgentDiagnosis, TrajectoryStep};
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json},
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
        None => Err(aiome_core_contracts::error::AiomeError::ArtifactNotFound {
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
    let mut steps: Vec<TrajectoryStep> = state
        .job_queue
        .trajectory_store
        .fetch_trajectory(&job_id)
        .await
        .map_err(AppError::from)?;

    // 🛡️ [RED-TEAM PATCH] Scrub sensitive data
    for step in &mut steps {
        step.scrub();
    }

    let graph = infrastructure::trajectory_graph::TrajectoryGraph::build_graph(steps)
        .map_err(AppError::from)?;

    Ok(Json(json!(graph)))
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
        None => Err(aiome_core_contracts::error::AiomeError::ArtifactNotFound {
            path: format!("diagnosis:{}", job_id),
        }
        .into()),
    }
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct JobReviewPayload {
    pub status: String,
    pub comments: Option<String>,
}

/// POST /api/v1/jobs/:id/review
#[utoipa::path(
    post,
    path = "/api/v1/jobs/{id}/review",
    request_body = JobReviewPayload,
    responses(
        (status = 202, description = "Review submitted"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = String, Path, description = "Job ID")
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn submit_job_review(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Path(job_id): Path<String>,
    axum::Json(payload): axum::Json<JobReviewPayload>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    tracing::info!(
        "📝 Received review for job {}: status={}, comments={:?}",
        job_id,
        payload.status,
        payload.comments
    );

    // 1. Fetch the job to ensure it still exists and is in AwaitingInput state.
    let job_opt = state
        .job_queue
        .fetch_job(&job_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch job for review: {}", e)))?;

    let job = match job_opt {
        Some(j) => j,
        None => return Err(AppError::not_found("Job not found")),
    };

    if job.status != aiome_core::traits::JobStatus::AwaitingInput {
        // Gap 11: Race condition prevention
        return Ok((
            axum::http::StatusCode::CONFLICT,
            Json(serde_json::json!({"error": format!("Job is not in AwaitingInput state. Current state: {:?}", job.status)})),
        ).into_response());
    }

    if payload.status.to_lowercase() == "approved" {
        // Gap 8: Persist bypass marker in execution_log so the immune system knows it was overridden
        state
            .job_queue
            .store_execution_log(&job_id, "IMMUNE_BYPASS_APPROVED")
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to persist immune bypass flag: {}", e))
            })?;

        // Re-enqueue the job to resume processing
        state
            .job_queue
            .requeue_job(&job_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to requeue approved job: {}", e)))?;

        tracing::info!("✅ Job {} approved and requeued.", job_id);
    } else {
        // Rejected
        let reason = payload
            .comments
            .unwrap_or_else(|| "Rejected by user".to_string());

        // Trigger orchestrator cancellation so conductors can refund their escrows.
        // We do this before fail_job because fail_job marks it as completely failed in the DB.
        if let Some(dispatcher) = state.task_dispatcher.as_opt() {
            if let Err(e) = dispatcher.cancel_job(&job_id).await {
                tracing::error!(
                    "Failed to cancel orchestrated task for rejected job {}: {}",
                    job_id,
                    e
                );
            }
        }

        state
            .job_queue
            .fail_job(&job_id, &reason)
            .await
            .map_err(|e| AppError::internal(format!("Failed to mark job as failed: {}", e)))?;

        tracing::warn!("❌ Job {} rejected. Reason: {}", job_id, reason);
    }

    Ok((
        axum::http::StatusCode::OK,
        Json(serde_json::json!({"success": true, "job_id": job_id})),
    )
        .into_response())
}

/// GET /api/v1/jobs/awaiting-input
#[utoipa::path(
    get,
    path = "/api/v1/jobs/awaiting-input",
    responses(
        (status = 200, description = "Returns jobs requiring user input", body = Value)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_awaiting_input_jobs(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<impl axum::response::IntoResponse, AppError> {
    // Gap 4 & 6: Fetch recent jobs and filter for AwaitingInput
    let jobs = state
        .job_queue
        .fetch_recent_jobs(100)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch jobs: {}", e)))?;

    let awaiting_jobs: Vec<_> = jobs
        .into_iter()
        .filter(|j| j.status == aiome_core::traits::JobStatus::AwaitingInput)
        .collect();

    Ok(Json(awaiting_jobs))
}
