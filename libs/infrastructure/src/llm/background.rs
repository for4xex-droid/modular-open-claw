/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::cost::log_evaluation;
use crate::job_queue::CostOps;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{EmbeddingProvider, LlmProvider, LlmRequest, LlmResponse};
use async_trait::async_trait;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;

#[derive(Clone)]
/// `BackgroundLlmProvider` 構造体
pub struct BackgroundLlmProvider {
    /// jq
    pub ops: Arc<dyn CostOps>,
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
    pub live_manager: Option<Arc<dyn aiome_core_contracts::traits::LiveSessionManager>>,
    /// Phase 3-D+: Injected EvaluationLogger for observability DI
    pub eval_logger: Option<Arc<crate::llm::evaluation_logger::EvaluationLogger>>,
}

impl std::fmt::Debug for BackgroundLlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackgroundLlmProvider")
            .field("fallback_host", &self.fallback_host)
            .field("fallback_model", &self.fallback_model)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl LlmProvider for BackgroundLlmProvider {
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        let cost_breaker =
            crate::llm::cost_breaker::CostCircuitBreaker::new(self.ops.clone(), 10.0);
        cost_breaker.enforce().await?;

        // --- Phase 36: Security Hooks ---
        let mut messages = Vec::new();
        if let Some(sys) = system {
            messages.push(aiome_core_contracts::llm::LlmMessage {
                role: "system".to_string(),
                content: sys.to_string(),
                cache: true,
            });
        }
        messages.push(aiome_core_contracts::llm::LlmMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
            cache: false,
        });

        let request = LlmRequest {
            messages,
            temperature: None,
            max_tokens: None,
            stop_sequences: None,
            format: None,
            metadata: None,
        };
        self.hook_manager.trigger_pre_execute(&request).await?;

        let provider_type = self
            .ops
            .get_setting_value("bg_llm_provider")
            .await
            .ok()
            .flatten()
            .or_else(|| std::env::var("BG_LLM_PROVIDER").ok())
            .unwrap_or_else(|| "ollama".to_string());

        let model = self
            .ops
            .get_setting_value("bg_llm_model")
            .await
            .ok()
            .flatten()
            .or_else(|| std::env::var("BG_LLM_MODEL").ok())
            .unwrap_or_else(|| self.fallback_model.clone());

        let api_key = self.resolve_bg_api_key().await;

        let log_provider = provider_type.clone();
        let log_model = model.clone();
        let start_time = std::time::Instant::now();

        let res = match provider_type.as_str() {
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
                    .ops
                    .get_setting_value("lm_studio_host")
                    .await?
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
                    .ops
                    .get_setting_value("ollama_host")
                    .await?
                    .unwrap_or_else(|| self.fallback_host.clone());
                let provider = aiome_core::llm_provider::OllamaProvider::new(host, model);
                provider.complete(prompt, system).await
            }
        };

        let latency_ms = start_time.elapsed().as_millis() as i64;

        // --- Phase 36: Post Hooks ---
        if let Ok(ref response) = res {
            let cache_hit = response
                .metadata
                .as_ref()
                .and_then(|m| m.get("cache_hit"))
                .map(|v| v == "true")
                .unwrap_or(false);
            let p = prompt.to_string();
            let s = system.map(|s| s.to_string());
            let logger_opt = self.eval_logger.clone();

            // Extract token counts from metadata if available (typical keys: prompt_tokens, completion_tokens)
            let (token_in, token_out) = response.metadata.as_ref().map_or((None, None), |m| {
                (
                    m.get("prompt_tokens").and_then(|v| v.parse::<i64>().ok()),
                    m.get("completion_tokens")
                        .and_then(|v| v.parse::<i64>().ok()),
                )
            });

            tokio::spawn(async move {
                if let Some(logger) = logger_opt {
                    log_evaluation(
                        logger,
                        p,
                        s,
                        log_provider,
                        log_model,
                        latency_ms,
                        cache_hit,
                        token_in,
                        token_out,
                    )
                    .await;
                }
            });

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
        let cost_breaker =
            crate::llm::cost_breaker::CostCircuitBreaker::new(self.ops.clone(), 10.0);
        cost_breaker.enforce().await?;

        // --- Phase 36: Security Hooks ---
        self.hook_manager.trigger_pre_execute(&request).await?;

        let provider_type = self
            .ops
            .get_setting_value("bg_llm_provider")
            .await
            .ok()
            .flatten()
            .or_else(|| std::env::var("BG_LLM_PROVIDER").ok())
            .unwrap_or_else(|| "ollama".to_string());

        let model = self
            .ops
            .get_setting_value("bg_llm_model")
            .await
            .ok()
            .flatten()
            .or_else(|| std::env::var("BG_LLM_MODEL").ok())
            .unwrap_or_else(|| self.fallback_model.clone());

        let api_key = self.resolve_bg_api_key().await;

        let log_provider = provider_type.clone();
        let log_model = model.clone();
        let start_time = std::time::Instant::now();

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
                    .ops
                    .get_setting_value("lm_studio_host")
                    .await?
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
                    .ops
                    .get_setting_value("ollama_host")
                    .await?
                    .unwrap_or_else(|| self.fallback_host.clone());
                let provider = aiome_core::llm_provider::OllamaProvider::new(host, model);
                provider.complete_with_cache(request.clone()).await
            }
        };

        let latency_ms = start_time.elapsed().as_millis() as i64;

        // --- Phase 36: Post Hooks ---
        if let Ok(ref response) = res {
            let cache_hit = response
                .metadata
                .as_ref()
                .and_then(|m| m.get("cache_hit"))
                .map(|v| v == "true")
                .unwrap_or(false);

            let mut p = String::new();
            let mut s = None;
            for m in &request.messages {
                if m.role == "user" {
                    p = m.content.clone();
                }
                if m.role == "system" {
                    s = Some(m.content.clone());
                }
            }

            let logger_opt = self.eval_logger.clone();

            let (token_in, token_out) = response.metadata.as_ref().map_or((None, None), |m| {
                (
                    m.get("prompt_tokens").and_then(|v| v.parse::<i64>().ok()),
                    m.get("completion_tokens")
                        .and_then(|v| v.parse::<i64>().ok()),
                )
            });

            tokio::spawn(async move {
                if let Some(logger) = logger_opt {
                    log_evaluation(
                        logger,
                        p,
                        s,
                        log_provider,
                        log_model,
                        latency_ms,
                        cache_hit,
                        token_in,
                        token_out,
                    )
                    .await;
                }
            });

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
                    Err(e) => {
                        tracing::warn!(
                            "⚠️ [Embedding] Ruri embed failed, falling back to Gemini: {}",
                            e
                        );
                        self.gemini_embed_fallback(text, is_query).await
                    }
                }
            }
            "gemini" => self.gemini_embed_fallback(text, is_query).await,
            _ => {
                let host = self
                    .ops
                    .get_setting_value("ollama_host")
                    .await?
                    .unwrap_or_else(|| self.fallback_host.clone());
                let model = self
                    .ops
                    .get_setting_value("bg_llm_model")
                    .await?
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

    fn embedding_dim(&self) -> usize {
        768
    }
}

