/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AiomeError;
use crate::expression::Expression;
use aiome_core_contracts::expression::TtsStatus;
use async_trait::async_trait;
use tracing::{error, info};

/// TTS ワーカーが必要とする最小限のキューインターフェース (ISP)。
///
/// `JobQueue` の God Trait を直接モック化するのが困難なため、
/// TTS ワーカーが実際に使用する2つのメソッドのみを抽出しています。
/// `JobQueue` を実装する全ての型に対して blanket impl が提供されます。
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TtsQueue: Send + Sync {
    /// 保留中の Expression を最大 `limit` 件取得する。
    async fn fetch_expressions(&self, limit: i64) -> Result<Vec<Expression>, AiomeError>;
    /// Expression の状態を永続化する。
    async fn store_expression(&self, expression: &Expression) -> Result<(), AiomeError>;
}

#[async_trait]
impl<T: aiome_core_contracts::traits::JobQueue + ?Sized> TtsQueue for T {
    async fn fetch_expressions(&self, limit: i64) -> Result<Vec<Expression>, AiomeError> {
        aiome_core_contracts::traits::JobQueue::fetch_expressions(self, limit).await
    }
    async fn store_expression(&self, expression: &Expression) -> Result<(), AiomeError> {
        aiome_core_contracts::traits::JobQueue::store_expression(self, expression).await
    }
}

/// TTS生成をバックグラウンドで非同期に処理するワーカー (Phase 10.1a)。
pub struct TtsWorker;

impl TtsWorker {
    /// 未処理のExpressionをスキャンし、TTS生成を実行する
    ///
    /// # Arguments
    /// * `queue` - Expressionの保存・取得・更新を行うジョブキュー
    /// * `tts_provider` - TTSプロバイダー
    /// * `speaker_id` - 使用する話者ID
    /// * `artifacts_root` - 音声ファイルを保存するルートディレクトリ
    pub async fn process_pending_tts(
        queue: &dyn TtsQueue,
        tts_provider: &dyn aiome_core_contracts::traits::TtsProvider,
        speaker_id: &str,
        artifacts_root: &std::path::Path,
    ) -> Result<usize, AiomeError> {
        // 1. Fetch expressions (using a simpler approach since JobQueue doesn't have a status filter for expressions yet)
        let expressions = queue.fetch_expressions(50).await?;
        let pending: Vec<Expression> = expressions
            .into_iter()
            .filter(|e| e.tts_status == TtsStatus::NotRequested)
            .collect();

        if pending.is_empty() {
            return Ok(0);
        }

        info!(
            "🔊 [TtsWorker] Processing {} pending TTS requests",
            pending.len()
        );
        let mut processed = 0;

        for mut expr in pending {
            info!("🔊 [TtsWorker] Generating TTS for expression {}", expr.id);

            // 2. Mark as Generating
            expr.tts_status = TtsStatus::Generating;
            if let Err(e) = queue.store_expression(&expr).await {
                error!(
                    "🚨 [TtsWorker] Failed to update expression status to Generating: {:?}",
                    e
                );
                continue;
            }

            // 3. Call TtsProvider instead of hardcoded XTTS
            match tts_provider.synthesize(&expr.content, speaker_id).await {
                Ok(audio_bytes) => {
                    // 4. Save file to disk
                    let file_name = format!("tts_{}.wav", expr.id);
                    let file_path = artifacts_root.join(&file_name);

                    if let Err(e) = std::fs::write(&file_path, &audio_bytes) {
                        error!(
                            "🚨 [TtsWorker] Failed to write audio file to {:?}: {:?}",
                            file_path, e
                        );
                        expr.tts_status = TtsStatus::Failed;
                        if let Err(e) = queue.store_expression(&expr).await {
                            error!(
                                "🚨 [TtsWorker] Failed to persist TTS failure status for {}: {:?}",
                                expr.id, e
                            );
                        }
                        continue;
                    }

                    // 5. Update Expression record
                    expr.audio_path = Some(file_path.to_string_lossy().to_string());
                    // Rough duration estimation (bytes / bytes_per_sec)
                    // 16kHz 16bit mono is 32000 bytes/sec
                    expr.duration_ms = Some((audio_bytes.len() as f64 / 32.0) as i32);
                    expr.tts_status = TtsStatus::Ready;

                    if let Err(e) = queue.store_expression(&expr).await {
                        error!(
                            "🚨 [TtsWorker] Failed to update final expression record: {:?}",
                            e
                        );
                        continue;
                    }

                    info!(
                        "✅ [TtsWorker] TTS Ready for {}: {} bytes",
                        expr.id,
                        audio_bytes.len()
                    );
                    processed += 1;
                }
                Err(e) => {
                    error!(
                        "🚨 [TtsWorker] TTS Synthesis failed for {}: {:?}",
                        expr.id, e
                    );
                    expr.tts_status = TtsStatus::Failed;
                    if let Err(e) = queue.store_expression(&expr).await {
                        error!(
                            "🚨 [TtsWorker] Failed to persist TTS failure status for {}: {:?}",
                            expr.id, e
                        );
                    }
                }
            }
        }

        Ok(processed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core_contracts::error::AiomeError;
    use aiome_core_contracts::traits::TtsProvider;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Debug)]
    struct MockTtsProvider {
        call_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl TtsProvider for MockTtsProvider {
        async fn synthesize(&self, _text: &str, _voice_id: &str) -> Result<Vec<u8>, AiomeError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(vec![0u8; 1000])
        }
        async fn health_check(&self) -> Result<bool, AiomeError> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_tts_worker_uses_provider() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let provider = MockTtsProvider {
            call_count: call_count.clone(),
        };
        let artifacts_root = std::env::temp_dir();

        let expr = Expression {
            id: "test-1".to_string(),
            content: "Hello".to_string(),
            tts_status: TtsStatus::NotRequested,
            ..Default::default()
        };

        let mut queue = MockTtsQueue::new();
        let expr_clone = expr.clone();
        queue
            .expect_fetch_expressions()
            .returning(move |_| Ok(vec![expr_clone.clone()]));
        queue.expect_store_expression().returning(|_| Ok(()));

        let processed = TtsWorker::process_pending_tts(&queue, &provider, "p225", &artifacts_root)
            .await
            .unwrap(); // allow-anti-pattern

        assert_eq!(processed, 1);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
}
