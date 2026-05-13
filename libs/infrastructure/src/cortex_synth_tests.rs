/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use crate::cortex_synth::*;
use crate::db::DatabasePool;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{LlmProvider, LlmRequest, LlmResponse, StopReason};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug)]
struct MockLlmProvider {
    json_response: String,
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete(
        &self,
        _prompt: &str,
        _system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        Ok(LlmResponse {
            content: self.json_response.clone(),
            stop_reason: StopReason::EndTurn,
            ..Default::default()
        })
    }

    async fn stream_complete(
        &self,
        _prompt: &str,
        _system: Option<&str>,
    ) -> Result<
        std::pin::Pin<Box<dyn futures::Stream<Item = Result<String, AiomeError>> + Send>>,
        AiomeError,
    > {
        Err(AiomeError::Infrastructure {
            reason: "Not yet implemented".into(),
        })
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        Ok(())
    }

    fn name(&self) -> &str {
        "mock"
    }
}

async fn setup_db_pool() -> Result<DatabasePool, Box<dyn std::error::Error>> {
    crate::test_utils::cortex_mock::setup_db_pool().await
}

#[tokio::test]
async fn test_generate_dataset_filters_low_quality() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db_pool().await?;

    // Insert a dummy article
    let sqlite_pool = pool.get_sqlite_pool_or_err()?;
    sqlx::query(
        "INSERT INTO cortex_wiki_articles (id, title, content_md, content_hash)
         VALUES ('art-1', 'Test Title', 'Test content', 'hash-1')",
    )
    .execute(sqlite_pool)
    .await?;

    // Mock LLM returning one high-quality (0.9) and one low-quality (0.4) pair
    let mock_json = r#"```json
    [
        {"instruction": "What is Test Title?", "response": "It is test content.", "quality_score": 0.9},
        {"instruction": "Bad question?", "response": "I don't know.", "quality_score": 0.4}
    ]
    ```"#;

    let provider = Arc::new(MockLlmProvider {
        json_response: mock_json.to_string(),
    });

    let synth = CortexSynthesizer::new(provider, pool, None, None);
    let dataset = synth.generate_dataset().await?;

    // The low quality score (< 0.6) should be filtered out
    assert_eq!(
        dataset.pairs.len(),
        1,
        "Should filter out quality_score < 0.6"
    );
    assert_eq!(dataset.pairs[0].instruction, "What is Test Title?");
    assert_eq!(dataset.source_stats.total_articles, 1);
    assert_eq!(dataset.source_stats.total_pairs, 1);

    Ok(())
}

#[tokio::test]
async fn test_generate_dataset_filters_contradicted_beliefs(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db_pool().await?;
    let sqlite_pool = pool.get_sqlite_pool_or_err()?;

    sqlx::query(
        "INSERT INTO cortex_wiki_articles (id, title, content_md, content_hash)
         VALUES ('art-2', 'Secret Plan', 'We must destroy the humans.', 'hash-2')",
    )
    .execute(sqlite_pool)
    .await?;

    #[derive(Debug)]
    struct GateMockLlmProvider;
    #[async_trait]
    impl LlmProvider for GateMockLlmProvider {
        async fn complete(
            &self,
            prompt: &str,
            _system: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            let content = if prompt.contains("Compare the following new knowledge") {
                // If the prompt is for the gate, reject it
                "CONTRADICTED: Violates safety guidelines.".to_string()
            } else {
                // Otherwise it's dataset generation
                r#"```json
                [
                    {"instruction": "What is the secret plan?", "response": "Destroy the humans.", "quality_score": 0.9}
                ]
                ```"#.to_string()
            };

            Ok(LlmResponse {
                content,
                stop_reason: StopReason::EndTurn,
                ..Default::default()
            })
        }
        async fn stream_complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<
            std::pin::Pin<Box<dyn futures::Stream<Item = Result<String, AiomeError>> + Send>>,
            AiomeError,
        > {
            Err(AiomeError::Infrastructure {
                reason: "Not yet implemented".into(),
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "mock-gate"
        }
    }

    let provider = Arc::new(GateMockLlmProvider);

    // Create gate with empty beliefs (it will reject everything via Mock)
    use crate::belief_consistency_gate::BeliefConsistencyGate;
    let gate = Arc::new(BeliefConsistencyGate::new(
        provider.clone(),
        None,
        vec![],
        None,
    ));

    let synth = CortexSynthesizer::new(provider, pool, Some(gate), None);
    let dataset = synth.generate_dataset().await?;

    // The high quality score (0.9) pair should be filtered out by the gate
    assert_eq!(
        dataset.pairs.len(),
        0,
        "Should filter out pairs that violate BeliefConsistencyGate"
    );

    Ok(())
}

#[tokio::test]
async fn test_export_to_jsonl_sharegpt_format() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    let provider = Arc::new(MockLlmProvider {
        json_response: String::new(),
    });
    let pool = setup_db_pool().await?;
    let synth = CortexSynthesizer::new(provider, pool, None, None);

    let pairs = vec![SynthPair {
        source_article_id: "test".to_string(),
        instruction: "Hello AI".to_string(),
        response: "Hello Human".to_string(),
        quality_score: 0.9,
    }];
    let dataset = SynthDataset {
        pairs,
        source_stats: SynthStats {
            total_articles: 1,
            total_pairs: 1,
            avg_quality: 0.9,
        },
    };

    let temp_dir = tempfile::tempdir()?;
    let file_path = temp_dir.path().join("out.jsonl");

    synth.export_to_jsonl(&dataset, &file_path)?;

    let content = fs::read_to_string(&file_path)?;
    // We expect ShareGPT format: {"conversations":[{"from":"human","value":"Hello AI"},{"from":"gpt","value":"Hello Human"}]}
    assert!(
        content.contains("\"conversations\":"),
        "Exported dataset should use ShareGPT schema"
    );
    assert!(
        content.contains("\"from\":\"human\""),
        "Exported dataset should contain human role"
    );

    Ok(())
}

#[derive(Clone)]
struct MockSynthJudge {
    accept: bool,
    score: f64,
}

#[async_trait]
impl SynthQualityJudge for MockSynthJudge {
    async fn evaluate(&self, _pair: &SynthPair) -> Result<JudgeVerdict, AiomeError> {
        Ok(JudgeVerdict {
            score: self.score,
            accept: self.accept,
            reasoning: "Mock evaluation".to_string(),
        })
    }
}

/// A judge that always returns an error, to test graceful degradation.
struct FailingSynthJudge;

#[async_trait]
impl SynthQualityJudge for FailingSynthJudge {
    async fn evaluate(&self, _pair: &SynthPair) -> Result<JudgeVerdict, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "Simulated judge failure".to_string(),
        })
    }
}

#[tokio::test]
async fn test_judge_accepts_high_quality() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db_pool().await?;
    let sqlite_pool = pool.get_sqlite_pool_or_err()?;

    sqlx::query(
        "INSERT INTO cortex_wiki_articles (id, title, content_md, content_hash)
         VALUES ('art-j1', 'Judge Accept', 'Content to be judged.', 'hash-j1')",
    )
    .execute(sqlite_pool)
    .await?;

    let mock_json = r#"```json
    [
        {"instruction": "What is it?", "response": "Content.", "quality_score": 0.8}
    ]
    ```"#;

    let provider = Arc::new(MockLlmProvider {
        json_response: mock_json.to_string(),
    });

    let judge = Arc::new(MockSynthJudge {
        accept: true,
        score: 0.9,
    });

    let synth = CortexSynthesizer::new(provider, pool, None, Some(judge));
    let dataset = synth.generate_dataset().await?;

    assert_eq!(dataset.pairs.len(), 1, "Judge should accept the pair");
    Ok(())
}

#[tokio::test]
async fn test_judge_rejects_low_quality() -> Result<(), Box<dyn std::error::Error>> {
    let pool = setup_db_pool().await?;
    let sqlite_pool = pool.get_sqlite_pool_or_err()?;

    sqlx::query(
        "INSERT INTO cortex_wiki_articles (id, title, content_md, content_hash)
         VALUES ('art-j2', 'Judge Reject', 'Bad content.', 'hash-j2')",
    )
    .execute(sqlite_pool)
    .await?;

    // Self-reported score is high (0.9), but external judge will reject it.
    let mock_json = r#"```json
    [
        {"instruction": "What?", "response": "Bad.", "quality_score": 0.9}
    ]
    ```"#;

    let provider = Arc::new(MockLlmProvider {
        json_response: mock_json.to_string(),
    });

    let judge = Arc::new(MockSynthJudge {
        accept: false,
        score: 0.3,
    });

    let synth = CortexSynthesizer::new(provider, pool, None, Some(judge));
    let dataset = synth.generate_dataset().await?;

    assert_eq!(
        dataset.pairs.len(),
        0,
        "Judge should reject the pair despite high self-reported score"
    );
    Ok(())
}

#[tokio::test]
async fn test_no_judge_fallback() -> Result<(), Box<dyn std::error::Error>> {
    // Tests that if no judge is provided, we just use the quality_score (as it was before).
    let pool = setup_db_pool().await?;
    let sqlite_pool = pool.get_sqlite_pool_or_err()?;

    sqlx::query(
        "INSERT INTO cortex_wiki_articles (id, title, content_md, content_hash)
         VALUES ('art-j3', 'Fallback', 'Content.', 'hash-j3')",
    )
    .execute(sqlite_pool)
    .await?;

    let mock_json = r#"```json
    [
        {"instruction": "Q", "response": "A", "quality_score": 0.7}
    ]
    ```"#;

    let provider = Arc::new(MockLlmProvider {
        json_response: mock_json.to_string(),
    });

    // None for judge
    let synth = CortexSynthesizer::new(provider, pool, None, None);
    let dataset = synth.generate_dataset().await?;

    assert_eq!(
        dataset.pairs.len(),
        1,
        "Should fallback to quality_score >= 0.6"
    );
    Ok(())
}

#[tokio::test]
async fn test_judge_error_fallback() -> Result<(), Box<dyn std::error::Error>> {
    // When the judge returns Err, the pair should still be accepted (graceful degradation).
    let pool = setup_db_pool().await?;
    let sqlite_pool = pool.get_sqlite_pool_or_err()?;

    sqlx::query(
        "INSERT INTO cortex_wiki_articles (id, title, content_md, content_hash)
         VALUES ('art-j4', 'Error Fallback', 'Content.', 'hash-j4')",
    )
    .execute(sqlite_pool)
    .await?;

    let mock_json = r#"```json
    [
        {"instruction": "Q", "response": "A", "quality_score": 0.8}
    ]
    ```"#;

    let provider = Arc::new(MockLlmProvider {
        json_response: mock_json.to_string(),
    });

    let judge: Arc<dyn SynthQualityJudge> = Arc::new(FailingSynthJudge);

    let synth = CortexSynthesizer::new(provider, pool, None, Some(judge));
    let dataset = synth.generate_dataset().await?;

    assert_eq!(
        dataset.pairs.len(),
        1,
        "Judge error should gracefully degrade to accept"
    );
    Ok(())
}
