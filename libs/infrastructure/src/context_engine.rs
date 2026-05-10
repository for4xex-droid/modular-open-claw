/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{LlmMessage, LlmProvider, LlmRequest, LlmResponse};
use aiome_core_contracts::traits::{ChatStore, ContextDeps, JobQueue, KarmaRegistry};
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

    /// 記憶キュレーション: 重要度下限閾値（0.0〜1.0）
    #[serde(default = "default_curation_threshold")]
    pub memory_curation_threshold: f64,

    /// 記憶キュレーション: 最大保持エントリ数
    #[serde(default = "default_max_curated_entries")]
    pub max_curated_entries: usize,

    /// 適応ウィンドウ: 直近N日のデータを考慮
    #[serde(default = "default_adaptation_window_days")]
    pub adaptation_window_days: u32,

    /// ジョブ実行中の最大消費コスト（USドル）
    #[serde(default = "default_max_job_cost_usd")]
    pub max_job_cost_usd: f64,
}

fn default_max_job_cost_usd() -> f64 {
    5.0 // Default to $5.0 cap per job to prevent runaway automation
}

fn default_cortex_chars() -> usize {
    8000
}

fn default_tool_output_chars() -> usize {
    4000
}

fn default_curation_threshold() -> f64 {
    0.3
}

fn default_max_curated_entries() -> usize {
    100
}

fn default_adaptation_window_days() -> u32 {
    30
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
            memory_curation_threshold: default_curation_threshold(),
            max_curated_entries: default_max_curated_entries(),
            adaptation_window_days: default_adaptation_window_days(),
            max_job_cost_usd: default_max_job_cost_usd(),
        }
    }
}

/// LLM向けコンテキスト生成エンジン
pub struct ContextEngine {
    provider: Arc<dyn LlmProvider + Send + Sync>,
    job_queue: Arc<dyn ContextDeps>,
    semaphore: Arc<Semaphore>,
}

