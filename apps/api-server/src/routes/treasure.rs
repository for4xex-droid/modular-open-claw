/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use crate::auth::AuthenticatedUser;
use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TreasureFeedback {
    pub intent_id: Uuid,
    pub action: String, // "view", "click"
    pub metadata: Option<serde_json::Value>,
}

/// [POST] /api/v1/treasure
#[utoipa::path(
    post,
    path = "/api/v1/treasure",
    request_body = TreasureFeedback,
    responses(
        (status = 200, description = "Feedback recorded and reward calculated"),
        (status = 401, description = "Unauthorized access")
    ),
    security(("api_key" = []))
)]
pub async fn record_feedback(
    State(state): State<AppState>,
    Extension(AuthenticatedUser(claims)): Extension<AuthenticatedUser>,
    Json(feedback): Json<TreasureFeedback>,
) -> Result<impl IntoResponse, AppError> {
    // AS-1: AgentSense Feedback Processing
    // In AS-1, we just log this for now (and maybe reward karma)
    tracing::info!(
        "💰 [Treasure] Feedback from {}: {} on {}",
        claims.agent_id,
        feedback.action,
        feedback.intent_id
    );

    // TODO: Implement Karma rewarding logic here

    Ok(StatusCode::OK)
}
