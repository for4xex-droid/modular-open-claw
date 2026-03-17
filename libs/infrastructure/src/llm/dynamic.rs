/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::error::AiomeError;
use aiome_core::llm_provider::{EmbeddingProvider, LlmProvider, LlmResponse};
use crate::circuit_breaker::CircuitBreaker;
use crate::job_queue::SqliteJobQueue;
use crate::slo_engine::SloEngine;
use async_trait::async_trait;
use std::sync::Arc;
use tokio_stream::Stream;
use std::pin::Pin;

#[derive(Debug)]
pub struct DynamicLlmProvider {
    pub jq: Arc<SqliteJobQueue>,
    pub client: reqwest::Client,
    pub fallback_host: String,
    pub fallback_model: String,
    pub gemini_api_key: Option<secrecy::SecretString>,
    pub openai_api_key: Option<secrecy::SecretString>,
    pub anthropic_api_key: Option<secrecy::SecretString>,
    pub circuit_breaker: Arc<CircuitBreaker>,
    pub slo_engine: Arc<SloEngine>,
}

#[async_trait]
impl LlmProvider for DynamicLlmProvider {
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        let (provider_type, model) = self.resolve_config(false).await;

        if let Err(e) = self.circuit_breaker.check_state().await {
            return Err(AiomeError::Infrastructure {
                reason: e.to_string(),
            });
        }

        let result = match provider_type.as_str() {
            "gemini" => {
                let api_key = self.get_api_key("llm_api_key", "gemini").await;
                aiome_core::llm_provider::GeminiProvider::new(self.client.clone(), api_key, model)
                    .complete(prompt, system)
                    .await
            }
            "openai" => {
                let api_key = self.get_api_key("llm_api_key", "openai").await;
                aiome_core::llm_provider::OpenAiProvider::new(self.client.clone(), api_key, model)
                    .complete(prompt, system)
                    .await
            }
            "claude" => {
                let api_key = self.get_api_key("llm_api_key", "claude").await;
                aiome_core::llm_provider::ClaudeProvider::new(self.client.clone(), api_key, model)
                    .complete(prompt, system)
                    .await
            }
            "lmstudio" => {
                let host = self.get_host("lm_studio_host", "http://127.0.0.1:1234").await;
                aiome_core::llm_provider::LmStudioProvider::new(self.client.clone(), host, model)
                    .complete(prompt, system)
                    .await
            }
            _ => {
                let host = self.get_host("ollama_host", &self.fallback_host).await;
                aiome_core::llm_provider::OllamaProvider::new(host, model)
                    .complete(prompt, system)
                    .await
            }
        };

        self.handle_result(result).await
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        let (provider_type, model) = self.resolve_config(false).await;

        match provider_type.as_str() {
            "gemini" => {
                let api_key = self.get_api_key("llm_api_key", "gemini").await;
                aiome_core::llm_provider::GeminiProvider::new(self.client.clone(), api_key, model)
                    .stream_complete(prompt, system)
                    .await
            }
            "openai" => {
                let api_key = self.get_api_key("llm_api_key", "openai").await;
                aiome_core::llm_provider::OpenAiProvider::new(self.client.clone(), api_key, model)
                    .stream_complete(prompt, system)
                    .await
            }
            "claude" => {
                let api_key = self.get_api_key("llm_api_key", "claude").await;
                aiome_core::llm_provider::ClaudeProvider::new(self.client.clone(), api_key, model)
                    .stream_complete(prompt, system)
                    .await
            }
            "lmstudio" => {
                let host = self.get_host("lm_studio_host", "http://127.0.0.1:1234").await;
                aiome_core::llm_provider::LmStudioProvider::new(self.client.clone(), host, model)
                    .stream_complete(prompt, system)
                    .await
            }
            _ => {
                let host = self.get_host("ollama_host", &self.fallback_host).await;
                aiome_core::llm_provider::OllamaProvider::new(host, model)
                    .stream_complete(prompt, system)
                    .await
            }
        }
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.complete("ping", None).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "DynamicLlm"
    }
}

