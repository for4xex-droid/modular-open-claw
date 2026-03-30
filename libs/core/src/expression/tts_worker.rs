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
    /// * `tts_provider` - TTSプロバイダー
    /// * `speaker_id` - 使用する話者ID
    /// * `artifacts_root` - 音声ファイルを保存するルートディレクトリ
    pub async fn process_pending_tts(
        queue: &dyn JobQueue,
        tts_provider: &dyn aiome_contracts::traits::TtsProvider,
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
                        let _ = queue.store_expression(&expr).await;
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
                    let _ = queue.store_expression(&expr).await;
                }
            }
        }

        Ok(processed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_contracts::error::AiomeError;
    use aiome_contracts::traits::{JobQueue, TtsProvider};
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

    #[derive(Debug)]
    struct MockQueue {
        expr: Expression,
    }

    #[async_trait]
    impl JobQueue for MockQueue {
        async fn fetch_expressions(&self, _limit: i64) -> Result<Vec<Expression>, AiomeError> {
            Ok(vec![self.expr.clone()])
        }
        async fn store_expression(&self, _expr: &Expression) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn sign_swarm_payload(&self, _: &str) -> Result<String, AiomeError> {
            Ok("".into())
        }
        async fn sync_local_clock(&self, _: u64) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn tick_local_clock(&self) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn storage_gc(&self, _: f64) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn get_system_agent_id(&self) -> Result<uuid::Uuid, AiomeError> {
            Ok(uuid::Uuid::new_v4())
        }
    }

    #[async_trait]
    impl aiome_contracts::traits::TaskRegistry for MockQueue {
        async fn enqueue(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: Option<&str>,
            _: Option<aiome_contracts::security::PermissionManifest>,
            _: Option<uuid::Uuid>,
            _: i32,
        ) -> Result<String, AiomeError> {
            unimplemented!()
        }
        async fn dequeue(
            &self,
            _: &[&str],
        ) -> Result<Option<aiome_contracts::traits::Job>, AiomeError> {
            unimplemented!()
        }
        async fn fetch_job(
            &self,
            _: &str,
        ) -> Result<Option<aiome_contracts::traits::Job>, AiomeError> {
            unimplemented!()
        }
        async fn complete_job(&self, _: &str, _: Option<&str>) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn fail_job(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn fetch_top_performing_jobs(
            &self,
            _: i64,
        ) -> Result<Vec<aiome_contracts::traits::Job>, AiomeError> {
            unimplemented!()
        }
        async fn cancel_job(&self, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn update_job_status(
            &self,
            _: &str,
            _: aiome_contracts::traits::JobStatus,
        ) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn reclaim_zombie_jobs(&self, _: i64) -> Result<u64, AiomeError> {
            unimplemented!()
        }
        async fn set_creative_rating(&self, _: &str, _: i32) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn heartbeat_pulse(&self, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn purge_old_jobs(&self, _: i64) -> Result<u64, AiomeError> {
            unimplemented!()
        }
        async fn fetch_recent_jobs(
            &self,
            _: i64,
        ) -> Result<Vec<aiome_contracts::traits::Job>, AiomeError> {
            unimplemented!()
        }
        async fn get_pending_job_count(&self) -> Result<i64, AiomeError> {
            unimplemented!()
        }
        async fn get_job_count_since(
            &self,
            _: chrono::DateTime<chrono::Utc>,
        ) -> Result<i64, AiomeError> {
            unimplemented!()
        }
        async fn fetch_job_retry_count(&self, _: &str) -> Result<i64, AiomeError> {
            unimplemented!()
        }
        async fn reset_job_retry_count(&self, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
        async fn increment_job_retry_count(&self, _: &str) -> Result<bool, AiomeError> {
            unimplemented!()
        }
        async fn requeue_job(&self, _: &str) -> Result<(), AiomeError> {
            unimplemented!()
        }
    }

    #[async_trait]
    impl aiome_contracts::traits::SystemStateOps for MockQueue {
        async fn store_system_state(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_system_state(&self, _: &str) -> Result<Option<String>, AiomeError> {
            Ok(None)
        }
    }

    #[async_trait]
    impl aiome_contracts::traits::AuditStore for MockQueue {
        async fn store_execution_log(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn store_trajectory_step(
            &self,
            _: aiome_contracts::trajectory::TrajectoryStep,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_trajectory_steps(
            &self,
            _: &str,
        ) -> Result<Vec<aiome_contracts::trajectory::TrajectoryStep>, AiomeError> {
            Ok(vec![])
        }
        async fn get_security_request_count(
            &self,
            _: Option<uuid::Uuid>,
        ) -> Result<u32, AiomeError> {
            Ok(0)
        }
        async fn increment_security_request_count(
            &self,
            _: Option<uuid::Uuid>,
        ) -> Result<u32, AiomeError> {
            Ok(0)
        }
        async fn clear_trajectory_steps(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[async_trait]
    impl aiome_contracts::traits::ChatStore for MockQueue {
        async fn fetch_chat_history(
            &self,
            _: &str,
            _: i64,
        ) -> Result<Vec<serde_json::Value>, AiomeError> {
            Ok(vec![])
        }
        async fn store_chat_message(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn get_chat_memory_summary(
            &self,
            _: &str,
        ) -> Result<Option<(String, Option<String>)>, AiomeError> {
            Ok(None)
        }
        async fn update_chat_memory_summary(
            &self,
            _: &str,
            _: &str,
            _: Option<&str>,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn mark_chats_as_distilled(&self, _: &str, _: i64) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[async_trait]
    impl aiome_contracts::traits::KarmaRegistry for MockQueue {
        async fn fetch_relevant_karma(
            &self,
            _: &str,
            _: &str,
            _: i64,
            _: &str,
        ) -> Result<aiome_contracts::traits::KarmaSearchResult, AiomeError> {
            Ok(aiome_contracts::traits::KarmaSearchResult::empty())
        }
        async fn store_karma(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
            _: bool,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn adjust_karma_weight(&self, _: &str, _: i32) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn karma_decay_sweep(&self) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn fetch_undistilled_jobs(
            &self,
            _: i64,
        ) -> Result<Vec<aiome_contracts::traits::Job>, AiomeError> {
            Ok(vec![])
        }
        async fn mark_karma_extracted(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_all_karma(&self, _: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
            Ok(vec![])
        }
        async fn fetch_unincorporated_karma(
            &self,
            _: i64,
            _: &str,
        ) -> Result<Vec<serde_json::Value>, AiomeError> {
            Ok(vec![])
        }
        async fn mark_karma_as_incorporated(
            &self,
            _: Vec<String>,
            _: &str,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_relevant_karma_by_category(
            &self,
            _: &str,
            _: &str,
            _: i64,
        ) -> Result<aiome_contracts::traits::KarmaSearchResult, AiomeError> {
            Ok(aiome_contracts::traits::KarmaSearchResult::empty())
        }
    }

    #[async_trait]
    impl aiome_contracts::traits::AgentEvolver for MockQueue {
        async fn get_agent_stats(&self) -> Result<aiome_contracts::AgentStats, AiomeError> {
            Ok(aiome_contracts::AgentStats::default())
        }
        async fn add_resonance(&self, _: i32) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn add_tech_exp(&self, _: i32) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn add_creativity(&self, _: i32) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn sync_samsara_level(
            &self,
        ) -> Result<Option<aiome_contracts::SamsaraEvent>, AiomeError> {
            Ok(None)
        }
        async fn record_evolution_event(
            &self,
            _: i32,
            _: &str,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_evolution_history(
            &self,
            _: i64,
        ) -> Result<Vec<serde_json::Value>, AiomeError> {
            Ok(vec![])
        }
        async fn record_soul_mutation(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[async_trait]
    impl aiome_contracts::traits::ImmuneSystemOps for MockQueue {
        async fn store_immune_rule(
            &self,
            _: &aiome_contracts::contracts::ImmuneRule,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn delete_immune_rule(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_active_immune_rules(
            &self,
        ) -> Result<Vec<aiome_contracts::contracts::ImmuneRule>, AiomeError> {
            Ok(vec![])
        }
        async fn record_arena_match(
            &self,
            _: &aiome_contracts::contracts::ArenaMatch,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn get_immune_rules(
            &self,
        ) -> Result<Vec<aiome_contracts::contracts::ImmuneRule>, AiomeError> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl aiome_contracts::traits::FederationRegistry for MockQueue {
        async fn export_federated_data(
            &self,
            _: Option<&str>,
        ) -> Result<
            (
                Vec<aiome_contracts::contracts::KarmaEntry>,
                Vec<aiome_contracts::contracts::ImmuneRule>,
                Vec<aiome_contracts::contracts::ArenaMatch>,
            ),
            AiomeError,
        > {
            Ok((vec![], vec![], vec![]))
        }
        async fn import_federated_data(
            &self,
            _: Vec<aiome_contracts::contracts::KarmaEntry>,
            _: Vec<aiome_contracts::contracts::ImmuneRule>,
            _: Vec<aiome_contracts::contracts::ArenaMatch>,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn get_peer_sync_time(&self, _: &str) -> Result<Option<String>, AiomeError> {
            Ok(None)
        }
        async fn update_peer_sync_time(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn get_node_id(&self) -> Result<String, AiomeError> {
            Ok("test".into())
        }
        async fn fetch_unfederated_data(
            &self,
        ) -> Result<
            (
                Vec<aiome_contracts::contracts::KarmaEntry>,
                Vec<aiome_contracts::contracts::ImmuneRule>,
            ),
            AiomeError,
        > {
            Ok((vec![], vec![]))
        }
        async fn mark_as_federated(
            &self,
            _: Vec<String>,
            _: Vec<String>,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_federated_metrics(
            &self,
        ) -> Result<aiome_contracts::contracts::FederatedMetrics, AiomeError> {
            Ok(aiome_contracts::contracts::FederatedMetrics::default())
        }
    }

    #[async_trait]
    impl aiome_contracts::traits::BiomeRegistry for MockQueue {
        async fn get_biome_topic_status(
            &self,
            _: &str,
        ) -> Result<Option<(i32, Option<String>)>, AiomeError> {
            Ok(None)
        }
        async fn advance_biome_turn(&self, _: &str, _: i64) -> Result<i32, AiomeError> {
            Ok(0)
        }
        async fn fetch_biome_messages(
            &self,
            _: &str,
            _: i64,
        ) -> Result<Vec<serde_json::Value>, AiomeError> {
            Ok(vec![])
        }
        async fn store_biome_message(
            &self,
            _: &aiome_contracts::biome::BiomeMessage,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn update_biome_reputation(&self, _: &str, _: f64) -> Result<f64, AiomeError> {
            Ok(0.0)
        }
        async fn archive_biome_topic(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[async_trait]
    impl aiome_contracts::traits::SoulStore for MockQueue {
        async fn load_soul(&self, _: &str) -> Result<Option<serde_json::Value>, AiomeError> {
            Ok(None)
        }
        async fn store_soul_fragment(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_tts_worker_uses_provider() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let provider = MockTtsProvider {
            call_count: call_count.clone(),
        };
        let artifacts_root = std::env::temp_dir();

        let mut expr = Expression::default();
        expr.id = "test-1".to_string();
        expr.content = "Hello".to_string();
        expr.tts_status = TtsStatus::NotRequested;

        let queue = MockQueue { expr };

        let processed = TtsWorker::process_pending_tts(&queue, &provider, "p225", &artifacts_root)
            .await
            .unwrap();

        assert_eq!(processed, 1);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
}
