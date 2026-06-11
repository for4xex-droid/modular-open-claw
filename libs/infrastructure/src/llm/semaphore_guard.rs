/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_contracts::error::AiomeError;
use aiome_contracts::llm::{LlmProvider, LlmRequest, LlmResponse};
use async_trait::async_trait;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_stream::Stream;

/// LLMプロバイダーへの同時実行数をセマフォで制限するラッパー
#[derive(Debug)]
pub struct SemaphoreGuardedProvider {
    inner: Arc<dyn LlmProvider + Send + Sync>,
    semaphore: Arc<Semaphore>,
}

impl SemaphoreGuardedProvider {
    pub fn new(inner: Arc<dyn LlmProvider + Send + Sync>, semaphore: Arc<Semaphore>) -> Self {
        Self { inner, semaphore }
    }
}

#[async_trait]
impl LlmProvider for SemaphoreGuardedProvider {
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Semaphore acquisition failed: {}", e),
            })?;
        self.inner.complete(prompt, system).await
    }

    async fn complete_with_cache(&self, request: LlmRequest) -> Result<LlmResponse, AiomeError> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Semaphore acquisition failed: {}", e),
            })?;
        self.inner.complete_with_cache(request).await
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        let permit = self.semaphore.clone().acquire_owned().await.map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("Semaphore acquisition failed: {}", e),
            }
        })?;

        let stream = self.inner.stream_complete(prompt, system).await?;

        // SAFETY: `GuardedStream` holds a pinned stream (`Pin<Box<...>>`).
        // Because `Box` satisfies `Unpin` regardless of its target type, we can safely
        // delegate `poll_next` by projecting mutable access to `self.stream` without violating Pin invariants.
        // The `_permit` field ensures the semaphore permit is held alive for the lifetime of this stream.
        struct GuardedStream {
            stream: Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>,
            _permit: OwnedSemaphorePermit,
        }

        impl Stream for GuardedStream {
            type Item = Result<String, AiomeError>;
            fn poll_next(
                mut self: Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Option<Self::Item>> {
                self.stream.as_mut().poll_next(cx)
            }
        }

        let guarded = GuardedStream {
            stream,
            _permit: permit,
        };

        Ok(Box::pin(guarded))
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Semaphore acquisition failed: {}", e),
            })?;
        self.inner.test_connection().await
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct MockProvider {
        call_count: Arc<AtomicUsize>,
        current_concurrent: Arc<AtomicUsize>,
        max_concurrent: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);

            let cur = self.current_concurrent.fetch_add(1, Ordering::SeqCst) + 1;
            loop {
                let prev_max = self.max_concurrent.load(Ordering::SeqCst);
                if cur <= prev_max {
                    break;
                }
                if self
                    .max_concurrent
                    .compare_exchange(prev_max, cur, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    break;
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            self.current_concurrent.fetch_sub(1, Ordering::SeqCst);
            Ok(LlmResponse::default())
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "MockProvider"
        }
    }

    #[tokio::test]
    async fn test_semaphore_limit() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let current_concurrent = Arc::new(AtomicUsize::new(0));
        let max_concurrent = Arc::new(AtomicUsize::new(0));

        let mock = Arc::new(MockProvider {
            call_count: call_count.clone(),
            current_concurrent: current_concurrent.clone(),
            max_concurrent: max_concurrent.clone(),
        });
        let semaphore = Arc::new(Semaphore::new(1));
        let guarded = Arc::new(SemaphoreGuardedProvider::new(mock, semaphore));

        // 同時実行をテスト
        let g1 = guarded.clone();
        let g2 = guarded.clone();

        let t1 = tokio::spawn(async move {
            g1.complete("test", None).await.unwrap();
        });
        let t2 = tokio::spawn(async move {
            g2.complete("test", None).await.unwrap();
        });

        // 実行完了を待つ
        let _ = tokio::join!(t1, t2);

        // 両方とも実行できたことを検証
        assert_eq!(call_count.load(Ordering::SeqCst), 2);

        // 最大同時実行数が1を超えていないことを検証（セマフォが機能していることの証明）
        assert_eq!(max_concurrent.load(Ordering::SeqCst), 1);
    }
}
