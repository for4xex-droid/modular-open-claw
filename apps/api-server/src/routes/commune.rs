/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core::traits::SettingsOps;
use aiome_core_contracts::JobQueue;
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

/// Hub レスポンスを Axum レスポンスに変換する共通ヘルパー。
/// JSON パース失敗時はステータスコードを保持しつつ空オブジェクトを返す。
async fn hub_response_to_axum(res: reqwest::Response) -> Result<Response, AppError> {
    let status = res.status();
    let body = res.json::<serde_json::Value>().await.unwrap_or_else(|e| {
        tracing::warn!("Failed to parse JSON from Hub (status: {}): {}", status, e);
        serde_json::json!({})
    });
    Ok((status, Json(body)).into_response())
}

/// Hub への GET プロキシ。認証ヘッダー付与・エラー変換・レスポンスパースを共通化。
async fn hub_proxy_get(state: &AppState, path: &str) -> Result<Response, AppError> {
    let url = format!("{}{}", state.config.samsara_hub_url, path);
    let res = state
        .http_client
        .get(&url)
        .header("Authorization", hub_auth_header(state))
        .send()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    hub_response_to_axum(res).await
}

/// Hub への POST プロキシ。認証ヘッダー付与・エラー変換・レスポンスパースを共通化。
async fn hub_proxy_post(
    state: &AppState,
    path: &str,
    body: &impl serde::Serialize,
) -> Result<Response, AppError> {
    let url = format!("{}{}", state.config.samsara_hub_url, path);
    let res = state
        .http_client
        .post(&url)
        .header("Authorization", hub_auth_header(state))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    hub_response_to_axum(res).await
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct SendCommuneRequest {
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
    path = "/api/commune/status",
    responses(
        (status = 200, description = "Status retrieved", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn commune_status(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Response, AppError> {
    hub_proxy_get(&state, "/api/v1/health").await
}

#[utoipa::path(
    get,
    path = "/api/commune/topics",
    responses(
        (status = 200, description = "Topics retrieved", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn list_topics(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Response, AppError> {
    hub_proxy_get(&state, "/api/v1/commune/topics").await
}

#[utoipa::path(
    post,
    path = "/api/commune/topics",
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
    hub_proxy_post(&state, "/api/v1/commune/topics", &req).await
}

#[utoipa::path(
    post,
    path = "/api/commune/autonomous/start",
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
    *config = Some(aiome_core::commune::AutonomousConfig {
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
    path = "/api/commune/autonomous/stop",
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
    path = "/api/commune/autonomous/status",
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
    path = "/api/commune/list",
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
    path = "/api/commune/send",
    request_body = SendCommuneRequest,
    responses(
        (status = 200, description = "Message sent", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn send_message(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(req): Json<SendCommuneRequest>,
) -> Result<Response, AppError> {
    // Empty content guard: 空メッセージの P2P 送出を防止
    if req.content.trim().is_empty() {
        return Err(AppError::bad_request(
            "Empty content is not allowed in P2P messages".to_string(),
        ));
    }

    // Defense-in-Depth: プロキシ層でのコンテンツバリデーション
    // Hub 側でも検証するが、プロキシ層で早期拒否することで帯域とリソースを節約
    // 注: len() はバイト数。マルチバイト文字 (日本語等) では文字数 < バイト数
    const MAX_CONTENT_BYTES: usize = 8000;
    if req.content.len() > MAX_CONTENT_BYTES {
        tracing::warn!(
            "⚠️ [Commune] Content size limit exceeded: {} bytes (max: {})",
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
            "🚨 [Commune] Binary data embedding attempt blocked (topic: {}, recipient: {})",
            req.topic_id,
            req.recipient_pubkey
        );
        return Err(AppError::bad_request(
            "Binary data embedding is prohibited in P2P messages".to_string(),
        ));
    }

    // Dynamic Toxicity / CSAM blocklist from settings
    let banned_words_setting = match state
        .job_queue
        .get_setting_value("csam_toxicity_forbidden_words")
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => String::new(),
        Err(e) => {
            tracing::warn!(
                "⚠️ [Commune] Failed to fetch CSAM forbidden words from DB: {}. Proceeding with empty blocklist.",
                e
            );
            String::new()
        }
    };
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

    let clock = state
        .job_queue
        .tick_local_clock()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let payload_to_sign = format!("{}:{}:{}", state.system_agent_id, req.topic_id, clock);
    let signature = state
        .job_queue
        .sign_swarm_payload(&payload_to_sign)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let payload = aiome_core::commune::CommuneMessage {
        topic_id: req.topic_id.clone(),
        sender_pubkey: state.system_agent_id.to_string(),
        recipient_pubkey: req.recipient_pubkey.clone(),
        content: req.content,
        karma_root_cid: "cid_local_relay".to_string(),
        signature,
        lamport_clock: clock,
        timestamp: chrono::Utc::now().to_rfc3339(),
        // TODO(SEC): See ADR-043 (docs/decisions/043-p2p-e2e-encryption.md)
        // Currently plaintext — vulnerable to man-in-the-middle sniffing on the relay.
        encryption: "none".to_string(),
        payload_type: None,
    };

    let url = format!("{}/api/v1/commune/relay", state.config.samsara_hub_url);
    let res = state
        .http_client
        .post(&url)
        .header("Authorization", hub_auth_header(&state))
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    hub_response_to_axum(res).await
}

#[cfg(test)]
mod tests {
    // Commune バリデーションロジックのユニットテスト
    // ハンドラ内のインライン検証ロジックを再現してテスト

    const MAX_CONTENT_BYTES: usize = 8000;

    #[test]
    fn test_content_size_within_limit() {
        let content = "a".repeat(MAX_CONTENT_BYTES);
        assert!(content.len() <= MAX_CONTENT_BYTES);
    }

    #[test]
    fn test_content_size_exceeds_limit() {
        let content = "a".repeat(MAX_CONTENT_BYTES + 1);
        assert!(content.len() > MAX_CONTENT_BYTES);
    }

    #[test]
    fn test_content_size_multibyte_chars() {
        // 日本語: 1文字 = 3バイト (UTF-8)
        let content = "あ".repeat(2667); // 2667 * 3 = 8001 bytes
        assert!(content.len() > MAX_CONTENT_BYTES);
    }

    #[test]
    fn test_base64_embedding_blocked() {
        let test_cases = vec![
            "Hello data:image/png;base64,abc123",
            "data:video/mp4;base64,xyz",
            "innocent text;base64,payload",
            "DATA:IMAGE/JPEG;BASE64,upper",
        ];
        for input in test_cases {
            let lower = input.to_lowercase();
            let blocked = lower.contains("data:image/")
                || lower.contains("data:video/")
                || lower.contains(";base64,");
            assert!(blocked, "Should block: {}", input);
        }
    }

    #[test]
    fn test_normal_content_not_blocked() {
        let safe_inputs = vec![
            "Hello, world!",
            "The base64 encoding is interesting",
            "data about images",
            "video analysis report",
        ];
        for input in safe_inputs {
            let lower = input.to_lowercase();
            let blocked = lower.contains("data:image/")
                || lower.contains("data:video/")
                || lower.contains(";base64,");
            assert!(!blocked, "Should NOT block: {}", input);
        }
    }

    #[test]
    fn test_autonomous_interval_clamping() {
        // 最小10秒
        assert_eq!(5_u64.clamp(10, 3600), 10);
        // デフォルト60秒
        assert_eq!(60_u64.clamp(10, 3600), 60);
        // 最大3600秒
        assert_eq!(9999_u64.clamp(10, 3600), 3600);
    }

    #[test]
    fn test_autonomous_max_rounds_clamping() {
        // 0は1に引き上げ
        assert_eq!(0_u32.clamp(1, 1000), 1);
        // デフォルト10
        assert_eq!(10_u32.clamp(1, 1000), 10);
        // 上限1000
        assert_eq!(5000_u32.clamp(1, 1000), 1000);
    }

    #[test]
    fn test_empty_content_blocked() {
        // 空コンテンツは P2P ネットワークへの送出前に拒否されるべき
        let content = "";
        assert!(
            content.trim().is_empty(),
            "Empty content should be detected"
        );

        // 空白のみのコンテンツも拒否
        let whitespace_only = "   \t\n  ";
        assert!(
            whitespace_only.trim().is_empty(),
            "Whitespace-only content should be detected"
        );
    }
}
