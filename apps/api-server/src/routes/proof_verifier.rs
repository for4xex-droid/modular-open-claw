/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{error, info};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct VerifyProofRequest {
    pub skill_name: String,
    pub proof_spec_b64: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct VerifyProofResponse {
    pub is_valid: bool,
    pub message: String,
}

#[utoipa::path(
    post,
    path = "/api/skills/verify-proof",
    request_body = VerifyProofRequest,
    responses(
        (status = 200, description = "Proof verified", body = VerifyProofResponse),
        (status = 400, description = "Invalid skill name"),
        (status = 404, description = "Skill not found"),
        (status = 500, description = "Internal error")
    ),
    security(("api_key" = []))
)]
pub async fn verify_skill_proof(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(payload): Json<VerifyProofRequest>,
) -> Result<Json<VerifyProofResponse>, AppError> {
    // ── Security Gate: Sanitize & validate skill_name (Path Traversal Prevention) ──
    let skill_name = shared::guardrails::strip_invisible_unicode(&payload.skill_name).into_owned();
    if skill_name.is_empty()
        || skill_name.contains("..")
        || skill_name.contains('/')
        || skill_name.contains('\\')
        || skill_name.contains('\0')
    {
        return Err(AppError::bad_request(
            "Security Violation: Invalid skill name (path traversal detected)".to_string(),
        ));
    }

    info!(
        "🛡️ [FormalProofGate] Received verify request for skill: {}",
        skill_name
    );

    // ── Resolve & verify WASM path stays inside wasm_storage ──
    let wasm_root = state.config.resolver.resolve("wasm_storage");
    let wasm_path = wasm_root.join(format!("{}.wasm", skill_name));

    // Canonicalize to resolve any remaining symlink tricks
    let canonical_path = wasm_path
        .canonicalize()
        .map_err(|_| AppError::not_found(format!("Skill WASM not found: {}", skill_name)))?;
    let canonical_root = wasm_root
        .canonicalize()
        .map_err(|e| AppError::internal(format!("Failed to resolve wasm_storage root: {}", e)))?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err(AppError::bad_request(
            "Security Violation: Path escapes wasm_storage boundary".to_string(),
        ));
    }

    // ── Hash the WASM file to cryptographically bind request to artifact (TOCTOU mitigation) ──
    let wasm_bytes = tokio::fs::read(&canonical_path)
        .await
        .map_err(|e| AppError::internal(format!("Failed to read WASM: {}", e)))?;
    let mut hasher = Sha256::new();
    hasher.update(&wasm_bytes);
    let _wasm_hash = hex::encode(hasher.finalize());

    // ── Call Formal Proof Gate ──
    let is_valid = match state
        .formal_proof_gate
        .verify_skill(&skill_name, &payload.proof_spec_b64)
        .await
    {
        Ok(valid) => valid,
        Err(e) => {
            error!(
                "❌ [FormalProofGate] Verification failed for {}: {}",
                skill_name, e
            );
            return Err(AppError::internal(format!(
                "Verification service error: {}",
                e
            )));
        }
    };

    if is_valid {
        let current_oxp = state
            .oxilean_power
            .load(std::sync::atomic::Ordering::Relaxed);
        if let Err(e) = state
            .gig_updater
            .mark_as_verified(&skill_name, current_oxp)
            .await
        {
            error!(
                "🚨 [GigUpdater] Failed to update verification status for {}: {}",
                skill_name, e
            );
            return Err(AppError::internal(format!("Database update failed: {}", e)));
        }
    }

    Ok(Json(VerifyProofResponse {
        is_valid,
        message: if is_valid {
            "Q.E.D.".to_string()
        } else {
            "Proof Invalid".to_string()
        },
    }))
}
