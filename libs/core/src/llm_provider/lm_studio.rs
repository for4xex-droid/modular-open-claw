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

/// LM Studio Provider (OpenAI-compatible local server)
#[derive(Debug, Clone)]
pub struct LmStudioProvider {
    client: reqwest::Client,
    host: String,
    model: String,
}

impl LmStudioProvider {
    /// LM StudioやvLLM等のローカル推論サーバーにアクセスするためのプロバイダを初期化します
    pub fn new(client: reqwest::Client, host: String, model: String) -> Self {
        Self {
            client,
            host,
            model,
        }
    }
}

#[async_trait]
impl LlmProvider for LmStudioProvider {
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        let url = format!("{}/v1/chat/completions", self.host.trim_end_matches('/'));
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(serde_json::json!({ "role": "system", "content": sys }));
        }
        messages.push(serde_json::json!({ "role": "user", "content": prompt }));

        let payload = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "temperature": 0.7
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", "Bearer lm-studio")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("LM Studio request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("LM Studio error: {}", err_text),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("LM Studio parse failed: {}", e),
            })?;

        let choice = &body["choices"][0];
        let content = choice["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let stop_reason = match choice["finish_reason"].as_str() {
            Some("stop") => StopReason::EndTurn,
            Some("length") => StopReason::MaxTokens,
            Some(other) => StopReason::Other(other.to_string()),
            None => StopReason::EndTurn,
        };

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
        let url = format!("{}/v1/chat/completions", self.host.trim_end_matches('/'));
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(serde_json::json!({ "role": "system", "content": sys }));
        }
        messages.push(serde_json::json!({ "role": "user", "content": prompt }));

        let payload = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "temperature": 0.7,
            "stream": true
        });

        let mut resp = self
            .client
            .post(&url)
            .header("Authorization", "Bearer lm-studio")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("LM Studio stream request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("LM Studio stream error: {}", err_text),
            });
        }

        let stream = async_stream::stream! {
            let mut buffer = String::new();
            while let Ok(Some(chunk)) = resp.chunk().await {
                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                    buffer.push_str(&text);
                    while let Some(idx) = buffer.find("data: ") {
                        let total_len;
                        let line_opt = {
                            let remainder = &buffer[idx + 6..];
                            if let Some(end_idx) = remainder.find('\n') {
                                total_len = Some(idx + 6 + end_idx + 1);
                                Some(remainder[..end_idx].trim().to_string())
                            } else {
                                total_len = None;
                                None
                            }
                        };

                        if let (Some(t_len), Some(line)) = (total_len, line_opt) {
                            buffer = buffer[t_len..].to_string();
                            if line == "[DONE]" { break; }

                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                                if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                    yield Ok(content.to_string());
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
        self.complete("ping", None).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "LMStudio"
    }
}
