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

pub use aiome_core_contracts::llm::{
    EmbeddingProvider, LlmMessage, LlmProvider, LlmRequest, LlmResponse, StopReason,
};

/// Ollama (ローカルLLM) プロバイダー
#[derive(Debug, Clone)]
pub struct OllamaProvider {
    host: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    /// ローカルOllama環境接続用の新しいクライアントインスタンスを作成します
    pub fn new(host: String, model: String) -> Self {
        Self {
            host,
            model,
            client: crate::http::get_http_client().clone(),
        }
    }

    /// NG-21: 動的にLoRAモデルをOllamaへビルド・登録する
    pub async fn build_lora_model(
        host: &str,
        base_model: &str,
        adapter_path: &str,
        new_model_name: &str,
    ) -> Result<(), AiomeError> {
        tracing::info!(
            "🛠️ [Ollama] Building LoRA model: {} (Base: {}, Adapter: {})",
            new_model_name,
            base_model,
            adapter_path
        );

        let client = crate::http::get_http_client();
        let modelfile = format!("FROM {}\nADAPTER \"{}\"\n", base_model, adapter_path);

        let url = format!("{}/api/create", host);
        let payload = serde_json::json!({
            "name": new_model_name,
            "modelfile": modelfile,
            "stream": false
        });

        let resp = client.post(&url).json(&payload).send().await.map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("Failed to request Ollama model build: {}", e),
            }
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await.unwrap_or_default();
            tracing::error!("🚨 [Ollama] Build failed: {}", err_text);
            return Err(AiomeError::Infrastructure {
                reason: format!("Ollama model build failed [{}]: {}", status, err_text),
            });
        }

        tracing::info!(
            "✅ [Ollama] Successfully built LoRA model: {}",
            new_model_name
        );
        Ok(())
    }

    /// フォーマットを指定してリクエストを送信します
    pub async fn complete_with_format(
        &self,
        prompt: &str,
        system: Option<&str>,
        format: &str,
    ) -> Result<LlmResponse, AiomeError> {
        let url = format!("{}/api/chat", self.host);
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys
            }));
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": prompt
        }));

        let payload = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "think": false,
            "format": format,
            "options": {
                "num_predict": 300,
                "temperature": 0.7
            }
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama request failed: {}", e),
            })?;

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama response parse failed: {}", e),
            })?;

        let content = body["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let stop_reason = match body["done_reason"].as_str() {
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

    /// 構造化JSON出力を強制し、パース失敗時に1回リretryします
    pub async fn complete_structured<T: serde::de::DeserializeOwned>(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<T, AiomeError> {
        let response = self.complete_with_format(prompt, system, "json").await?;

        match serde_json::from_str::<T>(&response.content) {
            Ok(parsed) => Ok(parsed),
            Err(e) => {
                tracing::warn!("⚠️ [Ollama] JSON parse failed: {}. Retrying...", e);
                let retry_prompt = format!(
                    "前回の出力にJSONフォーマットエラーがありました。\nエラー: {}\n出力されたテキスト: {}\n\n正しいJSONフォーマットのみで再出力してください。",
                    e, response.content
                );
                let retry_response = self
                    .complete_with_format(&retry_prompt, system, "json")
                    .await?;
                serde_json::from_str::<T>(&retry_response.content).map_err(|e2| {
                    AiomeError::Infrastructure {
                        reason: format!("Ollama JSON parse failed after retry: {}", e2),
                    }
                })
            }
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete_with_cache(
        &self,
        request: aiome_core_contracts::llm::LlmRequest,
    ) -> Result<LlmResponse, AiomeError> {
        let url = format!("{}/api/chat", self.host);
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            })
            .collect();

        let mut payload = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "think": false,
            "options": {
                "num_predict": request.max_tokens.unwrap_or(300),
                "temperature": request.temperature.unwrap_or(0.7) as f64
            }
        });

        if let Some(format) = request.format {
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("format".into(), serde_json::Value::String(format));
            }
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Ollama error [{}]: {}", status, err_text),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama response parse failed: {}", e),
            })?;

        let content = body["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let stop_reason = match body["done_reason"].as_str() {
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
        let url = format!("{}/api/chat", self.host);
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys
            }));
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": prompt
        }));

        let payload = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
            "think": false,
            "options": {
                "num_predict": 4096,
                "temperature": 0.7
            }
        });

        let stream_client = crate::http::get_http_client();
        let mut resp = stream_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama stream request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("Ollama stream error: {}", resp.status()),
            });
        }

        let stream = async_stream::stream! {
            let mut incomplete_chunk = String::new();
            while let Ok(Some(chunk)) = resp.chunk().await {
                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                    incomplete_chunk.push_str(&text);

                    while let Some(idx) = incomplete_chunk.find('\n') {
                        let line = incomplete_chunk[..idx].to_string();
                        incomplete_chunk = incomplete_chunk[idx+1..].to_string();

                        if line.trim().is_empty() {
                            continue;
                        }

                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                            if let Some(content) = json.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                                yield Ok(content.to_string());
                            }
                        }
                    }
                } else {
                    yield Err(AiomeError::Infrastructure { reason: "Invalid UTF-8 in Ollama stream chunk".into() });
                }
            }
        };

        Ok(Box::pin(stream))
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        let url = format!("{}/api/tags", self.host);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama connection test failed: {}", e),
            })?;

        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("Ollama connection error: {}", resp.status()),
            });
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Ollama"
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn embed(&self, text: &str, _is_query: bool) -> Result<Vec<f32>, AiomeError> {
        let url = format!("{}/api/embeddings", self.host);
        let payload = serde_json::json!({
            "model": self.model,
            "prompt": text
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama embedding request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("Ollama embedding error: {}", resp.status()),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama embedding parse failed: {}", e),
            })?;

        let embedding = body["embedding"]
            .as_array()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "Ollama embedding missing in response".into(),
            })?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        let url = format!("{}/api/tags", self.host);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama embed connection test failed: {}", e),
            })?;
        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: "Ollama embed connection error".into(),
            });
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Ollama(Embed)"
    }

    fn embedding_dim(&self) -> usize {
        768
    }
}
