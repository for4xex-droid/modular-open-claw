/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::trend_sonar::TrendAdapter;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::traits::TrendItem;
use async_trait::async_trait;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::time::{Duration, Instant};

/// X API Rate Limiter: 1 request per 24 hours per token by default
static X_API_RATE_LIMITER: Lazy<DashMap<String, (Instant, Duration)>> = Lazy::new(DashMap::new);

pub struct XSignalProbe {
    bearer_token: String,
    endpoint: String,
}

impl XSignalProbe {
    pub fn new(bearer_token: String) -> Self {
        let endpoint = std::env::var("X_API_ENDPOINT")
            .unwrap_or_else(|_| "https://api.x.com/2/tweets/search/recent".to_string());
        Self {
            bearer_token,
            endpoint,
        }
    }

    #[cfg(test)]
    pub fn new_with_endpoint(bearer_token: String, endpoint: String) -> Self {
        Self {
            bearer_token,
            endpoint,
        }
    }
}

#[async_trait]
impl TrendAdapter for XSignalProbe {
    fn name(&self) -> &str {
        "XSignalProbe"
    }

    async fn fetch(&self, query: &str) -> Result<Vec<TrendItem>, AiomeError> {
        // Rate Limiter Check
        if let Some(mut time) = X_API_RATE_LIMITER.get_mut(&self.bearer_token) {
            let elapsed = time.0.elapsed();
            if elapsed < time.1 {
                tracing::warn!(
                    "XSignalProbe: Rate limit active. Skipping X API call. (Cooldown remaining: {}s)",
                    time.1.saturating_sub(elapsed).as_secs()
                );
                return Ok(Vec::new()); // Skip silently to protect API quota
            }
            *time = (Instant::now(), Duration::from_secs(86400));
        } else {
            X_API_RATE_LIMITER.insert(
                self.bearer_token.clone(),
                (Instant::now(), Duration::from_secs(86400)),
            );
        }

        tracing::info!("📡 [XSignalProbe] Fetching signals from X API: {}", query);

        let client = aiome_core::http::get_http_client();
        // searchPostsRecent functionality
        let response = client
            .get(&self.endpoint)
            .query(&[("query", query)])
            .header("Authorization", format!("Bearer {}", self.bearer_token))
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("X API request failed: {}", e),
            })?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after_secs = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(86400);

            tracing::warn!(
                "🚫 [XSignalProbe] X API 429 Rate Limited. Blocking proxy for {}s.",
                retry_after_secs
            );

            X_API_RATE_LIMITER.insert(
                self.bearer_token.clone(),
                (Instant::now(), Duration::from_secs(retry_after_secs)),
            );

            return Ok(Vec::new());
        }

        if !response.status().is_success() {
            tracing::warn!(
                "⚠️ [XSignalProbe] X API returned error status: {}",
                response.status()
            );
            return Ok(Vec::new()); // Do not panic, just return empty signal
        }

        let json: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to parse X API JSON: {}", e),
                })?;

        Self::parse_x_api_response(&json)
    }
}

impl XSignalProbe {
    pub fn parse_x_api_response(json: &serde_json::Value) -> Result<Vec<TrendItem>, AiomeError> {
        let mut results = Vec::new();

        if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
            for item in data.iter().take(5) {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    let cleaned_text = Self::sanitize_text(text);
                    if !cleaned_text.is_empty() {
                        results.push(TrendItem {
                            keyword: cleaned_text,
                            source: "X".to_string(),
                            score: 1.0, // Fixed unit score for raw signals
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    /// Strips control characters and normalizes whitespace
    fn sanitize_text(raw: &str) -> String {
        let no_newlines = raw.replace('\n', " ");
        no_newlines.chars().filter(|c| !c.is_control()).collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_x_signal_probe_name() {
        let probe = XSignalProbe::new("dummy".into());
        assert_eq!(probe.name(), "XSignalProbe");
    }

    #[tokio::test]
    async fn test_x_signal_probe_rate_limit() {
        // Use a unique token so it doesn't collide with other tests
        let probe = XSignalProbe::new("rate_limit_token".into());

        // Manually insert into rate limiter to simulate a recent call with 24h cooldown
        X_API_RATE_LIMITER.insert(
            "rate_limit_token".into(),
            (Instant::now(), Duration::from_secs(86400)),
        );

        // 2nd call: Should return Ok(vec![]) without hitting the unimplemented!() panic.
        let result = probe.fetch("test query").await;

        assert!(result.is_ok());
        let items = result.unwrap(); // allow-anti-pattern
        assert!(
            items.is_empty(),
            "Rate limited call should return empty vector"
        );
    }

    #[tokio::test]
    async fn test_x_signal_probe_429_retry_after() {
        let mock_server = wiremock::MockServer::start().await;

        let endpoint = format!("{}/2/tweets/search/recent", mock_server.uri());

        // Mock a 429 response with Retry-After header of 3600 seconds
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(429).insert_header("retry-after", "3600"))
            .mount(&mock_server)
            .await;

        let probe = XSignalProbe::new_with_endpoint("retry_token_429".into(), endpoint);

        // Call the API - it should hit the mock server, get 429, set the limit, and return empty Vec
        let result = probe.fetch("test").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        // Now verify the DashMap was updated correctly by inspecting its cooldown
        let entry = X_API_RATE_LIMITER
            .get("retry_token_429")
            .expect("Rate limiter should be set");
        assert_eq!(
            entry.1.as_secs(),
            3600,
            "Retry-After should be extracted and set as duration"
        );
    }

    #[test]
    fn test_parse_x_api_response_success() {
        let json_str = r#"{
            "data": [
                {"id": "1", "text": "AI agents are the future\n🚀"},
                {"id": "2", "text": "Learning Rust"}
            ]
        }"#;
        let json: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let result = XSignalProbe::parse_x_api_response(&json).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].keyword, "AI agents are the future 🚀");
        assert_eq!(result[1].keyword, "Learning Rust");
    }

    #[test]
    fn test_parse_x_api_response_empty_data() {
        let json_str = r#"{"data": []}"#;
        let json: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let result = XSignalProbe::parse_x_api_response(&json).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_x_api_response_missing_data_field() {
        let json_str = r#"{"errors": [{"detail": "Something went wrong"}]}"#;
        let json: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let result = XSignalProbe::parse_x_api_response(&json).unwrap();
        assert!(result.is_empty());
    }
}
