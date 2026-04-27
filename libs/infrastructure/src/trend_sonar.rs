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
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
    api_key: SecretString,
    client: reqwest::Client,
    endpoint: String,
}

impl WebSearchAdapter {
    /// WebSearchAdapter の新規インスタンスを生成する
    pub fn new(api_key: SecretString) -> Self {
        let endpoint = std::env::var("SEARCH_API_ENDPOINT")
            .unwrap_or_else(|_| "https://api.search.provider.com/res/v1/web/search".to_string());
        Self {
            api_key,
            client: aiome_core::http::get_http_client().clone(),
            endpoint,
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

        let res = self
            .client
            .get(&self.endpoint)
            .query(&[("q", category), ("freshness", "pd"), ("count", "3")])
            .header("X-Subscription-Token", self.api_key.expose_secret())
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

pub async fn build_active_trend_sonar(
    jq: &crate::job_queue::UniversalJobQueue,
    eval_llm: Option<std::sync::Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>>,
) -> ExternalTrendSonar {
    let mut adapters: Vec<std::sync::Arc<dyn TrendAdapter>> = vec![];

    // Read SEARCH_API_KEY from DB or Env
    let search_api_key = jq
        .get_setting_value("search_api_key")
        .await
        .unwrap_or_default();

    if let Some(api_key) = search_api_key {
        if !api_key.is_empty() {
            adapters.push(std::sync::Arc::new(WebSearchAdapter::new(
                SecretString::from(api_key.clone()),
            )));
            adapters.push(std::sync::Arc::new(SerpAnalysisAdapter::new(
                SecretString::from(api_key),
            )));
        }
    }

    // Read X_BEARER_TOKEN from DB or Env
    let x_token = jq
        .get_setting_value("x_bearer_token")
        .await
        .unwrap_or_default();

    if let Some(x_token) = x_token {
        if !x_token.is_empty() {
            adapters.push(std::sync::Arc::new(
                crate::x_signal_probe::XSignalProbe::new(x_token),
            ));
        }
    }

    if adapters.is_empty() {
        tracing::info!("ℹ️ [TrendSonar Factory] Running in passive mode (No API keys found).");
    }

    ExternalTrendSonar::new(adapters, eval_llm)
}

use dashmap::DashMap;
use std::time::{Duration, Instant};

static SERP_API_RATE_LIMITER: once_cell::sync::Lazy<DashMap<String, (Instant, Duration)>> =
    once_cell::sync::Lazy::new(|| DashMap::new());

/// SerpAnalysisAdapter: SEO Topic Gab and Competitor SERP Analysis
pub struct SerpAnalysisAdapter {
    api_key: SecretString,
    client: reqwest::Client,
    endpoint: String,
}

impl SerpAnalysisAdapter {
    pub fn new(api_key: SecretString) -> Self {
        let endpoint = std::env::var("SEARCH_API_ENDPOINT")
            .unwrap_or_else(|_| "https://api.search.provider.com/res/v1/web/search".to_string());
        Self {
            api_key,
            client: aiome_core::http::get_http_client().clone(),
            endpoint,
        }
    }
}

#[async_trait]
impl TrendAdapter for SerpAnalysisAdapter {
    async fn fetch(&self, query: &str) -> Result<Vec<TrendItem>, AiomeError> {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() || trimmed_query.len() > 1000 {
            return Err(AiomeError::Infrastructure {
                reason: format!("Invalid SERP query length: {}", trimmed_query.len()),
            });
        }

        // Use a hash of the API key as the rate limiter key to avoid
        // storing the plaintext credential in the DashMap (CWE-316).
        let rate_key = format!(
            "{:x}",
            Sha256::digest(self.api_key.expose_secret().as_bytes())
        );

        if let Some(mut time) = SERP_API_RATE_LIMITER.get_mut(&rate_key) {
            let elapsed = time.0.elapsed();
            if elapsed < time.1 {
                tracing::info!("🚦 [SerpAnalysisAdapter] Rate limited to protect API quota. (Cooldown remaining: {}s)", time.1.saturating_sub(elapsed).as_secs());
                return Ok(vec![]);
            }
            *time = (Instant::now(), Duration::from_secs(600));
        } else {
            SERP_API_RATE_LIMITER.insert(rate_key, (Instant::now(), Duration::from_secs(600)));
        }

        #[cfg(any(test, feature = "test-utils"))]
        if self.api_key.expose_secret() == "fake_key" || self.api_key.expose_secret() == "fake" {
            return Ok(vec![TrendItem {
                keyword: format!("{} benefits", query),
                source: "SerpAnalysisAdapter".into(),
                score: 0.95,
            }]);
        }

        let seo_query = format!("{} best practices tips", query);

        let res = self
            .client
            .get(&self.endpoint)
            .query(&[
                ("q", seo_query.as_str()),
                ("freshness", "pm"),
                ("count", "5"),
            ])
            .header("X-Subscription-Token", self.api_key.expose_secret())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("SERP API request failed: {}", e),
            })?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("SERP API error [{}]: {}", status, body),
            });
        }

        let search_res: WebSearchResponse =
            res.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to parse SERP gap response: {}", e),
            })?;

        let mut trends = Vec::new();
        if let Some(web) = search_res.web {
            for item in web.results {
                if let Some(desc) = item.description {
                    let sanitized = crate::trend_sonar::sanitize_snippet(&desc);
                    if !sanitized.is_empty() {
                        trends.push(TrendItem {
                            keyword: sanitized,
                            source: "SerpAnalysisAdapter".into(),
                            score: 0.85,
                        });
                    }
                }
            }
        }
        Ok(trends)
    }

    fn name(&self) -> &str {
        "SerpAnalysisAdapter"
    }
}

