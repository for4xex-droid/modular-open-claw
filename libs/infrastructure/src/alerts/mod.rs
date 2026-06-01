/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use async_trait::async_trait;
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[cfg(test)]
mod tests;

/// アラートの重要度レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum AlertLevel {
    /// 情報レベル
    Info,
    /// 警告レベル
    Warning,
    /// 致命的な障害レベル
    Critical,
}

/// アラート通知の送信先トレイト
#[async_trait]
pub trait AlertNotifier: Send + Sync {
    /// アラートを送信する
    async fn send_alert(
        &self,
        title: &str,
        message: &str,
        level: AlertLevel,
    ) -> Result<(), AiomeError>;
}

/// アラート通知の統合管理者
pub struct AlertManager {
    notifiers: Arc<RwLock<Vec<Arc<dyn AlertNotifier>>>>,
    // アラートの重複送信を防ぐデバウンスキャッシュ (タイトル + レベル ➔ 送信時刻)
    debounce_cache: Arc<RwLock<std::collections::HashMap<(String, AlertLevel), Instant>>>,
    debounce_duration: Duration,
}

impl AlertManager {
    /// 新規マネージャーを作成する
    pub fn new() -> Self {
        Self {
            notifiers: Arc::new(RwLock::new(Vec::new())),
            debounce_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            debounce_duration: Duration::from_secs(60), // 同一アラートは60秒間デバウンス
        }
    }

    /// 送信先（Notifier）を登録する
    pub async fn register_notifier(&self, notifier: Arc<dyn AlertNotifier>) {
        let mut notifiers = self.notifiers.write().await;
        notifiers.push(notifier);
    }

    /// アラートをトリガーする
    pub async fn trigger_alert(
        &self,
        title: &str,
        message: &str,
        level: AlertLevel,
    ) -> Result<(), AiomeError> {
        // デバウンスチェック & キャッシュクリーンアップ (OOM回避用の自動 Eviction)
        {
            let cache_key = (title.to_string(), level);
            let mut cache = self.debounce_cache.write().await;

            // 期限切れのエントリをスキャンして削除
            let debounce_dur = self.debounce_duration;
            cache.retain(|_, last_sent| last_sent.elapsed() < debounce_dur);

            if let Some(last_sent) = cache.get(&cache_key) {
                if last_sent.elapsed() < self.debounce_duration {
                    tracing::debug!(
                        "🔇 [AlertManager] Debounced duplicate alert: [{:?}] {}",
                        level,
                        title
                    );
                    return Ok(());
                }
            }
            cache.insert(cache_key, Instant::now());
        }

        let notifiers = self.notifiers.read().await;

        // 各通知先にアラートを送信 (Fail-Safe: 1つが落ちても他へは送り続ける)
        for notifier in notifiers.iter() {
            let notifier = notifier.clone();
            let title = title.to_string();
            let message = message.to_string();

            tokio::spawn(async move {
                if let Err(e) = notifier.send_alert(&title, &message, level).await {
                    tracing::error!("❌ [AlertManager] Notifier failed to send alert: {:?}", e);
                }
            });
        }

        Ok(())
    }
}

impl std::fmt::Debug for AlertManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlertManager")
            .field("debounce_duration", &self.debounce_duration)
            .finish()
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Discord Webhook を使用したアラート通知送信機
///
/// グローバル HTTP クライアント (`aiome_core::http::get_http_client()`) を使用し、
/// SSRF 保護と TCP 接続プールの恩恵を受ける。
pub struct DiscordNotifier;

impl DiscordNotifier {
    /// 新規 DiscordNotifier を作成する
    pub fn new() -> Self {
        Self
    }
}

impl Default for DiscordNotifier {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DiscordNotifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscordNotifier").finish()
    }
}

#[async_trait]
impl AlertNotifier for DiscordNotifier {
    async fn send_alert(
        &self,
        title: &str,
        message: &str,
        level: AlertLevel,
    ) -> Result<(), AiomeError> {
        // 環境変数から Webhook URL を取得
        let webhook_url = match std::env::var("DISCORD_WEBHOOK_URL") {
            Ok(url) if !url.is_empty() => url,
            _ => {
                // 未設定時はエラーにせず、フェイルセーフとして警告ログを出力し Ok(()) で早期リターン
                tracing::warn!("⚠️ [DiscordNotifier] DISCORD_WEBHOOK_URL is not set. Skipping alert notification.");
                return Ok(());
            }
        };

        // AlertLevel に応じた Discord Embed 用のカラーコード (10進数)
        // Info: 緑 (0x2ECC71 = 3066993)
        // Warning: 黄 (0xF1C40F = 15848463)
        // Critical: 赤 (0xE74C3C = 15158332)
        let color = match level {
            AlertLevel::Info => 3066993,
            AlertLevel::Warning => 15848463,
            AlertLevel::Critical => 15158332,
        };

        // 送信ペイロードの構築
        let payload = serde_json::json!({
            "embeds": [
                {
                    "title": title,
                    "description": message,
                    "color": color,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }
            ]
        });

        // グローバル HTTP クライアントを使用 (SSRF 保護 + TCP 接続プール)
        let client = aiome_core::http::get_http_client();

        // HTTP POST リクエスト送信 (リクエスト単位タイムアウト: 10秒)
        let response = client
            .post(&webhook_url)
            .timeout(Duration::from_secs(10))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to send Discord Webhook POST: {:?}", e),
            })?;

        // レスポンスステータスの検証
        if !response.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Discord Webhook API returned failure status: {}",
                    response.status()
                ),
            });
        }

        Ok(())
    }
}
