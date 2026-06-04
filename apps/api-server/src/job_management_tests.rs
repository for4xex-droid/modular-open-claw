/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
/*
 * Aiome - Job Management Integration Tests (TDD RED)
 */

use super::api_integration_tests::{create_test_server, test_bearer};
use axum::http::StatusCode;

#[tokio::test]
async fn test_job_cancel_non_existent() {
    let (server, _state, _tmp) = create_test_server().await;

    // G-24: POST /api/v1/jobs/:id/cancel
    let response = server
        .post("/api/v1/jobs/non-existent-id/cancel")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    // 現在はルートが存在しないため 404 になるはず。
    // 実装後は、対象がない場合に 404 を返すべき。
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_job_logs_non_existent() {
    let (server, _state, _tmp) = create_test_server().await;

    // G-25: GET /api/v1/jobs/:id/logs
    let response = server
        .get("/api/v1/jobs/non-existent-id/logs")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}
