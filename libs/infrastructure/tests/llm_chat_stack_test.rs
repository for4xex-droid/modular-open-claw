/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![allow(clippy::unwrap_used)]

use aiome_core::llm_provider::{LlmMessage, LlmProvider, LlmRequest, MockLlmProvider};
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::CACHE_SCOPE_CHANNEL_KEY;
use async_trait::async_trait;
use infrastructure::job_queue::CostOps;
use infrastructure::llm::caching_provider::CachingLlmProvider;
use infrastructure::llm::entropy_gate::EntropyGate;
use infrastructure::llm::humanizer_filter::HumanizerFilter;
use infrastructure::llm::humanizer_rules::default_rules_ja;
use infrastructure::llm::intelligent_router::IntelligentRouter;
use infrastructure::llm::semantic_cache::{SemanticCache, SqlSemanticCacheRepository};
use infrastructure::llm::writing_context::WritingContext;
use shared::config::LlmRouteMode;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
struct ZeroCostOps;

#[async_trait]
impl CostOps for ZeroCostOps {
    async fn aggregate_cost_hours(&self, _hours: i64) -> Result<f64, AiomeError> {
        Ok(0.0)
    }
    async fn aggregate_cost_days(&self, _days: i64) -> Result<f64, AiomeError> {
        Ok(0.0)
    }
    async fn aggregate_cost_by_job(&self, _job_id: &str) -> Result<f64, AiomeError> {
        Ok(0.0)
    }
}

#[async_trait]
impl aiome_core_contracts::traits::SettingsOps for ZeroCostOps {
    async fn do_get_setting(&self, _key: &str) -> Result<Option<String>, AiomeError> {
        Ok(None)
    }
    async fn do_set_setting(
        &self,
        _k: &str,
        _v: &str,
        _c: &str,
        _s: bool,
    ) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn do_get_all_settings(
        &self,
    ) -> Result<Vec<aiome_core_contracts::contracts::SystemSetting>, AiomeError> {
        Ok(vec![])
    }
    async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
        Ok(false)
    }
    async fn set_auto_expression_enabled(&self, _e: bool) -> Result<(), AiomeError> {
        Ok(())
    }
}

fn mock_chain(label: &str) -> Arc<dyn LlmProvider + Send + Sync> {
    Arc::new(MockLlmProvider {
        response: label.to_string(),
        should_fail: false,
    })
}

/// 本番 DI（FIX-6 後）と同一の順序: HF → [Caching(rules のみ)] → EG → IR
async fn build_stack(mode: LlmRouteMode) -> Arc<dyn LlmProvider + Send + Sync> {
    let pool = infrastructure::db::DatabasePool::new_sqlite("sqlite::memory:")
        .await
        .unwrap();
    let ts = Arc::new(
        infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
    );
    let jq = Arc::new(
        infrastructure::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
            .await
            .unwrap(),
    );
    infrastructure::job_queue::migrations::DbInitializer::init_db(&*jq)
        .await
        .unwrap();

    let router = IntelligentRouter::new(
        mode,
        false,
        512,
        mock_chain("fast"),
        mock_chain("fast_degraded"),
        mock_chain("smart"),
        Arc::new(ZeroCostOps),
        10.0,
    );
    let gate: Arc<dyn LlmProvider + Send + Sync> =
        Arc::new(EntropyGate::new(Arc::new(router), 2.0, 3));

    let core: Arc<dyn LlmProvider + Send + Sync> = if mode == LlmRouteMode::Rules {
        let repo = Arc::new(SqlSemanticCacheRepository::new(pool.clone()));
        let cache = Arc::new(SemanticCache::new(repo, None));
        Arc::new(CachingLlmProvider::new(gate, cache, 3600))
    } else {
        gate
    };

    Arc::new(HumanizerFilter::new(
        core,
        default_rules_ja(),
        WritingContext::Default,
    ))
}

