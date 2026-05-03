/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct SendBiomeRequest {
    pub recipient_pubkey: String,
    pub topic_id: String,
    pub content: String,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct StartAutonomousRequest {
    pub topic_id: String,
    pub peer_pubkey: String,
    pub interval_secs: Option<u64>,
    pub max_rounds: Option<u32>,
}

pub fn stubbed_response() -> Result<Response, AppError> {
    Ok((
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "Not Implemented",
            "message": "Biome (Federation/P2P) features are deferred to v1.5."
        })),
    )
        .into_response())
}

#[utoipa::path(
    get,
    path = "/api/biome/status",
    responses(
        (status = 501, description = "Not Implemented", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn biome_status(
    State(_state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Response, AppError> {
    stubbed_response()
}

#[utoipa::path(
    get,
    path = "/api/biome/topics",
    responses(
        (status = 501, description = "Not Implemented", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn list_topics(
    State(_state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Response, AppError> {
    stubbed_response()
}

#[utoipa::path(
    post,
    path = "/api/biome/topics",
    request_body = serde_json::Value,
    responses(
        (status = 501, description = "Not Implemented", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn create_topic(
    State(_state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(_req): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    stubbed_response()
}

#[utoipa::path(
    post,
    path = "/api/biome/autonomous/start",
    request_body = StartAutonomousRequest,
    responses(
        (status = 501, description = "Not Implemented", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn autonomous_start(
    State(_state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(_req): Json<StartAutonomousRequest>,
) -> Result<Response, AppError> {
    stubbed_response()
}

#[utoipa::path(
    post,
    path = "/api/biome/autonomous/stop",
    responses(
        (status = 501, description = "Not Implemented", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn autonomous_stop(
    State(_state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Response, AppError> {
    stubbed_response()
}

#[utoipa::path(
    get,
    path = "/api/biome/autonomous/status",
    responses(
        (status = 501, description = "Not Implemented", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn autonomous_status(
    State(_state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Response, AppError> {
    stubbed_response()
}

#[utoipa::path(
    get,
    path = "/api/biome/list",
    responses(
        (status = 501, description = "Not Implemented", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn list_messages(
    State(_state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Response, AppError> {
    stubbed_response()
}

#[utoipa::path(
    post,
    path = "/api/biome/send",
    request_body = SendBiomeRequest,
    responses(
        (status = 501, description = "Not Implemented", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn send_message(
    State(_state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(_req): Json<SendBiomeRequest>,
) -> Result<Response, AppError> {
    stubbed_response()
}
