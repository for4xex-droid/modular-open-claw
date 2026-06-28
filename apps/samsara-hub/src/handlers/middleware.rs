/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use axum::{extract::State, http::StatusCode};
use std::sync::Arc;
use tracing::warn;

use crate::handlers::verify_bearer;
use crate::state::HubState;

pub async fn auth_middleware(
    State(state): State<Arc<HubState>>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let mut authenticated = false;
    if auth_header.starts_with("Bearer ") {
        let token = auth_header.trim_start_matches("Bearer ");
        if let Ok(claims) = state.auth_manager.validate_token(token).await {
            // RBAC Enforcement: Hub operations require System, Admin, or Federated roles.
            if claims.roles.iter().any(|r| {
                matches!(
                    r,
                    shared::auth::Role::Admin
                        | shared::auth::Role::System
                        | shared::auth::Role::Federated
                )
            }) {
                authenticated = true;
            } else {
                warn!("⛔ [Hub] Access denied for roles: {:?}", claims.roles);
            }
        } else {
            warn!("⛔ [Hub] validate_token failed (token redacted)");
        }
    } else {
        warn!(
            "⛔ [Hub] Auth header does not start with Bearer. Received: {}",
            auth_header
        );
    }

    if !authenticated {
        if verify_bearer(auth_header, &state.secret) {
            authenticated = true;
        } else {
            warn!("⛔ [Hub] verify_bearer failed (secret redacted)");
        }
    }

    if authenticated {
        Ok(next.run(req).await)
    } else {
        warn!("⛔ [Hub] Unauthorized access attempt.");
        Err(StatusCode::UNAUTHORIZED)
    }
}
