/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use crate::cortex_compiler::*;
use crate::cortex_ingester::{CortexDocument, SourceType};
use crate::db::DatabasePool;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{LlmProvider, LlmRequest, LlmResponse, StopReason};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Semaphore;
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

async fn setup_db_pool() -> DatabasePool {
    crate::test_utils::cortex_mock::setup_db_pool()
        .await
        .unwrap()
}

#[tokio::test]
async fn test_extract_concepts() {
    let pool = setup_db_pool().await;
    let mock_json = r#"```json
        [
        {"name": "Async Rust", "description": "Programming paradigm in Rust", "source_ids": ["doc1"]},
        {"name": "Tokio", "description": "Async runtime", "source_ids": ["doc1"]}
    ]
    ```"#;
    let provider = Arc::new(MockLlmProvider {
        json_response: mock_json.to_string(),
    });
    let semaphore = Arc::new(Semaphore::new(1));

    let compiler = CortexCompiler::new(provider, pool, None, semaphore);

    let doc = CortexDocument {
        id: "doc1".to_string(),
        title: "Test".to_string(),
        source_url: None,
        content_md: "Tokio is an async runtime for Rust.".to_string(),
        content_hash: "hash".to_string(),
        source_type: SourceType::Manual,
        ingested_at: "".to_string(),
        tags: vec![],
        summary: None,
        wiki_article_refs: vec![],
    };

    let concepts = compiler.extract_concepts(&doc).await.unwrap();
    assert_eq!(concepts.len(), 2);
    assert_eq!(concepts[0].name, "Async Rust");
    assert_eq!(concepts[1].name, "Tokio");
}

#[tokio::test]
async fn test_run_compilation_cycle() {
    let pool = setup_db_pool().await;
    let mock_json = r#"```json
        [
        {"name": "Common Topic", "description": "Both docs mention this", "source_ids": ["doc1", "doc2"]},
        {"name": "Unique Topic", "description": "Only doc 1", "source_ids": ["doc1"]}
    ]
    ```"#;
    let provider = Arc::new(MockLlmProvider {
        json_response: mock_json.to_string(),
    });
    let semaphore = Arc::new(Semaphore::new(1));

    // Add two documents to DB
    let sqlite_pool = pool.get_sqlite_pool_or_err().unwrap();
    sqlx::query("INSERT INTO cortex_documents (id, title, content_md, content_hash, source_type) VALUES ('doc1', 'T1', 'C1', 'H1', 'manual')").execute(sqlite_pool).await.unwrap();
    sqlx::query("INSERT INTO cortex_documents (id, title, content_md, content_hash, source_type) VALUES ('doc2', 'T2', 'C2', 'H2', 'manual')").execute(sqlite_pool).await.unwrap();

    let compiler = CortexCompiler::new(provider, pool, None, semaphore);

    // Running cycle should create a new article for "Common Topic"
    let report = compiler.run_compilation_cycle().await.unwrap();
    assert!(report.new_articles > 0, "Should create new article");
}

#[tokio::test]
async fn test_update_backlinks_and_typed_links() {
    let pool = setup_db_pool().await;
    let provider = Arc::new(MockLlmProvider {
        json_response: "[]".to_string(),
    });
    let semaphore = Arc::new(Semaphore::new(1));
    let compiler = CortexCompiler::new(provider, pool.clone(), None, semaphore);

    let sqlite_pool = pool.get_sqlite_pool_or_err().unwrap();
    // A mentions B
    sqlx::query("INSERT INTO cortex_wiki_articles (id, title, content_md, content_hash) VALUES ('art1', 'Topic A', 'This talks about Topic B and stuff.', 'hash1')").execute(sqlite_pool).await.unwrap();
    sqlx::query("INSERT INTO cortex_wiki_articles (id, title, content_md, content_hash) VALUES ('art2', 'Topic B', 'This is B.', 'hash2')").execute(sqlite_pool).await.unwrap();
    // C: "extends Topic B" and "contradict Topic A" are spaced >200 chars apart
    // so the ±100 char context window around each match does NOT overlap,
    // ensuring each typed link gets the correct keyword classification.
    sqlx::query("INSERT INTO cortex_wiki_articles (id, title, content_md, content_hash) VALUES ('art3', 'Topic C', 'This module extends Topic B by adding more features.                                                                                                                                              It also might contradict Topic A in some edge cases.', 'hash3')").execute(sqlite_pool).await.unwrap();

    // Call update backlinks explicitly or via cycle
    compiler.update_backlinks_and_typed_links().await.unwrap();

    let row = sqlx::query("SELECT backlinks FROM cortex_wiki_articles WHERE id = 'art1'")
        .fetch_one(sqlite_pool)
        .await
        .unwrap();
    use sqlx::Row;
    let backlinks_json: String = row.get("backlinks");
    assert!(
        backlinks_json.contains("Topic B"),
        "art1 should have backlink to Topic B"
    );

    // Check typed links for C -> B and C -> A
    #[derive(sqlx::FromRow)]
    struct TypedLinkRow {
        source: String,
        target: String,
        link_type: String,
    }

    let links: Vec<TypedLinkRow> = sqlx::query_as::<_, TypedLinkRow>(
        "SELECT source_article_id as source, target_article_id as target, link_type FROM cortex_typed_links ORDER BY source, target"
    )
    .fetch_all(sqlite_pool)
    .await
    .unwrap();

    let a_b = links
        .iter()
        .find(|l| l.source == "art1" && l.target == "art2")
        .expect("A -> B exists");
    assert_eq!(a_b.link_type, "references");

    let c_a = links
        .iter()
        .find(|l| l.source == "art3" && l.target == "art1")
        .expect("C -> A exists");
    assert_eq!(c_a.link_type, "contradicts");

    let c_b = links
        .iter()
        .find(|l| l.source == "art3" && l.target == "art2")
        .expect("C -> B exists");
    assert_eq!(c_b.link_type, "extends");
}

#[tokio::test]
async fn test_lint_wiki_not_stub() {
    let pool = setup_db_pool().await;
    let provider = Arc::new(MockLlmProvider { json_response: r#"```json
        [{"issue_type": "Contradiction", "article_id": "art1", "description": "Contradicts...", "suggested_action": "Fix"}]
    ```"#.to_string() });
    let semaphore = Arc::new(Semaphore::new(1));
    let compiler = CortexCompiler::new(provider, pool.clone(), None, semaphore);

    // Should return the mock issues instead of an empty vec immediately
    let issues = compiler.lint_wiki().await.unwrap();
    assert_eq!(
        issues.len(),
        1,
        "lint_wiki should actually call LLM and return issues"
    );
}

#[tokio::test]
async fn test_cross_cycle_two_source_principle() {
    let pool = setup_db_pool().await;
    let mock_json = r#"```json
        [
        {"name": "Persistent Topic", "description": "Spans cycles", "source_ids": []}
    ]
    ```"#;
    let provider = Arc::new(MockLlmProvider {
        json_response: mock_json.to_string(),
    });
    let semaphore = Arc::new(Semaphore::new(1));
    let compiler = CortexCompiler::new(provider, pool.clone(), None, semaphore);

    let sqlite_pool = pool.get_sqlite_pool_or_err().unwrap();

    // CYCLE 1: Insert doc1
    sqlx::query("INSERT INTO cortex_documents (id, title, content_md, content_hash, source_type) VALUES ('doc1', 'T1', 'C1', 'H1', 'manual')").execute(sqlite_pool).await.unwrap();

    // First cycle - only 1 source, so new_articles should be 0
    let report1 = compiler.run_compilation_cycle().await.unwrap();
    assert_eq!(
        report1.new_articles, 0,
        "Cycle 1 should NOT create article for 1 source"
    );

    // Check that doc1 is marked as compiled
    let compiled1: i64 =
        sqlx::query_scalar("SELECT compiled FROM cortex_documents WHERE id = 'doc1'")
            .fetch_one(sqlite_pool)
            .await
            .unwrap();
    assert_eq!(compiled1, 1, "doc1 should be compiled");

    // CYCLE 2: Insert doc2
    sqlx::query("INSERT INTO cortex_documents (id, title, content_md, content_hash, source_type) VALUES ('doc2', 'T2', 'C2', 'H2', 'manual')").execute(sqlite_pool).await.unwrap();

    // Second cycle - reads doc2, merges with doc1 from concept index, so new_articles should be >= 1
    let report2 = compiler.run_compilation_cycle().await.unwrap();
    assert!(
        report2.new_articles > 0,
        "Cycle 2 SHOULD create article by merging cross-cycle sources"
    );
}

#[tokio::test]
async fn test_concept_index_merge_not_overwrite() {
    let pool = setup_db_pool().await;
    let mock_json = r#"```json
        [
        {"name": "Merge Test", "description": "JSON array merging", "source_ids": []}
    ]
    ```"#;
    let provider = Arc::new(MockLlmProvider {
        json_response: mock_json.to_string(),
    });
    let semaphore = Arc::new(Semaphore::new(1));
    let compiler = CortexCompiler::new(provider, pool.clone(), None, semaphore);

    let sqlite_pool = pool.get_sqlite_pool_or_err().unwrap();

    // 1st insert
    sqlx::query("INSERT INTO cortex_documents (id, title, content_md, content_hash, source_type) VALUES ('doc1', 'T1', 'C1', 'H1', 'manual')").execute(sqlite_pool).await.unwrap();
    compiler.run_compilation_cycle().await.unwrap();

    // 2nd insert
    sqlx::query("INSERT INTO cortex_documents (id, title, content_md, content_hash, source_type) VALUES ('doc2', 'T2', 'C2', 'H2', 'manual')").execute(sqlite_pool).await.unwrap();
    compiler.run_compilation_cycle().await.unwrap();

    // Check index
    let doc_ids_json: String = sqlx::query_scalar(
        "SELECT document_ids FROM cortex_concept_index WHERE concept = 'merge test'",
    )
    .fetch_one(sqlite_pool)
    .await
    .unwrap();

    // Must contain both doc1 and doc2
    assert!(
        doc_ids_json.contains("doc1"),
        "Index MUST retain earlier doc1"
    );
    assert!(doc_ids_json.contains("doc2"), "Index MUST include new doc2");
}

#[tokio::test]
async fn test_compilation_cycle_calls_backlinks() {
    let pool = setup_db_pool().await;
    // Mock issues to test lint_wiki is called and its results are included
    let mock_json = r#"```json
        [{"issue_type": "Contradiction", "article_id": "art1", "description": "Contradicts...", "suggested_action": "Fix"}]
    ```"#;
    let provider = Arc::new(MockLlmProvider {
        json_response: mock_json.to_string(),
    });
    let semaphore = Arc::new(Semaphore::new(1));
    let compiler = CortexCompiler::new(provider, pool.clone(), None, semaphore);

    let sqlite_pool = pool.get_sqlite_pool_or_err().unwrap();
    sqlx::query("INSERT INTO cortex_documents (id, title, content_md, content_hash, source_type) VALUES ('doc1', 'T1', 'C1', 'H1', 'manual')").execute(sqlite_pool).await.unwrap();

    let report = compiler.run_compilation_cycle().await.unwrap();

    // The report must contain the issue returned by the mock lint_wiki
    assert_eq!(
        report.issues.len(),
        1,
        "run_compilation_cycle did not include lint_wiki issues"
    );
}
