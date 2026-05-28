/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use shared::auth::Role;
use uuid::Uuid;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BanRequest {
    pub agent_id: Uuid,
    pub reason: String,
    pub severity: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UnbanRequest {
    pub agent_id: Uuid,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BanResponse {
    pub status: String,
}

/// アカウントを違反としてBANする (Admin専用)
pub async fn create_ban(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Json(req): Json<BanRequest>,
) -> Result<impl IntoResponse, AppError> {
    // RBAC: Admin ロール要求
    if !auth.roles.iter().any(|r| matches!(r, Role::Admin)) {
        return Err(AppError::forbidden("Access denied: Admin role required"));
    }

    state
        .ban_store
        .ban(&req.agent_id, &req.reason, &req.severity, "admin")
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(BanResponse {
        status: "success".to_string(),
    }))
}

/// アカウントのBANを解除する (Admin専用)
pub async fn remove_ban(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Json(req): Json<UnbanRequest>,
) -> Result<impl IntoResponse, AppError> {
    if !auth.roles.iter().any(|r| matches!(r, Role::Admin)) {
        return Err(AppError::forbidden("Access denied: Admin role required"));
    }

    state
        .ban_store
        .unban(&req.agent_id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(BanResponse {
        status: "success".to_string(),
    }))
}

/// 現在の全BANレコード一覧を取得する (Admin専用)
pub async fn list_bans(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
) -> Result<impl IntoResponse, AppError> {
    if !auth.roles.iter().any(|r| matches!(r, Role::Admin)) {
        return Err(AppError::forbidden("Access denied: Admin role required"));
    }

    let bans = state
        .ban_store
        .list_bans()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(bans))
}
