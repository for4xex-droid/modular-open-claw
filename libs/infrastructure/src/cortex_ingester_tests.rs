use crate::cortex_ingester::{CortexIngester, SourceType};
use aiome_core::error::AiomeError;
use aiome_core_contracts::llm::{LlmProvider, LlmResponse, StopReason};
use async_trait::async_trait;
use std::sync::Arc;

// Create a simple mock using async-trait to avoid mockall issues with async traits in this crate version
struct MockLlm;
impl std::fmt::Debug for MockLlm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MockLlm")
    }
}
#[async_trait]
impl LlmProvider for MockLlm {
    fn name(&self) -> &str {
        "mock"
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn complete(
        &self,
        prompt: &str,
        _preamble: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        if prompt.contains("EXTRACT_FAIL") {
            Ok(LlmResponse {
                content: "FAILED".to_string(),
                stop_reason: StopReason::EndTurn,
                usage: Default::default(),
            })
        } else {
            Ok(LlmResponse {
                content: "```json\n{\n  \"title\": \"Sample Cortex Document\",\n  \"summary\": \"A test summary\",\n  \"tags\": [\"test\", \"cortex\"],\n  \"entities\": [{\"name\": \"AI\", \"type\": \"Technology\"}]\n}\n```".to_string(),
                stop_reason: StopReason::EndTurn,
                usage: Default::default(),
            })
        }
    }

    async fn complete_structured(
        &self,
        _prompt: &str,
        _schema: &serde_json::Value,
    ) -> Result<String, AiomeError> {
        unimplemented!()
    }

    async fn stream_complete(
        &self,
        _prompt: &str,
        _preamble: Option<&str>,
        _tx: tokio::sync::mpsc::Sender<String>,
    ) -> Result<aiome_core_contracts::llm::UsageStats, AiomeError> {
        unimplemented!()
    }
}

// Global mock pool initialization helper could be used here but keeping tests self-contained is better

#[tokio::test]
async fn test_ingest_url_security_block() {
    let provider = Arc::new(MockLlm);
    let jq = Arc::new(crate::test_utils::GlobalMockJobQueue::new());

    let ingester = CortexIngester::new(provider, jq);

    // Test that private/local URLs are blocked by SecurityPolicy
    let res = ingester.ingest_url("http://127.0.0.1/admin").await;
    assert!(res.is_err());

    match res.unwrap_err() {
        AiomeError::SecurityViolation { reason } => {
            assert!(reason.contains("Invalid or restricted URL"));
        }
        _ => panic!("Expected SecurityViolation"),
    }
}

#[tokio::test]
async fn test_ingest_text() {
    let provider = Arc::new(MockLlm);
    let jq = Arc::new(crate::test_utils::GlobalMockJobQueue::new());

    let ingester = CortexIngester::new(provider, jq);

    let doc = ingester
        .ingest_text("Sample Title", "This is the content of the manual text")
        .await
        .unwrap();

    assert_eq!(doc.title, "Sample Title");
    assert_eq!(doc.source_type.as_str(), "manual");
    assert_eq!(doc.metadata.summary, "A test summary");
    assert_eq!(doc.metadata.tags, vec!["test", "cortex"]);
}

#[tokio::test]
async fn test_delete_document() {
    // Tests the ingestion and subsequent mock deletion
    let provider = Arc::new(MockLlm);
    let jq = Arc::new(crate::test_utils::GlobalMockJobQueue::new());
    let ingester = CortexIngester::new(provider, jq);

    // We cannot truly delete from SQLite since we're using a MockJobQueue that doesn't implement it perfectly yet,
    // but we can ensure the cortex_ingester method doesn't panic and returns Ok.

    let res = ingester.delete_document("test-uuid").await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_ingest_pdf() {
    let provider = Arc::new(MockLlm);
    let jq = Arc::new(crate::test_utils::GlobalMockJobQueue::new());
    let ingester = CortexIngester::new(provider, jq);

    // In an actual test, we would provide a small valid PDF byte array.
    // However, pdf_extract checks for magic bytes ("%PDF"). We expect an Infrastructure error for invalid PDF.
    let invalid_pdf_data = b"NOT A PDF";

    let res = ingester.ingest_pdf(invalid_pdf_data, "Test PDF").await;
    assert!(res.is_err());

    match res.unwrap_err() {
        AiomeError::Infrastructure { reason } => {
            assert!(reason.contains("Failed to extract"));
        }
        _ => panic!("Expected Infrastructure error"),
    }
}
