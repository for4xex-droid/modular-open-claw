/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AuthorizeRequest {
    pub client_id: String,
    pub response_type: String,
    pub redirect_uri: Option<String>,
    pub scope: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct TokenRequest {
    pub grant_type: String,
    pub code: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/authorize",
    params(
        ("client_id" = String, Query),
        ("response_type" = String, Query)
    ),
    responses(
        (status = 200, description = "OAuth 2.1 Authorization Endpoint")
    )
)]
pub async fn authorize_handler(
    Query(query): Query<AuthorizeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Generate a simple auth code for standard OAuth code flow.
    let code = format!("auth_code_{}", query.client_id);
    Ok(Json(serde_json::json!({
        "code": code,
        "state": query.state
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/token",
    request_body = TokenRequest,
    responses(
        (status = 200, description = "OAuth 2.1 Token Endpoint", body = TokenResponse)
    )
)]
pub async fn token_handler(
    State(state): State<AppState>,
    Json(payload): Json<TokenRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    // Basic validation of the authorization code flow
    if payload.grant_type != "authorization_code" {
        return Err(AppError::bad_request("Unsupported grant_type"));
    }

    // Issue a real JWT using AuthManager
    use chrono::Utc;
    use shared::auth::AiomeCustomClaims;

    let now = Utc::now().timestamp() as usize;
    let exp = now + 3600; // 1 hour expiration

    let claims = AiomeCustomClaims {
        sub: payload
            .client_id
            .unwrap_or_else(|| "unknown_client".to_string()),
        ekyc_verified: false,
        agent_id: uuid::Uuid::nil(),
        roles: vec!["user".to_string()],
        exp,
        iat: now,
        iss: "aiome_identity".to_string(),
    };

    let token = state.auth_manager.issue_token(claims).await.map_err(|e| {
        tracing::error!("Failed to issue token: {:?}", e);
        AppError::internal("Token generation failed")
    })?;

    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: None,
    }))
}
