/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

//! # TrendSonar — トレンド収集ツール (External Search API Integration)
//!
//! 定時でトレンドキーワードを取得する。
//! 外部への通信はすべて reqwest で行い、HTML/URLの除去等の検疫処理（Context Sanitization）を実施する。

use aiome_core::contracts::{TrendRequest, TrendResponse};
use aiome_core::error::AiomeError;
use aiome_core::traits::{AgentAct, TrendItem, TrendSource};
use async_trait::async_trait;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

static URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"https?://\S+").expect("Invalid regex"));
static HTML_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").expect("Invalid regex"));
static WS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("Invalid regex"));

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
fn sanitize_snippet(snippet: &str) -> String {
    let mut text = snippet.to_string();

    // Strip URLs
    text = URL_RE.replace_all(&text, "").to_string();

    // Strip HTML Tags
    text = HTML_RE.replace_all(&text, "").to_string();

    // Clean up HTML Entities (basic ones)
    text = text
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#39;", "'");

    // Collapse whitespace
    text = WS_RE.replace_all(&text, " ").to_string();

    text.trim().to_string()
}

#[async_trait]
pub trait TrendAdapter: Send + Sync {
    async fn fetch(&self, query: &str) -> Result<Vec<TrendItem>, AiomeError>;
    fn name(&self) -> &str;
}

pub struct WebSearchAdapter {
    api_key: String,
    client: reqwest::Client,
}

impl WebSearchAdapter {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: aiome_core::http::get_http_client().clone(),
        }
    }
}

#[async_trait]
impl TrendAdapter for WebSearchAdapter {
    fn name(&self) -> &str { "WebSearch" }

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
}

impl ExternalTrendSonar {
    pub fn new(adapters: Vec<std::sync::Arc<dyn TrendAdapter>>) -> Self {
        Self { adapters }
    }
}

#[async_trait]
impl TrendSource for ExternalTrendSonar {
    async fn get_trends(&self, category: &str) -> Result<Vec<TrendItem>, AiomeError> {
        let mut all_trends = Vec::new();
        for adapter in &self.adapters {
            match adapter.fetch(category).await {
                Ok(mut trends) => all_trends.append(&mut trends),
                Err(e) => tracing::warn!("⚠️ [TrendSonar] Adapter {} failed: {}", adapter.name(), e),
            }
        }
        Ok(all_trends)
    }
}

#[async_trait]
impl AgentAct for ExternalTrendSonar {
    type Input = TrendRequest;
    type Output = TrendResponse;

    async fn execute(
        &self,
        input: Self::Input,
        _jail: &bastion::fs_guard::Jail,
    ) -> Result<Self::Output, AiomeError> {
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
        fn name(&self) -> &str { &self.name }
    }

    #[tokio::test]
    async fn test_trend_sonar_aggregation() {
        let adapter1: Arc<dyn TrendAdapter> = Arc::new(MockAdapter {
            name: "source1".into(),
            trends: vec![TrendItem { keyword: "rust".into(), source: "source1".into(), score: 1.0 }],
        });
        let adapter2: Arc<dyn TrendAdapter> = Arc::new(MockAdapter {
            name: "source2".into(),
            trends: vec![TrendItem { keyword: "aiome".into(), source: "source2".into(), score: 0.8 }],
        });

        let sonar = ExternalTrendSonar::new(vec![adapter1, adapter2]);
        let results = sonar.get_trends("tech").await.unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|t| t.keyword == "rust"));
        assert!(results.iter().any(|t| t.keyword == "aiome"));
    }
}
