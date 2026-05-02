/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::job_queue::UniversalJobQueue;
use crate::llm::cost_breaker::CostCircuitBreaker;
use aiome_core::error::AiomeError;
use aiome_core_contracts::rlm::{RlmConfig, RlmProvider, RlmResponse};
use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RlmClient {
    base_url: String,
    client: Client,
    jq: Arc<UniversalJobQueue>,
}

impl RlmClient {
    pub fn new(base_url: String, jq: Arc<UniversalJobQueue>) -> Self {
        Self {
            base_url,
            client: aiome_core::http::get_http_client().clone(),
            jq,
        }
    }
}

#[async_trait]
impl RlmProvider for RlmClient {
    async fn deep_complete(
        &self,
        prompt: &str,
        config: RlmConfig,
    ) -> Result<RlmResponse, AiomeError> {
        // Defense-in-Depth: 入力値バリデーション
        const MAX_PROMPT_BYTES: usize = 128 * 1024; // 128KB
        if prompt.len() > MAX_PROMPT_BYTES {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Prompt exceeds maximum length ({} bytes > {} bytes)",
                    prompt.len(),
                    MAX_PROMPT_BYTES
                ),
            });
        }
        if config.max_depth == 0 {
            return Err(AiomeError::Infrastructure {
                reason: "RlmConfig.max_depth must be greater than 0".into(),
            });
        }
        if config.max_budget_usd <= 0.0 || config.max_budget_usd.is_nan() {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "RlmConfig.max_budget_usd must be a positive number, got: {}",
                    config.max_budget_usd
                ),
            });
        }

        let breaker = CostCircuitBreaker::new(self.jq.clone(), config.max_budget_usd);
        breaker.enforce().await?;

        let url = format!("{}/v1/rlm/complete", self.base_url);

        let payload = serde_json::json!({
            "prompt": prompt,
            "config": config,
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to call RLM sidecar: {}", e),
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            // DoS 防止: エラーボディを 1KB に切り詰め
            let body = resp.text().await.unwrap_or_default();
            let truncated_body: String = body.chars().take(1024).collect();
            return Err(AiomeError::Infrastructure {
                reason: format!("RLM sidecar returned error {}: {}", status, truncated_body),
            });
        }

        let rlm_resp =
            resp.json::<RlmResponse>()
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to parse RLM sidecar response: {}", e),
                })?;

        Ok(rlm_resp)
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        let url = format!("{}/health", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("RLM sidecar health check failed: {}", e),
            })?;
        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("RLM sidecar health check returned status {}", resp.status()),
            });
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "rlm_client"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DatabasePool;
    use crate::job_queue::trajectory_store::SqliteTrajectoryStore;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup_jq() -> Arc<UniversalJobQueue> {
        let pool = DatabasePool::new_sqlite("sqlite::memory:").await.unwrap();
        let ts = Arc::new(SqliteTrajectoryStore::new(pool.clone()));
        let jq = Arc::new(
            UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(),
        );
        crate::job_queue::migrations::DbInitializer::init_db(&*jq)
            .await
            .unwrap();
        jq
    }

    #[tokio::test]
    async fn test_rlm_client_deep_complete_success() {
        let mock_server = MockServer::start().await;
        let jq = setup_jq().await;
        let client = RlmClient::new(mock_server.uri(), jq);

        let response_body = serde_json::json!({
            "content": "Deeply reasoned answer.",
            "recursion_depth": 3,
            "cost_usd": 0.045
        });

        Mock::given(method("POST"))
            .and(path("/v1/rlm/complete"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let result = client
            .deep_complete(
                "What is gravity?",
                RlmConfig {
                    max_depth: 3,
                    max_budget_usd: 1.0,
                },
            )
            .await;

        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.content, "Deeply reasoned answer.");
        assert_eq!(res.recursion_depth, 3);
        assert_eq!(res.cost_usd, 0.045);
    }

    #[tokio::test]
    async fn test_rlm_client_cost_breaker_trip() {
        let mock_server = MockServer::start().await;
        let jq = setup_jq().await;

        // Simulate high usage by inserting a massive cost log into the DB
        let pool = jq.pool.get_sqlite_pool().unwrap();
        sqlx::query("INSERT INTO jobs (id, category, topic, style_name, karma_directives, status) VALUES ('rlm1', 'agent', 'rlm', 'default', '{}', 'Pending')")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO resource_usage_logs (job_id, provider_name, model_name, usage_type, amount, estimated_cost_usd, created_at) VALUES ('rlm1', 'rlm', 'rlm', 'inference', 1, 1000.0, datetime('now', '-1 hour'))")
            .execute(pool)
            .await
            .unwrap();

        // 1000 USD spent > 1.0 USD budget
        let client = RlmClient::new(mock_server.uri(), jq);

        let result = client
            .deep_complete(
                "Should fail?",
                RlmConfig {
                    max_depth: 3,
                    max_budget_usd: 1.0,
                },
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Cost limit exceeded"));
    }

    #[tokio::test]
    async fn test_rlm_client_rejects_zero_max_depth() {
        let mock_server = MockServer::start().await;
        let jq = setup_jq().await;
        let client = RlmClient::new(mock_server.uri(), jq);

        let result = client
            .deep_complete(
                "test",
                RlmConfig {
                    max_depth: 0,
                    max_budget_usd: 1.0,
                },
            )
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("max_depth must be greater than 0"));
    }

    #[tokio::test]
    async fn test_rlm_client_rejects_negative_budget() {
        let mock_server = MockServer::start().await;
        let jq = setup_jq().await;
        let client = RlmClient::new(mock_server.uri(), jq);

        let result = client
            .deep_complete(
                "test",
                RlmConfig {
                    max_depth: 3,
                    max_budget_usd: -1.0,
                },
            )
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be a positive number"));
    }

    #[tokio::test]
    async fn test_rlm_client_rejects_nan_budget() {
        let mock_server = MockServer::start().await;
        let jq = setup_jq().await;
        let client = RlmClient::new(mock_server.uri(), jq);

        let result = client
            .deep_complete(
                "test",
                RlmConfig {
                    max_depth: 3,
                    max_budget_usd: f64::NAN,
                },
            )
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be a positive number"));
    }

    #[tokio::test]
    async fn test_rlm_client_rejects_oversized_prompt() {
        let mock_server = MockServer::start().await;
        let jq = setup_jq().await;
        let client = RlmClient::new(mock_server.uri(), jq);

        let huge_prompt = "x".repeat(128 * 1024 + 1);
        let result = client
            .deep_complete(
                &huge_prompt,
                RlmConfig {
                    max_depth: 3,
                    max_budget_usd: 1.0,
                },
            )
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceeds maximum length"));
    }
}
