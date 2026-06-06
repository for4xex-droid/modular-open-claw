/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 */

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use nurture_bridge::auth::AiomeCustomClaims;

pub struct McpAuth(pub AiomeCustomClaims);

#[async_trait]
impl<S> FromRequestParts<S> for McpAuth
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({ "error": "認証ヘッダーがありません" })),
                )
                    .into_response()
            })?;

        if !auth_header.starts_with("Bearer ") {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Bearer トークンが必要です" })),
            )
                .into_response());
        }

        let token = &auth_header[7..];

        let ext = parts
            .extensions
            .get::<crate::state::SharedState>()
            .ok_or_else(|| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": "Internal State Error" })),
                )
                    .into_response()
            })?;

        match ext.auth_manager.validate_token(token).await {
            Ok(claims) => Ok(McpAuth(claims)),
            Err(e) => {
                tracing::warn!("🚨 [Nurture-Auth] Token validation failed: {}", e);
                Err((
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({ "error": "無効なトークンです" })),
                )
                    .into_response())
            }
        }
    }
}
