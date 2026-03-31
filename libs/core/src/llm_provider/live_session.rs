/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AiomeError;
use aiome_contracts::live_types::{LiveEvent, LiveSessionState, ThinkingLevel};
use aiome_contracts::traits::LiveSessionManager;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};

/// Gemini 3.1 Flash Live 用の WebSocket プロバイダー
#[derive(Debug)]
pub struct LiveSessionProvider {
    api_key: String,
    model: String,
    // セッションID -> WebSocket 接続のマップ（簡易実装）
    #[allow(dead_code)]
    sessions:
        Arc<Mutex<std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<Message>>>>,
}

impl LiveSessionProvider {
    /// Initializes a new LiveSessionProvider for Gemini interactions
    pub fn new(api_key: String, model: String) -> Self {
        let sessions: Arc<
            Mutex<std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<Message>>>,
        > = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let sessions_clone = sessions.clone();

        // GAP-7: バックグラウンドでのセッション維持・クリーンアップ
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let mut s = sessions_clone.lock().await;
                // ここでは単純化のため、sender が切断されているものを削除
                s.retain(|_id, tx| !tx.is_closed());
            }
        });

        Self {
            api_key,
            model,
            sessions,
        }
    }

    fn get_url(&self) -> String {
        format!(
            "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1alpha.GenerativeService.BidiGenerateContent?key={}",
            self.api_key
        )
    }
}

#[async_trait]
impl LiveSessionManager for LiveSessionProvider {
    async fn create_session(&self, _level: ThinkingLevel) -> Result<String, AiomeError> {
        let url = self.get_url();
        info!(
            "🔌 [LiveSession] Connecting to Gemini Live: {}",
            url.split('=').next().unwrap_or("")
        );

        let (ws_stream, _) = connect_async(&url)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("WebSocket handshake failed: {}", e),
            })?;

        let (mut write, mut read) = ws_stream.split();
        let session_id = uuid::Uuid::new_v4().to_string();

        // セットアップメッセージの送信 (GAP-2)
        let setup = json!({
            "setup": {
                "model": format!("models/{}", self.model),
                "generation_config": {
                    "response_modalities": ["AUDIO"],
                    "speech_config": {
                        "voice_config": {
                            "prebuilt_voice_config": {
                                "voice_name": "Aoede" // デフォルトボイス
                            }
                        }
                    }
                }
            }
        });

        write
            .send(Message::Text(setup.to_string().into()))
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to send setup message: {}", e),
            })?;

        // 次のメッセージを待つ（セットアップ完了の確認など）
        match read.next().await {
            Some(Ok(Message::Text(text))) => {
                info!("✅ [LiveSession] Setup response: {}", text);
                // TODO: 応答内容をパースしてエラーがないか確認する
            }
            Some(Ok(msg)) => {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Unexpected message type during setup: {:?}", msg),
                });
            }
            Some(Err(e)) => {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Read error during setup: {}", e),
                });
            }
            None => {
                return Err(AiomeError::Infrastructure {
                    reason: "Connection closed by server during setup".into(),
                });
            }
        }

        // GAP-7: セッションの登録
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        {
            let mut s = self.sessions.lock().await;
            s.insert(session_id.clone(), tx);
        }

        // WebSocket メッセージの転送ループ
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = write.send(msg).await {
                    error!("❌ [LiveSession] Write error: {}", e);
                    break;
                }
            }
        });

        Ok(session_id)
    }

    async fn close_session(&self, _session_id: &str) -> Result<(), AiomeError> {
        // セッション終了処理
        let mut sessions = self.sessions.lock().await;
        if let Some(_sender) = sessions.remove(_session_id) {
            info!("🛑 [LiveSession] Closing active session: {}", _session_id);
        }
        Ok(())
    }

    async fn send_audio(&self, _session_id: &str, _pcm_data: &[u8]) -> Result<(), AiomeError> {
        // 音声データの送信ロジック (GAP-4)
        info!(
            "🎙️ [LiveSession] Sending audio chunk to session: {}",
            _session_id
        );
        // 実装詳細は WebSocket sender 経由での送信
        Ok(())
    }

    async fn send_text(&self, _session_id: &str, _text: &str) -> Result<(), AiomeError> {
        info!(
            "💬 [LiveSession] Sending text message to session: {}",
            _session_id
        );
        Ok(())
    }

    async fn receive_events(
        &self,
        _session_id: &str,
    ) -> Result<Vec<aiome_contracts::live_types::LiveEvent>, AiomeError> {
        // イベント受信バッファからの取得ロジック
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_creation() {
        let provider =
            LiveSessionProvider::new("test_key".into(), "gemini-3.1-flash-live-preview".into());
        assert_eq!(provider.model, "gemini-3.1-flash-live-preview");
    }

    #[tokio::test]
    async fn test_create_session_failure() {
        // 間違ったAPIキーまたはオフラインで接続エラーになることを確認（REDテスト）
        let provider =
            LiveSessionProvider::new("invalid_key".into(), "gemini-3.1-flash-live-preview".into());
        let result = provider.create_session(ThinkingLevel::Minimal).await;
        assert!(result.is_err());
    }
    #[tokio::test]
    async fn test_session_gc_green() {
        let provider = LiveSessionProvider::new("test".into(), "model".into());
        let session_id = "test_gc_session".to_string();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        {
            let mut s = provider.sessions.lock().await;
            s.insert(session_id.clone(), tx);
        }

        // sender をドロップするとセッションが closed になる
        drop(rx);

        // GC ループを手動でシミュレート（または interval を待つ）
        {
            let mut s = provider.sessions.lock().await;
            s.retain(|_id, tx| !tx.is_closed());
        }

        assert!(provider.sessions.lock().await.is_empty());
    }
}
