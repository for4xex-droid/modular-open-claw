/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AiomeError;
use async_trait::async_trait;
use reqwest;
use serde_json;

pub use aiome_core_contracts::llm::EmbeddingProvider;

/// Ruri-v3 ローカル Embedding プロバイダー
/// Python サイドカー (tools/ruri-embed-server) 経由で ruri-v3-310m を利用
#[derive(Debug, Clone)]
pub struct RuriProvider {
    client: reqwest::Client,
    base_url: String,
}

impl RuriProvider {
    /// 埋め込みベクトルの生成専用の推論サーバー(Ruri等)クライアントを作成します
    pub fn new(client: reqwest::Client, base_url: String) -> Self {
        Self { client, base_url }
    }
}

#[async_trait]
impl EmbeddingProvider for RuriProvider {
    async fn embed(&self, text: &str, is_query: bool) -> Result<Vec<f32>, AiomeError> {
        if text.trim().is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "Cannot generate embedding for empty text".into(),
            });
        }

        let url = format!("{}/embed", self.base_url);
        let mode = if is_query { "query" } else { "document" };
        let payload = serde_json::json!({
            "text": text,
            "mode": mode
        });

        let resp = self.client.post(&url)
            .timeout(std::time::Duration::from_secs(30))
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AiomeError::Infrastructure {
                        reason: format!("Ruri embedding timed out after 30s ({})", self.base_url)
                    }
                } else {
                    AiomeError::Infrastructure {
                        reason: format!("Ruri embedding request failed (is ruri-embed-server running on {}?): {}", self.base_url, e)
                    }
                }
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Ruri embedding error: {}", err_text),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ruri embedding parse failed: {}", e),
            })?;

        let embedding = body["embedding"]
            .as_array()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "Ruri embedding missing in response".into(),
            })?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        let url = format!("{}/health", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ruri connection failed: {}", e),
            })?;
        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: "Ruri connection error".into(),
            });
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Ruri-v3(Embed)"
    }

    fn embedding_dim(&self) -> usize {
        768
    }
}
