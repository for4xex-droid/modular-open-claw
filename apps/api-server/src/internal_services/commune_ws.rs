/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use secrecy::ExposeSecret;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};
use url::Url;

use crate::AppState;
use aiome_core::contracts::HubMessage;
use aiome_core_contracts::{CommuneRegistry, JobQueue, SettingsOps};
use shared::sql_fetch_optional;

/// Hub への WebSocket クライアントを管理する構造体
pub struct CommuneWsClient {
    state: AppState,
}

impl CommuneWsClient {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// `SupervisedTask` から呼ばれる、バックグラウンドでの WebSocket 受信ループの開始
    pub async fn run_supervised(self, ct: tokio_util::sync::CancellationToken) {
        let mut backoff = Duration::from_secs(1);
        loop {
            tokio::select! {
                _ = ct.cancelled() => {
                    info!("🛑 [CommuneWsClient] Shutdown requested, exiting loop.");
                    return;
                }
                _ = async {
                    // 1. 自分が待ち受けるべきチャネルIDを取得
                    let channel_id_opt = match self.get_my_channel_id().await {
                        Ok(Some(id)) => Some(id),
                        Ok(None) => {
                            sleep(Duration::from_secs(10)).await;
                            None
                        }
                        Err(e) => {
                            error!("🚨 [CommuneWsClient] Failed to fetch channel ID: {}", e);
                            sleep(Duration::from_secs(10)).await;
                            None
                        }
                    };

                    if let Some(channel_id) = channel_id_opt {
                        info!("🔌 [CommuneWsClient] Connecting to Hub for channel: {}", channel_id);
                        match self.connect_and_loop_supervised(&channel_id, &ct).await {
                            Ok(_) => {
                                info!("🔌 [CommuneWsClient] Connection closed gracefully. Reconnecting...");
                                backoff = Duration::from_secs(1);
                            }
                            Err(e) => {
                                error!("🚨 [CommuneWsClient] Connection error: {}. Retrying in {:?}...", e, backoff);
                                tokio::select! {
                                    _ = ct.cancelled() => {}
                                    _ = sleep(backoff) => {
                                        backoff = std::cmp::min(backoff * 2, Duration::from_secs(30));
                                    }
                                }
                            }
                        }
                    }
                } => {}
            }
        }
    }

    /// 自身のメタデータフリー通信用のチャネルIDをDB（system_settings）から取得する
    async fn get_my_channel_id(&self) -> Result<Option<String>, String> {
        let pool = self.state.db_pool.get_inner();
        const Q_SQLITE: &str = "SELECT value FROM system_settings WHERE key = ?";
        const Q_PG: &str = "SELECT value FROM system_settings WHERE key = $1";

        let opt: Option<String> = sql_fetch_optional!(
            &**pool,
            (String,),
            sqlite: Q_SQLITE,
            pg: Q_PG,
            "metadata_free_receive_channel"
        )
        .unwrap_or(None)
        .map(|r| r.0);

        Ok(opt)
    }

    /// ノードの秘密鍵（シード）をDBから取得する
    async fn get_node_seed(&self) -> Result<zeroize::Zeroizing<[u8; 32]>, String> {
        let pool = self.state.db_pool.get_inner();
        const Q_SQLITE: &str = "SELECT value FROM system_state WHERE key = ?";
        const Q_PG: &str = "SELECT value FROM system_state WHERE key = $1";

        let opt: Option<String> = sql_fetch_optional!(
            &**pool,
            (String,),
            sqlite: Q_SQLITE,
            pg: Q_PG,
            "node_privkey"
        )
        .unwrap_or(None)
        .map(|r| r.0);

        if let Some(privkey_b64) = opt {
            let priv_bytes = BASE64_STANDARD
                .decode(privkey_b64)
                .map_err(|e| format!("Corrupt node key: {}", e))?;
            if priv_bytes.len() != 32 {
                return Err("Corrupt node key (invalid length)".to_string());
            }
            let mut seed = zeroize::Zeroizing::new([0u8; 32]);
            seed.copy_from_slice(&priv_bytes);
            Ok(seed)
        } else {
            Err("Node private key not found".to_string())
        }
    }

