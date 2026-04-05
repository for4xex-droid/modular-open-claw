/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core_contracts::syndicate::{Guild, GuildMember, SyndicateOps};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateGuildRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AddMemberRequest {
    pub agent_id: Uuid,
    pub role: String,
}

/// [POST] /api/v1/syndicate/guilds
#[utoipa::path(
    post,
    path = "/api/v1/syndicate/guilds",
    request_body = CreateGuildRequest,
    responses(
        (status = 201, description = "Guild created", body = serde_json::Value),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn create_guild(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Json(req): Json<CreateGuildRequest>,
) -> Result<impl IntoResponse, AppError> {
    let store = state.syndicate_store.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Syndicate Store not enabled".into(),
        }
    })?;

    // Sanitize input (G-21 & GlassWorm Shield)
    let sanitized_name = aiome_core::security_impl::purge_entities(
        &shared::guardrails::strip_invisible_unicode(&req.name),
    );
    let sanitized_description = req.description.map(|d| {
        aiome_core::security_impl::purge_entities(&shared::guardrails::strip_invisible_unicode(&d))
    });

    let id = store
        .create_guild(sanitized_name, sanitized_description, auth.agent_id)
        .await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

/// [GET] /api/v1/syndicate/guilds
#[utoipa::path(
    get,
    path = "/api/v1/syndicate/guilds",
    responses(
        (status = 200, description = "List of guilds", body = [Guild]),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn list_guilds(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<impl IntoResponse, AppError> {
    let store = state.syndicate_store.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Syndicate Store not enabled".into(),
        }
    })?;

    let guilds = store.fetch_guilds().await?;
    Ok(Json(guilds))
}

/// [DELETE] /api/v1/syndicate/guilds/:id
#[utoipa::path(
    delete,
    path = "/api/v1/syndicate/guilds/{id}",
    responses(
        (status = 200, description = "Guild deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found")
    ),
    params(
        ("id" = Uuid, Path, description = "Guild ID")
    ),
    security(("api_key" = []))
)]
pub async fn delete_guild(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let store = state.syndicate_store.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Syndicate Store not enabled".into(),
        }
    })?;

    store.delete_guild(id, auth.agent_id).await?;
    Ok(StatusCode::OK)
}

/// [POST] /api/v1/syndicate/guilds/:id/members
#[utoipa::path(
    post,
    path = "/api/v1/syndicate/guilds/{id}/members",
    request_body = AddMemberRequest,
    responses(
        (status = 200, description = "Member added"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found")
    ),
    params(
        ("id" = Uuid, Path, description = "Guild ID")
    ),
    security(("api_key" = []))
)]
pub async fn add_member(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Path(id): Path<Uuid>,
    Json(req): Json<AddMemberRequest>,
) -> Result<impl IntoResponse, AppError> {
    let store = state.syndicate_store.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Syndicate Store not enabled".into(),
        }
    })?;

    store
        .add_member(id, req.agent_id, req.role, auth.agent_id)
        .await?;
    Ok(StatusCode::OK)
}

/// [GET] /api/v1/syndicate/guilds/:id/members
#[utoipa::path(
    get,
    path = "/api/v1/syndicate/guilds/{id}/members",
    responses(
        (status = 200, description = "List of guild members", body = [GuildMember]),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found")
    ),
    params(
        ("id" = Uuid, Path, description = "Guild ID")
    ),
    security(("api_key" = []))
)]
pub async fn list_members(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let store = state.syndicate_store.as_opt().ok_or_else(|| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Syndicate Store not enabled".into(),
        }
    })?;

    let members = store.fetch_members(id).await?;
    Ok(Json(members))
}
