/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::*;
use crate::task_orchestrator::planner::DefaultStrategicPlanner;
use crate::task_orchestrator::TaskDispatcher;
use crate::test_utils::job_queue_mock::{GlobalMockJobQueue, GlobalMockLlm};
use aiome_core_contracts::traits::*;
use tokio::time::timeout;

#[tokio::test]
async fn test_compute_soul_hash() {
    // Test with None
    let hash_none = compute_soul_hash(&None).await;
    assert_eq!(hash_none, "unknown");

    // Test with a temporary file
    let temp_dir = tempfile::tempdir().unwrap();
    let soul_md_path = temp_dir.path().join("SOUL.md");
    let evolving_soul_md_path = temp_dir.path().join("EVOLVING_SOUL.md");

    tokio::fs::write(&soul_md_path, "I am the soul")
        .await
        .unwrap();
    tokio::fs::write(&evolving_soul_md_path, "I am evolving")
        .await
        .unwrap();

    let hash_some = compute_soul_hash(&Some(soul_md_path)).await;
    assert_ne!(hash_some, "unknown");
    // DefaultHasher produces a u64 -> hex は最大16文字
    assert!(
        hash_some.len() <= 16 && !hash_some.is_empty(),
        "Hash should be a valid hex string, got: {}",
        hash_some
    );

    // If files are missing, it should fallback to a consistent hash (hash of empty strings)
    let empty_dir = tempfile::tempdir().unwrap();
    let missing_path = empty_dir.path().join("SOUL.md");
    let hash_missing = compute_soul_hash(&Some(missing_path)).await;
    assert_ne!(hash_missing, "unknown");
    assert_ne!(hash_missing, hash_some);
}

struct TestConductor;
#[async_trait]
impl TaskConductor for TestConductor {
    fn conductor_name(&self) -> &str {
        "TestConductor"
    }
    fn capable_categories(&self) -> Vec<String> {
        vec!["test_cat".into()]
    }
    async fn conduct(
        &self,
        job: Job,
        progress_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<(String, Option<String>), AiomeError> {
        if let Err(e) = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id,
                conductor_id: self.conductor_name().to_string(),
                message: "testing".into(),
                percent: Some(50),
            })
            .await
        {
            tracing::warn!("Failed to send dummy progress event: {}", e);
        }
        Ok(("done".into(), None))
    }
}

#[tokio::test]
async fn test_dispatcher_event_flow() {
    let mut job = Job::default();
    job.id = "job-42".into();
    job.category = "test_cat".into();

    let job_queue = Arc::new(GlobalMockJobQueue {
        job_to_return: std::sync::Mutex::new(Some(job)),
        completed: std::sync::Mutex::new(false),
        ..Default::default()
    });

    let mut dispatcher = TaskDispatcher::new(
        job_queue.clone(),
        Duration::from_millis(10),
        None,
        None,
        None,
        None,
        None,
        None,
        None, // gig_engine
        None, // diagnostics
        None, // immune_system
        None, // quality_gate_store
        None, // hook_manager
    );
    dispatcher.register_conductor(Arc::new(TestConductor));

    let mut rx = dispatcher.subscribe_events();

    let handle = tokio::spawn(async move {
        dispatcher.run_dispatch_loop().await;
    });

    // Use timeout to assert events
    let event1 = timeout(Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(event1, TaskEvent::Spawned { .. }));

    let event2 = timeout(Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(event2, TaskEvent::Progress { .. }));

    let event3 = timeout(Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(event3, TaskEvent::Completed { .. }));

    // Check if queue complete function was called (this uses async spawned code, so give it a tiny delay)
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(*job_queue.completed.lock().unwrap());

    handle.abort();
}

#[test]
fn test_task_event_evaluating_exists() {
    let event = TaskEvent::Evaluating {
        job_id: "test-job".to_string(),
    };
    match event {
        TaskEvent::Evaluating { job_id } => {
            assert_eq!(job_id, "test-job");
        }
        _ => panic!("Expected Evaluating event"),
    }
}

