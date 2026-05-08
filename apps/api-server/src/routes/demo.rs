/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::app_state::AppState;
use crate::autonomous_demo::AutonomousDemo;
use aiome_core_contracts::error::AiomeError;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

/// デモの開始要請
#[utoipa::path(
    post,
    path = "/api/v1/demo/start",
    responses(
        (status = 200, description = "Demo started", body = Value),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn start_demo(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<impl IntoResponse, AiomeError> {
    // デモをバックグラウンドで開始
    tokio::spawn(AutonomousDemo::run(state));

    Ok((
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Autonomous demo started in background"
        })),
    ))
}