    /// WebSocket の接続および受信イベントループ
    async fn connect_and_loop_supervised(
        &self,
        channel_id: &str,
        ct: &tokio_util::sync::CancellationToken,
    ) -> Result<(), String> {
        let mut base_url = self.state.config.samsara_hub_url.clone();
        if base_url.starts_with("http://") {
            base_url = base_url.replace("http://", "ws://");
        } else if base_url.starts_with("https://") {
            base_url = base_url.replace("https://", "wss://");
        }

        let ws_url = format!(
            "{}/api/v1/federation/ws?channel_local_id={}",
            base_url, channel_id
        );
        let request_url = Url::parse(&ws_url).map_err(|e| format!("Invalid WS URL: {}", e))?;

        let token = self.state.federation_secret.expose_secret();

        let request = axum::http::Request::builder()
            .method("GET")
            .uri(&ws_url)
            .header(
                axum::http::header::AUTHORIZATION,
                format!("Bearer {}", token),
            )
            .header("Host", request_url.host_str().unwrap_or(""))
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .body(())
            .map_err(|e| e.to_string())?;

        let (ws_stream, _) = connect_async(request)
            .await
            .map_err(|e| format!("WebSocket handshake failed: {}", e))?;

        let (mut write, mut read) = ws_stream.split();

        // 30秒ごとの Ping 送信
        let mut ping_interval = tokio::time::interval(Duration::from_secs(30));

        let mut loop_err = None;

        loop {
            tokio::select! {
                _ = ct.cancelled() => {
                    let _ = write.send(Message::Close(None)).await;
                    break;
                }
                _ = ping_interval.tick() => {
                    if write.send(Message::Ping(axum::body::Bytes::new())).await.is_err() {
                        loop_err = Some("Failed to send ping".to_string());
                        break;
                    }
                }
                msg_opt = read.next() => {
                    match msg_opt {
                        Some(Ok(Message::Text(text))) => {
                            if let Err(e) = self.handle_incoming_text(&text).await {
                                error!("🚨 [CommuneWsClient] Error handling message: {}", e);
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            break;
                        }
                        Some(Err(e)) => {
                            loop_err = Some(format!("Read error: {}", e));
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(err) = loop_err {
            Err(err)
        } else {
            Ok(())
        }
    }

    /// 受信したテキストメッセージの処理
    async fn handle_incoming_text(&self, text: &str) -> Result<(), String> {
        let hub_msg: HubMessage =
            serde_json::from_str(text).map_err(|e| format!("Failed to parse HubMessage: {}", e))?;

        if let HubMessage::ZeroMetadataCommuneRelay(envelope) = hub_msg {
            info!(
                "🔒 [CommuneWsClient] Received metadata-free envelope for channel: {}",
                envelope.channel_local_id
            );

            let seed = self.get_node_seed().await?;
            let decrypted_msg = shared::crypto::decrypt_commune_envelope(&envelope, &*seed)
                .map_err(|e| format!("Failed to decrypt envelope: {}", e))?;

            let banned_words_setting = match self
                .state
                .job_queue
                .get_setting_value("csam_toxicity_forbidden_words")
                .await
            {
                Ok(Some(v)) => v,
                Ok(None) => String::new(),
                Err(e) => {
                    warn!("⚠️ Failed to fetch CSAM forbidden words: {}. Proceeding with empty blocklist.", e);
                    String::new()
                }
            };
            let banned_words: Vec<String> = banned_words_setting
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();

            if let Err(e) = infrastructure::job_queue::federation::P2pSanitizer::sanitize(
                &decrypted_msg.content,
                &banned_words,
            ) {
                return Err(format!(
                    "CSAM/Toxicity sanitization blocked incoming message: {}",
                    e
                ));
            }

            let clock = self
                .state
                .job_queue
                .tick_local_clock()
                .await
                .map_err(|e| format!("Clock tick failed: {}", e))?;

            let mut msg_to_store = decrypted_msg;
            let _ = self
                .state
                .job_queue
                .sync_local_clock(msg_to_store.lamport_clock)
                .await;
            msg_to_store.lamport_clock = clock;

            self.state
                .job_queue
                .store_commune_message(&msg_to_store)
                .await
                .map_err(|e| format!("Failed to store commune message: {}", e))?;

            info!(
                "📬 [CommuneWsClient] Stored decrypted commune message from topic: {}",
                msg_to_store.topic_id
            );
        }

        Ok(())
    }
}
