/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use axum_test::TestServer;
use commerce_protocol::identity::ActorId;
use nurture_api::routes::nurture_routes;
use nurture_api::state::AppState;
use nurture_bridge::auth::MockAuthManager;
use serial_test::serial;
use sqlx::SqlitePool;
use tempfile::tempdir;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_mcp_server_tools_list_and_call() {
    let tdir = tempdir().unwrap();
    let db_path = tdir.path().join("test_mcp.db");
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap()))
        .await
        .unwrap();

    // Run migrations
    sqlx::migrate!("../../migrations/sqlite")
        .run(&pool)
        .await
        .unwrap();

    let buyer_id = Uuid::new_v4();
    let system_id = Uuid::new_v4();
    let token = format!("mock_valid_token_{}", buyer_id);

    let cancel_token = tokio_util::sync::CancellationToken::new();
    let db_url = format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap());
    let store = std::sync::Arc::new(
        nurture_bridge::job_queue::trajectory_store::SqliteTrajectoryStore::from_db_path(&db_url)
            .await
            .unwrap(),
    ) as std::sync::Arc<dyn nurture_bridge::trajectory::TrajectoryStore>;
    let job_queue: std::sync::Arc<dyn nurture_bridge::traits::JobQueue> =
        std::sync::Arc::new(nurture_bridge::job_queue::UniversalJobQueue::from_pool(
            nurture_bridge::db::DatabasePool::Sqlite(pool.clone()),
            store,
        ));

    // Setup state and app
    let state = AppState::init(
        nurture_bridge::db::DatabasePool::Sqlite(pool.clone()),
        job_queue,
        nurture_core::policy::EconomyPolicy::default(),
        ActorId(system_id),
        cancel_token,
        "test_nurture_secret".to_string().into(),
        None,
        None,
        std::sync::Arc::new(MockAuthManager::new()),
        "test_drm_master_key".to_string().into(),
        std::sync::Arc::new(nurture_infra::storage::MockAssetStorage::new()),
        None,
        "localhost".to_string(),
        "50051".to_string(),
    )
    .await
    .unwrap();

    // We assume MCP routes are mounted under /api/v1/mcp in nurture_routes
    let app = nurture_routes(state);
    let server = TestServer::new(app).unwrap();

    // 1. Initialize MCP
    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "TestClient",
                "version": "1.0.0"
            }
        },
        "id": 1
    });

    let res_init = server
        .post("/mcp/message?sessionId=test-session")
        .add_header("Authorization", &format!("Bearer {}", token))
        .json(&init_req)
        .await;

    res_init.assert_status_success(); // 202 ACCEPTED

    let call_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "sandbox_exec",
            "arguments": {
                "code": "print('hello from mcp test')",
                "input_data": null
            }
        },
        "id": 2
    });

    let res_call = server
        .post("/mcp/message?sessionId=test-session")
        .add_header("Authorization", &format!("Bearer {}", token))
        .json(&call_req)
        .await;

    // We expect the server to accept the message (it will process in background and push to SSE).
    // So status should be ACCEPTED.
    // If it returns NOT FOUND or BAD REQUEST, our MCP implementation is missing or flawed.
    assert_eq!(res_call.status_code(), axum::http::StatusCode::ACCEPTED);
}