#[async_trait]
impl EmbeddingProvider for DynamicLlmProvider {
    async fn embed(&self, text: &str, is_query: bool) -> Result<Vec<f32>, AiomeError> {
        let (provider_type, model) = self.resolve_config(false).await;

        match provider_type.as_str() {
            "gemini" => {
                let api_key = self.get_api_key("llm_api_key", "gemini").await;
                aiome_core::llm_provider::GeminiProvider::new(self.client.clone(), api_key, model)
                    .embed(text, is_query)
                    .await
            }
            _ => {
                let host = self.get_host("ollama_host", &self.fallback_host).await;
                aiome_core::llm_provider::OllamaProvider::new(host, model)
                    .embed(text, is_query)
                    .await
            }
        }
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.embed("ping", false).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "DynamicEmbedding"
    }
}

impl DynamicLlmProvider {
    async fn resolve_config(&self, is_bg: bool) -> (String, String) {
        let prefix = if is_bg { "bg_" } else { "" };
        
        let provider = self.jq.get_setting_value(&format!("{}llm_provider", prefix)).await.ok().flatten()
            .or_else(|| if is_bg { std::env::var("BG_LLM_PROVIDER").ok() } else { None })
            .unwrap_or_else(|| "ollama".to_string());
            
        let model = self.jq.get_setting_value(&format!("{}llm_model", prefix)).await.ok().flatten()
            .or_else(|| if is_bg { std::env::var("BG_LLM_MODEL").ok() } else { None })
            .unwrap_or_else(|| self.fallback_model.clone());

        (provider, model)
    }

    async fn get_api_key(&self, setting_name: &str, provider_name: &str) -> String {
        self.jq.get_setting_value(setting_name).await.ok().flatten()
            .or_else(|| match provider_name {
                "gemini" => self.gemini_api_key.as_ref().map(|s| secrecy::ExposeSecret::expose_secret(s).to_string()),
                "openai" => self.openai_api_key.as_ref().map(|s| secrecy::ExposeSecret::expose_secret(s).to_string()),
                "claude" => self.anthropic_api_key.as_ref().map(|s| secrecy::ExposeSecret::expose_secret(s).to_string()),
                _ => None,
            })
            .unwrap_or_default()
    }

    async fn get_host(&self, setting_name: &str, default: &str) -> String {
        self.jq.get_setting_value(setting_name).await.ok().flatten().unwrap_or_else(|| default.to_string())
    }

    async fn handle_result<T>(&self, result: Result<T, AiomeError>) -> Result<T, AiomeError> {
        match result {
            Ok(res) => {
                self.circuit_breaker.record_success().await;
                Ok(res)
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                self.slo_engine.record_error().await;
                Err(e)
            }
        }
    }
}

#[derive(Debug)]
pub struct BackgroundLlmProvider {
    pub jq: Arc<SqliteJobQueue>,
    pub client: reqwest::Client,
    pub fallback_model: String,
    pub fallback_host: String,
    pub gemini_api_key: Option<secrecy::SecretString>,
    pub openai_api_key: Option<secrecy::SecretString>,
    pub anthropic_api_key: Option<secrecy::SecretString>,
}

#[async_trait]
impl LlmProvider for BackgroundLlmProvider {
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<LlmResponse, AiomeError> {
        let provider_type = self.jq.get_setting_value("bg_llm_provider").await.ok().flatten()
            .or_else(|| std::env::var("BG_LLM_PROVIDER").ok())
            .unwrap_or_else(|| "ollama".to_string());

        let model = self.jq.get_setting_value("bg_llm_model").await.ok().flatten()
            .or_else(|| std::env::var("BG_LLM_MODEL").ok())
            .unwrap_or_else(|| self.fallback_model.clone());

        let api_key = self.resolve_bg_api_key().await;

        match provider_type.as_str() {
            "gemini" => {
                aiome_core::llm_provider::GeminiProvider::new(self.client.clone(), api_key, model)
                    .complete(prompt, system).await
            }
            "openai" => {
                aiome_core::llm_provider::OpenAiProvider::new(self.client.clone(), api_key, model)
                    .complete(prompt, system).await
            }
            "claude" => {
                aiome_core::llm_provider::ClaudeProvider::new(self.client.clone(), api_key, model)
                    .complete(prompt, system).await
            }
            "lmstudio" => {
                let host = self.jq.get_setting_value("lm_studio_host").await.ok().flatten()
                    .unwrap_or_else(|| "http://127.0.0.1:1234".to_string());
                aiome_core::llm_provider::LmStudioProvider::new(self.client.clone(), host, model)
                    .complete(prompt, system).await
            }
            _ => {
                let host = self.jq.get_setting_value("ollama_host").await.ok().flatten()
                    .unwrap_or_else(|| self.fallback_host.clone());
                aiome_core::llm_provider::OllamaProvider::new(host, model)
                    .complete(prompt, system).await
            }
        }
    }

