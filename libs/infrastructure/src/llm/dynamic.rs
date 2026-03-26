/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::circuit_breaker::CircuitBreaker;
use crate::job_queue::UniversalJobQueue;
use crate::slo_engine::SloEngine;
use aiome_contracts::error::AiomeError;
use aiome_contracts::llm::{EmbeddingProvider, LlmProvider, LlmRequest, LlmResponse};
use aiome_contracts::traits::JobQueue;
use async_trait::async_trait;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;

#[derive(Debug)]
/// `DynamicLlmProvider` 構造体
pub struct DynamicLlmProvider {
    /// jq
    pub jq: Arc<UniversalJobQueue>,
    /// client
    pub client: reqwest::Client,
    /// fallback_host
    pub fallback_host: String,
    /// fallback_model
    pub fallback_model: String,
    /// Google Gemini APIキー（SecretStringで保護）
    pub gemini_api_key: Option<secrecy::SecretString>,
    /// OpenAI APIキー（SecretStringで保護）
    pub openai_api_key: Option<secrecy::SecretString>,
    /// Anthropic APIキー（SecretStringで保護）
    pub anthropic_api_key: Option<secrecy::SecretString>,
    /// circuit_breaker
    pub circuit_breaker: Arc<CircuitBreaker>,
    /// slo_engine
    pub slo_engine: Arc<SloEngine>,
    /// hook_manager
    pub hook_manager: Arc<crate::security::hook_manager::HookManager>,
    /// live_manager (Phase 6)
    pub live_manager: Option<Arc<dyn aiome_contracts::traits::LiveSessionManager>>,
}

#[async_trait]
impl LlmProvider for DynamicLlmProvider {
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        let cost_breaker = crate::llm::cost_breaker::CostCircuitBreaker::new(self.jq.clone(), 10.0);
        cost_breaker.enforce().await?;

