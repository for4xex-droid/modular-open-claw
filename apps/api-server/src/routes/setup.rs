/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::{error::AppError, AppState};
use aiome_core_contracts::SettingsOps;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use shared::auth::{AiomeCustomClaims, Role};
use tracing::info;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetupInitRequest {
    pub admin_email: String,
    pub admin_password: String,
    pub ai_name: String,
    pub view_mode: String,
    pub language: String,
    pub tos_accepted: bool,
    /// 同意した利用規約の版（docs/legal/CONSENT_SPEC.md §2）。旧クライアント互換のため省略可。
    #[serde(default)]
    pub tos_version: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SetupInitResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: usize,
}

#[utoipa::path(
    post,
    path = "/api/v1/setup/init",
    request_body = SetupInitRequest,
    responses(
        (status = 200, description = "Setup completed successfully", body = SetupInitResponse),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Setup already completed")
    )
)]
// auth-exempt
pub async fn setup_init(
    State(state): State<AppState>,
    Json(payload): Json<SetupInitRequest>,
) -> Result<Json<SetupInitResponse>, AppError> {
    // 1. Check if admin account already exists
    let admin_exists = state
        .job_queue
        .get_inner()
        .get_setting_value("admin_password_hash")
        .await
        .ok()
        .flatten()
        .is_some();

    if admin_exists {
        return Err(AppError::forbidden("Setup has already been completed"));
    }

    if payload.admin_password.len() < 12 {
        return Err(AppError::bad_request(
            "Password must be at least 12 characters",
        ));
    }

    if !payload.tos_accepted {
        return Err(AppError::bad_request("Terms of Service must be accepted"));
    }

    // Validate email format (RFC-lite: no spaces, has @, has domain)
    let email_trimmed = payload.admin_email.trim();
    if email_trimmed.is_empty()
        || !email_trimmed.contains('@')
        || email_trimmed.contains(' ')
        || email_trimmed.len() > 254
    {
        return Err(AppError::bad_request("Invalid email address format"));
    }

    // Validate view_mode against known variants (legacy aliases accepted, stored as simple/cockpit)
    let allowed_modes = [
        "simple",
        "cockpit",
        "beginner",
        "intermediate",
        "advanced",
        "expert",
    ];
    if !allowed_modes.contains(&payload.view_mode.as_str()) {
        return Err(AppError::bad_request(
            "Invalid view_mode. Must be one of: simple, cockpit, beginner, intermediate, advanced, expert",
        ));
    }

    // 2. Hash password with argon2id
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(payload.admin_password.as_bytes(), &salt)
        .map_err(|e| {
            tracing::error!("Password hashing failed: {}", e);
            AppError::internal("Failed to secure password")
        })?
        .to_string();

    // 3. Save settings directly using update_setting (bypasses ALLOWED_KEYS intentionally)
    // admin_password_hash and admin_email are in SECRETS, so they will be masked in GET /api/v1/settings
    state
        .job_queue
        .get_inner()
        .update_setting("admin_password_hash", &password_hash, "auth", true)
        .await?;
    state
        .job_queue
        .get_inner()
        .update_setting("admin_email", email_trimmed, "auth", true)
        .await?;

    // Save AI Name, View Mode, Language
    let ai_name = if payload.ai_name.trim().is_empty() {
        "Watchtower"
    } else {
        payload.ai_name.trim()
    };
    state
        .job_queue
        .get_inner()
        .update_setting("ai_name", ai_name, "system", false)
        .await?;
    // U2-1: normalize legacy values to simple | cockpit
    let view_mode_normalized = match payload.view_mode.as_str() {
        "beginner" | "simple" => "simple",
        "intermediate" | "advanced" | "expert" | "cockpit" => "cockpit",
        other => other,
    };
    state
        .job_queue
        .get_inner()
        .update_setting("view_mode", view_mode_normalized, "ui", false)
        .await?;
    state
        .job_queue
        .get_inner()
        .update_setting("language", &payload.language, "ui", false)
        .await?;
    state
        .job_queue
        .get_inner()
        .update_setting("tos_accepted", "true", "legal", false)
        .await?;
    // 同意証跡: どの版に・いつ同意したかを記録する（規約改訂時の再同意判定に使用）
    if let Some(version) = payload
        .tos_version
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty() && v.len() <= 32)
    {
        state
            .job_queue
            .get_inner()
            .update_setting("tos_accepted_version", version, "legal", false)
            .await?;
    }
    state
        .job_queue
        .get_inner()
        .update_setting("tos_accepted_at", &Utc::now().to_rfc3339(), "legal", false)
        .await?;

    // 4. Initialize SOUL programmatically
    let safe_name: String = ai_name
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(64)
        .collect();

    if let Err(e) = state.soul_mutator.generate_initial_soul(&safe_name).await {
        tracing::error!("Failed to generate initial soul during setup: {:?}", e);
        // We continue even if SOUL generation fails, as it's not fatal for account creation
    }

    // 5. Generate and return JWT
    let now = Utc::now().timestamp() as usize;
    let exp = now + 86400 * 30; // 30 days expiration for human UI login

    let claims = AiomeCustomClaims {
        sub: email_trimmed.to_string(),
        ekyc_verified: false,
        agent_id: uuid::uuid!("00000000-0000-0000-0000-000000000001"), // Local admin sentinel UUID
        roles: vec![Role::Admin],
        exp,
        iat: now,
        iss: "aiome_identity".to_string(),
    };

    let token = state.auth_manager.issue_token(claims).await.map_err(|e| {
        tracing::error!("Failed to issue token during setup: {:?}", e);
        AppError::internal("Token generation failed")
    })?;

    info!("🎉 Initial setup completed successfully for admin account");

    Ok(Json(SetupInitResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: 86400 * 30,
    }))
}
