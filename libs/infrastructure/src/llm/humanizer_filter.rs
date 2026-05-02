/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::humanizer_rules::{HumanizerAction, HumanizerRule};
use super::writing_context::WritingContext;
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::{LlmProvider, LlmResponse};
use async_trait::async_trait;
use regex::Regex;
use std::fmt;
use std::sync::Arc;
use tracing::{info, warn};

/// LLMの出力から「AIくささ」を除去するためのミドルウェア
pub struct HumanizerFilter {
    inner: Arc<dyn LlmProvider + Send + Sync>,
    rules: Vec<HumanizerRule>,
    context: WritingContext,
}

impl fmt::Debug for HumanizerFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HumanizerFilter")
            .field("inner", &self.inner.name())
            .field("context", &self.context)
            .field("rules_count", &self.rules.len())
            .finish()
    }
}

impl HumanizerFilter {
    /// 新しい `HumanizerFilter` インスタンスを作成します。
    pub fn new(
        inner: Arc<dyn LlmProvider + Send + Sync>,
        rules: Vec<HumanizerRule>,
        context: WritingContext,
    ) -> Self {
        Self {
            inner,
            rules,
            context,
        }
    }

    /// JSON文字列である可能性があるか判定（簡易検証）
    fn is_likely_json(text: &str) -> bool {
        let trimmed = text.trim();
        (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    }

    /// JSON内のテキストフィールドと生テキストを判別しつつ、ルールを適用する
    /// 現状は、JSONらしければパースしてテキスト部だけ書き換えるのが理想だが、
    /// 安全のため、JSONと見られる場合はフィルタをスキップするか、ログのみ出力する。
    fn apply_rules(&self, text: &str) -> String {
        // LlmResponse.content は場合によってJSONのことがある。JSONなら破壊したくない。
        if Self::is_likely_json(text) {
            // Jsonと推測される場合は現状スキップ
            // （必要に応じてserde_jsonでパースし、特定フィールドのみ適用できるよう拡張可能）
            return text.to_string();
        }

        let mut current_text = text.to_string();

        for rule in &self.rules {
            // コンテキスト判定
            if !rule.active_contexts.is_empty() && !rule.active_contexts.contains(&self.context) {
                continue;
            }

            if rule.pattern.is_match(&current_text) {
                match &rule.action {
                    HumanizerAction::Replace(replacement) => {
                        current_text = rule
                            .pattern
                            .replace_all(&current_text, replacement)
                            .to_string();
                    }
                    HumanizerAction::Delete => {
                        current_text = rule.pattern.replace_all(&current_text, "").to_string();
                    }
                    HumanizerAction::LogWarning => {
                        warn!(
                            "📝 [HumanizerFilter] AI-pattern detected ({}): Rule '{}'",
                            self.context_name(),
                            rule.name
                        );
                    }
                }
            }
        }

        current_text
    }

    fn context_name(&self) -> &'static str {
        match self.context {
            WritingContext::Chat => "Chat",
            WritingContext::Manifesto => "Manifesto",
            WritingContext::TechLog => "TechLog",
            WritingContext::Dream => "Dream",
            WritingContext::Default => "Default",
        }
    }
}

#[async_trait]
impl LlmProvider for HumanizerFilter {
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        let mut response = self.inner.complete(prompt, system).await?;

        // フィルタ適用
        let original_len = response.content.len();
        response.content = self.apply_rules(&response.content);
        let new_len = response.content.len();

        if original_len != new_len {
            info!(
                "📝 [HumanizerFilter] Applied AI-writing filters. Length: {} -> {}",
                original_len, new_len
            );
        }

        Ok(response)
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<
        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<String, AiomeError>> + Send>>,
        AiomeError,
    > {
        // ストリーミングのフィルタリングはバッファリングが必要なため複雑。
        // 現在はそのままフォールバックへ流す（または逐次適用が必要だが今回はパススルー）。
        self.inner.stream_complete(prompt, system).await
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.inner.test_connection().await
    }