fn user_request(prompt: &str, format: Option<&str>, channel: &str) -> LlmRequest {
    let mut meta = HashMap::new();
    meta.insert(CACHE_SCOPE_CHANNEL_KEY.to_string(), channel.to_string());
    LlmRequest {
        messages: vec![LlmMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
            cache: false,
        }],
        temperature: None,
        max_tokens: None,
        stop_sequences: None,
        format: format.map(|s| s.to_string()),
        metadata: Some(meta),
    }
}

/// T1: rules + 短文 → Fast
#[tokio::test]
async fn t1_rules_short_prompt_routes_fast() {
    let stack = build_stack(LlmRouteMode::Rules).await;
    let resp = stack
        .complete_with_cache(user_request("hello", None, "ch-1"))
        .await
        .unwrap();
    assert_eq!(resp.content, "fast");
}

/// T2: rules + format=json → Smart（かつキャッシュバイパス）
#[tokio::test]
async fn t2_rules_json_format_routes_smart() {
    let stack = build_stack(LlmRouteMode::Rules).await;
    let resp = stack
        .complete_with_cache(user_request("hello", Some("json"), "ch-1"))
        .await
        .unwrap();
    assert_eq!(resp.content, "smart");
}

/// T2b Negative: json は 2 回目も cache_hit が付かない
#[tokio::test]
async fn t2b_json_does_not_write_cache() {
    let stack = build_stack(LlmRouteMode::Rules).await;
    let _ = stack
        .complete_with_cache(user_request("hello json", Some("JSON"), "ch-1"))
        .await
        .unwrap();
    let second = stack
        .complete_with_cache(user_request("hello json", Some("json"), "ch-1"))
        .await
        .unwrap();
    assert!(
        second
            .metadata
            .as_ref()
            .and_then(|m| m.get("cache_hit"))
            .is_none(),
        "json format must bypass cache write/read"
    );
}

/// T3: legacy → 常に Smart（キャッシュ層なし）
#[tokio::test]
async fn t3_legacy_always_smart() {
    let stack = build_stack(LlmRouteMode::Legacy).await;
    let resp = stack
        .complete_with_cache(user_request("hello", None, "ch-1"))
        .await
        .unwrap();
    assert_eq!(resp.content, "smart");
}

/// T3b Negative: legacy 2 連打でも cache_hit が付かない
#[tokio::test]
async fn t3b_legacy_never_cache_hit() {
    let stack = build_stack(LlmRouteMode::Legacy).await;
    let _ = stack
        .complete_with_cache(user_request("hello cache", None, "ch-1"))
        .await
        .unwrap();
    let second = stack
        .complete_with_cache(user_request("hello cache", None, "ch-1"))
        .await
        .unwrap();
    assert!(
        second
            .metadata
            .as_ref()
            .and_then(|m| m.get("cache_hit"))
            .is_none(),
        "legacy must not install CachingLlmProvider"
    );
}

/// T4: rules + 同一リクエスト 2 回目はキャッシュヒット（cache_hit metadata）
#[tokio::test]
async fn t4_rules_second_call_hits_cache() {
    let stack = build_stack(LlmRouteMode::Rules).await;
    let first = stack
        .complete_with_cache(user_request("hello cache", None, "ch-1"))
        .await
        .unwrap();
    assert!(first
        .metadata
        .as_ref()
        .and_then(|m| m.get("cache_hit"))
        .is_none());
    let second = stack
        .complete_with_cache(user_request("hello cache", None, "ch-1"))
        .await
        .unwrap();
    assert_eq!(
        second
            .metadata
            .as_ref()
            .and_then(|m| m.get("cache_hit"))
            .map(String::as_str),
        Some("true")
    );
}

/// T5 Negative: チャネル横断で cache HIT しない
#[tokio::test]
async fn t5_channel_scope_prevents_cross_channel_hit() {
    let stack = build_stack(LlmRouteMode::Rules).await;
    let _ = stack
        .complete_with_cache(user_request("shared prompt", None, "ch-a"))
        .await
        .unwrap();
    let other = stack
        .complete_with_cache(user_request("shared prompt", None, "ch-b"))
        .await
        .unwrap();
    assert!(
        other
            .metadata
            .as_ref()
            .and_then(|m| m.get("cache_hit"))
            .is_none(),
        "cross-channel cache hit must not occur"
    );
}
