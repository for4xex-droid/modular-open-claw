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

/// OpenAI Chat Completions Provider
#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: secrecy::SecretString,
    model: String,
    base_url: Option<String>,
}

impl OpenAiProvider {
    /// OpenAI API互換のプロバイダを初期化します
    pub fn new(client: reqwest::Client, api_key: secrecy::SecretString, model: String) -> Self {
        Self {
            client,
            api_key,
            model,
            base_url: None,
        }
    }

    /// OpenAI API互換のプロバイダをベースURL指定で初期化します
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
impl LlmProvider for OpenAiProvider {
    async fn complete_with_cache(&self, request: LlmRequest) -> Result<LlmResponse, AiomeError> {
        let base = self.base_url.as_deref().unwrap_or("https://api.openai.com");
        let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));
        let mut messages = Vec::new();

        for m in request.messages {
            messages.push(serde_json::json!({
                "role": m.role,
                "content": m.content
            }));
        }

        let mut payload = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "temperature": request.temperature.unwrap_or(0.7)
        });

        if let Some(max_tokens) = request.max_tokens {
            payload["max_tokens"] = serde_json::json!(max_tokens);
        }
        if let Some(stop) = request.stop_sequences {
            payload["stop"] = serde_json::json!(stop);
        }
        if let Some(format) = request.format {
            if format == "json" {
                payload["response_format"] = serde_json::json!({ "type": "json_object" });
            }
        }

        let resp = self
            .client
            .post(url)
            .header(
                "Authorization",
                format!(
                    "Bearer {}",
                    secrecy::ExposeSecret::expose_secret(&self.api_key)
                ),
            )
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("OpenAI request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("OpenAI error: {}", err_text),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("OpenAI parse failed: {}", e),
            })?;

        let choice = &body["choices"][0];
        let content = choice["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let stop_reason = match choice["finish_reason"].as_str() {
            Some("stop") => StopReason::EndTurn,
            Some("length") => StopReason::MaxTokens,
            Some("tool_calls") | Some("function_call") => StopReason::ToolUse,
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
        let base = self.base_url.as_deref().unwrap_or("https://api.openai.com");
        let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(serde_json::json!({ "role": "system", "content": sys }));
        }
        messages.push(serde_json::json!({ "role": "user", "content": prompt }));

        let payload = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true
        });

        let mut resp = self
            .client
            .post(url)
            .header(
                "Authorization",
                format!(
                    "Bearer {}",
                    secrecy::ExposeSecret::expose_secret(&self.api_key)
                ),
            )
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("OpenAI stream request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("OpenAI stream error: {}", err_text),
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
        "OpenAI"
    }
}
