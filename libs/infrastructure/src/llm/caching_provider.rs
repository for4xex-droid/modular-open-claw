/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::semantic_cache::SemanticCache;
use super::utils::{cache_scope_channel, compute_request_cache_key};
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::{LlmMessage, LlmProvider, LlmRequest, LlmResponse};
use aiome_core_contracts::llm::{
    CACHE_SCOPE_CHANNEL_KEY, ROUTE_MODE_KEY, ROUTE_REASON_KEY, ROUTE_TIER_KEY,
    ROUTE_TIER_LOCKED_KEY,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;

const CACHE_HIT_METADATA_KEY: &str = "cache_hit";

/// SemanticCache の薄い LlmProvider ラッパー（ADR-058 Phase 4）。
/// EntropyGate 外側・HumanizerFilter 内側に配置する（OP-099 FIX-6）。
pub struct CachingLlmProvider {
    inner: Arc<dyn LlmProvider + Send + Sync>,
    cache: Arc<SemanticCache>,
    ttl_seconds: i64,
}

impl fmt::Debug for CachingLlmProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachingLlmProvider")
            .field("inner", &self.inner.name())
            .field("ttl_seconds", &self.ttl_seconds)
            .finish()
    }
}

impl CachingLlmProvider {
    pub fn new(
        inner: Arc<dyn LlmProvider + Send + Sync>,
        cache: Arc<SemanticCache>,
        ttl_seconds: i64,
    ) -> Self {
        Self {
            inner,
            cache,
            ttl_seconds,
        }
    }

    fn mark_cache_hit(mut response: LlmResponse, request: &LlmRequest) -> LlmResponse {
        let meta = response.metadata.get_or_insert_with(HashMap::new);
        meta.insert(CACHE_HIT_METADATA_KEY.to_string(), "true".to_string());
        // Restore route observability lost when SemanticCache stores content-only.
        if let Some(req_meta) = request.metadata.as_ref() {
            for key in [
                ROUTE_TIER_KEY,
                ROUTE_REASON_KEY,
                ROUTE_MODE_KEY,
                ROUTE_TIER_LOCKED_KEY,
            ] {
                if let Some(v) = req_meta.get(key) {
                    meta.entry(key.to_string()).or_insert_with(|| v.clone());
                }
            }
        }
        response
    }

    fn should_bypass_cache(request: &LlmRequest) -> bool {
        if cache_scope_channel(request).is_none() {
            return true;
        }
        request
            .format
            .as_deref()
            .map(str::trim)
            .is_some_and(|s| s.eq_ignore_ascii_case("json"))
    }
}

