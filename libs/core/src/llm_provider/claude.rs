/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AiomeError;
use async_trait::async_trait;
use reqwest;
use secrecy;
use serde_json;
use std::pin::Pin;
use tokio_stream::Stream;

pub use aiome_core_contracts::llm::{LlmProvider, LlmRequest, LlmResponse, StopReason};

/// Anthropic Claude Provider (Messages API)
#[derive(Debug, Clone)]
pub struct ClaudeProvider {
    client: reqwest::Client,
    api_key: secrecy::SecretString,
    model: String,
    base_url: Option<String>,
}

impl ClaudeProvider {
    /// Anthropic Claude APIを利用するためのプロバイダを初期化します
    pub fn new(client: reqwest::Client, api_key: secrecy::SecretString, model: String) -> Self {
        Self {
            client,
            api_key,
            model,
            base_url: None,
        }
    }

    /// Claude APIプロバイダをベースURL指定で初期化します
    pub fn with_base_url(
        client: reqwest::Client,
        api_key: secrecy::SecretString,
        model: String,
        base_url: String,
    ) -> Self {
        Self {
            client,
            api_key,
            model,
            base_url: Some(base_url),
        }
    }
}

#[async_trait]
impl LlmProvider for ClaudeProvider {
    async fn complete_with_cache(&self, request: LlmRequest) -> Result<LlmResponse, AiomeError> {
        let base = self
            .base_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com");
        let url = format!("{}/v1/messages", base.trim_end_matches('/'));
        let mut messages = Vec::new();
        let mut system = Vec::new();

        for m in request.messages {
            let mut content_block = serde_json::json!({
                "type": "text",
                "text": m.content
            });
            if m.cache {
                content_block["cache_control"] = serde_json::json!({ "type": "ephemeral" });
            }

            match m.role.as_str() {
                "system" => {
                    system.push(content_block);
                }
                _ => {
                    messages.push(serde_json::json!({
                        "role": m.role,
                        "content": [content_block]
                    }));
                }
            }
        }

        let mut payload = serde_json::json!({
            "model": self.model,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "messages": messages
        });

        if !system.is_empty() {
            payload["system"] = serde_json::json!(system);
        }

        if let Some(temp) = request.temperature {
            payload["temperature"] = serde_json::json!(temp);
        }
        if let Some(stop) = request.stop_sequences {
            payload["stop_sequences"] = serde_json::json!(stop);
        }

        let resp = self
            .client
            .post(url)
            .header(
                "x-api-key",
                secrecy::ExposeSecret::expose_secret(&self.api_key),
            )
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "prompt-caching-2024-07-31")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Claude request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Claude error: {}", err_text),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Claude parse failed: {}", e),
            })?;

        let content = body["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let stop_reason = match body["stop_reason"].as_str() {
            Some("end_turn") => StopReason::EndTurn,
            Some("tool_use") => StopReason::ToolUse,
            Some("max_tokens") => StopReason::MaxTokens,
            Some("stop_sequence") => StopReason::StopSequence,
            Some(other) => StopReason::Other(other.to_string()),
            None => StopReason::EndTurn,
        };

        Ok(LlmResponse {
            content,
            stop_reason,
            ..Default::default()
        })
    }

    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        use aiome_core_contracts::llm::{LlmMessage, LlmRequest};
        let mut messages = Vec::new();
        if let Some(sys) = system {
            messages.push(LlmMessage {
                role: "system".into(),
                content: sys.into(),
                cache: false,
            });
        }
        messages.push(LlmMessage {
            role: "user".into(),
            content: prompt.into(),
            cache: false,
        });

        let request = LlmRequest {
            messages,
            ..Default::default()
        };
        self.complete_with_cache(request).await
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        let base = self
            .base_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com");
        let url = format!("{}/v1/messages", base.trim_end_matches('/'));

        let payload = serde_json::json!({
            "model": self.model,
            "max_tokens": 4096,
            "system": system,
            "messages": [
                { "role": "user", "content": prompt }
            ],
            "stream": true
        });

        let mut resp = self
            .client
            .post(url)
            .header(
                "x-api-key",
                secrecy::ExposeSecret::expose_secret(&self.api_key),
            )
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Claude stream request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Claude stream error: {}", err_text),
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
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                                if let Some(content) = json["delta"]["text"].as_str() {
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
        "Claude"
    }
}
