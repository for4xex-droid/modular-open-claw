use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::JobStatus;
use aiome_core_contracts::traits::TaskRegistry;
use async_trait::async_trait;
use infrastructure::buzz::generator::BuzzContentGenerator;
use infrastructure::buzz::scheduler::BuzzScheduler;
use infrastructure::buzz::worker::process_pending_buzz;
use infrastructure::job_queue::UniversalJobQueue;
use std::sync::Arc;

async fn create_test_queue() -> UniversalJobQueue {
    let pool = infrastructure::db::DatabasePool::new_sqlite("sqlite::memory:")
        .await
        .unwrap();
    let ts = std::sync::Arc::new(
        infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
    );
    UniversalJobQueue::new(pool.clone(), None, ts)
        .await
        .expect("Failed to create test job queue")
}

#[derive(Debug, Clone)]
struct MockLlmProvider;

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete(
        &self,
        _prompt: &str,
        _system: Option<&str>,
    ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
        Ok(aiome_core::llm_provider::LlmResponse {
            content: "Mock generated buzz".to_string(),
            stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
            ..Default::default()
        })
    }
    async fn complete_with_cache(
        &self,
        _request: aiome_core_contracts::llm::LlmRequest,
    ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
        self.complete("", None).await
    }
    fn name(&self) -> &str {
        "Mock"
    }
    async fn test_connection(&self) -> Result<(), AiomeError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_process_pending_buzz_generates_draft_when_allowed() {
    let jq = Arc::new(create_test_queue().await);

    let mock_llm = MockLlmProvider;

    let gen = Arc::new(BuzzContentGenerator::new(Arc::new(mock_llm)));
    let sched = BuzzScheduler::new(90, 4);

    // Initial run: no jobs in queue, should generate one draft
    process_pending_buzz(&jq, &gen, &sched)
        .await
        .expect("Process failed");

    // Fetch jobs, there should be exactly one pending buzz draft
    let jobs = jq.fetch_recent_jobs(100).await.expect("List failed");
    let pending_buzz: Vec<_> = jobs
        .into_iter()
        .filter(|j| j.category == "buzz" && j.status == JobStatus::Pending)
        .collect();
    assert_eq!(pending_buzz.len(), 1, "Should have created 1 draft");
    let draft_json: serde_json::Value =
        serde_json::from_str(pending_buzz[0].output_artifacts.as_deref().unwrap_or("{}")).unwrap();
    assert_eq!(
        draft_json.get("text").and_then(|v| v.as_str()),
        Some("Mock generated buzz")
    );

    // Second run immediately: should NOT generate because pending draft already exists
    process_pending_buzz(&jq, &gen, &sched)
        .await
        .expect("Process failed");
    let jobs2 = jq.fetch_recent_jobs(100).await.expect("List failed");
    let pending_buzz2: Vec<_> = jobs2
        .into_iter()
        .filter(|j| j.category == "buzz" && j.status == JobStatus::Pending)
        .collect();
    assert_eq!(
        pending_buzz2.len(),
        1,
        "Should not have created a second draft"
    );
}

/// LLM failure must propagate as AiomeError, not silently succeed
#[tokio::test]
async fn test_process_pending_buzz_propagates_llm_error() {
    #[derive(Debug, Clone)]
    struct FailingLlmProvider;

    #[async_trait]
    impl LlmProvider for FailingLlmProvider {
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
            Err(AiomeError::Infrastructure {
                reason: "LLM service unavailable".into(),
            })
        }
        async fn complete_with_cache(
            &self,
            _request: aiome_core_contracts::llm::LlmRequest,
        ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
            self.complete("", None).await
        }
        fn name(&self) -> &str {
            "FailingMock"
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    let jq = Arc::new(create_test_queue().await);
    let gen = Arc::new(BuzzContentGenerator::new(Arc::new(FailingLlmProvider)));
    let sched = BuzzScheduler::new(0, 10); // no interval gate, no daily limit gate

    let result = process_pending_buzz(&jq, &gen, &sched).await;
    assert!(
        result.is_err(),
        "LLM failure must propagate, not be swallowed"
    );

    // Verify no jobs were partially created
    let jobs = jq.fetch_recent_jobs(100).await.expect("List failed");
    let buzz_jobs: Vec<_> = jobs.into_iter().filter(|j| j.category == "buzz").collect();
    // The enqueue happens before generate in actual flow? No — generate happens first.
    // So no job should exist at all.
    assert_eq!(
        buzz_jobs.len(),
        0,
        "No partial job should exist after LLM failure"
    );
}

/// Existing pending draft must block new generation (idempotency guard)
#[tokio::test]
async fn test_process_pending_buzz_skips_when_pending_exists() {
    let jq = Arc::new(create_test_queue().await);
    let gen = Arc::new(BuzzContentGenerator::new(Arc::new(MockLlmProvider)));
    let sched = BuzzScheduler::new(0, 10); // no restrictions

    // First run: generates a draft
    process_pending_buzz(&jq, &gen, &sched)
        .await
        .expect("First run failed");

    // Draft is now Pending — second run should skip
    process_pending_buzz(&jq, &gen, &sched)
        .await
        .expect("Second run failed");

    let jobs = jq.fetch_recent_jobs(100).await.expect("List failed");
    let pending: Vec<_> = jobs
        .into_iter()
        .filter(|j| j.category == "buzz" && j.status == JobStatus::Pending)
        .collect();
    assert_eq!(pending.len(), 1, "Only one pending draft must exist");
}