impl ContextEngine {
    /// 新しいインスタンスを生成する
    pub fn new(
        provider: Arc<dyn LlmProvider + Send + Sync>,
        job_queue: Arc<dyn ContextDeps>,
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

    /// ツール出力の事前圧縮（LLM不要）と重複排除
    pub fn prune_tool_results(history: &mut Vec<serde_json::Value>) {
        let mut to_keep = Vec::new();
        let mut last_hash = None;

        // 重複排除 (連続した tool_result を排除。最新=末尾を残すため逆順に処理)
        for msg in history.iter().rev() {
            if msg["role"] == "tool" {
                let content = msg["content"].as_str().unwrap_or("");
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                content.hash(&mut hasher);
                let h = hasher.finish();

                if Some(h) != last_hash {
                    to_keep.push(msg.clone());
                    last_hash = Some(h);
                }
            } else {
                to_keep.push(msg.clone());
                last_hash = None;
            }
        }
        to_keep.reverse();
        *history = to_keep;

        // 事前圧縮
        for msg in history.iter_mut() {
            if msg["role"] == "tool" {
                let content = msg["content"].as_str().unwrap_or("");
                let lower = content.to_lowercase();
                let is_error = lower.contains("error:")
                    || lower.contains("command failed")
                    || lower.contains("[guardrail block]")
                    || lower.contains("exception:");

                if !is_error {
                    let lines: Vec<&str> = content.lines().collect();
                    if lines.len() > 5 {
                        let mut new_content = lines[0..5].join("\n");
                        new_content
                            .push_str(&format!("\n[...truncated {} lines...]", lines.len() - 5));
                        msg["content"] = serde_json::json!(new_content);
                    }
                }
            }
        }
    }

    /// カルマ（教訓）の重複排除
    pub fn dedup_karma(entries: &mut Vec<aiome_core_contracts::traits::KarmaEntry>) {
        let mut seen = std::collections::HashSet::new();
        let mut to_keep = Vec::new();
        // 最新のものを残すため逆順処理
        for entry in entries.iter().rev() {
            if seen.insert(entry.lesson.clone()) {
                to_keep.push(entry.clone());
            }
        }
        to_keep.reverse();
        *entries = to_keep;
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
        let mut history = self
            .job_queue
            .fetch_chat_history(channel_id, max_recent_turns)
            .await?;

        Self::prune_tool_results(&mut history);

        Ok((summary, history))
    }

    /// Compresses history if it exceeds the threshold
    pub async fn maintain_context(
        &self,
        channel_id: &str,
        threshold: usize,
    ) -> Result<(), AiomeError> {
        static INEFFECTIVE_COUNT: std::sync::LazyLock<dashmap::DashMap<String, u8>> =
            std::sync::LazyLock::new(dashmap::DashMap::new);

        if let Some(count) = INEFFECTIVE_COUNT.get(channel_id) {
            if *count >= 2 {
                INEFFECTIVE_COUNT.insert(channel_id.to_string(), 0);
                warn!("🧠 [ContextEngine] Anti-thrashing triggered for {}: skipping compression this cycle.", channel_id);
                return Ok(());
            }
        }

        // Fetch more than recent to check for compression need
        let mut all_recent = self
            .job_queue
            .fetch_chat_history(channel_id, 100) // 常に多めに取得
            .await?;

        Self::prune_tool_results(&mut all_recent);

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

                let redactor = crate::security::secret_redactor::SecretRedactor::new();

                let recent_context = to_compress
                    .iter()
                    .map(|m| {
                        let mut content = m["content"].as_str().unwrap_or("").to_string();
                        content = redactor.redact(&content).into_owned(); // Step 1 Redactor

                        if let Some(meta) = m.get("metadata") {
                            if let Some(r) = meta.get("reasoning").and_then(|v| v.as_str()) {
                                content = format!("<thinking>\n{}\n</thinking>\n{}", r, content);
                            }
                        }
                        format!("{}: {}", m["role"], content)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let prompt = format!(
                    "以下のこれまでの要約と新しい会話履歴の内容を統合し、簡潔かつ重要なコンテキストを保持した新しい要約を作成してください。\n\n現在の要約:\n{}\n\n追加の会話履歴:\n{}\n\n出力形式: 重要な事実、ユーザーの意図、現在の状況をまとめた日本語の段落。余計な挨拶は不要。",
                    current_summary, recent_context
                );
                let system = Some("あなたは会話要約アシスタントです。");

                match self.provider.complete(&prompt, system).await {
                    Ok(resp) => {
                        let original_chars = recent_context.len();
                        let new_chars = resp.content.trim().len();
                        let ratio = if new_chars > 0 {
                            original_chars as f64 / new_chars as f64
                        } else {
                            0.0
                        };

                        if ratio < 1.1 && original_chars > 0 {
                            let mut count =
                                INEFFECTIVE_COUNT.entry(channel_id.to_string()).or_insert(0);
                            *count += 1;
                        } else {
                            INEFFECTIVE_COUNT.insert(channel_id.to_string(), 0);
                        }

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

                        warn!(
                            "🧠 [ContextEngine] Context compression (NOT deletion): {} chars summarized to {} chars (ratio: {:.2}x) for {}",
                            original_chars, new_chars, ratio, channel_id
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
        let mut facts = self
            .job_queue
            .fetch_relevant_karma_by_category("RAG Context", category, limit)
            .await?;

        Self::dedup_karma(&mut facts.entries);

        let budget = ContextBudget::default();
        let filtered_entries: Vec<_> = facts
            .entries
            .into_iter()
            .filter(|entry| {
                if let Some(valence) = entry.somatic_valence {
                    valence.abs() >= budget.memory_curation_threshold
                } else {
                    true
                }
            })
            .take(budget.max_curated_entries)
            .collect();

        let mood = Self::calculate_mood_summary(&filtered_entries);
        let safe_summary = summary.map(|s| sanitize_for_prompt(&s));

        let mut fact_block = String::new();
        for entry in filtered_entries {
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
            .map(|m| {
                let mut content = m["content"].as_str().unwrap_or("").to_string();
                if let Some(meta) = m.get("metadata") {
                    if let Some(r) = meta.get("reasoning").and_then(|v| v.as_str()) {
                        content = format!("<thinking>\n{}\n</thinking>\n{}", r, content);
                    }
                }
                format!("{}: {}", m["role"], content)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let final_history = if history_text.len() > budget.max_history_chars {
            format!(
                "{}... (truncated)",
                shared::strings::truncate_bytes_safely(&history_text, budget.max_history_chars)
            )
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
            sanitize_for_prompt(
                shared::strings::truncate_bytes_safely(&summary, budget.max_summary_chars).as_ref(),
            ) + "... (truncated)"
        } else {
            sanitize_for_prompt(&summary)
        };

        // 2. 関連カルマの取得 (Karma Budget)
        // limit を多くして取得し、バジェットに収まるようにトリミング
        let mut karma = self
            .job_queue
            .fetch_relevant_karma_by_category("Context Search", category, 100)
            .await?;

        Self::dedup_karma(&mut karma.entries);

        let filtered_entries: Vec<_> = karma
            .entries
            .into_iter()
            .filter(|entry| {
                if let Some(valence) = entry.somatic_valence {
                    valence.abs() >= budget.memory_curation_threshold
                } else {
                    true
                }
            })
            .take(budget.max_curated_entries)
            .collect();

        let mood = Self::calculate_mood_summary(&filtered_entries);

        let mut karma_text = String::new();
        for entry in filtered_entries {
            let safe_lesson = sanitize_for_prompt(&entry.lesson);
            let line = format!("- {}\n", safe_lesson);
            if karma_text.len() + line.len() > budget.max_karma_chars {
                break;
            }
            karma_text.push_str(&line);
        }

        // 3. 会話履歴の取得 (History Budget)
        let mut history = self.job_queue.fetch_chat_history(channel_id, 20).await?;
        Self::prune_tool_results(&mut history);

        let mut history_text = String::new();
        // 直近のメッセージから順にバジェットに詰め込む
        for m in history.iter().rev() {
            let role = m["role"].as_str().unwrap_or("user");
            let mut content = m["content"].as_str().unwrap_or("").to_string();

            if let Some(meta) = m.get("metadata") {
                if let Some(r) = meta.get("reasoning").and_then(|v| v.as_str()) {
                    content = format!("<thinking>\n{}\n</thinking>\n{}", r, content);
                }
            }

            // RT-8: メッセージ単位での切り詰め (1メッセージが巨大すぎる場合の保護)
            let safe_content = if content.len() > budget.max_history_chars {
                shared::strings::truncate_bytes_safely(&content, budget.max_history_chars)
                    .into_owned()
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

    #[test]
    fn test_prune_tool_results() {
        use serde_json::json;
        let mut history = vec![
            json!({ "role": "tool", "content": "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7" }),
            json!({ "role": "tool", "content": "Error: Database connection failed" }),
            json!({ "role": "tool", "content": "Exact same output" }),
            json!({ "role": "tool", "content": "Exact same output" }), // should be deduplicated
            json!({ "role": "user", "content": "Exact same output" }), // not a tool, should remain
        ];

        ContextEngine::prune_tool_results(&mut history);

        // Deduplication (consecutive identical tool results)
        assert_eq!(
            history.len(),
            4,
            "Should deduplicate consecutive identical tool results"
        );
        assert_eq!(history[2]["content"], "Exact same output");

        // Success truncation (Line 1..5)
        let first = history[0]["content"].as_str().unwrap();
        assert!(first.contains("Line 1"));
        assert!(first.contains("Line 5"));
        assert!(!first.contains("Line 6"), "Should truncate lines after 5");
        assert!(
            first.contains("[...truncated 2 lines...]"),
            "Should append truncation notice"
        );

        // Error preservation
        let second = history[1]["content"].as_str().unwrap();
        assert_eq!(
            second, "Error: Database connection failed",
            "Should preserve full error messages"
        );
    }

    #[test]
    fn test_dedup_karma() {
        let mut entry1 = KarmaEntry::default();
        entry1.lesson = "Never do X".into();

        let mut entry2 = KarmaEntry::default();
        entry2.lesson = "Never do X".into(); // duplicate

        let mut entry3 = KarmaEntry::default();
        entry3.lesson = "Always do Y".into();

        let mut entries = vec![entry1, entry2, entry3];
        ContextEngine::dedup_karma(&mut entries);

        assert_eq!(
            entries.len(),
            2,
            "Should deduplicate identical karma lessons"
        );
        assert_eq!(entries[0].lesson, "Never do X");
        assert_eq!(entries[1].lesson, "Always do Y");
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
            .unwrap();

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
            .unwrap();

        // Act
        let (context, _) = engine
            .get_context_with_facts(channel_id, category, 10)
            .await
            .unwrap();

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
            .unwrap();

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
            .unwrap();

        // Act
        let (context, _) = engine
            .get_context_with_facts(channel_id, category, 10)
            .await
            .unwrap();

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
            .store_chat_message(channel_id, "user", &huge_content, None)
            .await
            .unwrap();

        // Custom budget with 1000 char history limit
        let mut budget = ContextBudget::default();
        budget.max_history_chars = 1000;

        // Act
        // Use the existing budgeted method
        let (_, history) = engine
            .fetch_budgeted_context(channel_id, "Category", budget)
            .await
            .unwrap();

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

    #[tokio::test]
    async fn test_maintain_context_compression_ratio_logging() {
        // Arrange
        let provider = Arc::new(crate::job_queue::tests::MockLlmProvider {
            json_response: "Compressed summary".into(),
        });
        let (job_queue, _tmp) = crate::job_queue::tests::create_test_queue().await;
        let job_queue = Arc::new(job_queue);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
        let engine = ContextEngine::new(provider, job_queue.clone(), semaphore);

        let channel_id = "test_compression_channel";

        // Add some messages to exceed the threshold
        for i in 0..10 {
            job_queue
                .store_chat_message(
                    channel_id,
                    "user",
                    &format!("Message {}", i).repeat(100),
                    None,
                )
                .await
                .unwrap();
        }

        // Act
        // Threshold is set to 1000 so it triggers compression
        let result = engine.maintain_context(channel_id, 1000).await;

        // Assert
        assert!(result.is_ok(), "maintain_context should succeed");

        // Ensure the summary was updated
        let (summary, _) = job_queue
            .get_chat_memory_summary(channel_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            summary, "Compressed summary",
            "Summary should be updated with LLM response"
        );
    }
}
