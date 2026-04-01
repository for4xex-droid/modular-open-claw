/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use axum::extract::{Json, State};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Serialize, utoipa::ToSchema)]
pub struct EkycSessionResponse {
    pub session_url: String,
    pub session_id: String,
    pub status: String,
}

/// EKYC 検証セッションを開始・URL取得
#[utoipa::path(
    post,
    path = "/api/v1/ekyc/session",
    responses(
        (status = 200, description = "セッション開始成功", body = EkycSessionResponse),
        (status = 401, description = "認証エラー")
    ),
    security(
        ("jwt" = [])
    )
)]
pub async fn create_ekyc_session_handler(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
) -> Result<Json<EkycSessionResponse>, AppError> {
    info!(
        "🛡️ [eKYC] Initiating verification for agent: {}",
        auth.agent_id
    );

    let session = state
        .ekyc_engine
        .create_verification_session(&auth.agent_id.to_string())
        .await?;

    // セッションIDを保存
    state
        .ekyc_session_store
        .save(&auth.agent_id.to_string(), &session.session_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to save EKYC session: {}", e)))?;

    Ok(Json(EkycSessionResponse {
        session_url: session.url,
        session_id: session.session_id,
        status: "requires_input".to_string(),
    }))
}
