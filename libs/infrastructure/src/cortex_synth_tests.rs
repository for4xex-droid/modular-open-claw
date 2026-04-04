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
            metadata: None,
            reasoning: None,
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
        unimplemented!()
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        Ok(())
    }

    fn name(&self) -> &str {
        "mock"
    }
}

async fn setup_db_pool() -> Result<DatabasePool, Box<dyn std::error::Error>> {
    let pool = DatabasePool::new_sqlite("sqlite::memory:").await?;
    let sqlite_pool = pool.get_sqlite_pool_or_err()?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS cortex_wiki_articles (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            content_md TEXT NOT NULL,
            concepts TEXT DEFAULT '[]',
            backlinks TEXT DEFAULT '[]',
            source_refs TEXT DEFAULT '[]',
            content_hash TEXT NOT NULL,
            version INTEGER DEFAULT 1,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(sqlite_pool)
    .await?;

    Ok(pool)
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

    let synth = CortexSynthesizer::new(provider, pool, None);
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
