use super::common::*;
use crate::app_state::Component;
use axum::http::StatusCode;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_gig_lifecycle() {
    let (server, _state, _tmp) = create_test_server().await;
    let auth = test_bearer();
    let requester_id = uuid::Uuid::new_v4();

    // 1. Publish Intent
    let publish_req = json!({
        "id": uuid::Uuid::new_v4(),
        "requester_id": requester_id,
        "description": "Test work request",
        "criteria": [
            {
                "type": "OracleJudge",
                "config": {
                    "rubric_prompt": "Is it good?",
                    "min_score": 0.0,
                    "model": null
                }
            }
        ],
        "max_budget_coins": 100,
        "category": "Other",
        "deadline": (chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339()
    });

    let resp = server
        .post("/api/v1/gig/publish")
        .add_header(axum::http::header::AUTHORIZATION, &auth)
        .json(&publish_req)
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::CREATED);
    let intent_id_json = resp.json::<serde_json::Value>();
    let intent_id = uuid::Uuid::parse_str(intent_id_json["id"].as_str().unwrap()).unwrap();

    // 2. Submit Bid
    let bid_id = uuid::Uuid::new_v4();
    let bidder_id = uuid::Uuid::new_v4();
    let bid_req = json!({
        "id": bid_id,
        "intent_id": intent_id,
        "bidder_id": bidder_id,
        "price_coins": 50,
        "est_duration_sec": 3600,
        "deposit_amount": 10
    });

    let resp = server
        .post("/api/v1/gig/bid")
        .add_header(axum::http::header::AUTHORIZATION, &auth)
        .json(&bid_req)
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);

    // 3. Accept Bid
    let resp = server
        .post(&format!("/api/v1/gig/accept/{}/{}", intent_id, bid_id))
        .add_header(axum::http::header::AUTHORIZATION, &auth)
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);

    // 4. Deliver
    let gig_artifacts_dir = _tmp.path().join("gig_artifacts");
    if !gig_artifacts_dir.exists() {
        std::fs::create_dir_all(&gig_artifacts_dir).unwrap();
    }
    std::fs::write(gig_artifacts_dir.join("artifact.txt"), "mock").unwrap();
    let deliver_req = json!({
        "order_id": intent_id,
        "deliverer_id": bidder_id,
        "artifact_path": "artifact.txt",
        "metadata": {}
    });
    let resp = server
        .post("/api/v1/gig/deliver")
        .add_header(axum::http::header::AUTHORIZATION, &auth)
        .json(&deliver_req)
        .await;
    println!("Deliver 500 Response: {:?}", resp.text());
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);

    // 5. Verify
    let resp = server
        .post(&format!("/api/v1/gig/verify/{}", intent_id))
        .add_header(axum::http::header::AUTHORIZATION, &auth)
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);
    let verify_res = resp.json::<aiome_core_contracts::gig::VerificationResult>();
    assert!(verify_res.passed);
}
#[serial]
#[tokio::test]
async fn test_awaiting_input_job_lifecycle() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    // 1. Insert a mock AwaitingInput job directly into DB
    let pool = state.db_pool.get_inner().get_sqlite_pool_or_err().unwrap();
    let test_job_id = "test-awaiting-input-job";

    // NOTE: This will fail until the SQLite CHECK constraint migration (Gap 10) is applied, or it might pass if AwaitingInput is already in the old check constraint.
    // The previous analysis showed AwaitingInput was IN the constraint, but Cancelled wasn't.
    sqlx::query(
        "INSERT INTO jobs (id, category, topic, style_name, karma_directives, status, priority) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(test_job_id)
    .bind("Goal")
    .bind("Dangerous action")
    .bind("default")
    .bind("[]")
    .bind("AwaitingInput")
    .bind(100)
    .execute(pool)
    .await.expect("Failed to insert mock AwaitingInput job");

    // 2. Test GET /api/v1/jobs/awaiting-input (RED: 404 expected initially)
    let get_resp = server
        .get("/api/v1/jobs/awaiting-input")
        .add_header("Authorization", &bearer)
        .await;

    assert_eq!(
        get_resp.status_code(),
        axum::http::StatusCode::OK,
        "Expected OK from /api/v1/jobs/awaiting-input"
    );

    let jobs: Vec<aiome_core_contracts::Job> = get_resp.json();
    assert!(
        jobs.iter().any(|j| j.id == test_job_id),
        "Expected the test job to be in the awaiting-input list"
    );

    // 3. Test POST /api/v1/jobs/{id}/review - Approve (RED: expected 200 OK after wiring, currently 202)
    let payload = serde_json::json!({
        "status": "approved",
        "comments": "Safe to proceed"
    });

    let review_resp = server
        .post(&format!("/api/v1/jobs/{}/review", test_job_id))
        .add_header("Authorization", &bearer)
        .json(&payload)
        .await;

    assert_eq!(
        review_resp.status_code(),
        axum::http::StatusCode::OK,
        "Expected OK when approving an AwaitingInput job"
    );

    // Verify it was requeued (status = Pending) and execution_log has bypass marker
    let updated_job = state
        .job_queue
        .fetch_job(test_job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_job.status, aiome_core_contracts::JobStatus::Pending);
    assert_eq!(
        updated_job.execution_log.as_deref(),
        Some("IMMUNE_BYPASS_APPROVED")
    );

    // 4. Test POST /api/v1/jobs/{id}/review - Race condition block (RED: expected 409 Conflict)
    let duplicate_resp = server
        .post(&format!("/api/v1/jobs/{}/review", test_job_id))
        .add_header("Authorization", &bearer)
        .json(&payload)
        .await;

    assert_eq!(
        duplicate_resp.status_code(),
        axum::http::StatusCode::CONFLICT,
        "Expected CONFLICT when approving a job that is no longer AwaitingInput"
    );
}
#[serial]
#[tokio::test]
async fn test_cancel_awaiting_input_job() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let pool = state.db_pool.get_inner().get_sqlite_pool_or_err().unwrap();
    let test_job_id = "test-cancel-awaiting-input-job";

    sqlx::query(
        "INSERT INTO jobs (id, category, topic, style_name, karma_directives, status, priority) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(test_job_id)
    .bind("Goal")
    .bind("Another action")
    .bind("default")
    .bind("[]")
    .bind("AwaitingInput")
    .bind(100)
    .execute(pool)
    .await.expect("Failed to insert mock AwaitingInput job for cancel test");

    // Test POST /api/v1/jobs/{id}/cancel (RED: currently fails to cancel AwaitingInput)
    let cancel_resp = server
        .post(&format!("/api/v1/jobs/{}/cancel", test_job_id))
        .add_header("Authorization", &bearer)
        .await;

    assert_eq!(
        cancel_resp.status_code(),
        axum::http::StatusCode::OK,
        "Expected OK when cancelling an AwaitingInput job"
    );

    let updated_job = state
        .job_queue
        .fetch_job(test_job_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        updated_job.status,
        aiome_core_contracts::JobStatus::Cancelled
    );
}
#[serial]
#[tokio::test]
async fn test_compute_semaphore_limits_concurrency() {
    let (_server, state, _tmp) = create_test_server().await;

    // compute_semaphore が正しく 1 で初期化されているかをテストする
    let semaphore = state.compute_semaphore.get_inner();
    let permits = semaphore.available_permits();

    // Mac Unified Memory の枯渇を防ぐため、並列実行可能な重いタスク（LoRA, ImageGen）は「1つ」のみに制限されるべき
    assert_eq!(
        permits, 1,
        "compute_semaphore must restrict heavy tasks to 1 concurrent execution to prevent OOM / Kernel panic"
    );

    // 強制的にロックを取得して1つ消費した場合、もう available は 0 になることを確認
    let _permit = semaphore
        .try_acquire()
        .expect("Should be able to acquire the single compute permit");
    assert_eq!(semaphore.available_permits(), 0);
}
