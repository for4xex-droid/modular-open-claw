/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use std::sync::Arc;

use super::*;

/// Build a local-only Background LLM instance (shared by fast_provider and chat Fast tier).
pub fn build_local_background_provider(
    config: &Arc<shared::config::AiomeConfig>,
    db: &DatabaseResult,
    live_manager: Option<Arc<dyn aiome_core_contracts::traits::LiveSessionManager>>,
    enforce_cost_limit: bool,
) -> Arc<infrastructure::llm::dynamic::BackgroundLlmProvider> {
    Arc::new(infrastructure::llm::dynamic::BackgroundLlmProvider {
        ops: db.job_queue.clone(),
        client: db.http_client.clone(),
        fallback_model: config.local_fast_model.clone(),
        fallback_host: config.ollama_host.clone(),
        gemini_api_key: None,
        openai_api_key: None,
        anthropic_api_key: None,
        hook_manager: db.hook_manager.clone(),
        live_manager,
        eval_logger: Some(db.eval_logger.clone()),
        enforce_cost_limit,
        pin_local: true,
    })
}

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
        enforce_cost_limit: true,
        pin_local: false,
    });

    let bg_provider: Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync> =
        bg_instance.clone();
    let embed_provider: Arc<dyn aiome_core::llm_provider::EmbeddingProvider> = bg_instance.clone();

    db.job_queue
        .set_embedding_provider(embed_provider.clone())
        .await;

    let embed_type = std::env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "ruri".to_string());
    tracing::info!(
        "🧠 [LLM] Front-end: DynamicLlm (DB-configured), Background: {} ({}), Embedding: {}, RouteMode: {}",
        std::env::var("BG_LLM_PROVIDER").unwrap_or_else(|_| "ollama".to_string()),
        std::env::var("BG_LLM_MODEL").unwrap_or_else(|_| "gemma4:26b".to_string()),
        embed_type,
        config.llm_route_mode,
    );

    if std::env::var("ENABLE_TOOL_REVIEWER").unwrap_or_else(|_| "true".to_string()) == "true" {
        let reviewer_hook = infrastructure::security::ToolCallReviewerHook::new(
            bg_provider.clone(),
            20,
            Some(db.eval_logger.clone()),
        );
        db.hook_manager.add_hook(Arc::new(reviewer_hook));
        tracing::info!("🛡️ [HookManager] ToolCallReviewerHook registered successfully");
    }

    let local_ollama = build_local_background_provider(config, db, live_manager.clone(), true);
    let local_degraded = build_local_background_provider(config, db, live_manager.clone(), false);

    let fast_provider_router = Arc::new(infrastructure::llm::fallback_router::FallbackRouter::new(
        local_ollama.clone() as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>,
        bg_provider.clone(),
        3,
    ))
        as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>;

    let fast_provider =
        if config.local_fallback_policy == shared::config::LocalFallbackPolicy::LocalOnly {
            local_ollama.clone() as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>
        } else {
            fast_provider_router
        };

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
        local_provider: local_ollama
            as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>,
        local_provider_degraded: local_degraded
            as Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>,
    })
}
