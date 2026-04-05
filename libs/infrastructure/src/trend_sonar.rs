/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! # TrendSonar — トレンド収集ツール (External Search API Integration)
//!
//! 定時でトレンドキーワードを取得する。
//! 外部への通信はすべて reqwest で行い、HTML/URLの除去等の検疫処理（Context Sanitization）を実施する。

use aiome_core_contracts::contracts::{TrendRequest, TrendResponse};
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::traits::{TrendItem, TrendSource};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Responses from External Web Search API
#[derive(Deserialize, Debug)]
struct WebSearchResponse {
    web: Option<WebWebResults>,
}

#[derive(Deserialize, Debug)]
struct WebWebResults {
    results: Vec<WebResultItem>,
}

#[derive(Deserialize, Debug)]
struct WebResultItem {
    description: Option<String>,
}

/// Context Sanitization: strips HTML tags, excessive whitespace, and URLs
pub(crate) fn sanitize_snippet(snippet: &str) -> String {
    aiome_core::security_impl::purge_entities(snippet)
}

#[async_trait]
/// トレンドソースからのデータ取得インターフェース
pub trait TrendAdapter: Send + Sync {
    /// クエリに基づきトレンドアイテムを取得する
    async fn fetch(&self, query: &str) -> Result<Vec<TrendItem>, AiomeError>;
    /// アダプターの名前を返す
    fn name(&self) -> &str;
}

/// Web検索（Tavily等）を使用するトレンドアダプター
pub struct WebSearchAdapter {
    api_key: String,
    client: reqwest::Client,
}

impl WebSearchAdapter {
    /// WebSearchAdapter の新規インスタンスを生成する
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: aiome_core::http::get_http_client().clone(),
        }
    }
}

#[async_trait]
impl TrendAdapter for WebSearchAdapter {
    fn name(&self) -> &str {
        "WebSearch"
    }

    async fn fetch(&self, category: &str) -> Result<Vec<TrendItem>, AiomeError> {
        tracing::debug!(
            "WebSearchAdapter: Fetching trends for query '{}'...",
            category
        );

        let endpoint = std::env::var("SEARCH_API_ENDPOINT")
            .unwrap_or_else(|_| "https://api.search.provider.com/res/v1/web/search".to_string());

        let res = self
            .client
            .get(&endpoint)
            .query(&[("q", category), ("freshness", "pd"), ("count", "3")])
            .header("X-Subscription-Token", &self.api_key)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("External Search API request failed: {}", e),
            })?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            tracing::error!("External Search API Error [{}]: {}", status, body);
            return Err(AiomeError::Infrastructure {
                reason: format!("External Search API error [{}]: {}", status, body),
            });
        }

        let search_res: WebSearchResponse =
            res.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to parse Search API response: {}", e),
            })?;

        let mut trends = Vec::new();
        if let Some(web) = search_res.web {
            for item in web.results {
                if let Some(desc) = item.description {
                    let sanitized = sanitize_snippet(&desc);
                    if !sanitized.is_empty() {
                        trends.push(TrendItem {
                            keyword: sanitized,
                            source: "ExternalSearch".to_string(),
                            score: 1.0,
                        });
                    }
                }
            }
        }

        Ok(trends)
    }
}

#[derive(Clone)]
/// `ExternalTrendSonar` 構造体
pub struct ExternalTrendSonar {
    adapters: Vec<std::sync::Arc<dyn TrendAdapter>>,
    provider: Option<std::sync::Arc<dyn aiome_core::llm_provider::LlmProvider>>,
}

impl ExternalTrendSonar {
    /// ExternalTrendSonar の新規インスタンスを生成する
    pub fn new(
        adapters: Vec<std::sync::Arc<dyn TrendAdapter>>,
        provider: Option<std::sync::Arc<dyn aiome_core::llm_provider::LlmProvider>>,
    ) -> Self {
        Self { adapters, provider }
    }
}