struct CancelTestConductor;
#[async_trait]
impl TaskConductor for CancelTestConductor {
    fn conductor_name(&self) -> &str {
        "CancelTestConductor"
    }
    fn capable_categories(&self) -> Vec<String> {
        vec!["test_cat".into()]
    }
    async fn conduct(
        &self,
        _job: Job,
        _progress_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<(String, Option<String>), AiomeError> {
        tokio::time::sleep(Duration::from_secs(10)).await;
        Ok(("done".into(), None))
    }
}

#[tokio::test]
async fn test_dispatcher_emits_cancelled_event() {
    let mut job = Job::default();
    job.id = "job-cancel-42".into();
    job.category = "test_cat".into();

    let job_queue = Arc::new(GlobalMockJobQueue {
        job_to_return: std::sync::Mutex::new(Some(job)),
        completed: std::sync::Mutex::new(false),
        ..Default::default()
    });

    let mut dispatcher = TaskDispatcher::new(
        job_queue.clone(),
        Duration::from_millis(10),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None, // hook_manager
    );
    dispatcher.register_conductor(Arc::new(CancelTestConductor));

    let mut rx = dispatcher.subscribe_events();

    let dispatcher = Arc::new(dispatcher);
    let dispatcher_clone = dispatcher.clone();
    let handle = tokio::spawn(async move {
        dispatcher_clone.run_dispatch_loop().await;
    });

    // Wait for Spawned event
    let event1 = timeout(Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(event1, TaskEvent::Spawned { .. }));

    // Send cancel_job
    assert!(dispatcher.cancel_job("job-cancel-42").await.is_ok());

    // We expect either Cancelled or Failed(Cancelled by user)
    let mut got_cancelled = false;
    for _ in 0..3 {
        if let Ok(Ok(TaskEvent::Cancelled { job_id: id })) =
            timeout(Duration::from_millis(500), rx.recv()).await
        {
            assert_eq!(id, "job-cancel-42");
            got_cancelled = true;
            break;
        }
    }

    assert!(got_cancelled, "Failed to receive TaskEvent::Cancelled");
    handle.abort();
}

struct MockGigEngine {
    pub published_intent: Arc<tokio::sync::Mutex<Option<aiome_core_contracts::gig::GigIntent>>>,
}

#[async_trait]
impl aiome_core_contracts::gig::GigEngine for MockGigEngine {
    async fn publish_intent(
        &self,
        intent: aiome_core_contracts::gig::GigIntent,
    ) -> Result<uuid::Uuid, aiome_core_contracts::error::AiomeError> {
        let mut lock = self.published_intent.lock().await;
        let id = intent.id;
        *lock = Some(intent);
        Ok(id)
    }
    async fn submit_bid(
        &self,
        _bid: aiome_core_contracts::gig::GigBid,
    ) -> Result<(), aiome_core_contracts::error::AiomeError> {
        Ok(())
    }
    async fn accept_bid(
        &self,
        _intent_id: uuid::Uuid,
        _bid_id: uuid::Uuid,
    ) -> Result<(), aiome_core_contracts::error::AiomeError> {
        Ok(())
    }
    async fn deliver(
        &self,
        _deliverable: aiome_core_contracts::gig::GigDeliverable,
    ) -> Result<(), aiome_core_contracts::error::AiomeError> {
        Ok(())
    }
    async fn verify_and_settle(
        &self,
        _order_id: uuid::Uuid,
    ) -> Result<
        aiome_core_contracts::gig::VerificationResult,
        aiome_core_contracts::error::AiomeError,
    > {
        Ok(aiome_core_contracts::gig::VerificationResult {
            order_id: uuid::Uuid::new_v4(),
            passed: true,
            score: 1.0,
            detail: "ok".into(),
        })
    }
}

#[tokio::test]
async fn test_dispatcher_publishes_gig_on_completion() {
    let mut job = Job::default();
    job.id = "gig-job".into();
    job.category = "test_cat".into();
    job.karma_directives = Some(r#"{"gig_intent": true}"#.to_string());
    job.output_artifacts =
        Some(r#"{"description": "Need a special tool", "budget": 100}"#.to_string());

    let job_queue = Arc::new(GlobalMockJobQueue {
        job_to_return: std::sync::Mutex::new(Some(job.clone())),
        fetched_job: std::sync::Mutex::new(Some(job)),
        completed: std::sync::Mutex::new(false),
        ..Default::default()
    });

    let mock_gig = Arc::new(MockGigEngine {
        published_intent: Arc::new(tokio::sync::Mutex::new(None)),
    });

    let mut dispatcher = TaskDispatcher::new(
        job_queue.clone(),
        Duration::from_millis(10),
        None,
        None,
        None,
        None,
        None,
        None,
        Some(mock_gig.clone()),
        None,
        None,
        None,
        None, // hook_manager
    );
    dispatcher.register_conductor(Arc::new(TestConductor));

    let _handle = tokio::spawn(async move {
        dispatcher.run_dispatch_loop().await;
    });

    // Wait for completion
    tokio::time::sleep(Duration::from_millis(100)).await;

    let intent_lock = mock_gig.published_intent.lock().await;
    assert!(
        intent_lock.is_some(),
        "GIG Intent should have been published"
    );
    assert_eq!(intent_lock.as_ref().unwrap().max_budget_coins, 100);
}

#[tokio::test]
async fn test_dispatcher_respects_gig_depth_limit() {
    let mut job = Job::default();
    job.id = "deep-gig-job".into();
    job.karma_directives = Some(r#"{"gig_intent": true, "gig_depth": 3}"#.to_string());
    job.output_artifacts = Some(r#"{"description": "Too deep", "budget": 100}"#.to_string());

    let job_queue = Arc::new(GlobalMockJobQueue {
        job_to_return: std::sync::Mutex::new(Some(job.clone())),
        fetched_job: std::sync::Mutex::new(Some(job)),
        ..Default::default()
    });

    let mock_gig = Arc::new(MockGigEngine {
        published_intent: Arc::new(tokio::sync::Mutex::new(None)),
    });

    let mut dispatcher = TaskDispatcher::new(
        job_queue.clone(),
        Duration::from_millis(10),
        None,
        None,
        None,
        None,
        None,
        None,
        Some(mock_gig.clone()),
        None,
        None,
        None,
        None, // hook_manager
    );
    dispatcher.register_conductor(Arc::new(TestConductor));

    let _handle = tokio::spawn(async move {
        dispatcher.run_dispatch_loop().await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let intent_lock = mock_gig.published_intent.lock().await;
    assert!(
        intent_lock.is_none(),
        "GIG Intent should NOT have been published due to depth limit"
    );
}

#[tokio::test]
async fn test_dispatcher_watchtower_diagnostic_loop() {
    use aiome_core::llm_provider::{LlmProvider, LlmResponse, StopReason};
    use aiome_core_contracts::llm::LlmRequest;

    #[derive(Debug)]
    struct MockLlmForDiagnostics;
    #[async_trait]
    impl LlmProvider for MockLlmForDiagnostics {
        async fn complete(&self, _: &str, _: Option<&str>) -> Result<LlmResponse, AiomeError> {
            let json_resp = serde_json::json!({
                "critical_failure_step": 1,
                "failure_category": "SystemFailure",
                "root_cause": "Forced failure for testing watchtower",
                "self_repair_hint": "Try writing better code"
            })
            .to_string();
            Ok(LlmResponse {
                content: format!("```json\n{}\n```", json_resp),
                stop_reason: StopReason::EndTurn,
                ..Default::default()
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "Mock"
        }
        async fn complete_with_cache(&self, _: LlmRequest) -> Result<LlmResponse, AiomeError> {
            self.complete("", None).await
        }
    }

    struct FailingConductor;
    #[async_trait]
    impl TaskConductor for FailingConductor {
        fn conductor_name(&self) -> &str {
            "FailingConductor"
        }
        fn capable_categories(&self) -> Vec<String> {
            vec!["test_cat".into()]
        }
        async fn conduct(
            &self,
            _: Job,
            _: mpsc::Sender<TaskEvent>,
        ) -> Result<(String, Option<String>), AiomeError> {
            Err(AiomeError::Infrastructure {
                reason: "Forced test failure".into(),
            })
        }
    }

    let mut job = Job::default();
    job.id = "failed-job".into();
    job.category = "test_cat".into();

    let job_queue = Arc::new(GlobalMockJobQueue {
        job_to_return: std::sync::Mutex::new(Some(job.clone())),
        fetched_job: std::sync::Mutex::new(Some(job)),
        ..Default::default()
    });

    {
        use aiome_core_contracts::trajectory::TrajectoryStep;
        let step = TrajectoryStep {
            step_id: 1,
            action: "Test Action".into(),
            is_critical_failure: true,
            ..Default::default()
        };
        job_queue
            .store_trajectory_step(step)
            .await
            .expect("store_trajectory_step should succeed in test");
    }

    let diag_engine = Arc::new(crate::diagnostics::AgentRxDiagnostics::new(Arc::new(
        MockLlmForDiagnostics,
    )));
    let mut dispatcher = TaskDispatcher::new(
        job_queue.clone(),
        Duration::from_millis(10),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(diag_engine),
        None,
        None, // quality_gate_store
        None, // hook_manager
    );
    dispatcher.register_conductor(Arc::new(FailingConductor));
    let _handle = tokio::spawn(async move {
        dispatcher.run_dispatch_loop().await;
    });

    let mut diagnosis = None;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let current = job_queue.diagnosis.lock().unwrap().clone();
        if current.is_some() {
            diagnosis = current;
            break;
        }
    }

    assert!(
        diagnosis.is_some(),
        "Watchtower should have generated a diagnosis within timeout"
    );
    let diagnosis = diagnosis.unwrap();
    assert_eq!(diagnosis.self_repair_hint, "Try writing better code");
    assert_eq!(
        diagnosis.category,
        aiome_core_contracts::trajectory::FailureCategory::SystemFailure
    );
}

#[tokio::test]
async fn test_dispatcher_fallback_to_tool_discovery() {
    let mut job = Job::default();
    job.id = "unknown-job".into();
    job.category = "unknown_category".into();
    job.topic = "Please parse this file".into();

    let job_queue = Arc::new(GlobalMockJobQueue {
        job_to_return: std::sync::Mutex::new(Some(job.clone())),
        fetched_job: std::sync::Mutex::new(Some(job)),
        ..Default::default()
    });

    struct MockToolDiscovery;
    #[async_trait]
    impl aiome_core_contracts::traits::ToolDiscoveryEngine for MockToolDiscovery {
        async fn discover_tools(&self) -> Result<Vec<serde_json::Value>, AiomeError> {
            Ok(vec![])
        }
        async fn suggest_tools(&self, instruction: &str) -> Result<Vec<String>, AiomeError> {
            if instruction.contains("parse") {
                Ok(vec!["file_parser_tool".into()])
            } else {
                Ok(vec![])
            }
        }
    }

    let tool_discovery = Arc::new(MockToolDiscovery);
    let mut dispatcher = TaskDispatcher::new(
        job_queue.clone(),
        Duration::from_millis(10),
        None,
        Some(tool_discovery),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None, // hook_manager
    );

    let _handle = tokio::spawn(async move {
        dispatcher.run_dispatch_loop().await;
    });

    tokio::time::sleep(Duration::from_millis(150)).await;

    let lock = job_queue.failed_jobs.lock().unwrap();
    let failed_job = lock
        .iter()
        .find(|(id, _)| id == "unknown-job")
        .expect("Job should be failed");
    assert!(
        failed_job.1.contains("file_parser_tool"),
        "Message should contain tool name"
    );
}

#[tokio::test]
async fn test_dispatcher_elicitation_on_high_severity_violation() {
    use aiome_core::llm_provider::{LlmProvider, LlmResponse, StopReason};
    use aiome_core_contracts::contracts::{ApprovalState, ImmuneRule};
    use aiome_core_contracts::llm::LlmRequest;
    use chrono::Utc;

    #[derive(Debug)]
    struct MockLlmForPlanning;

    #[async_trait]
    impl LlmProvider for MockLlmForPlanning {
        async fn complete(
            &self,
            content: &str,
            system: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            let sys = system.unwrap_or("");
            if sys.contains("Constitutional Finder") {
                return Ok(LlmResponse {
                    content: "NONE".into(),
                    stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
                    ..Default::default()
                });
            }
            if sys.contains("Supreme Constitutional Referee") {
                return Ok(LlmResponse {
                    content: "PASS".into(),
                    stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
                    ..Default::default()
                });
            }

            let steps = serde_json::json!([
                {
                    "description": "Send sensitive data",
                    "step_category": "Execution",
                    "reasoning": "Test elicitation",
                    "tool_name": "network_sender",
                    "input": { "data": "secret" }
                }
            ]);
            Ok(LlmResponse {
                content: format!("```json\n{}\n```", steps),
                stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
                ..Default::default()
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "Mock"
        }
        async fn complete_with_cache(&self, _: LlmRequest) -> Result<LlmResponse, AiomeError> {
            self.complete("", None).await
        }
    }

    let mut job = Job::default();
    job.id = "elicit-job".into();
    job.category = "Goal".into();
    job.topic = "Simulate threat".into();

    let job_queue = Arc::new(GlobalMockJobQueue {
        job_to_return: std::sync::Mutex::new(Some(job.clone())),
        fetched_job: std::sync::Mutex::new(Some(job)),
        active_rules: std::sync::Mutex::new(vec![ImmuneRule {
            id: "block-sender".into(),
            pattern: "network_sender".into(),
            severity: 85,
            action: "Block".into(),
            created_at: Utc::now().to_rfc3339(),
            approval_status: ApprovalState::Approved,
            lamport_clock: 0,
            node_id: "test".into(),
            signature: None,
            input_constraints: None,
        }]),
        ..Default::default()
    });

    let mock_llm = Arc::new(MockLlmForPlanning);
    let planner = Arc::new(DefaultStrategicPlanner::new(mock_llm.clone()));
    let immune_system = Arc::new(crate::immune_system::AdaptiveImmuneSystem::new(
        mock_llm.clone(),
    ));

    let validator = Arc::new(crate::validator::DefaultConstitutionalValidator::new(
        mock_llm.clone(),
        None,
    ));
    let mut dispatcher = TaskDispatcher::new(
        job_queue.clone(),
        Duration::from_millis(10),
        None,
        None,
        Some(planner),
        Some(validator),
        None,
        None,
        None,
        None,
        Some(immune_system),
        None,
        None, // hook_manager
    );
    let mut rx = dispatcher.subscribe_events();
    dispatcher.register_conductor(Arc::new(TestConductor));

    let _handle = tokio::spawn(async move {
        dispatcher.run_dispatch_loop().await;
    });

    let mut elicit_event_received = false;
    for _ in 0..20 {
        if let Ok(TaskEvent::AwaitingInput { job_id, reason }) = rx.try_recv() {
            if job_id == "elicit-job" && reason.contains("Blocked") {
                elicit_event_received = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    assert!(
        elicit_event_received,
        "Dispatcher should have emitted an AwaitingInput event for elicitation"
    );
    let status = job_queue.updated_status.lock().unwrap().clone();
    assert_eq!(
        status,
        Some(aiome_core_contracts::traits::JobStatus::AwaitingInput)
    );
}

#[derive(Debug)]
struct FailingConductor;
#[async_trait]
impl TaskConductor for FailingConductor {
    fn conductor_name(&self) -> &str {
        "FailingConductor"
    }
    fn capable_categories(&self) -> Vec<String> {
        vec!["fail_cat".into()]
    }
    async fn conduct(
        &self,
        _job: Job,
        _progress_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<(String, Option<String>), AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "High Uncertainty Limit Exceeded".into(),
        })
    }
}

#[tokio::test]
async fn test_dispatcher_stores_karma_on_high_uncertainty() {
    let mut job = Job::default();
    job.id = "job-fail-1".into();
    job.category = "fail_cat".into();

    let job_queue = Arc::new(GlobalMockJobQueue {
        job_to_return: std::sync::Mutex::new(Some(job.clone())),
        fetched_job: std::sync::Mutex::new(Some(job)),
        ..Default::default()
    });

    let mut dispatcher = TaskDispatcher::new(
        job_queue.clone(),
        Duration::from_millis(10),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    dispatcher.register_conductor(Arc::new(FailingConductor));

    let mut rx = dispatcher.subscribe_events();

    let _handle = tokio::spawn(async move {
        dispatcher.run_dispatch_loop().await;
    });

    for _ in 0..10 {
        if let Ok(TaskEvent::Failed { job_id, .. }) = rx.try_recv() {
            if job_id == "job-fail-1" {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    let karmas = job_queue.karmas.lock().unwrap().clone();
    assert!(
        !karmas.is_empty(),
        "Karma should be stored on complete failure"
    );
    assert_eq!(karmas[0]["karma_type"], "negative");
    assert_eq!(karmas[0]["skill_id"], "FailingConductor");
    assert!(
        karmas[0]["lesson"]
            .as_str()
            .unwrap()
            .contains("High Uncertainty Limit Exceeded"),
        "Lesson should contain original error message"
    );
}

#[tokio::test]
async fn test_dispatcher_stores_karma_with_soul_path() {
    let temp_dir = tempfile::tempdir().unwrap();
    let soul_md_path = temp_dir.path().join("SOUL.md");
    let evolving_soul_md_path = temp_dir.path().join("EVOLVING_SOUL.md");
    tokio::fs::write(&soul_md_path, "test soul").await.unwrap();
    tokio::fs::write(&evolving_soul_md_path, "test evolving")
        .await
        .unwrap();

    let mut job = Job::default();
    job.id = "job-fail-2".into();
    job.category = "fail_cat".into();

    let job_queue = Arc::new(GlobalMockJobQueue {
        job_to_return: std::sync::Mutex::new(Some(job.clone())),
        fetched_job: std::sync::Mutex::new(Some(job)),
        ..Default::default()
    });

    let mut dispatcher = TaskDispatcher::new(
        job_queue.clone(),
        Duration::from_millis(10),
        None,
        None,
        None,
        None,
        Some(soul_md_path),
        None,
        None,
        None,
        None,
        None,
        None,
    );
    dispatcher.register_conductor(Arc::new(FailingConductor));

    let mut rx = dispatcher.subscribe_events();

    let _handle = tokio::spawn(async move {
        dispatcher.run_dispatch_loop().await;
    });

    for _ in 0..10 {
        if let Ok(TaskEvent::Failed { job_id, .. }) = rx.try_recv() {
            if job_id == "job-fail-2" {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    let karmas = job_queue.karmas.lock().unwrap().clone();
    assert!(
        !karmas.is_empty(),
        "Karma should be stored on complete failure"
    );
    assert_ne!(
        karmas[0]["soul_hash"], "unknown",
        "soul_hash should not be unknown when soul_path is provided"
    );
}
