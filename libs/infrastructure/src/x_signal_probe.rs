use crate::trend_sonar::TrendAdapter;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::traits::TrendItem;
use async_trait::async_trait;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::time::{Duration, Instant};

/// X API Rate Limiter: 1 request per 24 hours per token.
static X_API_RATE_LIMITER: Lazy<DashMap<String, Instant>> = Lazy::new(DashMap::new);

pub struct XSignalProbe {
    bearer_token: String,
}

impl XSignalProbe {
    pub fn new(bearer_token: String) -> Self {
        Self { bearer_token }
    }
}

#[async_trait]
impl TrendAdapter for XSignalProbe {
    fn name(&self) -> &str {
        "XSignalProbe"
    }

    async fn fetch(&self, query: &str) -> Result<Vec<TrendItem>, AiomeError> {
        // Rate Limiter Check (24 hours = 86400 secs)
        if let Some(mut time) = X_API_RATE_LIMITER.get_mut(&self.bearer_token) {
            if time.elapsed() < Duration::from_secs(86400) {
                tracing::warn!(
                    "XSignalProbe: Rate limit active (24h cooldown). Skipping X API call."
                );
                return Ok(Vec::new()); // Skip silently to protect API quota
            }
            *time = Instant::now();
        } else {
            X_API_RATE_LIMITER.insert(self.bearer_token.clone(), Instant::now());
        }

        tracing::info!("📡 [XSignalProbe] Fetching signals from X API: {}", query);

        let client = reqwest::Client::new();
        // searchPostsRecent functionality
        let response = client
            .get("https://api.twitter.com/2/tweets/search/recent")
            .query(&[("query", query)])
            .header("Authorization", format!("Bearer {}", self.bearer_token))
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("X API request failed: {}", e),
            })?;

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
}

impl XSignalProbe {
    /// Strips control characters and normalizes whitespace
    fn sanitize_text(raw: &str) -> String {
        let no_newlines = raw.replace('\n', " ");
        no_newlines.chars().filter(|c| !c.is_control()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_x_signal_probe_name() {
        let probe = XSignalProbe::new("dummy".into());
        assert_eq!(probe.name(), "XSignalProbe");
    }

    #[tokio::test]
    async fn test_x_signal_probe_rate_limit() {
        // Clear global state for test
        X_API_RATE_LIMITER.clear();

        let probe = XSignalProbe::new("test_token".into());

        // 1st call: Should hit unimplemented and panic, but let's assume we implement it soon.
        // Let's assert it panics with "TDD RED PHASE" for now to verify it's trying to fetch.
        // Wait, tokio::test doesn't easily let us catch panics in async without UnwindSafe.
        // Let's just expect the second call to return Ok(vec![]) immediately due to rate limit.

        // Manually insert into rate limiter to simulate a recent call
        X_API_RATE_LIMITER.insert("test_token".into(), Instant::now());

        // 2nd call: Should return Ok(vec![]) without hitting the unimplemented!() panic.
        let result = probe.fetch("test query").await;

        assert!(result.is_ok());
        let items = result.unwrap(); // allow-anti-pattern
        assert!(
            items.is_empty(),
            "Rate limited call should return empty vector"
        );
    }
}