    async fn stream_complete(&self, _prompt: &str, _system: Option<&str>) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        Err(AiomeError::Infrastructure { reason: "Streaming not implemented for BackgroundProvider yet".to_string() })
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.complete("ping", None).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "BackgroundLlm"
    }
}

#[async_trait]
impl EmbeddingProvider for BackgroundLlmProvider {
    async fn embed(&self, text: &str, is_query: bool) -> Result<Vec<f32>, AiomeError> {
        let embed_provider = std::env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "ruri".to_string());

        match embed_provider.as_str() {
            "ruri" => {
                let ruri_url = std::env::var("RURI_EMBED_URL")
                    .unwrap_or_else(|_| "http://localhost:8100".to_string());
                let ruri = aiome_core::llm_provider::RuriProvider::new(
                    self.client.clone(),
                    ruri_url.clone(),
                );
                match ruri.embed(text, is_query).await {
                    Ok(vec) => Ok(vec),
                    Err(_) => self.gemini_embed_fallback(text, is_query).await
                }
            }
            "gemini" => self.gemini_embed_fallback(text, is_query).await,
            _ => {
                let host = self.jq.get_setting_value("ollama_host").await.ok().flatten()
                    .unwrap_or_else(|| self.fallback_host.clone());
                let model = self.jq.get_setting_value("bg_llm_model").await.ok().flatten()
                    .or_else(|| std::env::var("BG_LLM_MODEL").ok())
                    .unwrap_or_else(|| self.fallback_model.clone());
                aiome_core::llm_provider::OllamaProvider::new(host, model)
                    .embed(text, is_query)
                    .await
            }
        }
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.embed("ping", false).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "BackgroundEmbedding"
    }
}

impl BackgroundLlmProvider {
    async fn resolve_bg_api_key(&self) -> String {
        self.jq.get_setting_value("bg_llm_api_key").await.ok().flatten()
            .or_else(|| self.gemini_api_key.as_ref().map(|s| secrecy::ExposeSecret::expose_secret(s).to_string()))
            .or_else(|| self.openai_api_key.as_ref().map(|s| secrecy::ExposeSecret::expose_secret(s).to_string()))
            .or_else(|| self.anthropic_api_key.as_ref().map(|s| secrecy::ExposeSecret::expose_secret(s).to_string()))
            .unwrap_or_default()
    }

    async fn gemini_embed_fallback(
        &self,
        text: &str,
        is_query: bool,
    ) -> Result<Vec<f32>, AiomeError> {
        let mut api_key = self.resolve_bg_api_key().await;
        if api_key.is_empty() {
            api_key = self.jq.get_setting_value("llm_api_key").await.ok().flatten().unwrap_or_default();
        }
        if api_key.is_empty() {
            api_key = self.gemini_api_key.as_ref().map(|s| secrecy::ExposeSecret::expose_secret(s).to_string()).unwrap_or_default();
        }
        if api_key.is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "No embedding provider available: ruri-embed-server not running and no Gemini API key configured".into()
            });
        }
        aiome_core::llm_provider::GeminiProvider::new(
            self.client.clone(),
            api_key,
            "gemini-embedding-001".to_string(),
        )
        .embed(text, is_query)
        .await
    }
}
