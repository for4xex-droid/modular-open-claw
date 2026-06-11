/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#![allow(dead_code)]
#![allow(clippy::default_constructed_unit_structs)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::type_complexity)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::manual_inspect)]

use std::sync::Arc;

use super::*;

pub async fn init_llm_providers(
    config: &Arc<shared::config::AiomeConfig>,
    db: &DatabaseResult,
    live_manager: Option<Arc<dyn aiome_core_contracts::traits::LiveSessionManager>>,
) -> anyhow::Result<ProviderResult> {
    // === 🏗️ STAGE 3/7: Engine ===
    let provider = Arc::new(infrastructure::llm::dynamic::DynamicLlmProvider {
        ops: db.job_queue.clone(),
        client: db.http_client.clone(),
        fallback_host: config.ollama_host.clone(),
        fallback_model: config.ollama_model.clone(),
        gemini_api_key: config.gemini_api_key.clone(),
        openai_api_key: config.openai_api_key.clone(),
        anthropic_api_key: config.anthropic_api_key.clone(),
        circuit_breaker: db.circuit_breaker.clone(),
        slo_engine: db.slo_engine.clone(),
        hook_manager: db.hook_manager.clone(),
        live_manager: live_manager.clone(),
        eval_logger: Some(db.eval_logger.clone()),
    });

    let bg_instance = Arc::new(infrastructure::llm::dynamic::BackgroundLlmProvider {
        ops: db.job_queue.clone(),
        client: db.http_client.clone(),
        fallback_model: config.ollama_model.clone(),
        fallback_host: config.ollama_host.clone(),
        gemini_api_key: config.gemini_api_key.clone(),
        openai_api_key: config.openai_api_key.clone(),
        anthropic_api_key: config.anthropic_api_key.clone(),
        hook_manager: db.hook_manager.clone(),
        live_manager: live_manager.clone(),
        eval_logger: Some(db.eval_logger.clone()),
    });

    let bg_provider: Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync> =
        bg_instance.clone();
    let embed_provider: Arc<dyn aiome_core::llm_provider::EmbeddingProvider> = bg_instance.clone();

    // Wire embedding provider back to job_queue (resolves circular dependency)
    db.job_queue
        .set_embedding_provider(embed_provider.clone())
        .await;

    let embed_type = std::env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "ruri".to_string());
    tracing::info!(
        "🧠 [LLM] Front-end: DynamicLlm (DB-configured), Background: {} ({}), Embedding: {}",
        std::env::var("BG_LLM_PROVIDER").unwrap_or_else(|_| "ollama".to_string()),
        std::env::var("BG_LLM_MODEL").unwrap_or_else(|_| "gemma4:26b".to_string()),
        embed_type,
    );

    if std::env::var("ENABLE_TOOL_REVIEWER").unwrap_or_else(|_| "true".to_string()) == "true" {
        let reviewer_hook = infrastructure::security::ToolCallReviewerHook::new(
            bg_provider.clone(),
            20, // max_reviews_per_session
            Some(db.eval_logger.clone()),
        );
        db.hook_manager.add_hook(Arc::new(reviewer_hook));
        tracing::info!("🛡️ [HookManager] ToolCallReviewerHook registered successfully");
    }

    // === Fast tier provider (Local LLM First) ===
    let fast_model = config.local_fast_model.clone();
    let fast_host = config.ollama_host.clone();

    let local_ollama = Arc::new(infrastructure::llm::dynamic::BackgroundLlmProvider {
        ops: db.job_queue.clone(),
        client: db.http_client.clone(),
        fallback_model: fast_model.clone(),
        fallback_host: fast_host.clone(),
        gemini_api_key: None, // ローカルオンリー
        openai_api_key: None,
        anthropic_api_key: None,
        hook_manager: db.hook_manager.clone(),
        live_manager: live_manager.clone(),
        eval_logger: Some(db.eval_logger.clone()),
    });

    // FallbackRouter で wrap: local → bg_provider(クラウド含む) フェイルオーバー
    let fast_provider_router = Arc::new(infrastructure::llm::fallback_router::FallbackRouter::new(
        local_ollama.clone() as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>,
        bg_provider.clone(),
        3, // failure threshold
    ))
        as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>;

    // local_fallback_policy == "local_only" の場合は FallbackRouter を使わず
    // local_ollama 単体を渡す（フォールバックなし）
    let fast_provider =
        if config.local_fallback_policy == shared::config::LocalFallbackPolicy::LocalOnly {
            local_ollama as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>
        } else {
            fast_provider_router
        };

    // fast_provider を SemaphoreGuardedProvider でラップして、ローカルLLMへの同時実行数を制限 (P1: 一貫性修正)
    let local_llm_semaphore = Arc::new(tokio::sync::Semaphore::new(config.local_llm_concurrency));
    let fast_provider = Arc::new(
        infrastructure::llm::semaphore_guard::SemaphoreGuardedProvider::new(
            fast_provider,
            local_llm_semaphore,
        ),
    ) as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>;

    Ok(ProviderResult {
        provider: provider as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>,
        bg_provider,
        embed_provider,
        fast_provider,
    })
}
