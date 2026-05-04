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
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use secrecy::ExposeSecret;

/// Samsara Hub への S2S 認証ヘッダーを生成する。
fn hub_auth_header(state: &AppState) -> String {
    format!("Bearer {}", state.federation_secret.expose_secret())
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct SendBiomeRequest {
    pub recipient_pubkey: String,
    pub topic_id: String,
    pub content: String,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct StartAutonomousRequest {
    pub topic_id: String,
    pub peer_pubkey: String,
    pub interval_secs: Option<u64>,
    pub max_rounds: Option<u32>,
}

#[utoipa::path(
    get,
    path = "/api/biome/status",
    responses(
        (status = 200, description = "Status retrieved", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn biome_status(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Response, AppError> {
    let url = format!("{}/api/v1/health", state.config.samsara_hub_url);
    let res = state
        .http_client
        .get(&url)
        .header("Authorization", hub_auth_header(&state))
        .send()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let status = res.status();
    let body = res.json::<serde_json::Value>().await.unwrap_or_else(|e| {
        tracing::warn!("Failed to parse JSON from Hub (status: {}): {}", status, e);
        serde_json::json!({})
    });
    Ok((status, Json(body)).into_response())
}

#[utoipa::path(
    get,
    path = "/api/biome/topics",
    responses(
        (status = 200, description = "Topics retrieved", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn list_topics(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Response, AppError> {
    let url = format!("{}/api/v1/biome/topics", state.config.samsara_hub_url);
    let res = state
        .http_client
        .get(&url)
        .header("Authorization", hub_auth_header(&state))
        .send()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let status = res.status();
    let body = res.json::<serde_json::Value>().await.unwrap_or_else(|e| {
        tracing::warn!("Failed to parse JSON from Hub (status: {}): {}", status, e);
        serde_json::json!({})
    });
    Ok((status, Json(body)).into_response())
}

#[utoipa::path(
    post,
    path = "/api/biome/topics",
    request_body = serde_json::Value,
    responses(
        (status = 201, description = "Topic created", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn create_topic(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(req): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    let url = format!("{}/api/v1/biome/topics", state.config.samsara_hub_url);
    let res = state
        .http_client
        .post(&url)
        .header("Authorization", hub_auth_header(&state))
        .json(&req)
        .send()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let status = res.status();
    let body = res.json::<serde_json::Value>().await.unwrap_or_else(|e| {
        tracing::warn!("Failed to parse JSON from Hub (status: {}): {}", status, e);
        serde_json::json!({})
    });
    Ok((status, Json(body)).into_response())
}

#[utoipa::path(
    post,
    path = "/api/biome/autonomous/start",
    request_body = StartAutonomousRequest,
    responses(
        (status = 200, description = "Started autonomous mode", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn autonomous_start(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(req): Json<StartAutonomousRequest>,
) -> Result<Response, AppError> {
    // clamp: 最小10秒(DoS防止), 最大3600秒(1時間の上限で暴走防止)
    let interval_secs = req.interval_secs.unwrap_or(60).clamp(10, 3600);
    // clamp: 最小1ラウンド(意味のない0を排除), 最大1000(リソース枯渇防止)
    let max_rounds = req.max_rounds.unwrap_or(10).clamp(1, 1000);
    state
        .autonomous_running
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let mut config = state.autonomous_config.write().await;
    *config = Some(aiome_core::biome::AutonomousConfig {
        topic_id: req.topic_id,
        peer_pubkey: req.peer_pubkey,
        interval_secs,
        max_rounds,
    });

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"status": "autonomous started"})),
    )
        .into_response())
}

#[utoipa::path(
    post,
    path = "/api/biome/autonomous/stop",
    responses(
        (status = 200, description = "Stopped autonomous mode", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn autonomous_stop(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Response, AppError> {
    state
        .autonomous_running
        .store(false, std::sync::atomic::Ordering::SeqCst);
    let mut config = state.autonomous_config.write().await;
    *config = None;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"status": "autonomous stopped"})),
    )
        .into_response())
}

#[utoipa::path(
    get,
    path = "/api/biome/autonomous/status",
    responses(
        (status = 200, description = "Autonomous status", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn autonomous_status(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Response, AppError> {
    let is_running = state
        .autonomous_running
        .load(std::sync::atomic::Ordering::SeqCst);
    let config = state.autonomous_config.read().await;

    let conf_val = match &*config {
        Some(c) => serde_json::json!({
            "topic_id": c.topic_id,
            "peer_pubkey": c.peer_pubkey,
            "interval_secs": c.interval_secs,
            "max_rounds": c.max_rounds
        }),
        None => serde_json::json!(null),
    };

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "running": is_running,
            "config": conf_val
        })),
    )
        .into_response())
}

#[utoipa::path(
    get,
    path = "/api/biome/list",
    responses(
        (status = 200, description = "List recent messages", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn list_messages(
    State(_state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Response, AppError> {
    // MVP: Return empty list until Hub provides a dedicated messages endpoint
    Ok((StatusCode::OK, Json(serde_json::json!([]))).into_response())
}

#[utoipa::path(
    post,
    path = "/api/biome/send",
    request_body = SendBiomeRequest,
    responses(
        (status = 200, description = "Message sent", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn send_message(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(req): Json<SendBiomeRequest>,
) -> Result<Response, AppError> {
    // Defense-in-Depth: プロキシ層でのコンテンツバリデーション
    // Hub 側でも検証するが、プロキシ層で早期拒否することで帯域とリソースを節約
    // 注: len() はバイト数。マルチバイト文字 (日本語等) では文字数 < バイト数
    const MAX_CONTENT_BYTES: usize = 8000;
    if req.content.len() > MAX_CONTENT_BYTES {
        tracing::warn!(
            "⚠️ [Biome] Content size limit exceeded: {} bytes (max: {})",
            req.content.len(),
            MAX_CONTENT_BYTES
        );
        return Err(AppError::bad_request(format!(
            "Content exceeds maximum size ({} > {} bytes)",
            req.content.len(),
            MAX_CONTENT_BYTES
        )));
    }

    // P2P ネットワークへのバイナリデータ埋め込みを禁止 (CSAM 防御)
    let lower_content = req.content.to_lowercase();
    if lower_content.contains("data:image/")
        || lower_content.contains("data:video/")
        || lower_content.contains(";base64,")
    {
        tracing::warn!(
            "🚨 [Biome] Binary data embedding attempt blocked (topic: {}, recipient: {})",
            req.topic_id,
            req.recipient_pubkey
        );
        return Err(AppError::bad_request(
            "Binary data embedding is prohibited in P2P messages".to_string(),
        ));
    }

    // Dynamic Toxicity / CSAM blocklist from settings
    let banned_words_setting = state
        .job_queue
        .get_setting_value("csam_toxicity_forbidden_words")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let banned_words: Vec<String> = banned_words_setting
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    if let Err(e) =
        infrastructure::job_queue::federation::P2pSanitizer::sanitize(&req.content, &banned_words)
    {
        return Err(AppError::bad_request(e.to_string()));
    }

    let payload = aiome_core::biome::BiomeMessage {
        topic_id: req.topic_id.clone(),
        sender_pubkey: state.system_agent_id.to_string(),
        recipient_pubkey: req.recipient_pubkey.clone(),
        content: req.content,
        // MVP: Hardcoded defaults until full Federation crypto is implemented
        karma_root_cid: "cid_local_relay".to_string(),
        signature: "unsigned_local_relay".to_string(),
        lamport_clock: 0,
        timestamp: chrono::Utc::now().to_rfc3339(),
        encryption: "none".to_string(),
    };

    let url = format!("{}/api/v1/biome/relay", state.config.samsara_hub_url);
    let res = state
        .http_client
        .post(&url)
        .header("Authorization", hub_auth_header(&state))
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let status = res.status();
    let body = res.json::<serde_json::Value>().await.unwrap_or_else(|e| {
        tracing::warn!("Failed to parse JSON from Hub (status: {}): {}", status, e);
        serde_json::json!({})
    });
    Ok((status, Json(body)).into_response())
}
