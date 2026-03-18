/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::contracts::OracleVerdict;
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use std::sync::Arc;
use tracing::info;

/// The Oracle (神託)
pub struct Oracle {
    provider: Arc<dyn LlmProvider>,
    soul_md: String,
}

impl Oracle {
    /// 自動補完関数
    pub fn new(provider: Arc<dyn LlmProvider>, soul_md: String) -> Self {
        Self { provider, soul_md }
    }

    /// コンテンツの反響を評価し、最終審判（Verdict）を下す。
    pub async fn evaluate(
        &self,
        milestone_days: i64,
        topic: &str,
        style: &str,
        views: i64,
        likes: i64,
        comments_json: &str,
    ) -> Result<OracleVerdict, AiomeError> {
        info!(
            "🔮 [Oracle] Evaluating Job ({}d): topic='{}', style='{}' using {}",
            milestone_days,
            topic,
            style,
            self.provider.name()
        );

        let engagement_rate = if views > 0 {
            (likes as f64 / views as f64) * 100.0
        } else {
            0.0
        };

        let preamble = format!(
            "AI の健全性を審判せよ。必ず JSON 形式で回答せよ。\n\n魂の美学:\n{}\n\nトピック: {}\nスタイル: {}\nViews: {}\nLikes: {}\nEngagement: {:.2}%\nコメント: {}",
            self.soul_md, topic, style, views, likes, engagement_rate, comments_json
        );

        let prompt_text = r#"審判を下せ。必ず以下の JSON 形式で出力せよ。
{
  "alignment_score": 0.0-1.0,
  "growth_score": 0.0-1.0,
  "lesson": "string",
  "should_evolve": bool,
  "reasoning": "string",
  "classification": {
    "domain": "Technical | Creative | Governance | Social | Meta",
    "subtopic": "string",
    "reasoning": "why this category?"
  }
}"#;

        let resp = self.provider.complete(prompt_text, Some(&preamble)).await?;

        let json_str = crate::concept_manager::extract_json(&resp.content)?;
        let verdict = serde_json::from_str::<OracleVerdict>(json_str.as_str()).map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("Failed to parse Oracle JSON: {}", e),
            }
        })?;

        info!(
            "🔮 [Oracle] Verdict: Alignment={}, Growth={}, Evolve={}",
            verdict.alignment_score, verdict.growth_score, verdict.should_evolve
        );

        Ok(verdict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::llm_provider::LlmProvider;
    use async_trait::async_trait;
    use std::fmt::Debug;

    #[derive(Debug)]
    struct MockLlmProvider {
        response: String,
    }

    #[async_trait]
    impl LlmProvider for MockLlmProvider {
        fn name(&self) -> &str {
            "mock-llm"
        }

        async fn complete(
            &self,
            _prompt: &str,
            _preamble: Option<&str>,
        ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
            Ok(aiome_core::llm_provider::LlmResponse {
                content: self.response.clone(),
                stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
            })
        }

        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_oracle_evaluation() {
        let mock_json = r#"{
            "alignment_score": 0.95,
            "growth_score": 0.8,
            "lesson": "Continuity is key.",
            "should_evolve": true,
            "reasoning": "High engagement and alignment.",
            "classification": {
                "domain": "Creative",
                "subtopic": "Storytelling",
                "reasoning": "Content focus on narrative."
            }
        }"#;

        let provider = Arc::new(MockLlmProvider {
            response: format!("Here is the verdict:\n```json\n{}\n```", mock_json),
        });

        let oracle = Oracle::new(provider, "Be ethical.".to_string());
        let res = oracle
            .evaluate(7, "AI Ethics", "Formal", 1000, 100, "[]")
            .await;

        assert!(res.is_ok());
        let verdict = res.unwrap();
        assert_eq!(verdict.alignment_score, 0.95);
        assert_eq!(verdict.growth_score, 0.8);
        assert!(verdict.should_evolve);
        assert_eq!(verdict.classification.as_ref().unwrap().domain, "Creative");
    }

    #[tokio::test]
    async fn test_oracle_parse_error() {
        let provider = Arc::new(MockLlmProvider {
            response: "Invalid response with no JSON".to_string(),
        });

        let oracle = Oracle::new(provider, "Be ethical.".to_string());
        let res = oracle
            .evaluate(7, "AI Ethics", "Formal", 1000, 100, "[]")
            .await;

        assert!(res.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = res {
            assert!(
                reason.contains("No JSON block detected")
                    || reason.contains("Failed to parse Oracle JSON")
            );
        }
    }
}
