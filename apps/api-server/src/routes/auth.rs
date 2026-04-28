/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core::traits::SettingsOps;
use axum::{
    extract::{Query, State},
    response::Json,
};
use infrastructure::job_queue::SecurityOps;
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
// auth-exempt: OAuth フロー
#[tracing::instrument(skip_all, fields(path = "/api/v1/auth/authorize"))]
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
// auth-exempt: OAuth フロー
#[tracing::instrument(skip_all, fields(path = "/api/v1/auth/token"))]
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
        roles: vec![shared::auth::Role::User],
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

#[utoipa::path(
    delete,
    path = "/api/v1/auth/delete",
    responses(
        (status = 200, description = "Account deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(("api_key" = []))
)]
pub async fn delete_account_handler(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::warn!("🗑️ Account deletion requested for agent: {}", auth.agent_id);

    // 1. Send Forget request to Nurture
    let client = aiome_core::http::get_http_client();
    let nurture_url = match state.job_queue.get_setting_value("nurture_url").await {
        Ok(Some(url)) if !url.is_empty() => url,
        _ => {
            tracing::warn!(
                "nurture_url setting is not configured; Nurture PII scrub will be skipped"
            );
            String::new()
        }
    };

    let mut nurture_notified = false;

    if !nurture_url.is_empty() {
        let secret = std::env::var("API_SERVER_SECRET").map_err(|_| {
            tracing::error!("API_SERVER_SECRET is not set; cannot sign OxiLean certificate for account deletion");
            AppError::internal("Server misconfiguration: signing secret unavailable")
        })?;

        // Create an OxiLean Certificate for internal request
        let ts = chrono::Utc::now().to_rfc3339();
        let cert = aiome_core_contracts::oxilean::OxiLeanProofCertificate::generate(
            "aiome_system".to_string(),
            1000,
            ts,
            &secret,
        );
        let cert_json = serde_json::to_string(&cert).map_err(|e| {
            tracing::error!("Failed to serialize OxiLean certificate: {}", e);
            AppError::internal("Certificate serialization failed")
        })?;
        let cert_b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, cert_json);

        let delete_url = format!(
            "{}/internal/forget/{}",
            nurture_url.trim_end_matches('/'),
            auth.agent_id
        );
        match client
            .post(&delete_url)
            .header("x-oxilean-proof-certificate", cert_b64)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                nurture_notified = true;
            }
            Ok(resp) => {
                tracing::error!(
                    "Nurture returned error on account deletion: {}",
                    resp.status()
                );
            }
            Err(e) => {
                tracing::error!("Failed to notify Nurture of account deletion: {}", e);
            }
        }
    }

    // 2. Delete data from Aiome database (cortex_chat_history, settings, etc)
    state
        .job_queue
        .forget_actor(auth.agent_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to scrub PII data: {}", e);
            AppError::internal("Failed to scrub PII data")
        })?;

    tracing::info!(
        "✅ Account data successfully purged for agent: {} (nurture_notified: {})",
        auth.agent_id,
        nurture_notified
    );
    Ok(Json(serde_json::json!({
        "status": "deleted",
        "nurture_pii_scrubbed": nurture_notified
    })))
}
