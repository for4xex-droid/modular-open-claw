/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AiomeError;
use async_trait::async_trait;
use std::fmt::Debug;
// Unused imports removed.
use reqwest;
use serde_json;

use std::pin::Pin;
use tokio_stream::Stream;

pub use aiome_contracts::llm::{
    EmbeddingProvider, LlmMessage, LlmProvider, LlmRequest, LlmResponse, StopReason,
};

// --- 実装 ---

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
            reasoning: None,
            metadata: None,
        })
    }

    /// 構造化JSON出力を強制し、パース失敗時に1回リトライします
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
        request: aiome_contracts::llm::LlmRequest,
    ) -> Result<LlmResponse, AiomeError> {
        let url = format!("{}/api/chat", self.host);
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                // Ollama API generally does not expect a 'cache' field in messages.
                // We remove it from the payload to maintain strict compatibility.
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
            reasoning: None,
            metadata: None,
        })
    }

    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        use aiome_contracts::llm::{LlmMessage, LlmRequest};
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

        // Use the shared client
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

                    // Ollama streams NDJSON. We split by newline.
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
}

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
        // Proxy might not return stop_reason yet, assume EndTurn
        let stop_reason = StopReason::EndTurn;

        Ok(LlmResponse {
            content,
            stop_reason,
            reasoning: None,
            metadata: None,
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
        // Vault proxy connection test
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

// --- Cloud Provider Implementations ---

/// Google Gemini Provider
/// Gemini Interactions API スキル実装 (Phase 5)
pub mod interactions;

#[derive(Debug, Clone)]
pub struct GeminiProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: Option<String>,
}

impl GeminiProvider {
    /// Google Gemini互換のLLMプロバイダを初期化します
    pub fn new(client: reqwest::Client, api_key: String, model: String) -> Self {
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
        api_key: String,
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
                    // Default to user role
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

        // Add generation config if needed
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

        if !config.as_object().unwrap().is_empty() {
            payload["generationConfig"] = config;
        }

        let resp = self
            .client
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
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
            reasoning: None,
            metadata: None,
        })
    }

    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        use aiome_contracts::llm::{LlmMessage, LlmRequest};
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
            .header("x-goog-api-key", &self.api_key)
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
            "gemini-embedding-001".to_string() // Fallback to standard embedding model
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
            .header("x-goog-api-key", &self.api_key)
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
}

/// OpenAI Chat Completions Provider
#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: Option<String>,
}

impl OpenAiProvider {
    /// OpenAI API互換のプロバイダを初期化します
    pub fn new(client: reqwest::Client, api_key: String, model: String) -> Self {
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
        api_key: String,
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
            // OpenAI handles "system" role in the same messages array
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
            .header("Authorization", format!("Bearer {}", self.api_key))
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
            reasoning: None,
            metadata: None,
        })
    }

    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        use aiome_contracts::llm::{LlmMessage, LlmRequest};
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
            .header("Authorization", format!("Bearer {}", self.api_key))
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

/// Anthropic Claude Provider (Messages API)
#[derive(Debug, Clone)]
pub struct ClaudeProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: Option<String>,
}

impl ClaudeProvider {
    /// Anthropic Claude APIを利用するためのプロバイダを初期化します
    pub fn new(client: reqwest::Client, api_key: String, model: String) -> Self {
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
        api_key: String,
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
            .header("x-api-key", &self.api_key)
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
            reasoning: None,
            metadata: None,
        })
    }

    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        use aiome_contracts::llm::{LlmMessage, LlmRequest};
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
            .header("x-api-key", &self.api_key)
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
            reasoning: None,
            metadata: None,
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
}

/// Infrastructure テスト用のモックLLM
#[cfg(any(test, debug_assertions))]
#[derive(Debug, Clone, Default)]
pub struct MockLlmProvider {
    /// Mock response content.
    pub response: String,
    /// Force the mock to return an error.
    pub should_fail: bool,
}

