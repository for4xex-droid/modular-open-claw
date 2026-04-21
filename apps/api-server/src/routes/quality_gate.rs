/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use axum::{extract::Query, extract::State, response::Json};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct HistoryParams {
    pub limit: Option<u32>,
}

#[utoipa::path(
    get,
    path = "/api/v1/quality-gate/history",
    params(
        ("limit" = Option<u32>, Query, description = "Number of items to fetch (max 100)")
    ),
    responses(
        (status = 200, description = "Recent quality gate history", body = [infrastructure::quality_gate_store::QualityGateEntry]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - System agent only")
    ),
    security(("api_key" = []))
)]
pub async fn get_quality_gate_history(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Query(params): Query<HistoryParams>,
) -> Result<Json<Vec<infrastructure::quality_gate_store::QualityGateEntry>>, AppError> {
    if auth.agent_id != state.system_agent_id {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Access denied".to_string(),
        }
        .into());
    }

    let limit = params.limit.unwrap_or(50).min(100);

    let store = state.quality_gate_store.get_inner().clone();
    let history = store.list_recent(limit).await?;

    Ok(Json(history))
}
