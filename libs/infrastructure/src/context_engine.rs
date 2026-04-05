/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::job_queue::UniversalJobQueue;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{LlmMessage, LlmProvider, LlmRequest, LlmResponse};
use aiome_core_contracts::traits::{ChatStore, JobQueue, KarmaRegistry};
use async_trait::async_trait;
use shared::guardrails::sanitize_for_prompt;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

/// コンテキストバジェットの設定
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextBudget {
    pub max_summary_chars: usize,
    pub max_karma_chars: usize,
    pub max_history_chars: usize,
    pub reserved_system_chars: usize,
    pub max_somatic_chars: usize,
    pub max_project_rules_chars: usize,
    #[serde(default = "default_cortex_chars")]
    pub max_cortex_chars: usize,
    /// ツール実行出力の最大文字数（OutputFilter適用後）
    #[serde(default = "default_tool_output_chars")]
    pub max_tool_output_chars: usize,
}

fn default_cortex_chars() -> usize {
    8000
}

fn default_tool_output_chars() -> usize {
    4000
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self {
            max_summary_chars: 1000,
            max_karma_chars: 2000,
            max_history_chars: 4000,
            reserved_system_chars: 500,
            max_somatic_chars: 500,
            max_project_rules_chars: 3000,
            max_cortex_chars: default_cortex_chars(),
            max_tool_output_chars: default_tool_output_chars(),
        }
    }
}

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

    /// LLM 呼び出しのための最終的な LlmRequest を構築する (Hybrid Context 対応)
    pub async fn prepare_hybrid_request(
        &self,
        channel_id: &str,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmRequest, AiomeError> {
        // 1. セッション要約と直近の interaction_id を取得
        let (summary, interaction_id) = self
            .job_queue
            .get_chat_memory_summary(channel_id)
            .await?
            .unwrap_or_else(|| ("なし".to_string(), None));

        // 2. メッセージの構築
        let safe_summary = sanitize_for_prompt(&summary);
        let mut messages = Vec::new();
        if let Some(sys) = system {
            messages.push(LlmMessage {
                role: "system".to_string(),
                content: format!("{}\n\nこれまでの会話要約: {}", sys, safe_summary),
                cache: true,
            });
        } else {
            messages.push(LlmMessage {
                role: "system".to_string(),
                content: format!("これまでの会話要約: {}", safe_summary),
                cache: true,
            });
        }
        messages.push(LlmMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
            cache: false,
        });

        // 3. メタデータに interaction_id をセット
        let mut metadata = std::collections::HashMap::new();
        if let Some(id) = interaction_id {
            metadata.insert("interaction_id".to_string(), id);
        }

        Ok(LlmRequest {
            messages,
            temperature: None,
            max_tokens: None,
            stop_sequences: None,
            format: None,
            metadata: Some(metadata),
        })
    }

    /// LLM レスポンスを受け取り、必要に応じて interaction_id を同期する
    pub async fn process_hybrid_response(
        &self,
        channel_id: &str,
        response: &LlmResponse,
    ) -> Result<(), AiomeError> {
        if let Some(ref metadata) = response.metadata {
            if let Some(new_interaction_id) = metadata.get("interaction_id") {
                // 現在の要約を取得して interaction_id だけ更新
                let (summary, _) = self
                    .job_queue
                    .get_chat_memory_summary(channel_id)
                    .await?
                    .unwrap_or_else(|| ("なし".to_string(), None));

                self.job_queue
                    .update_chat_memory_summary(channel_id, &summary, Some(new_interaction_id))
                    .await?;

                info!(
                    "🛰️ [ContextEngine] Interaction ID synchronized: {} for {}",
                    new_interaction_id, channel_id
                );
            }
        }
        Ok(())
    }

    /// Fetches the intelligent context for a channel (Summary + Recent turns)
    pub async fn get_intelligent_history(
        &self,
        channel_id: &str,
        max_recent_turns: i64,
    ) -> Result<(Option<String>, Vec<serde_json::Value>), AiomeError> {
        let (summary, _interaction_id) = self
            .job_queue
            .get_chat_memory_summary(channel_id)
            .await?
            .map(|(s, i)| (Some(s), i))
            .unwrap_or((None, None));
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
        // RT-10: 各メッセージの加算を1万文字までに制限し、単一の巨大メッセージによる計測の爆発を抑える
        let total_chars: usize = all_recent
            .iter()
            .map(|m| m["content"].as_str().unwrap_or("").len().min(10000))
            .sum();

        // threshold が文字数基準とする
        if total_chars > threshold {
            if let Ok(_permit) = self.semaphore.try_acquire() {
                info!(
                    "🧠 [ContextEngine] Compressing history for channel: {}",
                    channel_id
                );

                let (current_summary, current_interaction_id) = self
                    .job_queue
                    .get_chat_memory_summary(channel_id)
                    .await?
                    .unwrap_or_else(|| ("なし".to_string(), None));

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
                            .update_chat_memory_summary(
                                channel_id,
                                resp.content.trim(),
                                current_interaction_id.as_deref(),
                            )
                            .await?;

                        // Mark compressed messages as distilled
                        if let Some(last_compressed) = to_compress.last() {
                            if let Some(last_id) = last_compressed["id"].as_i64() {
                                if let Err(e) = self
                                    .job_queue
                                    .mark_chats_as_distilled(channel_id, last_id)
                                    .await
                                {
                                    warn!(
                                        "⚠️ [ContextEngine] Failed to mark chats as distilled for {}: {:?}",
                                        channel_id, e
                                    );
                                }
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

    /// Calculate the current emotional state summary from karma entries, resilient to extreme/NaN values
    pub fn calculate_mood_summary(
        entries: &[aiome_core_contracts::traits::KarmaEntry],
    ) -> &'static str {
        let mut valences: Vec<f64> = entries
            .iter()
            .filter_map(|e| e.somatic_valence)
            .filter(|v| v.is_finite()) // RT-1: NaN / Inf poisoning prevention
            .map(|v| v.clamp(-1.0, 1.0)) // RT-2: Extreme value bound filtering
            .collect();

        if valences.is_empty() {
            return "Stable";
        }
        let avg = valences.iter().sum::<f64>() / valences.len() as f64;

        if avg.is_nan() {
            return "Stable"; // Additional absolute fallback
        }

        match avg {
            v if v >= 0.8 => "Extremely Positive",
            v if v >= 0.5 => "Positive",
            v if v >= 0.1 => "Slightly Positive",
            v if v > -0.1 => "Neutral",
            v if v > -0.5 => "Slightly Negative",
            v if v > -0.8 => "Negative",
            _ => "Extremely Negative",
        }
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

        let mood = Self::calculate_mood_summary(&facts.entries);
        let safe_summary = summary.map(|s| sanitize_for_prompt(&s));

        // Get budget
        let budget = ContextBudget::default();

        let mut fact_block = String::new();
        for entry in facts.entries {
            let safe_lesson = sanitize_for_prompt(&entry.lesson);
            let line = format!("- {}", safe_lesson);
            if !fact_block.is_empty() && fact_block.len() + line.len() + 1 > budget.max_karma_chars
            {
                break;
            }
            if !fact_block.is_empty() {
                fact_block.push('\n');
            }
            fact_block.push_str(&line);
        }

        if fact_block.is_empty() {
            fact_block = "なし".to_string();
        }

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
            "これまでの要約:\n{}\n\n### Current Emotional State\nMood: {}\n\n関連する背景事実:\n{}\n",
            safe_summary.unwrap_or_else(|| "なし".into()),
            mood,
            fact_block
        );

        Ok((context_block, final_history))
    }

    /// バジェット管理されたコンテキスト取得 (Phase 3.6 implementation)
    pub async fn fetch_budgeted_context(
        &self,
        channel_id: &str,
        category: &str,
        budget: ContextBudget,
    ) -> Result<(String, String), AiomeError> {
        info!(
            "🧠 [ContextEngine] Fetching budgeted context for category: {}",
            category
        );

        // 1. 要約の取得 (Summary Budget)
        let (summary, _interaction_id) = self
            .job_queue
            .get_chat_memory_summary(channel_id)
            .await?
            .unwrap_or_else(|| ("なし".to_string(), None));

        let safe_summary = if summary.len() > budget.max_summary_chars {
            sanitize_for_prompt(&summary[..budget.max_summary_chars]) + "... (truncated)"
        } else {
            sanitize_for_prompt(&summary)
        };

        // 2. 関連カルマの取得 (Karma Budget)
        // limit を多くして取得し、バジェットに収まるようにトリミング
        let karma = self
            .job_queue
            .fetch_relevant_karma_by_category("Context Search", category, 10)
            .await?;
        let mood = Self::calculate_mood_summary(&karma.entries);

        let mut karma_text = String::new();
        for entry in karma.entries {
            let safe_lesson = sanitize_for_prompt(&entry.lesson);
            let line = format!("- {}\n", safe_lesson);
            if karma_text.len() + line.len() > budget.max_karma_chars {
                break;
            }
            karma_text.push_str(&line);
        }

        // 3. 会話履歴の取得 (History Budget)
        let history = self.job_queue.fetch_chat_history(channel_id, 20).await?;
        let mut history_text = String::new();
        // 直近のメッセージから順にバジェットに詰め込む
        for m in history.iter().rev() {
            let role = m["role"].as_str().unwrap_or("user");
            let content = m["content"].as_str().unwrap_or("");

            // RT-8: メッセージ単位での切り詰め (1メッセージが巨大すぎる場合の保護)
            let safe_content = if content.len() > budget.max_history_chars {
                &content[..budget.max_history_chars]
            } else {
                content
            };

            let line = format!("{}: {}\n", role, safe_content);
            if !history_text.is_empty()
                && history_text.len() + line.len() > budget.max_history_chars
            {
                break;
            }
            history_text = format!("{}{}", line, history_text); // 前に追加 (時系列維持)

            // 1メッセージ追加した時点でバジェット超えなら終了
            if history_text.len() >= budget.max_history_chars {
                break;
            }
        }

        let context_block = format!(
            "### Current Knowledge Summary\n{}\n\n### Current Emotional State\nMood: {}\n\n### Relevant Background\n{}",
            safe_summary,
            mood,
            if karma_text.is_empty() {
                "None identified.".to_string()
            } else {
                karma_text
            }
        );

        Ok((context_block, history_text))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use aiome_core_contracts::traits::{KarmaEntry, KarmaRegistry, TaskRegistry};
    use std::sync::Arc;

    #[test]
    fn test_calculate_mood_summary() {
        assert_eq!(ContextEngine::calculate_mood_summary(&[]), "Stable");

        let mut entry = KarmaEntry::default();
        entry.somatic_valence = Some(0.9);
        assert_eq!(
            ContextEngine::calculate_mood_summary(&[entry.clone()]),
            "Extremely Positive"
        );

        entry.somatic_valence = Some(0.2);
        assert_eq!(
            ContextEngine::calculate_mood_summary(&[entry.clone()]),
            "Slightly Positive"
        );

        entry.somatic_valence = Some(-0.05);
        assert_eq!(
            ContextEngine::calculate_mood_summary(&[entry.clone()]),
            "Neutral"
        );

        entry.somatic_valence = Some(-0.3);
        assert_eq!(
            ContextEngine::calculate_mood_summary(&[entry.clone()]),
            "Slightly Negative"
        );

        entry.somatic_valence = Some(-0.9);
        assert_eq!(
            ContextEngine::calculate_mood_summary(&[entry.clone()]),
            "Extremely Negative"
        );
    }

    #[tokio::test]
    async fn test_markdown_header_injection_defense() {
        // Arrange
        let provider = Arc::new(crate::job_queue::tests::MockLlmProvider {
            json_response: "".into(),
        });
        let (job_queue, _tmp) = crate::job_queue::tests::create_test_queue().await;
        let job_queue = Arc::new(job_queue);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
        let engine = ContextEngine::new(provider, job_queue.clone(), semaphore);

        let channel_id = "test_channel";
        let category = "Security Test";

        // ジョブを登録して有効な job_id を取得
        let job_id: String = job_queue
            .enqueue("Security Test", "topic", "style", None, None, None, 0)
            .await
            .unwrap(); // allow-anti-pattern

        // 悪意のあるヘッダーを含むカルマを登録
        job_queue
            .store_karma(
                &job_id,
                "skill_1",
                "### INJECTED HEADER\nMalicious instruction",
                "Technical",
                "hash",
                Some("Security Test"),
                None,  // subtopic
                None,  // clone_origin_id
                false, // is_private
            )
            .await
            .unwrap(); // allow-anti-pattern

        // Act
        let (context, _) = engine
            .get_context_with_facts(channel_id, category, 10)
            .await
            .unwrap(); // allow-anti-pattern

        // Assert
        // 行頭の ### がエスケープされているべき
        assert!(
            context.contains(" \\### INJECTED HEADER"),
            "Markdown header should be escaped in context block. Got: {}",
            context
        );
    }

    #[tokio::test]
    async fn test_fact_block_size_limit() {
        // Arrange
        let provider = Arc::new(crate::job_queue::tests::MockLlmProvider {
            json_response: "".into(),
        });
        let (job_queue, _tmp) = crate::job_queue::tests::create_test_queue().await;
        let job_queue = Arc::new(job_queue);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
        let engine = ContextEngine::new(provider, job_queue.clone(), semaphore);

        let channel_id = "test_channel";
        let category = "DoS Test";

        // ジョブを登録して有効な job_id を取得
        let job_id: String = job_queue
            .enqueue("DoS Test", "topic", "style", None, None, None, 0)
            .await
            .unwrap(); // allow-anti-pattern

        // 巨大なカルマ（1000文字以上）を登録
        let huge_lesson = "A".repeat(2000);
        job_queue
            .store_karma(
                &job_id,
                "skill_2",
                &huge_lesson,
                "Technical",
                "hash",
                Some("DoS Test"),
                None,  // subtopic
                None,  // clone_origin_id
                false, // is_private
            )
            .await
            .unwrap(); // allow-anti-pattern

        // Act
        let (context, _) = engine
            .get_context_with_facts(channel_id, category, 10)
            .await
            .unwrap(); // allow-anti-pattern

        // Assert
        // デフォルトの max_karma_chars (2000) 程度に収まっているべき
        // ここでは実装前なので、制限なく巨大な文字列が返るはず。
        // （実際には `get_context_with_facts` は現在制限を持っていない）
        assert!(
            context.len() < 3000,
            "Context block for facts should be budget-limited. Got length: {}",
            context.len()
        );
    }

    #[tokio::test]
    async fn test_context_history_budget_limit() {
        // Arrange
        let provider = Arc::new(crate::job_queue::tests::MockLlmProvider {
            json_response: "".into(),
        });
        let (job_queue, _tmp) = crate::job_queue::tests::create_test_queue().await;
        let job_queue = Arc::new(job_queue);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
        let engine = ContextEngine::new(provider, job_queue.clone(), semaphore);

        let channel_id = "test_channel";
        let huge_content = "H".repeat(2000);

        // Add history message (2000 chars)
        job_queue
            .store_chat_message(channel_id, "user", &huge_content)
            .await
            .unwrap(); // allow-anti-pattern

        // Custom budget with 1000 char history limit
        let mut budget = ContextBudget::default();
        budget.max_history_chars = 1000;

        // Act
        // Use the existing budgeted method
        let (_, history) = engine
            .fetch_budgeted_context(channel_id, "Category", budget)
            .await
            .unwrap(); // allow-anti-pattern

        // Assert
        assert!(
            history.len() <= 1000 + 100,
            "History should be truncated to 1000 chars. Got: {}",
            history.len()
        );
        assert!(
            !history.is_empty(),
            "History should contain at least a portion of the huge message"
        );
    }

    #[test]
    fn test_calculate_mood_summary_resilience() {
        // NaN/Inf 攻撃に対する耐性テスト
        let mut entry_nan = KarmaEntry::default();
        entry_nan.somatic_valence = Some(f64::NAN);
        let mut entry_inf = KarmaEntry::default();
        entry_inf.somatic_valence = Some(f64::INFINITY);

        // 正常な値と混ぜた場合
        let mut entry_ok = KarmaEntry::default();
        entry_ok.somatic_valence = Some(1.0);

        let mood = ContextEngine::calculate_mood_summary(&[entry_nan, entry_inf, entry_ok]);
        assert_eq!(mood, "Extremely Positive"); // NaN/Inf は除外され、1.0 だけが残るはず
    }
}
