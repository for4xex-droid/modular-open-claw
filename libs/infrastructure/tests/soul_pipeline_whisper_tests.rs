/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use aiome_core::llm_provider::{LlmProvider, LlmResponse};
use async_trait::async_trait;
use infrastructure::llm::whisper_middleware::WhisperMiddleware;
use infrastructure::samsara_engine::DefaultSamsaraEngine;
use infrastructure::soul_adapter::CoreDomainAdapter;
use soul::model::{AgentSoul, Experience};
use soul::pipeline::SoulPipeline;
use std::sync::Arc;

#[derive(Debug)]
struct MockLlm;

#[async_trait]
impl LlmProvider for MockLlm {
    fn name(&self) -> &str {
        "MockLlm"
    }
    async fn test_connection(&self) -> Result<(), AiomeError> {
        Ok(())
    }
    async fn complete(
        &self,
        _prompt: &str,
        _system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        Ok(LlmResponse {
            content: "[]".to_string(),
            stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
            reasoning: None,
            metadata: None,
        })
    }
    async fn complete_with_cache(
        &self,
        _request: aiome_core_contracts::llm::LlmRequest,
    ) -> Result<LlmResponse, AiomeError> {
        self.complete("", None).await
    }
}

#[tokio::test]
async fn test_soul_pipeline_with_whisper_integration() {
    let mock_llm = Arc::new(MockLlm);
    let ts = std::sync::Arc::new(
        infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(
            infrastructure::db::DatabasePool::Sqlite(
                sqlx::sqlite::SqlitePoolOptions::new()
                    .connect("sqlite::memory:")
                    .await
                    .unwrap(),
            ),
        ),
    );
    let adapter = CoreDomainAdapter::new(
        Arc::new(
            infrastructure::job_queue::UniversalJobQueue::new(infrastructure::db::DatabasePool::new_sqlite(":memory:").await.unwrap(), None, ts)
                .await
                .unwrap(),
        ),
        None,
    );
    let engine = DefaultSamsaraEngine::new(mock_llm, "test".to_string());

    let mut pipeline = SoulPipeline::new(adapter, engine);
    pipeline.add_middleware(Box::new(WhisperMiddleware::new()));

    let mut soul = AgentSoul::new("test-agent".into());
    let exp = Experience {
        content: "User asked for a subscription.".to_string(),
        outcome_valence: 0.5, // Positive valence should trigger a specific whisper
        ..Default::default()
    };

    let _ = pipeline.process_experience(&mut soul, exp).await.unwrap();

    // Check if the experience in the buffer has the whisper thought
    let processed_exp = soul
        .experience_buffer
        .last()
        .expect("Experience should be in buffer");
    assert!(
        processed_exp.content.contains("Whisper:"),
        "Processed experience should contain whisper thought"
    );
    assert!(
        processed_exp
            .content
            .contains("I'm starting to understand this user better"),
        "Should contain positive whisper"
    );
}
