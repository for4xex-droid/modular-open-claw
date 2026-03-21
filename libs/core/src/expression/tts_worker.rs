/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AiomeError;
use crate::expression::engine::ExpressionEngine;
use crate::expression::Expression;
use crate::traits::JobQueue;
use aiome_contracts::expression::TtsStatus;
use chrono::Utc;
use tracing::{error, info, warn};

/// Phase 10.1a: TTS生成をバックグラウンドで非同期に処理するワーカー
pub struct TtsWorker;

impl TtsWorker {
    /// 未処理のExpressionをスキャンし、TTS生成を実行する
    ///
    /// # Arguments
    /// * `queue` - Expressionの保存・取得・更新を行うジョブキュー
    /// * `xtts_endpoint` - XTTSサーバーのエンドポイントURL
    /// * `speaker_id` - 使用する話者ID
    /// * `artifacts_root` - 音声ファイルを保存するルートディレクトリ
    pub async fn process_pending_tts(
        queue: &dyn JobQueue,
        xtts_endpoint: &str,
        speaker_id: &str,
        artifacts_root: &std::path::Path,
    ) -> Result<usize, AiomeError> {
        // 1. Fetch expressions (using a simpler approach since JobQueue doesn't have a status filter for expressions yet)
        // Note: For now we fetch limit and filter in memory, but in a real system we'd add SQL filter.
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

            // 3. Call XTTS via ExpressionEngine logic
            match ExpressionEngine::synthesize_audio_xtts(&expr.content, speaker_id, xtts_endpoint)
                .await
            {
                Ok((audio_bytes, duration_ms)) => {
                    // 4. Save file to disk
                    let file_name = format!("tts_{}.wav", expr.id);
                    let file_path = artifacts_root.join(&file_name);

                    if let Err(e) = std::fs::write(&file_path, audio_bytes) {
                        error!(
                            "🚨 [TtsWorker] Failed to write audio file to {:?}: {:?}",
                            file_path, e
                        );
                        expr.tts_status = TtsStatus::Failed;
                        let _ = queue.store_expression(&expr).await;
                        continue;
                    }

                    // 5. Update Expression record
                    expr.audio_path = Some(file_path.to_string_lossy().to_string());
                    expr.duration_ms = Some(duration_ms as i32);
                    expr.tts_status = TtsStatus::Ready;

                    if let Err(e) = queue.store_expression(&expr).await {
                        error!(
                            "🚨 [TtsWorker] Failed to update final expression record: {:?}",
                            e
                        );
                        continue;
                    }

                    info!(
                        "✅ [TtsWorker] TTS Ready for {}: {}ms",
                        expr.id, duration_ms
                    );
                    processed += 1;
                }
                Err(e) => {
                    error!(
                        "🚨 [TtsWorker] XTTS Synthesis failed for {}: {:?}",
                        expr.id, e
                    );
                    expr.tts_status = TtsStatus::Failed;
                    let _ = queue.store_expression(&expr).await;
                }
            }
        }

        Ok(processed)
    }
}