#[async_trait]
impl LlmProvider for CachingLlmProvider {
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        let mut messages = Vec::new();
        if let Some(sys) = system {
            messages.push(LlmMessage {
                role: "system".to_string(),
                content: sys.to_string(),
                cache: false,
            });
        }
        messages.push(LlmMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
            cache: false,
        });
        // complete() には channel スコープが無いためキャッシュは bypass（Fail-Closed）。
        self.complete_with_cache(LlmRequest {
            messages,
            ..Default::default()
        })
        .await
    }

    async fn complete_with_cache(&self, request: LlmRequest) -> Result<LlmResponse, AiomeError> {
        if Self::should_bypass_cache(&request) {
            return self.inner.complete_with_cache(request).await;
        }

        let key = compute_request_cache_key(&request);

        if let Some(cached) = self.cache.get_by_key(&key).await? {
            return Ok(Self::mark_cache_hit(cached, &request));
        }

        let response = self.inner.complete_with_cache(request.clone()).await?;
        let provider_name = self.inner.name();
        if let Err(e) = self
            .cache
            .set_by_key(
                &key,
                &response,
                provider_name,
                provider_name,
                self.ttl_seconds,
            )
            .await
        {
            tracing::warn!("SemanticCache write failed (non-fatal): {}", e);
        }
        Ok(response)
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        // Streaming はキャッシュ対象外（計画 Phase 4）
        self.inner.stream_complete(prompt, system).await
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.inner.test_connection().await
    }

    fn name(&self) -> &str {
        "CachingLlmProvider"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::llm_provider::MockLlmProvider;

    fn scoped_request(prompt: &str, channel: &str) -> LlmRequest {
        let mut meta = HashMap::new();
        meta.insert(CACHE_SCOPE_CHANNEL_KEY.to_string(), channel.to_string());
        LlmRequest {
            messages: vec![LlmMessage {
                role: "user".into(),
                content: prompt.into(),
                cache: false,
            }],
            metadata: Some(meta),
            ..Default::default()
        }
    }

    async fn build_provider(response: &str) -> CachingLlmProvider {
        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = Arc::new(
            crate::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(),
        );
        crate::job_queue::migrations::DbInitializer::init_db(&*jq)
            .await
            .unwrap();

        let repo =
            Arc::new(super::super::semantic_cache::SqlSemanticCacheRepository::new(pool.clone()));
        let cache = Arc::new(SemanticCache::new(repo, None));
        let inner = Arc::new(MockLlmProvider {
            response: response.into(),
            should_fail: false,
        });
        CachingLlmProvider::new(inner, cache, 3600)
    }

    #[tokio::test]
    async fn test_caching_provider_hit() {
        let provider = build_provider("cached-answer").await;

        let first = provider
            .complete_with_cache(scoped_request("hello", "ch-1"))
            .await
            .unwrap();
        assert_eq!(first.content, "cached-answer");
        assert!(first
            .metadata
            .as_ref()
            .and_then(|m| m.get(CACHE_HIT_METADATA_KEY))
            .is_none());

        let second = provider
            .complete_with_cache(scoped_request("hello", "ch-1"))
            .await
            .unwrap();
        assert_eq!(second.content, "cached-answer");
        assert_eq!(
            second
                .metadata
                .as_ref()
                .and_then(|m| m.get(CACHE_HIT_METADATA_KEY))
                .map(String::as_str),
            Some("true")
        );
    }

    #[tokio::test]
    async fn test_caching_provider_bypasses_without_channel_scope() {
        let provider = build_provider("answer").await;
        let first = provider.complete("hello", None).await.unwrap();
        assert!(first
            .metadata
            .as_ref()
            .and_then(|m| m.get(CACHE_HIT_METADATA_KEY))
            .is_none());
        let second = provider.complete("hello", None).await.unwrap();
        assert!(
            second
                .metadata
                .as_ref()
                .and_then(|m| m.get(CACHE_HIT_METADATA_KEY))
                .is_none(),
            "complete() without channel_id must not cache"
        );
    }

    #[tokio::test]
    async fn test_caching_provider_channel_isolation() {
        let provider = build_provider("answer").await;
        let _ = provider
            .complete_with_cache(scoped_request("hello", "ch-a"))
            .await
            .unwrap();
        let other = provider
            .complete_with_cache(scoped_request("hello", "ch-b"))
            .await
            .unwrap();
        assert!(
            other
                .metadata
                .as_ref()
                .and_then(|m| m.get(CACHE_HIT_METADATA_KEY))
                .is_none(),
            "different channel_id must not hit cache"
        );
    }

    #[tokio::test]
    async fn test_caching_provider_history_does_not_collide() {
        let provider = build_provider("answer").await;

        let first_req = scoped_request("hello", "ch-1");
        let _ = provider.complete_with_cache(first_req).await.unwrap();

        let mut meta = HashMap::new();
        meta.insert(CACHE_SCOPE_CHANNEL_KEY.to_string(), "ch-1".to_string());
        let second_req = LlmRequest {
            messages: vec![
                LlmMessage {
                    role: "user".into(),
                    content: "prev".into(),
                    cache: false,
                },
                LlmMessage {
                    role: "assistant".into(),
                    content: "ok".into(),
                    cache: false,
                },
                LlmMessage {
                    role: "user".into(),
                    content: "hello".into(),
                    cache: false,
                },
            ],
            metadata: Some(meta),
            ..Default::default()
        };
        let second = provider.complete_with_cache(second_req).await.unwrap();
        assert!(
            second
                .metadata
                .as_ref()
                .and_then(|m| m.get(CACHE_HIT_METADATA_KEY))
                .is_none(),
            "different history must not hit cache keyed only by final user message"
        );
    }
}