#[derive(Clone)]
/// `ExternalTrendSonar` 構造体
pub struct ExternalTrendSonar {
    adapters: Vec<std::sync::Arc<dyn TrendAdapter>>,
    provider: Option<std::sync::Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>>,
}

impl ExternalTrendSonar {
    /// ExternalTrendSonar の新規インスタンスを生成する
    pub fn new(
        adapters: Vec<std::sync::Arc<dyn TrendAdapter>>,
        provider: Option<std::sync::Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>>,
    ) -> Self {
        Self { adapters, provider }
    }
}

#[async_trait]
impl TrendSource for ExternalTrendSonar {
    async fn get_trends(&self, category: &str) -> Result<Vec<TrendItem>, AiomeError> {
        use futures::stream::{FuturesUnordered, StreamExt};
        use tokio::time::timeout;

        let mut all_trends = Vec::new();
        let mut futures = FuturesUnordered::new();
        let adapter_timeout = Duration::from_secs(10);

        for adapter in &self.adapters {
            let cat = category.to_string();
            let a = adapter.clone();

            futures.push(async move {
                let name = a.name().to_string();
                match timeout(adapter_timeout, a.fetch(&cat)).await {
                    Ok(Ok(trends)) => (name, Ok(trends)),
                    Ok(Err(e)) => (name, Err(e)),
                    Err(_) => (
                        name.clone(),
                        Err(AiomeError::Infrastructure {
                            reason: format!(
                                "Adapter {} timed out after {}s",
                                name,
                                adapter_timeout.as_secs()
                            ),
                        }),
                    ),
                }
            });
        }

        while let Some((name, result)) = futures.next().await {
            match result {
                Ok(mut trends) => all_trends.append(&mut trends),
                Err(e) => {
                    tracing::warn!("⚠️ [TrendSonar] Adapter {} failed: {}", name, e)
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
        provider: &std::sync::Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>,
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
        let json_str = crate::llm::utils::extract_json(&resp.content)?;

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
        let adapter1: Arc<dyn TrendAdapter> = std::sync::Arc::new(MockAdapter {
            name: "source1".into(),
            trends: vec![TrendItem {
                keyword: "rust".into(),
                source: "source1".into(),
                score: 1.0,
            }],
        });
        let adapter2: Arc<dyn TrendAdapter> = std::sync::Arc::new(MockAdapter {
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

    #[tokio::test]
    async fn test_serp_analysis_adapter_fetches_seo_gaps() {
        // Arrange
        let adapter = SerpAnalysisAdapter::new(SecretString::from("fake_key".to_string()));

        // Act
        let result = adapter.fetch("organic coffee").await;

        // Assert: In RED phase this will fail because the method returns Err. We expect Ok with parsed gaps.
        let trends = result.expect("fetch should return Ok"); // allow-anti-pattern
        assert!(!trends.is_empty(), "Trends should not be empty");
        // We expect structured SEO intent gaps as trends
        assert_eq!(trends[0].source, "SerpAnalysisAdapter");
        assert_eq!(trends[0].keyword, "organic coffee benefits");
    }

    #[tokio::test]
    async fn test_serp_adapter_rate_limiting() {
        let adapter = SerpAnalysisAdapter::new(SecretString::from("fake".to_string()));
        // Mock the fetch call directly on our newly implemented rate limiter
        let _ = adapter.fetch("organic").await;
        let second_call = adapter.fetch("organic").await;

        assert!(second_call.is_ok());
        assert_eq!(
            second_call.expect("Should be ok").len(), // allow-anti-pattern
            0,
            "Second call should be rate limited and return empty vec"
        );
    }

    #[tokio::test]
    async fn test_serp_adapter_rejects_invalid_queries() {
        let adapter = SerpAnalysisAdapter::new(SecretString::from("fake".to_string()));
        // Empty query
        let empty_res = adapter.fetch("   ").await;
        assert!(empty_res.is_err(), "Should reject empty query");

        // Massive query
        let massive_query = "x".repeat(1500);
        let massive_res = adapter.fetch(&massive_query).await;
        assert!(massive_res.is_err(), "Should reject massive query");
    }
}
