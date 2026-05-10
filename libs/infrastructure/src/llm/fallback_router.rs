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
        let mut current_prompt = prompt.to_string();
        let mut retries = 0;

        // 1. サーキットブレーカーの状態確認
        if self.circuit_breaker.check_state().await.is_ok() {
            loop {
                match self.primary.complete(&current_prompt, system).await {
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
                        let err_msg = e.to_string();
                        tracing::warn!(
                            "⚠️ [FallbackRouter] Primary LLM failed (try {}): {}",
                            retries + 1,
                            err_msg
                        );

                        if retries < self.max_retries {
                            if err_msg.contains("429")
                                || err_msg.contains("Rate Limit")
                                || err_msg.contains("Too Many Requests")
                            {
                                // Exponential backoff
                                let delay = 2_u64.pow(retries as u32);
                                tracing::warn!(
                                    "⏳ [FallbackRouter] 429 Rate Limit. Retrying in {}s...",
                                    delay
                                );
                                tokio::time::sleep(Duration::from_secs(delay)).await;
                                retries += 1;
                                continue;
                            } else if err_msg.contains("400")
                                || err_msg.contains("Context Length")
                                || err_msg.contains("context_length_exceeded")
                            {
                                // Context trim
                                tracing::warn!("✂️ [FallbackRouter] 400 Context Length Exceeded. Trimming prompt and retrying...");
                                let total_chars = current_prompt.chars().count();
                                let chars_to_keep = (total_chars as f64 * 0.8) as usize;
                                if chars_to_keep > 0 && chars_to_keep < total_chars {
                                    let chars_to_drop = total_chars - chars_to_keep;
                                    current_prompt = format!(
                                        "...[trimmed for context length]...\n{}",
                                        current_prompt
                                            .chars()
                                            .skip(chars_to_drop)
                                            .collect::<String>()
                                    );
                                    retries += 1;
                                    continue;
                                }
                            } else if err_msg.contains("500")
                                || err_msg.contains("503")
                                || err_msg.contains("Internal Server Error")
                            {
                                // Immediate failover
                                tracing::error!(
                                    "🚨 [FallbackRouter] 5xx Server Error. Immediate failover."
                                );
                                break;
                            }
                        }

                        // If not retriable or retries exhausted
                        self.circuit_breaker.record_failure().await;
                        break;
                    }
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

    #[derive(Debug)]
    struct ClassifierMockLlm {
        pub response: String,
        pub error_msg: Option<String>,
        pub call_count: Arc<tokio::sync::Mutex<usize>>,
        pub received_prompts: Arc<tokio::sync::Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl LlmProvider for ClassifierMockLlm {
        async fn complete(
            &self,
            prompt: &str,
            _system: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            let mut count = self.call_count.lock().await;
            *count += 1;
            let mut prompts = self.received_prompts.lock().await;
            prompts.push(prompt.to_string());

            if let Some(msg) = &self.error_msg {
                if msg.contains("429") && *count < 3 {
                    return Err(AiomeError::LlmResponse {
                        source: anyhow::anyhow!("{}", msg),
                    });
                }
                if msg.contains("400") && *count < 2 {
                    return Err(AiomeError::LlmResponse {
                        source: anyhow::anyhow!("{}", msg),
                    });
                }
                if msg.contains("500") {
                    return Err(AiomeError::LlmResponse {
                        source: anyhow::anyhow!("{}", msg),
                    });
                }
            }

            Ok(LlmResponse {
                content: self.response.clone(),
                stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            })
        }
        async fn stream_complete(
            &self,
            _p: &str,
            _s: Option<&str>,
        ) -> Result<
            std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<String, AiomeError>> + Send>>,
            AiomeError,
        > {
            unimplemented!()
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "ClassifierMock"
        }
    }

    #[tokio::test]
    async fn test_error_classifier_429_rate_limit() {
        let call_count = Arc::new(tokio::sync::Mutex::new(0));
        let primary = Arc::new(ClassifierMockLlm {
            response: "primary_success".into(),
            error_msg: Some("HTTP 429: Too Many Requests".into()),
            call_count: call_count.clone(),
            received_prompts: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        });
        let fallback = Arc::new(MockLlmProvider {
            response: "fallback".into(),
            should_fail: false,
        });

        // 429 should trigger internal retry before fallback
        let router = FallbackRouter::new(primary, fallback, 3);
        let result = router.complete("hello", None).await.unwrap();

        // It should succeed on the 3rd try on primary
        assert_eq!(result.content, "primary_success");
        let count = *call_count.lock().await;
        assert_eq!(count, 3, "Should have retried 2 times");
    }

    #[tokio::test]
    async fn test_error_classifier_400_context_length() {
        let received_prompts = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let primary = Arc::new(ClassifierMockLlm {
            response: "primary_success".into(),
            error_msg: Some("HTTP 400: Context Length Exceeded".into()),
            call_count: Arc::new(tokio::sync::Mutex::new(0)),
            received_prompts: received_prompts.clone(),
        });
        let fallback = Arc::new(MockLlmProvider {
            response: "fallback".into(),
            should_fail: false,
        });

        let router = FallbackRouter::new(primary, fallback, 3);
        let long_prompt = "A".repeat(10000);
        let result = router.complete(&long_prompt, None).await.unwrap();

        assert_eq!(result.content, "primary_success");
        let prompts = received_prompts.lock().await;
        assert_eq!(prompts.len(), 2, "Should have retried once");
        assert!(
            prompts[1].len() < prompts[0].len(),
            "Prompt should have been trimmed on 400 error"
        );
    }

    #[tokio::test]
    async fn test_error_classifier_500_immediate_failover() {
        let call_count = Arc::new(tokio::sync::Mutex::new(0));
        let primary = Arc::new(ClassifierMockLlm {
            response: "primary_success".into(),
            error_msg: Some("HTTP 500: Internal Server Error".into()),
            call_count: call_count.clone(),
            received_prompts: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        });
        let fallback = Arc::new(MockLlmProvider {
            response: "fallback".into(),
            should_fail: false,
        });

        let router = FallbackRouter::new(primary, fallback, 3);
        let result = router.complete("hello", None).await.unwrap();

        // Should immediately failover to fallback without retrying primary
        assert_eq!(result.content, "fallback");
        let count = *call_count.lock().await;
        assert_eq!(count, 1, "Should only try primary once on 500");
    }
}