#[cfg(any(test, debug_assertions))]
#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete(
        &self,
        _prompt: &str,
        _system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        if self.should_fail {
            return Err(AiomeError::Infrastructure {
                reason: "Mock failure".into(),
            });
        }
        Ok(LlmResponse {
            content: if self.response.is_empty() {
                "{\"winner\": \"Skill A\", \"reasoning\": \"Mock victory\"}".to_string()
            } else {
                self.response.clone()
            },
            stop_reason: StopReason::EndTurn,
            reasoning: None,
            metadata: None,
        })
    }
    async fn stream_complete(
        &self,
        _prompt: &str,
        _system: Option<&str>,
    ) -> Result<
        Pin<Box<dyn tokio_stream::Stream<Item = Result<String, AiomeError>> + Send>>,
        AiomeError,
    > {
        if self.should_fail {
            return Err(AiomeError::Infrastructure {
                reason: "Mock failure".into(),
            });
        }
        let response = if self.response.is_empty() {
            "{\"winner\": \"Skill A\", \"reasoning\": \"Mock victory\"}".to_string()
        } else {
            self.response.clone()
        };

        let stream = tokio_stream::iter(vec![Ok(response)]);
        Ok(Box::pin(stream))
    }
    async fn test_connection(&self) -> Result<(), AiomeError> {
        Ok(())
    }
    fn name(&self) -> &str {
        "MockLLM"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_provider_initialization_and_names() {
        let client = crate::http::get_http_client().clone();

        let ollama =
            OllamaProvider::new("http://localhost:11434".to_string(), "llama3".to_string());
        assert_eq!(LlmProvider::name(&ollama), "Ollama");

        let gemini = GeminiProvider::new(client.clone(), "key".to_string(), "gemini".to_string());
        assert_eq!(LlmProvider::name(&gemini), "Gemini");

        let openai = OpenAiProvider::new(client.clone(), "key".to_string(), "gpt-4".to_string());
        assert_eq!(openai.name(), "OpenAI");

        let claude = ClaudeProvider::new(client.clone(), "key".to_string(), "claude".to_string());
        assert_eq!(claude.name(), "Claude");

        let lmstudio = LmStudioProvider::new(
            client.clone(),
            "http://localhost:1234".to_string(),
            "local".to_string(),
        );
        assert_eq!(lmstudio.name(), "LMStudio");
    }

    #[tokio::test]
    async fn test_lmstudio_complete_success() {
        let mock_server = MockServer::start().await;
        let mock_response = serde_json::json!({
            "choices": [{
                "message": {
                    "content": "Hello from mock LM Studio"
                }
            }]
        });

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        let client = crate::http::get_http_client().clone();
        let provider = LmStudioProvider::new(client, mock_server.uri(), "test-model".to_string());

        let result = provider.complete("Say hello", Some("System prompt")).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "Hello from mock LM Studio");
    }

    #[tokio::test]
    async fn test_ollama_complete_json_format_and_options() {
        let mock_server = MockServer::start().await;

        // 期待されるリクエストボディの検証
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .and(wiremock::matchers::body_json(serde_json::json!({
                "model": "arrowcanaria",
                "messages": [{
                    "role": "user",
                    "content": "Give JSON"
                }],
                "stream": false,
                "think": false,
                "format": "json",
                "options": {
                    "num_predict": 300,
                    "temperature": 0.5
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": {
                    "role": "assistant",
                    "content": "{\"status\": \"ok\"}"
                },
                "done_reason": "stop"
            })))
            .mount(&mock_server)
            .await;

        let provider = OllamaProvider::new(mock_server.uri(), "arrowcanaria".to_string());

        // 現時点ではコードが未実装（format: json を送っていない）ため、
        // wiremock がマッチせず 404 を返し、テストが失敗するはず。
        use aiome_contracts::llm::{LlmMessage, LlmRequest};
        let request = LlmRequest {
            messages: vec![LlmMessage {
                role: "user".into(),
                content: "Give JSON".into(),
                cache: false,
            }],
            format: Some("json".into()),
            temperature: Some(0.5),
            ..Default::default()
        };
        let result = provider.complete_with_cache(request).await;

        assert!(
            result.is_ok(),
            "Request should succeed if matched, but will fail (404) if logic is missing: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().content, "{\"status\": \"ok\"}");
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq)]
    struct TestData {
        name: String,
        score: i32,
    }

    #[tokio::test]
    async fn test_ollama_complete_structured_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": { "role": "assistant", "content": "{\"name\": \"Alice\", \"score\": 100}" },
                "done_reason": "stop"
            })))
            .mount(&mock_server)
            .await;

        let provider = OllamaProvider::new(mock_server.uri(), "test".to_string());
        let result: Result<TestData, AiomeError> =
            provider.complete_structured("give data", None).await;

        assert!(
            result.is_ok(),
            "Should successfully parse valid JSON: {:?}",
            result.err()
        );
        assert_eq!(
            result.unwrap(),
            TestData {
                name: "Alice".into(),
                score: 100
            }
        );
    }

    #[tokio::test]
    async fn test_ollama_complete_structured_retry_on_failure() {
        let mock_server = MockServer::start().await;

        // 1回目: 壊れたJSON
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": { "role": "assistant", "content": "{\"name\": \"Bob\", \"score\": " },
                "done_reason": "stop"
            })))
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        // 2回目: 修正されたJSON
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": { "role": "assistant", "content": "{\"name\": \"Bob\", \"score\": 200}" },
                "done_reason": "stop"
            })))
            .mount(&mock_server)
            .await;

        let provider = OllamaProvider::new(mock_server.uri(), "test".to_string());
        let result: Result<TestData, AiomeError> =
            provider.complete_structured("give data", None).await;

        assert!(
            result.is_ok(),
            "Should succeed after retry: {:?}",
            result.err()
        );
        assert_eq!(
            result.unwrap(),
            TestData {
                name: "Bob".into(),
                score: 200
            }
        );
    }

    #[tokio::test]
    async fn test_ollama_complete_with_cache_success() {
        let mock_server = MockServer::start().await;
        use aiome_contracts::llm::{LlmMessage, LlmRequest};

        // 期待値: キャッシュフラグが含まれたメッセージリストが送信されること
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .and(wiremock::matchers::body_json(serde_json::json!({
                "model": "test",
                "messages": [
                    { "role": "system", "content": "You are helpful" },
                    { "role": "user", "content": "Hello" }
                ],
                "stream": false,
                "think": false,
                "format": "json",
                "options": { "num_predict": 300, "temperature": 0.5 }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": { "role": "assistant", "content": "Hi!" },
                "done_reason": "stop"
            })))
            .mount(&mock_server)
            .await;

        let provider = OllamaProvider::new(mock_server.uri(), "test".to_string());
        let mut request = LlmRequest::default();
        request.temperature = Some(0.5);
        request.format = Some("json".into());
        request.messages.push(LlmMessage {
            role: "system".into(),
            content: "You are helpful".into(),
            cache: true,
        });
        request.messages.push(LlmMessage {
            role: "user".into(),
            content: "Hello".into(),
            cache: false,
        });

        let result = provider.complete_with_cache(request).await;

        // 現在、デフォルト実装はメッセージの cache フラグを捨てて complete() を呼ぶため、
        // WireMock の期待値 Body JSON と一致せず 404 (Mismatch) になるはず。
        assert!(
            result.is_ok(),
            "Expected wiremock match but likely 404: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_gemini_complete_with_cache_sends_full_history() {
        let mock_server = MockServer::start().await;

        // We expect Gemini API to receive multiple messages in "contents"
        Mock::given(method("POST"))
            .and(path(format!("/v1beta/models/test-model:generateContent")))
            .and(wiremock::matchers::body_json(serde_json::json!({
                "contents": [
                    { "role": "user", "parts": [{ "text": "Hello" }] },
                    { "role": "model", "parts": [{ "text": "Hi there!" }] },
                    { "role": "user", "parts": [{ "text": "Who are you?" }] }
                ],
                "system_instruction": { "parts": [{ "text": "You are a helpful assistant." }] }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "candidates": [{
                    "content": { "parts": [{ "text": "I am Aiome." }] },
                    "finishReason": "STOP"
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = crate::http::get_http_client().clone();
        let provider = GeminiProvider::with_base_url(
            client,
            "test-key".into(),
            "test-model".into(),
            mock_server.uri(),
        );

        use aiome_contracts::llm::{LlmMessage, LlmRequest};
        let request = LlmRequest {
            messages: vec![
                LlmMessage {
                    role: "system".into(),
                    content: "You are a helpful assistant.".into(),
                    cache: false,
                },
                LlmMessage {
                    role: "user".into(),
                    content: "Hello".into(),
                    cache: false,
                },
                LlmMessage {
                    role: "assistant".into(),
                    content: "Hi there!".into(),
                    cache: false,
                },
                LlmMessage {
                    role: "user".into(),
                    content: "Who are you?".into(),
                    cache: false,
                },
            ],
            ..Default::default()
        };

        let result = provider.complete_with_cache(request).await;

        // This will currently FAIL because GeminiProvider uses default complete_with_cache
        // which only extracts the last user message and calls complete("Who are you?", Some("...")).
        // The mock expects the full array in "contents".
        assert!(result.is_ok(), "Request failed: {:?}", result.err());
        assert_eq!(result.unwrap().content, "I am Aiome.");
    }
}
