/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::job_queue::UniversalJobQueue;
use aiome_contracts::llm::{LlmMessage, LlmRequest};
use aiome_contracts::traits::CapabilityProvider;
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn};

/// LLM向けコンテキスト生成エンジン
pub struct ContextEngine {
    provider: Arc<dyn LlmProvider + Send + Sync>,
    job_queue: Arc<UniversalJobQueue>,
    semaphore: Arc<Semaphore>,
}

impl ContextEngine {
    /// 新しいインスタンスを生成する
    pub fn new(
        provider: Arc<dyn LlmProvider + Send + Sync>,
        job_queue: Arc<UniversalJobQueue>,
        semaphore: Arc<Semaphore>,
    ) -> Self {
        Self {
            provider,
            job_queue,
            semaphore,
        }
    }

    /// Fetches the intelligent context for a channel (Summary + Recent turns)
    pub async fn get_intelligent_history(
        &self,
        channel_id: &str,
        max_recent_turns: i64,
    ) -> Result<(Option<String>, Vec<serde_json::Value>), AiomeError> {
        let summary = self.job_queue.get_chat_memory_summary(channel_id).await?;
        let history = self
            .job_queue
            .fetch_chat_history(channel_id, max_recent_turns)
            .await?;
        Ok((summary, history))
    }

    /// Compresses history if it exceeds the threshold
    pub async fn maintain_context(
        &self,
        channel_id: &str,
        threshold: usize,
    ) -> Result<(), AiomeError> {
        // Fetch more than recent to check for compression need
        let all_recent = self
            .job_queue
            .fetch_chat_history(channel_id, 100) // 常に多めに取得
            .await?;

        // 概算トークン数（文字数 * 0.5 程度だが、ここでは単純に文字数で閾値を判定する）
        let total_chars: usize = all_recent
            .iter()
            .map(|m| m["content"].as_str().unwrap_or("").len())
            .sum();

        // threshold が文字数基準とする
        if total_chars > threshold {
            if let Ok(_permit) = self.semaphore.try_acquire() {
                info!(
                    "🧠 [ContextEngine] Compressing history for channel: {}",
                    channel_id
                );

                let current_summary = self
                    .job_queue
                    .get_chat_memory_summary(channel_id)
                    .await?
                    .unwrap_or_else(|| "なし".to_string());

                // Take the oldest half of messages to compress
                let compress_count = all_recent.len() / 2;
                let to_compress = &all_recent[..compress_count];
                let recent_context = to_compress
                    .iter()
                    .map(|m| format!("{}: {}", m["role"], m["content"]))
                    .collect::<Vec<_>>()
                    .join("\n");

                let request = LlmRequest {
                    messages: vec![
                        LlmMessage {
                            role: "system".into(),
                            content: "あなたは会話要約アシスタントです。".into(),
                            cache: true, // システム指示をキャッシュ
                        },
                        LlmMessage {
                            role: "user".into(),
                            content: format!(
                                "以下のこれまでの要約と新しい会話履歴の内容を統合し、簡潔かつ重要なコンテキストを保持した新しい要約を作成してください。\n\n現在の要約:\n{}\n\n追加の会話履歴:\n{}\n\n出力形式: 重要な事実、ユーザーの意図、現在の状況をまとめた日本語の段落。余計な挨拶は不要。",
                                current_summary, recent_context
                            ),
                            cache: false,
                        },
                    ],
                    ..Default::default()
                };

                match self.provider.complete_with_cache(request).await {
                    Ok(resp) => {
                        self.job_queue
                            .update_chat_memory_summary(channel_id, resp.content.trim())
                            .await?;

                        // Mark compressed messages as distilled
                        if let Some(last_compressed) = to_compress.last() {
                            if let Some(last_id) = last_compressed["id"].as_i64() {
                                let _ = self
                                    .job_queue
                                    .mark_chats_as_distilled(channel_id, last_id)
                                    .await;
                            }
                        }

                        info!(
                            "✅ [ContextEngine] Context compressed successfully for {}",
                            channel_id
                        );
                    }
                    Err(e) => {
                        warn!("⚠️ [ContextEngine] Failed to compress context: {:?}", e);
                    }
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl CapabilityProvider for ContextEngine {
    fn capability_name(&self) -> &str {
        "ContextEngine"
    }

    fn capability_description(&self) -> &str {
        "AIのための長期・短期記憶とコンテキスト圧縮機能を提供します。"
    }

    fn capability_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "functions": [
                {
                    "name": "get_intelligent_history",
                    "description": "チャネルの会話履歴と要約を取得します。"
                },
                {
                    "name": "maintain_context",
                    "description": "会話履歴が長い場合に要約による圧縮を実行します。"
                }
            ]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_queue::UniversalJobQueue;
    use aiome_core::llm_provider::LlmProvider;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    #[derive(Debug)]
    struct MockLlm {
        reply: String,
    }

    #[async_trait::async_trait]
    impl LlmProvider for MockLlm {
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
            Ok(aiome_core::llm_provider::LlmResponse {
                content: self.reply.clone(),
                stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
            })
        }
        fn name(&self) -> &str {
            "mock-llm"
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_get_intelligent_history() {
        let jq = UniversalJobQueue::new(":memory:").await.unwrap();
        jq.insert_chat_message("user-1", "user", "Hello")
            .await
            .unwrap();
        jq.update_chat_memory_summary("user-1", "Initial summary")
            .await
            .unwrap();

        let engine = ContextEngine::new(
            Arc::new(MockLlm {
                reply: "compressed".into(),
            }),
            Arc::new(jq),
            Arc::new(Semaphore::new(1)),
        );

        let (summary, history) = engine.get_intelligent_history("user-1", 10).await.unwrap();
        assert_eq!(summary.unwrap(), "Initial summary");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0]["content"], "Hello");
    }

    #[tokio::test]
    async fn test_maintain_context_compression() {
        let jq = UniversalJobQueue::new(":memory:").await.unwrap();
        // Insert many messages to exceed threshold
        for i in 0..10 {
            jq.insert_chat_message("user-1", "user", &format!("Message {}", i))
                .await
                .unwrap();
        }

        let engine = ContextEngine::new(
            Arc::new(MockLlm {
                reply: "New compressed summary".into(),
            }),
            Arc::new(jq.clone()),
            Arc::new(Semaphore::new(1)),
        );

        // threshold = 50 chars. Each message "Message X" is ~9 chars. 10 * 9 = 90 > 50.
        engine.maintain_context("user-1", 50).await.unwrap();

        let summary = jq.get_chat_memory_summary("user-1").await.unwrap();
        assert_eq!(summary.unwrap(), "New compressed summary");

        // Old messages should be distilled? ContextEngine marks them as distilled.
        // We can check if more than half are still considered "recent" by internal fetch?
        // Actually ContextEngine just calls mark_chats_as_distilled.
    }
}
