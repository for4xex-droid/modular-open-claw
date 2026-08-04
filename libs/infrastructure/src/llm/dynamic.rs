/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::circuit_breaker::CircuitBreaker;
use crate::job_queue::CostOps;
use crate::slo_engine::SloEngine;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{EmbeddingProvider, LlmProvider, LlmRequest, LlmResponse};
use aiome_core_contracts::traits::JobQueue;
use async_trait::async_trait;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;

#[derive(Clone)]
/// `DynamicLlmProvider` 構造体
pub struct DynamicLlmProvider {
    /// jq
    pub ops: Arc<dyn CostOps>,
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
    pub live_manager: Option<Arc<dyn aiome_core_contracts::traits::LiveSessionManager>>,
    /// Phase 3-D+: Injected EvaluationLogger for observability DI
    pub eval_logger: Option<Arc<crate::llm::evaluation_logger::EvaluationLogger>>,
}

impl std::fmt::Debug for DynamicLlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicLlmProvider")
            .field("fallback_host", &self.fallback_host)
            .field("fallback_model", &self.fallback_model)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl LlmProvider for DynamicLlmProvider {
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
                cache: true, // Enable context caching for system preamble
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

        let (provider_type, model) = self.resolve_config(false).await;

        if let Err(e) = self.circuit_breaker.check_state().await {
            return Err(AiomeError::Infrastructure {
                reason: e.to_string(),
            });
        }

        let log_provider = provider_type.clone();
        let log_model = model.clone();
        let start_time = std::time::Instant::now();

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
                    .get_host("lm_studio_host", shared::config::DEFAULT_LM_STUDIO_HOST)
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

        let latency_ms = start_time.elapsed().as_millis() as i64;

        // --- Phase 36: Post Hooks ---
        if let Ok(ref response) = result {
            let cache_hit = response
                .metadata
                .as_ref()
                .and_then(|m| m.get("cache_hit"))
                .map(|v| v == "true")
                .unwrap_or(false);
            let p = prompt.to_string();
            let s = system.map(|s| s.to_string());
            let logger_opt = self.eval_logger.clone();

            let (token_in, token_out) = response.metadata.as_ref().map_or((None, None), |m| {
                (
                    m.get("prompt_tokens").and_then(|v| v.parse::<i64>().ok()),
                    m.get("completion_tokens")
                        .and_then(|v| v.parse::<i64>().ok()),
                )
            });
            let (route_tier, route_reason, route_mode) =
                super::cost::route_fields_from_metadata(response.metadata.as_ref());

            // Non-blocking log using tokio::spawn
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
                        route_tier,
                        route_reason,
                        route_mode,
                    )
                    .await;
                }
            });

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
        let cost_breaker =
            crate::llm::cost_breaker::CostCircuitBreaker::new(self.ops.clone(), 10.0);
        cost_breaker.enforce().await?;

        // --- Phase 36: Security Hooks (Streaming) ---
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

        // DS-1 FIX: Apply Circuit Breaker protection to streaming results
        if self.circuit_breaker.check_state().await.is_err() {
            self.slo_engine.record_error().await;
            return Err(AiomeError::Infrastructure {
                reason: "Circuit Breaker is OPEN (LLM service degraded)".into(),
            });
        }

        let (provider_type, model) = self.resolve_config(false).await;

        // Deep Scan Layer Fix: Capture context for logging intercept
        let log_provider = provider_type.clone();
        let log_model = model.clone();
        let p = prompt.to_string();
        let s = system.map(|s| s.to_string());
        let logger_opt = self.eval_logger.clone();
        let start_time = std::time::Instant::now();

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
                    .get_host("lm_studio_host", shared::config::DEFAULT_LM_STUDIO_HOST)
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

        match self.handle_result(result).await {
            Ok(mut stream) => {
                use futures::StreamExt;
                let stream_wrapper = async_stream::stream! {
                    let mut success_flag = false;
                    while let Some(chunk) = stream.next().await {
                        if chunk.is_ok() {
                            success_flag = true;
                        }
                        yield chunk;
                    }
                    if success_flag {
                         let latency_ms = start_time.elapsed().as_millis() as i64;
                         tokio::spawn(async move {
                             if let Some(logger) = logger_opt {
                                 log_evaluation(logger, p, s, log_provider, log_model, latency_ms, false, None, None, None, None, None).await;
                             }
                         });
                    }
                };
                Ok(Box::pin(stream_wrapper)
                    as Pin<
                        Box<dyn Stream<Item = Result<String, AiomeError>> + Send>,
                    >)
            }
            Err(e) => Err(e),
        }
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.complete("ping", None).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "DynamicLlm"
    }

    async fn complete_with_cache(&self, request: LlmRequest) -> Result<LlmResponse, AiomeError> {
        let cost_breaker =
            crate::llm::cost_breaker::CostCircuitBreaker::new(self.ops.clone(), 10.0);
        cost_breaker.enforce().await?;

        // --- Phase 36: Security Hooks ---
        self.hook_manager.trigger_pre_execute(&request).await?;

        let (provider_type, model) = self.resolve_config(false).await;

        if let Err(e) = self.circuit_breaker.check_state().await {
            return Err(AiomeError::Infrastructure {
                reason: e.to_string(),
            });
        }

        let log_provider = provider_type.clone();
        let log_model = model.clone();
        let start_time = std::time::Instant::now();

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
                    .get_host("lm_studio_host", shared::config::DEFAULT_LM_STUDIO_HOST)
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

        let latency_ms = start_time.elapsed().as_millis() as i64;

        // --- Phase 36: Post Hooks ---
        if let Ok(ref response) = result {
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
            // OP-099 fix (H-1): IR は request 側に route_* を注入するため request 優先
            let (req_tier, req_reason, req_mode) =
                super::cost::route_fields_from_metadata(request.metadata.as_ref());
            let (resp_tier, resp_reason, resp_mode) =
                super::cost::route_fields_from_metadata(response.metadata.as_ref());
            let (route_tier, route_reason, route_mode) = (
                req_tier.or(resp_tier),
                req_reason.or(resp_reason),
                req_mode.or(resp_mode),
            );

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
                        route_tier,
                        route_reason,
                        route_mode,
                    )
                    .await;
                }
            });

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

    fn embedding_dim(&self) -> usize {
        768 // Standard for nomic-embed-text / Gemini
    }
}

