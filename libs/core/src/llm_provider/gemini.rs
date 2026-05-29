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

pub use aiome_core_contracts::llm::{
    EmbeddingProvider, LlmProvider, LlmRequest, LlmResponse, StopReason,
};

/// Google Gemini LLM Provider implementation
#[derive(Debug, Clone)]
pub struct GeminiProvider {
    client: reqwest::Client,
    api_key: secrecy::SecretString,
    model: String,
    base_url: Option<String>,
}

impl GeminiProvider {
    /// Google Gemini互換のLLMプロバイダを初期化します
    pub fn new(client: reqwest::Client, api_key: secrecy::SecretString, model: String) -> Self {
        Self {
            client,
            api_key,
            model,
            base_url: None,
        }
    }

    /// テスト用にベースURLを指定して初期化します
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
impl LlmProvider for GeminiProvider {
    async fn complete_with_cache(&self, request: LlmRequest) -> Result<LlmResponse, AiomeError> {
        let base = self
            .base_url
            .as_deref()
            .unwrap_or("https://generativelanguage.googleapis.com");
        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            base.trim_end_matches('/'),
            self.model
        );

        let mut contents = Vec::new();
        let mut system_instruction = None;

        for m in request.messages {
            match m.role.as_str() {
                "system" => {
                    system_instruction = Some(serde_json::json!({
                        "parts": [{ "text": m.content }]
                    }));
                }
                "assistant" | "model" => {
                    contents.push(serde_json::json!({
                        "role": "model",
                        "parts": [{ "text": m.content }]
                    }));
                }
                _ => {
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": [{ "text": m.content }]
                    }));
                }
            }
        }

        let mut payload = serde_json::json!({
            "contents": contents
        });

        if let Some(sys) = system_instruction {
            payload["system_instruction"] = sys;
        }

        let mut config = serde_json::json!({});
        if let Some(temp) = request.temperature {
            config["temperature"] = serde_json::json!(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            config["maxOutputTokens"] = serde_json::json!(max_tokens);
        }
        if let Some(stop) = request.stop_sequences {
            config["stopSequences"] = serde_json::json!(stop);
        }

        if !config.as_object().map(|obj| obj.is_empty()).unwrap_or(true) {
            payload["generationConfig"] = config;
        }

        let resp = self
            .client
            .post(&url)
            .header(
                "x-goog-api-key",
                secrecy::ExposeSecret::expose_secret(&self.api_key),
            )
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Gemini request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Gemini error {}: {}", url, err_text),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Gemini parse failed: {}", e),
            })?;

        let candidate = &body["candidates"][0];
        let content = candidate["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let stop_reason = match candidate["finishReason"].as_str() {
            Some("STOP") => StopReason::EndTurn,
            Some("MAX_TOKENS") => StopReason::MaxTokens,
            Some("SAFETY") => StopReason::Other("safety".to_string()),
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
            .unwrap_or("https://generativelanguage.googleapis.com");
        let url = format!(
            "{}/v1beta/models/{}:streamGenerateContent?alt=sse",
            base.trim_end_matches('/'),
            self.model
        );

        let payload = serde_json::json!({
            "contents": [{
                "parts": [{ "text": prompt }]
            }],
            "system_instruction": system.map(|s| {
                serde_json::json!({ "parts": [{ "text": s }] })
            })
        });

        let mut resp = self
            .client
            .post(&url)
            .header(
                "x-goog-api-key",
                secrecy::ExposeSecret::expose_secret(&self.api_key),
            )
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Gemini stream request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Gemini stream error: {}", err_text),
            });
        }

        let stream = async_stream::stream! {
            let mut buffer = String::new();
            while let Ok(Some(chunk)) = resp.chunk().await {
                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                    buffer.push_str(&text);
                    while let Some(idx) = buffer.find("data: ") {
                        let total_len;
                        let json_str_opt = {
                            let remainder = &buffer[idx + 6..];
                            if let Some(end_idx) = remainder.find("\n\n") {
                                total_len = Some(idx + 6 + end_idx + 2);
                                Some(remainder[..end_idx].to_string())
                            } else {
                                total_len = None;
                                None
                            }
                        };

                        if let (Some(t_len), Some(json_str)) = (total_len, json_str_opt) {
                            buffer = buffer[t_len..].to_string();
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                                if let Some(content) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
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
        "Gemini"
    }
}

#[async_trait]
impl EmbeddingProvider for GeminiProvider {
    async fn embed(&self, text: &str, _is_query: bool) -> Result<Vec<f32>, AiomeError> {
        let embedding_model = if self.model.contains("embed") {
            self.model.clone()
        } else {
            "gemini-embedding-001".to_string()
        };

        let base = self
            .base_url
            .as_deref()
            .unwrap_or("https://generativelanguage.googleapis.com");
        let url = format!(
            "{}/v1beta/models/{}:embedContent",
            base.trim_end_matches('/'),
            embedding_model
        );

        let payload = serde_json::json!({
            "model": format!("models/{}", embedding_model),
            "content": {
                "parts": [{ "text": text }]
            }
        });

        let resp = self
            .client
            .post(&url)
            .header(
                "x-goog-api-key",
                secrecy::ExposeSecret::expose_secret(&self.api_key),
            )
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Gemini embedding request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Gemini embedding error: {}", err_text),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Gemini embedding parse failed: {}", e),
            })?;

        let embedding = body["embedding"]["values"]
            .as_array()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "Gemini embedding missing in response".into(),
            })?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.embed("ping", false).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "Gemini(Embed)"
    }

    fn embedding_dim(&self) -> usize {
        768
    }
}
