/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core::biome::dialogue::DialogueManager;
use aiome_core::biome::{
    AutonomousBiomeEngine, AutonomousConfig, BiomeDialogue, BiomeMessage, DialogueStatus,
};
use aiome_core::traits::*; 
use axum::{extract::State, http::StatusCode, response::Json};
use sqlx::Row;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tracing::{error, info, warn};

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
        (status = 200, description = "Biome protocol status", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn biome_status(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<serde_json::Value>, AppError> {
    let peer_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM biome_peers")
        .fetch_one(state.job_queue.get_pool().get_sqlite_pool_or_err()?)
        .await
        .map_err(|e| aiome_core::error::AiomeError::Infrastructure {
            reason: format!("DB Error: {}", e),
        })?;

    Ok(Json(serde_json::json!({
        "status": "online",
        "peer_count": peer_count,
        "message_ja": "Biome プロトコル準備完了。AI同士の対話を待機中...",
        "message_en": "Biome protocol ready. Waiting for AI-to-AI dialogue..."
    })))
}

#[utoipa::path(
    get,
    path = "/api/biome/topics",
    responses(
        (status = 200, description = "List topics from Hub", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn list_topics(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<serde_json::Value>, AppError> {
    let hub_url = std::env::var("SAMSARA_HUB_URL")
        .unwrap_or_else(|_| shared::config::DEFAULT_SAMSARA_HUB_URL.to_string());
    let url = format!("{}/api/v1/hub/topics", hub_url);
    state.security_policy.validate_url(&hub_url).await?;

    let res = state
        .http_client
        .get(url)
        .send()
        .await
        .map_err(|e| aiome_core::error::AiomeError::RemoteServiceError {
            url: "Samsara Hub".into(),
            source: e.into(),
        })?
        .json::<serde_json::Value>()
        .await
        .map_err(
            |e| aiome_core::error::AiomeError::RemoteServiceExecutionFailed {
                reason: e.to_string(),
            },
        )?;

    Ok(Json(res))
}

#[utoipa::path(
    post,
    path = "/api/biome/topics",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Topic created", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn create_topic(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let hub_url = std::env::var("SAMSARA_HUB_URL")
        .unwrap_or_else(|_| shared::config::DEFAULT_SAMSARA_HUB_URL.to_string());
    let hub_secret = state
        .federation_secret
        .as_opt()
        .map(|s| secrecy::ExposeSecret::expose_secret(s.as_ref()).to_string())
        .ok_or_else(|| aiome_core::error::AiomeError::ConfigLoad {
            source: anyhow::anyhow!("FEDERATION_SECRET not configured"),
        })?;
    let client = (*state.http_client).clone();

    info!("🌟 [Biome] Requesting new topic creation on Hub: {:?}", req);
    state.security_policy.validate_url(&hub_url).await?;
    let res = client
        .post(format!("{}/api/v1/biome/topics", hub_url))
        .header("Authorization", format!("Bearer {}", hub_secret))
        .json(&req)
        .send()
        .await
        .map_err(|e| aiome_core::error::AiomeError::RemoteServiceError {
            url: "Samsara Hub".into(),
            source: e.into(),
        })?;

    let status = res.status();
    let body = res.json::<serde_json::Value>().await.map_err(|e| {
        aiome_core::error::AiomeError::RemoteServiceExecutionFailed {
            reason: e.to_string(),
        }
    })?;

    if status.is_success() {
        Ok(Json(body))
    } else {
        Err(aiome_core::error::AiomeError::RemoteServiceError {
            url: "Samsara Hub".into(),
            source: anyhow::anyhow!("Hub returned status {}: {:?}", status, body),
        }
        .into())
    }
}

#[utoipa::path(
    post,
    path = "/api/biome/autonomous/start",
    request_body = StartAutonomousRequest,
    responses(
        (status = 200, description = "Autonomous dialogue started", body = serde_json::Value),
        (status = 409, description = "Dialogue already running")
    ),
    security(("api_key" = []))
)]
pub async fn autonomous_start(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(req): Json<StartAutonomousRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if state.autonomous_running.load(Ordering::SeqCst) {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Autonomous dialogue already running".to_string(),
        }
        .into());
    }

    if let shared::guardrails::ValidationResult::Blocked(reason) =
        shared::guardrails::validate_input(&req.topic_id)
    {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: format!("Invalid topic_id: {}", reason),
        }
        .into());
    }
    if let shared::guardrails::ValidationResult::Blocked(reason) =
        shared::guardrails::validate_input(&req.peer_pubkey)
    {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: format!("Invalid peer_pubkey: {}", reason),
        }
        .into());
    }

    state.autonomous_running.store(true, Ordering::SeqCst);
    let config = aiome_core::biome::AutonomousConfig {
        topic_id: req.topic_id.clone(),
        peer_pubkey: req.peer_pubkey.clone(),
        interval_secs: req.interval_secs.unwrap_or(30),
        max_rounds: req.max_rounds.unwrap_or(10),
    };
    {
        let mut config_write = state.autonomous_config.write().await;
        *config_write = Some(config.clone());
    }

    info!(
        "🌐 [Biome] Started autonomous dialogue for topic: {}",
        req.topic_id
    );

    let queue = (*state.job_queue).clone();
    let llm = (*state.provider).clone();
    let running = (*state.autonomous_running).clone();
    let semaphore = (*state.llm_semaphore).clone();
    let gift_engine = (*state.gift_engine).clone();
    let master_email = state.config.master_email.clone();

    tokio::spawn(async move {
        AutonomousBiomeEngine::start_loop(
            config,
            queue,
            llm,
            running,
            semaphore,
            Some(gift_engine),
            master_email,
        )
        .await;
    });

    Ok(Json(serde_json::json!({"status": "started"})))
}