impl DynamicLlmProvider {
    async fn resolve_config(&self, is_bg: bool) -> (String, String) {
        let prefix = if is_bg { "bg_" } else { "" };

        let provider = self
            .ops
            .get_setting_value(&format!("{}llm_provider", prefix))
            .await
            .ok()
            .flatten()
            .or_else(|| {
                if is_bg {
                    std::env::var("BG_LLM_PROVIDER").ok()
                } else {
                    std::env::var("LLM_PROVIDER").ok()
                }
            })
            .unwrap_or_else(|| "ollama".to_string());

        let model: String = self
            .ops
            .get_setting_value(&format!("{}llm_model", prefix))
            .await
            .ok()
            .flatten()
            .or_else(|| {
                if is_bg {
                    std::env::var("BG_LLM_MODEL").ok()
                } else {
                    std::env::var("LLM_MODEL").ok()
                }
            })
            .unwrap_or_else(|| self.fallback_model.clone());

        (provider, model)
    }

    async fn get_api_key(&self, setting_name: &str, provider_name: &str) -> secrecy::SecretString {
        self.ops
            .get_setting_value(setting_name)
            .await
            .ok()
            .flatten()
            .map(|s| secrecy::SecretString::from(s))
            .or_else(|| match provider_name {
                "gemini" => self.gemini_api_key.clone(),
                "openai" => self.openai_api_key.clone(),
                "claude" => self.anthropic_api_key.clone(),
                _ => None,
            })
            .unwrap_or_else(|| secrecy::SecretString::from(String::new()))
    }

    async fn get_host(&self, setting_name: &str, default: &str) -> String {
        self.ops
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

// 外部互換の re-export
pub use super::background::BackgroundLlmProvider;
pub use super::cost::{calculate_cost_coins, calculate_cost_usd, log_evaluation};
