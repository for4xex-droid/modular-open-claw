/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::job_queue::UniversalJobQueue;
use aiome_contracts::llm::{LlmMessage, LlmRequest, LlmProvider};
use aiome_contracts::error::AiomeError;
use aiome_contracts::traits::JobQueue;
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

                let prompt = format!(
                    "以下のこれまでの要約と新しい会話履歴の内容を統合し、簡潔かつ重要なコンテキストを保持した新しい要約を作成してください。\n\n現在の要約:\n{}\n\n追加の会話履歴:\n{}\n\n出力形式: 重要な事実、ユーザーの意図、現在の状況をまとめた日本語の段落。余計な挨拶は不要。",
                    current_summary, recent_context
                );
                let system = Some("あなたは会話要約アシスタントです。");

                match self.provider.complete(&prompt, system).await {
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

    /// RAG: 会話履歴に加えて、関連する事実（カルマ）を統合して取得する
    pub async fn get_context_with_facts(
        &self,
        channel_id: &str,
        category: &str,
        limit: i64,
    ) -> Result<(String, String), AiomeError> {
        // 1. 会話履歴とサマリの取得
        let (summary, history) = self.get_intelligent_history(channel_id, 20).await?;

        // 2. 関連事実の取得 (RAG)
        let facts = self
            .job_queue
            .fetch_relevant_karma_by_category("RAG Context", category, limit)
            .await?;

        let fact_block = if facts.entries.is_empty() {
            "なし".to_string()
        } else {
            facts
                .entries
                .iter()
                .map(|e| format!("- {}", e.lesson))
                .collect::<Vec<_>>()
                .join("\n")
        };

        // 3. トークン制限管理 (文字数 * 0.5 程度の概算)
        let history_text = history
            .iter()
            .map(|m| format!("{}: {}", m["role"], m["content"]))
            .collect::<Vec<_>>()
            .join("\n");

        let final_history = if history_text.len() > 4000 {
            format!("{}... (truncated)", &history_text[..4000])
        } else {
            history_text
        };

        let context_block = format!(
            "これまでの要約:\n{}\n\n関連する背景事実:\n{}\n",
            summary.unwrap_or_else(|| "なし".into()),
            fact_block
        );

        Ok((context_block, final_history))
    }
}
