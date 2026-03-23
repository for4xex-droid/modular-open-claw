/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::error::AiomeError;
use infrastructure::job_queue::UniversalJobQueue;
use std::sync::Arc;
use tokio::sync::OnceCell;

static DB: OnceCell<Arc<UniversalJobQueue>> = OnceCell::const_new();
static LLM: OnceCell<Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>> =
    OnceCell::const_new();
static IMMUNE: OnceCell<Arc<infrastructure::immune_system::AdaptiveImmuneSystem>> =
    OnceCell::const_new();

/// `get_db` 関数
pub async fn get_db() -> Result<&'static Arc<UniversalJobQueue>, AiomeError> {
    DB.get_or_try_init(|| async {
        let db_path = std::env::var("AIOME_DB_PATH")
            .unwrap_or_else(|_| "sqlite://workspace/aiome.db".to_string());
        let queue = UniversalJobQueue::new(&db_path).await?;
        Ok(Arc::new(queue))
    })
    .await
}

/// `get_llm_provider` 関数
pub async fn get_llm_provider(
) -> Result<&'static Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>, AiomeError> {
    LLM.get_or_try_init(|| async {
        let db = get_db().await?;
        let client = aiome_core::http::get_http_client().clone();

        let provider_type = db
            .get_setting_value("llm_provider")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "ollama".to_string());

        let model_setting = db.get_setting_value("llm_model").await.ok().flatten();
        let model = if let Some(m) = model_setting {
            m
        } else if let Ok(Some(m)) = db.get_setting_value("ollama_model").await {
            m
        } else {
            std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3.5:9b".to_string())
        };

        let provider: Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync> =
            match provider_type.as_str() {
                "gemini" => {
                    let api_key = if let Ok(key) = std::env::var("GEMINI_API_KEY") {
                        std::env::remove_var("GEMINI_API_KEY");
                        key
                    } else {
                        db.get_setting_value("llm_api_key")
                            .await
                            .ok()
                            .flatten()
                            .unwrap_or_default()
                    };
                    Arc::new(aiome_core::llm_provider::GeminiProvider::new(
                        client, api_key, model,
                    ))
                }
                "openai" => {
                    let api_key = if let Ok(key) = std::env::var("OPENAI_API_KEY") {
                        std::env::remove_var("OPENAI_API_KEY");
                        key
                    } else {
                        db.get_setting_value("llm_api_key")
                            .await
                            .ok()
                            .flatten()
                            .unwrap_or_default()
                    };
                    Arc::new(aiome_core::llm_provider::OpenAiProvider::new(
                        client, api_key, model,
                    ))
                }
                "claude" => {
                    let api_key = if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
                        std::env::remove_var("ANTHROPIC_API_KEY");
                        key
                    } else {
                        db.get_setting_value("llm_api_key")
                            .await
                            .ok()
                            .flatten()
                            .unwrap_or_default()
                    };
                    Arc::new(aiome_core::llm_provider::ClaudeProvider::new(
                        client, api_key, model,
                    ))
                }
                "lmstudio" => {
                    let host = db
                        .get_setting_value("lm_studio_host")
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| shared::config::DEFAULT_LM_STUDIO_HOST.to_string());
                    Arc::new(aiome_core::llm_provider::LmStudioProvider::new(
                        client, host, model,
                    ))
                }
                _ => {
                    let host = db
                        .get_setting_value("ollama_host")
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| {
                            std::env::var("OLLAMA_HOST")
                                .unwrap_or_else(|_| shared::config::DEFAULT_OLLAMA_HOST.to_string())
                        });
                    Arc::new(aiome_core::llm_provider::OllamaProvider::new(host, model))
                }
            };
        Ok(provider)
    })
    .await
}

/// `get_immune` 関数
pub async fn get_immune(
) -> Result<&'static Arc<infrastructure::immune_system::AdaptiveImmuneSystem>, AiomeError> {
    IMMUNE
        .get_or_try_init(|| async {
            let provider = get_llm_provider().await?;
            let immune_system =
                infrastructure::immune_system::AdaptiveImmuneSystem::new(provider.clone());
            Ok(Arc::new(immune_system))
        })
        .await
}
