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
use std::pin::Pin;
use tokio_stream::Stream;

pub use aiome_core_contracts::llm::{LlmProvider, LlmResponse, StopReason};

/// Abyss Vault (Key Proxy) 経由の Gemini プロバイダー (DEPRECATED: Direct GeminiProvider推奨)
#[derive(Debug, Clone)]
pub struct AbyssVaultProvider {
    proxy_url: String,
    caller_id: String,
    client: reqwest::Client,
}

impl AbyssVaultProvider {
    /// Abyss Vault(APIキー隔離プロセス)経由での安全なリクエストを送信するクライアントを作成します
    pub fn new(proxy_url: String, caller_id: String) -> Self {
        Self {
            proxy_url,
            caller_id,
            client: crate::http::get_http_client().clone(),
        }
    }
}

#[async_trait]
impl LlmProvider for AbyssVaultProvider {
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        let payload = serde_json::json!({
            "caller_id": self.caller_id,
            "prompt": prompt,
            "system": system,
            "endpoint": "gemini"
        });

        let resp = self
            .client
            .post(format!("{}/api/v1/llm/complete", self.proxy_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("VaultProxy request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("VaultProxy returned error: {}", resp.status()),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("VaultProxy response parse failed: {}", e),
            })?;

        let content = body["result"].as_str().unwrap_or("").to_string();
        let stop_reason = StopReason::EndTurn;

        Ok(LlmResponse {
            content,
            stop_reason,
            ..Default::default()
        })
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        let payload = serde_json::json!({
            "caller_id": self.caller_id,
            "prompt": prompt,
            "system": system,
            "endpoint": "gemini"
        });

        let mut resp = self
            .client
            .post(format!("{}/api/v1/llm/stream", self.proxy_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("VaultProxy stream request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("VaultProxy returned error: {}", resp.status()),
            });
        }

        let stream = async_stream::stream! {
            let mut buffer = String::new();
            while let Ok(Some(chunk)) = resp.chunk().await {
                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                    buffer.push_str(&text);

                    while let Some(data_idx) = buffer.find("data: ") {
                        let remainder = &buffer[data_idx + 6..];
                        if let Some(end_idx) = remainder.find("\n\n") {
                            let json_str = remainder[..end_idx].to_string();
                            let total_len = data_idx + 6 + end_idx + 2;
                            buffer = buffer[total_len..].to_string();

                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                                if let Some(text_chunk) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                                    yield Ok(text_chunk.to_string());
                                }
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        let url = format!("{}/api/v1/health", self.proxy_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("VaultProxy connection failed: {}", e),
            })?;
        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: "VaultProxy connection error".into(),
            });
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "AbyssVault(Gemini)"
    }
}