    fn name(&self) -> &str {
        "HumanizerFilter"
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::humanizer_rules::{default_rules_ja, HumanizerAction, HumanizerRule};
    use super::*;
    use crate::llm::writing_context::WritingContext;
    use aiome_core::llm_provider::MockLlmProvider;
    use regex::Regex;

    #[tokio::test]
    async fn test_em_dash_replacement() {
        let base = Arc::new(MockLlmProvider {
            response: "これはテスト——です".into(),
            should_fail: false,
        });
        let filter = HumanizerFilter::new(base, default_rules_ja(), WritingContext::Default);
        let res = filter.complete("prompt", None).await.unwrap(); // allow-anti-pattern
        assert_eq!(res.content, "これはテスト、です");
    }

    #[tokio::test]
    async fn test_chatbot_artifacts_removal() {
        let base = Arc::new(MockLlmProvider {
            response: "詳細は以下の通りです。お役に立てれば幸いです！".into(),
            should_fail: false,
        });
        let filter = HumanizerFilter::new(base, default_rules_ja(), WritingContext::Default);
        let res = filter.complete("prompt", None).await.unwrap(); // allow-anti-pattern
        assert_eq!(res.content, "詳細は以下の通りです。");
    }

    #[tokio::test]
    async fn test_sycophantic_tone_removal() {
        let base = Arc::new(MockLlmProvider {
            response: "素晴らしいご質問ですね！はい、可能です。".into(),
            should_fail: false,
        });
        let filter = HumanizerFilter::new(base, default_rules_ja(), WritingContext::Default);
        let res = filter.complete("prompt", None).await.unwrap(); // allow-anti-pattern
        assert_eq!(res.content, "はい、可能です。");
    }

    #[tokio::test]
    async fn test_json_content_preserved() {
        let json_str = r#"{"text": "これはテスト——です", "emotion": "neutral"}"#;
        let base = Arc::new(MockLlmProvider {
            response: json_str.into(),
            should_fail: false,
        });
        let filter = HumanizerFilter::new(base, default_rules_ja(), WritingContext::Default);
        let res = filter.complete("prompt", None).await.unwrap(); // allow-anti-pattern
                                                                  // JSON形式の場合はパース破壊を防ぐためにフィルタがスキップされることを期待
        assert_eq!(res.content, json_str);
    }

    #[tokio::test]
    async fn test_filter_passthrough() {
        let normal_text = "明日の天気は晴れです。";
        let base = Arc::new(MockLlmProvider {
            response: normal_text.into(),
            should_fail: false,
        });
        let filter = HumanizerFilter::new(base, default_rules_ja(), WritingContext::Default);
        let res = filter.complete("prompt", None).await.unwrap(); // allow-anti-pattern
        assert_eq!(res.content, normal_text);
    }

    #[tokio::test]
    async fn test_writing_context_filtering() {
        let text = "これはテスト——です";
        let base = Arc::new(MockLlmProvider {
            response: text.into(),
            should_fail: false,
        });
        // 特定のルールのみのテスト
        let rules = vec![HumanizerRule {
            name: "em_dash_replacement",
            pattern: Regex::new(r"——").unwrap(), // allow-anti-pattern
            action: HumanizerAction::Replace("、".to_string()),
            active_contexts: vec![WritingContext::Chat], // Chatのみ有効
        }];

        let filter_manifesto =
            HumanizerFilter::new(base.clone(), rules.clone(), WritingContext::Manifesto);
        let res1 = filter_manifesto.complete("prompt", None).await.unwrap(); // allow-anti-pattern
                                                                             // Manifestoではルール適用されない
        assert_eq!(res1.content, text);

        let filter_chat = HumanizerFilter::new(base.clone(), rules, WritingContext::Chat);
        let res2 = filter_chat.complete("prompt", None).await.unwrap(); // allow-anti-pattern
                                                                        // Chatでは適用される
        assert_eq!(res2.content, "これはテスト、です");
    }
}
