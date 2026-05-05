/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::{LlmProvider, LlmResponse};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

use std::fmt;

/// プライマリLLMが失敗した際に、代替(Fallback)LLMへ切り替えるルーター。
/// サーキットブレーカーとリトライ上限、および安全なデフォルト応答を備える。
pub struct FallbackRouter {
    primary: Arc<dyn LlmProvider + Send + Sync>,
    fallback: Arc<dyn LlmProvider + Send + Sync>,
    circuit_breaker: CircuitBreaker,
    #[allow(dead_code)]
    max_retries: usize,
}

impl fmt::Debug for FallbackRouter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FallbackRouter")
            .field("primary", &self.primary.name())
            .field("fallback", &self.fallback.name())
            .finish()
    }
}

impl FallbackRouter {
    /// FallbackRouter を新しく作成します。
    pub fn new(
        primary: Arc<dyn LlmProvider + Send + Sync>,
        fallback: Arc<dyn LlmProvider + Send + Sync>,
        failure_threshold: usize,
    ) -> Self {
        let config = CircuitBreakerConfig {
            failure_threshold,
            reset_timeout: Duration::from_secs(60),
        };
        Self {
            primary,
            fallback,
            circuit_breaker: CircuitBreaker::new("PrimaryLLM", config),
            max_retries: 2,
        }
    }
}

#[async_trait]
impl LlmProvider for FallbackRouter {
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        // 1. サーキットブレーカーの状態確認
        if self.circuit_breaker.check_state().await.is_ok() {
            match self.primary.complete(prompt, system).await {
                Ok(resp) => {
                    self.circuit_breaker.record_success().await;
                    return Ok(LlmResponse {
                        content: resp.content,
                        stop_reason: resp.stop_reason,
                        reasoning: resp.reasoning,
                        metadata: resp.metadata,
                    });
                }
                Err(e) => {
                    tracing::warn!(
                        "⚠️ [FallbackRouter] Primary LLM failed: {}. Recording failure.",
                        e
                    );
                    self.circuit_breaker.record_failure().await;
                }
            }
        } else {
            tracing::info!("🔌 [FallbackRouter] Circuit is OPEN. Skipping primary.");
        }

        // 2. フォールバック実行
        tracing::info!("🔄 [FallbackRouter] Attempting fallback LLM...");
        match self.fallback.complete(prompt, system).await {
            Ok(resp) => Ok(resp),
            Err(e) => {
                tracing::error!("🚨 [FallbackRouter] Fallback LLM also failed: {}", e);
                // 3. 安全なデフォルト応答 (Red Team要請事項)
                Ok(LlmResponse {
                    content: "{\"text\": \"ごめんなさい、ちょっと接続が不安定みたい。あとでまた話しかけてね！\", \"emotion\": \"neutral\", \"action\": \"none\"}".to_string(),
                    stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                    reasoning: None,
                    metadata: None,
                })
            }
        }
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<
        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<String, AiomeError>> + Send>>,
        AiomeError,
    > {
        // ストリーミングのフォールバックは複雑なため、現在はセカンダリを直接呼ぶか、エラーを返す。
        // ここでは簡回化のため、フォールバックへ委譲するロジックのみ。
        if self.circuit_breaker.check_state().await.is_ok() {
            match self.primary.stream_complete(prompt, system).await {
                Ok(stream) => return Ok(stream),
                Err(_) => {
                    self.circuit_breaker.record_failure().await;
                }
            }
        }
        self.fallback.stream_complete(prompt, system).await
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.primary.test_connection().await?;
        self.fallback.test_connection().await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "FallbackRouter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::llm_provider::MockLlmProvider;

    #[tokio::test]
    async fn test_fallback_logic_on_primary_failure() {
        let primary = Arc::new(MockLlmProvider {
            response: "primary".into(),
            should_fail: true,
        });
        let fallback = Arc::new(MockLlmProvider {
            response: "fallback".into(),
            should_fail: false,
        });
        let router = FallbackRouter::new(primary, fallback, 1);

        let result = router.complete("hello", None).await.unwrap();
        assert_eq!(result.content, "fallback");
    }

    #[tokio::test]
    async fn test_safe_default_on_both_failure() {
        let primary = Arc::new(MockLlmProvider {
            response: "primary".into(),
            should_fail: true,
        });
        let fallback = Arc::new(MockLlmProvider {
            response: "fallback".into(),
            should_fail: true,
        });
        let router = FallbackRouter::new(primary, fallback, 1);

        let result = router.complete("hello", None).await.unwrap();
        assert!(result.content.contains("ごめんなさい"));
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens() {
        let primary = Arc::new(MockLlmProvider {
            response: "primary".into(),
            should_fail: true,
        });
        let fallback = Arc::new(MockLlmProvider {
            response: "fallback".into(),
            should_fail: false,
        });
        let router = FallbackRouter::new(primary, fallback, 1);

        // 1. 最初はPrimaryを試して失敗し、Fallbackへ。
        router.complete("hello", None).await.unwrap();

        // 2. 失敗閾値1なので、CircuitはOPEN。
        assert!(router.circuit_breaker.check_state().await.is_err());

        // 3. 次のリクエストはPrimaryをスキップして即Fallbackへ。
        let result = router.complete("hello", None).await.unwrap();
        assert_eq!(result.content, "fallback");
    }
}