        // --- Phase 36: Security Hooks ---
        let request = LlmRequest {
            messages: vec![aiome_contracts::llm::LlmMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
                cache: false,
            }],
            temperature: None,
            max_tokens: None,
            stop_sequences: None,
            format: None,
            metadata: None,
        };
        self.hook_manager.trigger_pre_execute(&request).await?;

        let (provider_type, model) = self.resolve_config(false).await;

        if let Err(e) = self.circuit_breaker.check_state().await {
            return Err(AiomeError::Infrastructure {
                reason: e.to_string(),
            });
        }

        let result = match provider_type.as_str() {
            "gemini" => {
                let api_key = self.get_api_key("llm_api_key", "gemini").await;
                // InteractionsGeminiProvider を優先的に使用 (Phase 5)
                let provider =
                    aiome_core::llm_provider::interactions::InteractionsGeminiProvider::new(
                        self.client.clone(),
                        api_key,
                        model,
                    );
                provider.complete_with_cache(request.clone()).await
            }
            "openai" => {
                let api_key = self.get_api_key("llm_api_key", "openai").await;
                let provider = aiome_core::llm_provider::OpenAiProvider::new(
                    self.client.clone(),
                    api_key,
                    model,
                );
                provider.complete(prompt, system).await
            }
            "claude" => {
                let api_key = self.get_api_key("llm_api_key", "claude").await;
                let provider = aiome_core::llm_provider::ClaudeProvider::new(
                    self.client.clone(),
                    api_key,
                    model,
                );
                provider.complete(prompt, system).await
            }
            "lmstudio" => {
                let host = self
                    .get_host("lm_studio_host", "http://127.0.0.1:1234")
                    .await;
                let provider = aiome_core::llm_provider::LmStudioProvider::new(
                    self.client.clone(),
                    host,
                    model,
                );
                provider.complete(prompt, system).await
            }
            _ => {
                let host = self.get_host("ollama_host", &self.fallback_host).await;
                let provider = aiome_core::llm_provider::OllamaProvider::new(host, model);
                provider.complete(prompt, system).await
            }
        };

        let result = self.handle_result(result).await;

        // --- Phase 36: Post Hooks ---
        if let Ok(ref response) = result {
            self.hook_manager
                .trigger_post_execute(&request, response)
                .await?;
        }

        result
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        // --- Day 1: Cost Control ---
        let cost_breaker = crate::llm::cost_breaker::CostCircuitBreaker::new(self.jq.clone(), 10.0);
        cost_breaker.enforce().await?;

        // DS-1 FIX: Apply Circuit Breaker protection to streaming results
        if self.circuit_breaker.check_state().await.is_err() {
            self.slo_engine.record_error().await;
            return Err(AiomeError::Infrastructure {
                reason: "Circuit Breaker is OPEN (LLM service degraded)".into(),
            });
        }

        let (provider_type, model) = self.resolve_config(false).await;

        let result = match provider_type.as_str() {
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
                let host = self
                    .get_host("lm_studio_host", "http://127.0.0.1:1234")
                    .await;
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
        };

        self.handle_result(result).await
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.complete("ping", None).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "DynamicLlm"
    }

    async fn complete_with_cache(&self, request: LlmRequest) -> Result<LlmResponse, AiomeError> {
        let cost_breaker = crate::llm::cost_breaker::CostCircuitBreaker::new(self.jq.clone(), 10.0);
        cost_breaker.enforce().await?;

        // --- Phase 36: Security Hooks ---
        self.hook_manager.trigger_pre_execute(&request).await?;

        let (provider_type, model) = self.resolve_config(false).await;

        if let Err(e) = self.circuit_breaker.check_state().await {
            return Err(AiomeError::Infrastructure {
                reason: e.to_string(),
            });
        }

        let result = match provider_type.as_str() {
            "gemini" => {
                let api_key = self.get_api_key("llm_api_key", "gemini").await;
                let provider =
                    aiome_core::llm_provider::interactions::InteractionsGeminiProvider::new(
                        self.client.clone(),
                        api_key,
                        model,
                    );
                provider.complete_with_cache(request.clone()).await
            }
            "openai" => {
                let api_key = self.get_api_key("llm_api_key", "openai").await;
                let provider = aiome_core::llm_provider::OpenAiProvider::new(
                    self.client.clone(),
                    api_key,
                    model,
                );
                provider.complete_with_cache(request.clone()).await
            }
            "claude" => {
                let api_key = self.get_api_key("llm_api_key", "claude").await;
                let provider = aiome_core::llm_provider::ClaudeProvider::new(
                    self.client.clone(),
                    api_key,
                    model,
                );
                provider.complete_with_cache(request.clone()).await
            }
            "lmstudio" => {
                let host = self
                    .get_host("lm_studio_host", "http://127.0.0.1:1234")
                    .await;
                let provider = aiome_core::llm_provider::LmStudioProvider::new(
                    self.client.clone(),
                    host,
                    model,
                );
                provider.complete_with_cache(request.clone()).await
            }
            _ => {
                let host = self.get_host("ollama_host", &self.fallback_host).await;
                let provider = aiome_core::llm_provider::OllamaProvider::new(host, model);
                provider.complete_with_cache(request.clone()).await
            }
        };

        let result = self.handle_result(result).await;

        // --- Phase 36: Post Hooks ---
        if let Ok(ref response) = result {
            self.hook_manager
                .trigger_post_execute(&request, response)
                .await?;
        }

        result
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

        let provider = self
            .jq
            .get_setting_value(&format!("{}llm_provider", prefix))
            .await
            .ok()
            .flatten()
            .or_else(|| {
                if is_bg {
                    std::env::var("BG_LLM_PROVIDER").ok()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "ollama".to_string());

        let model: String = self
            .jq
            .get_setting_value(&format!("{}llm_model", prefix))
            .await
            .ok()
            .flatten()
            .or_else(|| {
                if is_bg {
                    std::env::var("BG_LLM_MODEL").ok()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| self.fallback_model.clone());

        (provider, model)
    }

    async fn get_api_key(&self, setting_name: &str, provider_name: &str) -> String {
        self.jq
            .get_setting_value(setting_name)
            .await
            .ok()
            .flatten()
            .or_else(|| match provider_name {
                "gemini" => self
                    .gemini_api_key
                    .as_ref()
                    .map(|s| secrecy::ExposeSecret::expose_secret(s).to_string()),
                "openai" => self
                    .openai_api_key
                    .as_ref()
                    .map(|s| secrecy::ExposeSecret::expose_secret(s).to_string()),
                "claude" => self
                    .anthropic_api_key
                    .as_ref()
                    .map(|s| secrecy::ExposeSecret::expose_secret(s).to_string()),
                _ => None,
            })
            .unwrap_or_default()
    }

    async fn get_host(&self, setting_name: &str, default: &str) -> String {
        self.jq
            .get_setting_value(setting_name)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| default.to_string())
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
/// `BackgroundLlmProvider` 構造体
pub struct BackgroundLlmProvider {
    /// jq
    pub jq: Arc<UniversalJobQueue>,
    /// client
    pub client: reqwest::Client,
    /// fallback_model
    pub fallback_model: String,
    /// fallback_host
    pub fallback_host: String,
    /// Google Gemini APIキー（SecretStringで保護）
    pub gemini_api_key: Option<secrecy::SecretString>,
    /// OpenAI APIキー（SecretStringで保護）
    pub openai_api_key: Option<secrecy::SecretString>,
    /// Anthropic APIキー（SecretStringで保護）
    pub anthropic_api_key: Option<secrecy::SecretString>,
    /// hook_manager
    pub hook_manager: Arc<crate::security::hook_manager::HookManager>,
    /// live_manager (Phase 6)
    pub live_manager: Option<Arc<dyn aiome_contracts::traits::LiveSessionManager>>,
}

#[async_trait]
impl LlmProvider for BackgroundLlmProvider {
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        let cost_breaker = crate::llm::cost_breaker::CostCircuitBreaker::new(self.jq.clone(), 10.0);
        cost_breaker.enforce().await?;

        // --- Phase 36: Security Hooks ---
        let request = LlmRequest {
            messages: vec![aiome_contracts::llm::LlmMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
                cache: false,
            }],
            temperature: None,
            max_tokens: None,
            stop_sequences: None,
            format: None,
            metadata: None,
        };
        self.hook_manager.trigger_pre_execute(&request).await?;

        let provider_type = self
            .jq
            .get_setting_value("bg_llm_provider")
            .await
            .ok()
            .flatten()
            .or_else(|| std::env::var("BG_LLM_PROVIDER").ok())
            .unwrap_or_else(|| "ollama".to_string());

        let model = self
            .jq
            .get_setting_value("bg_llm_model")
            .await
            .ok()
            .flatten()
            .or_else(|| std::env::var("BG_LLM_MODEL").ok())
            .unwrap_or_else(|| self.fallback_model.clone());

        let api_key = self.resolve_bg_api_key().await;

        let res = match provider_type.as_str() {
            // ... (keep existing patterns)
            "gemini" => {
                let provider = aiome_core::llm_provider::GeminiProvider::new(
                    self.client.clone(),
                    api_key,
                    model,
                );
                provider.complete(prompt, system).await
            }
            "openai" => {
                let provider = aiome_core::llm_provider::OpenAiProvider::new(
                    self.client.clone(),
                    api_key,
                    model,
                );
                provider.complete(prompt, system).await
            }
            "claude" => {
                let provider = aiome_core::llm_provider::ClaudeProvider::new(
                    self.client.clone(),
                    api_key,
                    model,
                );
                provider.complete(prompt, system).await
            }
            "lmstudio" => {
                let host = self
                    .jq
                    .get_setting_value("lm_studio_host")
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| shared::config::DEFAULT_LM_STUDIO_HOST.to_string());
                let provider = aiome_core::llm_provider::LmStudioProvider::new(
                    self.client.clone(),
                    host,
                    model,
                );
                provider.complete(prompt, system).await
            }
            _ => {
                let host = self
                    .jq
                    .get_setting_value("ollama_host")
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| self.fallback_host.clone());
                let provider = aiome_core::llm_provider::OllamaProvider::new(host, model);
                provider.complete(prompt, system).await
            }
        };

        // --- Phase 36: Post Hooks ---
        if let Ok(ref response) = res {
            self.hook_manager
                .trigger_post_execute(&request, response)
                .await?;
        }
        res
    }

    async fn stream_complete(
        &self,
        _prompt: &str,
        _system: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "Streaming not implemented for BackgroundProvider yet".to_string(),
        })
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.complete("ping", None).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "BackgroundLlm"
    }

    async fn complete_with_cache(&self, request: LlmRequest) -> Result<LlmResponse, AiomeError> {
        let cost_breaker = crate::llm::cost_breaker::CostCircuitBreaker::new(self.jq.clone(), 10.0);
        cost_breaker.enforce().await?;

        // --- Phase 36: Security Hooks ---
        self.hook_manager.trigger_pre_execute(&request).await?;

        let provider_type = self
            .jq
            .get_setting_value("bg_llm_provider")
            .await
            .ok()
            .flatten()
            .or_else(|| std::env::var("BG_LLM_PROVIDER").ok())
            .unwrap_or_else(|| "ollama".to_string());

        let model = self
            .jq
            .get_setting_value("bg_llm_model")
            .await
            .ok()
            .flatten()
            .or_else(|| std::env::var("BG_LLM_MODEL").ok())
            .unwrap_or_else(|| self.fallback_model.clone());

        let api_key = self.resolve_bg_api_key().await;

        let res = match provider_type.as_str() {
            "gemini" => {
                let provider = aiome_core::llm_provider::GeminiProvider::new(
                    self.client.clone(),
                    api_key,
                    model,
                );
                provider.complete_with_cache(request.clone()).await
            }
            "openai" => {
                let provider = aiome_core::llm_provider::OpenAiProvider::new(
                    self.client.clone(),
                    api_key,
                    model,
                );
                provider.complete_with_cache(request.clone()).await
            }
            "claude" => {
                let provider = aiome_core::llm_provider::ClaudeProvider::new(
                    self.client.clone(),
                    api_key,
                    model,
                );
                provider.complete_with_cache(request.clone()).await
            }
            "lmstudio" => {
                let host = self
                    .jq
                    .get_setting_value("lm_studio_host")
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| shared::config::DEFAULT_LM_STUDIO_HOST.to_string());
                let provider = aiome_core::llm_provider::LmStudioProvider::new(
                    self.client.clone(),
                    host,
                    model,
                );
                provider.complete_with_cache(request.clone()).await
            }
            _ => {
                let host = self
                    .jq
                    .get_setting_value("ollama_host")
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| self.fallback_host.clone());
                let provider = aiome_core::llm_provider::OllamaProvider::new(host, model);
                provider.complete_with_cache(request.clone()).await
            }
        };

        // --- Phase 36: Post Hooks ---
        if let Ok(ref response) = res {
            self.hook_manager
                .trigger_post_execute(&request, response)
                .await?;
        }
        res
    }
}