#[async_trait]
impl TrendSource for ExternalTrendSonar {
    async fn get_trends(&self, category: &str) -> Result<Vec<TrendItem>, AiomeError> {
        let mut all_trends = Vec::new();
        for adapter in &self.adapters {
            match adapter.fetch(category).await {
                Ok(mut trends) => all_trends.append(&mut trends),
                Err(e) => {
                    tracing::warn!("⚠️ [TrendSonar] Adapter {} failed: {}", adapter.name(), e)
                }
            }
        }

        if let Some(ref provider) = self.provider {
            if !all_trends.is_empty() {
                all_trends = self
                    .evaluate_trends_with_llm(provider, category, all_trends)
                    .await?;
            }
        }

        Ok(all_trends)
    }
}

impl ExternalTrendSonar {
    async fn evaluate_trends_with_llm(
        &self,
        provider: &std::sync::Arc<dyn aiome_core::llm_provider::LlmProvider>,
        category: &str,
        trends: Vec<TrendItem>,
    ) -> Result<Vec<TrendItem>, AiomeError> {
        let keywords_str = trends
            .iter()
            .map(|t| t.keyword.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let prompt = format!(
            "カテゴリ '{}' に関する以下のキーワードから、最も注目度が高く、かつ AI の自律成長に役立つと思われるものを 3つまで抽出し、0.0から1.0のスコアを付けて JSON形式で出力せよ。\n\nキーワード:\n{}\n\n出力形式: [{{ \"keyword\": \"string\", \"score\": 0.8 }}]",
            category, keywords_str
        );

        let resp = provider
            .complete(&prompt, Some("You are a Trend Analysis Expert."))
            .await?;
        let json_str = crate::concept_manager::extract_json(&resp.content)?;

        #[derive(serde::Deserialize)]
        struct EvaluatedTrend {
            keyword: String,
            score: f64,
        }

        let evaluated: Vec<EvaluatedTrend> =
            serde_json::from_str(&json_str).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to parse Trend Evaluation JSON: {}", e),
            })?;

        Ok(evaluated
            .into_iter()
            .map(|e| TrendItem {
                keyword: e.keyword,
                source: "LLM-Evaluated".to_string(),
                score: e.score,
            })
            .collect())
    }
}

// AgentAct was removed from traits or consolidated.
// We remove the impl if it's not found in aiome-contracts.
// Instead we provide an inherent execute method if needed.
impl ExternalTrendSonar {
    pub async fn execute_trend_search(
        &self,
        input: TrendRequest,
    ) -> Result<TrendResponse, AiomeError> {
        let trends = self.get_trends(&input.category).await?;
        Ok(TrendResponse { items: trends })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct MockAdapter {
        name: String,
        trends: Vec<TrendItem>,
    }

    #[async_trait]
    impl TrendAdapter for MockAdapter {
        async fn fetch(&self, _query: &str) -> Result<Vec<TrendItem>, AiomeError> {
            Ok(self.trends.clone())
        }
        fn name(&self) -> &str {
            &self.name
        }
    }

    #[tokio::test]
    async fn test_trend_sonar_aggregation() {
        let adapter1: Arc<dyn TrendAdapter> = Arc::new(MockAdapter {
            name: "source1".into(),
            trends: vec![TrendItem {
                keyword: "rust".into(),
                source: "source1".into(),
                score: 1.0,
            }],
        });
        let adapter2: Arc<dyn TrendAdapter> = Arc::new(MockAdapter {
            name: "source2".into(),
            trends: vec![TrendItem {
                keyword: "aiome".into(),
                source: "source2".into(),
                score: 0.8,
            }],
        });

        let sonar = ExternalTrendSonar::new(vec![adapter1, adapter2], None);
        let results = sonar.get_trends("tech").await.unwrap(); // allow-anti-pattern

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|t| t.keyword == "rust"));
        assert!(results.iter().any(|t| t.keyword == "aiome"));
    }

    #[test]
    fn test_ammonia_directly() {
        let input = "<script>alert(1)</script>";
        let sanitized = ammonia::clean(input);
        println!("AMMONIA Result: '{}'", sanitized);
        // assert_eq!(sanitized, "");
    }
}