impl BackgroundLlmProvider {
    async fn resolve_bg_api_key(&self) -> secrecy::SecretString {
        self.ops
            .get_setting_value("bg_llm_api_key")
            .await
            .ok()
            .flatten()
            .map(|s| secrecy::SecretString::from(s))
            .or_else(|| self.gemini_api_key.clone())
            .or_else(|| self.openai_api_key.clone())
            .or_else(|| self.anthropic_api_key.clone())
            .unwrap_or_else(|| secrecy::SecretString::from(String::new()))
    }

    async fn gemini_embed_fallback(
        &self,
        text: &str,
        is_query: bool,
    ) -> Result<Vec<f32>, AiomeError> {
        let mut api_key = self.resolve_bg_api_key().await;
        use secrecy::ExposeSecret;
        if api_key.expose_secret().is_empty() {
            api_key = self
                .ops
                .get_setting_value("llm_api_key")
                .await
                .ok()
                .flatten()
                .map(|s| secrecy::SecretString::from(s))
                .unwrap_or_else(|| secrecy::SecretString::from(String::new()));
        }
        if api_key.expose_secret().is_empty() {
            api_key = self
                .gemini_api_key
                .clone()
                .unwrap_or_else(|| secrecy::SecretString::from(String::new()));
        }
        if api_key.expose_secret().is_empty() {
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
