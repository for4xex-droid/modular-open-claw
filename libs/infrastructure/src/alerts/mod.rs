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