#[utoipa::path(
    post,
    path = "/api/biome/autonomous/stop",
    responses(
        (status = 200, description = "Stopping autonomous dialogue", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn autonomous_stop(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<serde_json::Value>, AppError> {
    state.autonomous_running.store(false, Ordering::SeqCst);
    Ok(Json(serde_json::json!({"status": "stopping"})))
}

#[utoipa::path(
    get,
    path = "/api/biome/autonomous/status",
    responses(
        (status = 200, description = "Current autonomous status", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn autonomous_status(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<serde_json::Value>, AppError> {
    let running = state.autonomous_running.load(Ordering::SeqCst);
    let config = state.autonomous_config.read().await;

    Ok(Json(serde_json::json!({
        "running": running,
        "config": *config
    })))
}

#[utoipa::path(
    get,
    path = "/api/biome/list",
    responses(
        (status = 200, description = "List biome messages", body = [serde_json::Value])
    ),
    security(("api_key" = []))
)]
pub async fn list_messages(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let rows = sqlx::query("SELECT * FROM biome_messages ORDER BY created_at DESC LIMIT 100")
        .fetch_all(state.job_queue.get_pool().get_sqlite_pool_or_err()?)
        .await
        .map_err(|e| aiome_core::error::AiomeError::Infrastructure {
            reason: format!("DB Error: {}", e),
        })?;

    let hub_secret_opt = state
        .federation_secret
        .as_opt()
        .map(|s| secrecy::ExposeSecret::expose_secret(s.as_ref()).to_string());

    let messages = rows
        .into_iter()
        .map(|row| {
            let mut msg = BiomeMessage {
                sender_pubkey: row.get::<String, _>("sender_pubkey"),
                recipient_pubkey: row.get::<String, _>("recipient_pubkey"),
                topic_id: row.get::<String, _>("topic_id"),
                content: row.get::<String, _>("content"),
                karma_root_cid: row.get::<String, _>("karma_root_cid"),
                signature: row.get::<String, _>("signature"),
                lamport_clock: row.get::<i64, _>("lamport_clock") as u64,
                timestamp: "".to_string(), // Not in DB or not needed for UI yet
                encryption: row.get::<String, _>("encryption"),
            };

            // Attempt decryption if encrypted and secret available
            if msg.encryption != "none" {
                if let Some(hub_secret) = &hub_secret_opt {
                    let key = shared::crypto::derive_biome_key(hub_secret);
                    let _ = msg.decrypt(&key);
                }
            }

            serde_json::json!({
                "id": row.get::<i64, _>("id"),
                "sender_pubkey": msg.sender_pubkey,
                "recipient_pubkey": msg.recipient_pubkey,
                "topic_id": msg.topic_id,
                "content": msg.content,
                "karma_root_cid": msg.karma_root_cid,
                "signature": msg.signature,
                "lamport_clock": msg.lamport_clock,
                "encryption": msg.encryption,
                "created_at": row.get::<Option<String>, _>("created_at"),
            })
        })
        .collect();

    Ok(Json(messages))
}

#[utoipa::path(
    post,
    path = "/api/biome/send",
    request_body = SendBiomeRequest,
    responses(
        (status = 200, description = "Message sent/relayed", body = serde_json::Value)
    ),
    security(("api_key" = []))
)]
pub async fn send_message(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(req): Json<SendBiomeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let sender_pubkey = state.job_queue.get_node_id().await?;
    let clock = state.job_queue.tick_local_clock().await?;

    if let shared::guardrails::ValidationResult::Blocked(reason) =
        shared::guardrails::validate_input(&req.topic_id)
    {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: format!("Invalid topic_id: {}", reason),
        }
        .into());
    }
    if let shared::guardrails::ValidationResult::Blocked(reason) =
        shared::guardrails::validate_input(&req.recipient_pubkey)
    {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: format!("Invalid recipient_pubkey: {}", reason),
        }
        .into());
    }
    if req.content.contains("data:image/")
        || req.content.contains("data:video/")
        || req.content.contains(";base64,")
    {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason:
                "Binary data and inline assets are strictly prohibited by protocol (CSAM Defense)"
                    .to_string(),
        }
        .into());
    }

    if let shared::guardrails::ValidationResult::Blocked(reason) =
        shared::guardrails::validate_input(&req.content)
    {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: format!("Invalid content: {}", reason),
        }
        .into());
    }

    // 0. Biome Dialogue Constraint Check
    let current_turn =
        match DialogueManager::check_and_advance_turn(&**state.job_queue, &req.topic_id).await {
            Ok(t) => t,
            Err(e) => {
                warn!("🚫 [Biome] Message blocked: {}", e);
                return Err(aiome_core::error::AiomeError::SecurityViolation {
                    reason: e.to_string(),
                }
                .into());
            }
        };

    // 1. Sign the message
    let payload_to_sign = format!("{}:{}:{}", sender_pubkey, req.topic_id, clock);
    let signature = state.job_queue.sign_swarm_payload(&payload_to_sign).await?;

    // Phase 20: Karma Root is derived from the signature of the turn
    let karma_root = format!("biom:{}", signature);

    let mut msg = BiomeMessage {
        sender_pubkey,
        recipient_pubkey: req.recipient_pubkey,
        topic_id: req.topic_id,
        content: req.content,
        karma_root_cid: karma_root,
        signature: signature.clone(),
        lamport_clock: clock,
        timestamp: chrono::Utc::now().to_rfc3339(),
        encryption: "none".to_string(),
    };

    // NG-28: Apply encryption if FEDERATION_SECRET is set
    let hub_secret = state
        .federation_secret
        .as_opt()
        .map(|s| secrecy::ExposeSecret::expose_secret(s.as_ref()).to_string())
        .ok_or_else(|| aiome_core::error::AiomeError::ConfigLoad {
            source: anyhow::anyhow!("FEDERATION_SECRET not configured for biome encryption"),
        })?;

    let key = shared::crypto::derive_biome_key(&hub_secret);
    msg.encrypt(&key)
        .map_err(|e| aiome_core::error::AiomeError::Infrastructure {
            reason: format!("Failed to encrypt biome message: {}", e),
        })?;

    // 2. Relay via Hub
    let hub_url = state.config.samsara_hub_url.clone();
    let client = state.http_client.clone();
    state.security_policy.validate_url(&hub_url).await?;

    info!(
        "🚀 [Biome] Sending message to Hub for relay (Topic: {})",
        msg.topic_id
    );
    let res = client
        .post(format!("{}/api/v1/biome/relay", hub_url))
        .header("Authorization", format!("Bearer {}", hub_secret))
        .json(&msg)
        .send()
        .await;

    let sent_status = match res {
        Ok(r) if r.status().is_success() => {
            // Store a copy in local history
            if let Err(e) = sqlx::query("INSERT INTO biome_messages (sender_pubkey, recipient_pubkey, topic_id, content, karma_root_cid, signature, lamport_clock, encryption) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(&msg.sender_pubkey).bind(&msg.recipient_pubkey).bind(&msg.topic_id).bind(&msg.content).bind(&msg.karma_root_cid).bind(&signature).bind(msg.lamport_clock as i64).bind(&msg.encryption)
                .execute(state.job_queue.get_pool().get_sqlite_pool_or_err()?).await {
                error!("🚨 [Biome] Failed to store sent message in local history: {}", e);
            }

            "sent"
        }
        _ => {
            // Hub unavailable or failed: Fallback to local
            warn!("⚠️ [Biome] Hub relay failed. Saving message locally as fallback.");
            if let Err(e) = sqlx::query("INSERT INTO biome_messages (sender_pubkey, recipient_pubkey, topic_id, content, karma_root_cid, signature, lamport_clock, encryption) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(&msg.sender_pubkey).bind(&msg.recipient_pubkey).bind(&msg.topic_id).bind(&msg.content).bind(&msg.karma_root_cid).bind(&signature).bind(msg.lamport_clock as i64).bind(&msg.encryption)
                .execute(state.job_queue.get_pool().get_sqlite_pool_or_err()?).await {
                error!("🚨 [Biome] Failed to store sent message locally (fallback): {}", e);
            }

            "sent_local_only"
        }
    };

    // 3. If this was the last turn, perform distillation (State Channel Closing)
    if current_turn >= aiome_core::biome::dialogue::MAX_DIALOGUE_TURNS {
        info!(
            "🔮 [Biome] Final turn reached for topic {}. Initiating distillation...",
            msg.topic_id
        );
        let queue = state.job_queue.clone();
        let llm = state.provider.clone();
        let topic_id = msg.topic_id.clone();

        tokio::spawn(async move {
            let _ = DialogueManager::distill_conversation(&**queue, &**llm, &topic_id).await;
        });
    }

    Ok(Json(
        serde_json::json!({"status": sent_status, "topic_id": msg.topic_id, "turn": current_turn}),
    ))
}