#[async_trait]
impl EmbeddingProvider for BackgroundLlmProvider {
    async fn embed(&self, text: &str, is_query: bool) -> Result<Vec<f32>, AiomeError> {
        let embed_provider =
            std::env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "ruri".to_string());

        match embed_provider.as_str() {
            "ruri" => {
                let ruri_url = std::env::var("RURI_EMBED_URL")
                    .unwrap_or_else(|_| shared::config::DEFAULT_RURI_EMBED_URL.to_string());
                let ruri = aiome_core::llm_provider::RuriProvider::new(
                    self.client.clone(),
                    ruri_url.clone(),
                );
                match ruri.embed(text, is_query).await {
                    Ok(vec) => Ok(vec),
                    Err(_) => self.gemini_embed_fallback(text, is_query).await,
                }
            }
            "gemini" => self.gemini_embed_fallback(text, is_query).await,
            _ => {
                let host = self
                    .jq
                    .get_setting_value("ollama_host")
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| self.fallback_host.clone());
                let model = self
                    .jq
                    .get_setting_value("bg_llm_model")
                    .await
                    .ok()
                    .flatten()
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
        self.jq
            .get_setting_value("bg_llm_api_key")
            .await
            .ok()
            .flatten()
            .or_else(|| {
                self.gemini_api_key
                    .as_ref()
                    .map(|s| secrecy::ExposeSecret::expose_secret(s).to_string())
            })
            .or_else(|| {
                self.openai_api_key
                    .as_ref()
                    .map(|s| secrecy::ExposeSecret::expose_secret(s).to_string())
            })
            .or_else(|| {
                self.anthropic_api_key
                    .as_ref()
                    .map(|s| secrecy::ExposeSecret::expose_secret(s).to_string())
            })
            .unwrap_or_default()
    }

    async fn gemini_embed_fallback(
        &self,
        text: &str,
        is_query: bool,
    ) -> Result<Vec<f32>, AiomeError> {
        let mut api_key = self.resolve_bg_api_key().await;
        if api_key.is_empty() {
            api_key = self
                .jq
                .get_setting_value("llm_api_key")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
        }
        if api_key.is_empty() {
            api_key = self
                .gemini_api_key
                .as_ref()
                .map(|s| secrecy::ExposeSecret::expose_secret(s).to_string())
                .unwrap_or_default();
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
