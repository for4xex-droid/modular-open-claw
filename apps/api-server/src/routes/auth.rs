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
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use infrastructure::job_queue::SecurityOps;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared::auth::AiomeCustomClaims;

#[derive(Debug, Deserialize)]
pub struct AuthorizeRequest {
    pub client_id: String,
    pub response_type: String,
    pub redirect_uri: Option<String>,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct TokenRequest {
    pub grant_type: String,
    pub code: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub code_verifier: Option<String>,
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
    State(state): State<AppState>,
    Query(query): Query<AuthorizeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Validate response_type per RFC 6749 §4.1.1
    if query.response_type != "code" {
        return Err(AppError::bad_request(
            "Unsupported response_type. Only 'code' is supported.",
        ));
    }

    // If code_challenge is present, code_challenge_method must be S256
    if query.code_challenge.is_some() {
        match &query.code_challenge_method {
            Some(method) if method == "S256" => { /* OK */ }
            Some(_) => {
                return Err(AppError::bad_request(
                    "Unsupported code_challenge_method. Only S256 is supported.",
                ));
            }
            None => {
                return Err(AppError::bad_request(
                    "code_challenge_method is required when code_challenge is provided.",
                ));
            }
        }
    }

    // Generate a cryptographic authorization code
    let code = uuid::Uuid::new_v4().to_string();

    state
        .pkce_cache
        .insert(
            code.clone(),
            (query.code_challenge.clone(), query.client_id.clone()),
        )
        .await;

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
    if payload.grant_type == "password" {
        // Password Grant (Quickstart / Admin login)
        let provided_secret = payload
            .client_secret
            .clone()
            .ok_or_else(|| AppError::bad_request("Missing client_secret for password grant"))?;

        use aiome_core_contracts::SettingsOps;
        let admin_hash = state
            .job_queue
            .get_inner()
            .get_setting_value("admin_password_hash")
            .await
            .ok()
            .flatten();

        let mut authenticated = false;

        if let Some(hash) = admin_hash {
            use argon2::{Argon2, PasswordHash, PasswordVerifier};
            if let Ok(parsed_hash) = PasswordHash::new(&hash) {
                let argon2 = Argon2::default();
                if argon2
                    .verify_password(provided_secret.as_bytes(), &parsed_hash)
                    .is_ok()
                {
                    authenticated = true;
                }
            }
        } else {
            // Fallback to .env for M2M or pre-setup modes
            use secrecy::ExposeSecret;
            let expected = state.api_server_secret.expose_secret();
            if crate::auth::verify_constant_time(provided_secret.as_bytes(), expected.as_bytes()) {
                authenticated = true;
            }
        }

        if !authenticated {
            return Err(AppError::forbidden("Invalid admin secret"));
        }
    } else if payload.grant_type == "authorization_code" {
        // Authorization Code Grant (OAuth)
        let code = payload
            .code
            .ok_or_else(|| AppError::bad_request("Missing code"))?;

        // Consume the code (one-time use).
        let (stored_challenge, stored_client_id) = state
            .pkce_cache
            .get(&code)
            .await
            .ok_or_else(|| AppError::bad_request("Invalid or expired authorization code"))?;
        state.pkce_cache.invalidate(&code).await;

        if let Some(client_id) = &payload.client_id {
            if client_id != &stored_client_id {
                return Err(AppError::bad_request("Client ID mismatch"));
            }
        }

        // PKCE verification
        if let Some(challenge) = stored_challenge {
            let verifier = payload
                .code_verifier
                .ok_or_else(|| AppError::bad_request("Missing code_verifier for PKCE"))?;

            let mut hasher = Sha256::new();
            hasher.update(verifier.as_bytes());
            let expected_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

            use subtle::ConstantTimeEq;
            let is_valid = challenge.as_bytes().ct_eq(expected_challenge.as_bytes());
            if !bool::from(is_valid) {
                return Err(AppError::bad_request("Invalid code_verifier"));
            }
        }
    } else {
        return Err(AppError::bad_request("Unsupported grant_type"));
    }

    // Issue a real JWT using AuthManager
    let now = Utc::now().timestamp() as usize;
    let exp = now + 3600; // 1 hour expiration

    let mut sub = payload.client_id.unwrap_or_else(|| "admin".to_string());
    if payload.grant_type == "password" {
        use aiome_core_contracts::SettingsOps;
        if let Some(email) = state
            .job_queue
            .get_inner()
            .get_setting_value("admin_email")
            .await
            .ok()
            .flatten()
        {
            sub = email;
        }
    }

    let claims = AiomeCustomClaims {
        sub,
        ekyc_verified: false,
        // Deterministic UUID for the local admin to pass the nil guard
        agent_id: uuid::uuid!("00000000-0000-0000-0000-000000000001"),
        roles: vec![shared::auth::Role::Admin],
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
        use secrecy::ExposeSecret;
        let secret = match state.api_server_secret.as_opt() {
            Some(s) => s.expose_secret().clone(),
            None => {
                tracing::error!("API_SERVER_SECRET is not initialized in AppState; cannot sign OxiLean certificate for account deletion");
                return Err(AppError::internal(
                    "Server misconfiguration: signing secret unavailable",
                ));
            }
        };

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
