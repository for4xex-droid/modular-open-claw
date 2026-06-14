/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::common::*;
use axum::http::StatusCode;
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_workflow_routes_auth() {
    let (server, _state, _tmp) = create_test_server().await;

    // 認証なしの場合は UNAUTHORIZED が返ることを検証
    let resp = server
        .post("/api/v1/workflows")
        .json(&serde_json::json!({}))
        .await;
    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let resp = server.get("/api/v1/workflows").await;
    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let resp = server
        .get(&format!("/api/v1/workflows/{}", Uuid::new_v4()))
        .await;
    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);
}

#[serial]
#[tokio::test]
async fn test_workflow_crud_roundtrip() {
    let (server, _state, _tmp) = create_test_server_with_limit(100).await;
    let bearer = test_bearer();

    let workflow_id = Uuid::new_v4().to_string();
    let payload = serde_json::json!({
        "id": workflow_id,
        "name": "Integration Test Workflow",
        "description": "TDD API Integration Test",
        "version": 1,
        "nodes": [
            {
                "id": "start-1",
                "node_type": {
                    "Start": {
                        "trigger": "Manual"
                    }
                },
                "label": "Start Node",
                "config": {},
                "position": { "x": 100.0, "y": 100.0 }
            }
        ],
        "edges": [],
        "variables": {},
        "created_at": "2026-06-13T00:00:00Z",
        "updated_at": "2026-06-13T00:00:00Z"
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 1. ワークフロー作成 (POST)
    let post_resp = server
        .post("/api/v1/workflows")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;
    assert_eq!(post_resp.status_code(), StatusCode::OK);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 2. ワークフロー一覧取得 (GET)
    let get_list_resp = server
        .get("/api/v1/workflows")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(get_list_resp.status_code(), StatusCode::OK);
    let list_json = get_list_resp.json::<serde_json::Value>();
    let list_arr = list_json.as_array().expect("List should be an array");
    assert!(list_arr.iter().any(|w| w["id"] == workflow_id));

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 3. ワークフロー詳細取得 (GET)
    let get_detail_resp = server
        .get(&format!("/api/v1/workflows/{}", workflow_id))
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(get_detail_resp.status_code(), StatusCode::OK);
    let detail_json = get_detail_resp.json::<serde_json::Value>();
    assert_eq!(detail_json["name"], "Integration Test Workflow");

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 4. ワークフロー更新 (PUT)
    let updated_payload = serde_json::json!({
        "id": workflow_id,
        "name": "Updated Workflow Name",
        "description": "Updated description",
        "version": 2,
        "nodes": [
            {
                "id": "start-1",
                "node_type": {
                    "Start": {
                        "trigger": "Manual"
                    }
                },
                "label": "Start Node",
                "config": {},
                "position": { "x": 100.0, "y": 100.0 }
            }
        ],
        "edges": [],
        "variables": {},
        "created_at": "2026-06-13T00:00:00Z",
        "updated_at": "2026-06-13T01:00:00Z"
    });
    let put_resp = server
        .put(&format!("/api/v1/workflows/{}", workflow_id))
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&updated_payload)
        .await;
    assert_eq!(put_resp.status_code(), StatusCode::OK);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 5. 更新後の確認 (GET)
    let get_detail_resp2 = server
        .get(&format!("/api/v1/workflows/{}", workflow_id))
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    let detail_json2 = get_detail_resp2.json::<serde_json::Value>();
    assert_eq!(detail_json2["name"], "Updated Workflow Name");

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 6. ワークフロー削除 (DELETE)
    let delete_resp = server
        .delete(&format!("/api/v1/workflows/{}", workflow_id))
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(delete_resp.status_code(), StatusCode::OK);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 7. 削除後の詳細取得 (GET) -> NotFound が返るべき
    let get_detail_resp3 = server
        .get(&format!("/api/v1/workflows/{}", workflow_id))
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(get_detail_resp3.status_code(), StatusCode::NOT_FOUND);
}

#[serial]
#[tokio::test]
async fn test_workflow_validate_api() {
    let (server, _state, _tmp) = create_test_server_with_limit(100).await;
    let bearer = test_bearer();
    let workflow_id = Uuid::new_v4();

    // 1. 正常なワークフロー定義
    let valid_payload = serde_json::json!({
        "id": workflow_id.to_string(),
        "name": "Validation Test Workflow",
        "description": "Validation testing",
        "version": 1,
        "nodes": [
            {
                "id": "start-1",
                "node_type": {
                    "Start": {
                        "trigger": "Manual"
                    }
                },
                "label": "Start Node",
                "config": {},
                "position": { "x": 100.0, "y": 100.0 }
            }
        ],
        "edges": [],
        "variables": {},
        "created_at": "2026-06-13T00:00:00Z",
        "updated_at": "2026-06-13T00:00:00Z"
    });

    let resp = server
        .post(&format!("/api/v1/workflows/{}/validate", workflow_id))
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&valid_payload)
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    // 2. 不正なワークフロー定義（Startノードがないなど）
    let invalid_payload = serde_json::json!({
        "id": workflow_id.to_string(),
        "name": "Invalid Workflow",
        "description": "No start node",
        "version": 1,
        "nodes": [], // 空のノード
        "edges": [],
        "variables": {},
        "created_at": "2026-06-13T00:00:00Z",
        "updated_at": "2026-06-13T00:00:00Z"
    });

    let resp = server
        .post(&format!("/api/v1/workflows/{}/validate", workflow_id))
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&invalid_payload)
        .await;
    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
}

#[serial]
#[tokio::test]
async fn test_workflow_execute_api() {
    let (server, _state, _tmp) = create_test_server_with_limit(100).await;
    let bearer = test_bearer();
    let workflow_id = Uuid::new_v4();

    // 最初にワークフローを作成
    let payload = serde_json::json!({
        "id": workflow_id.to_string(),
        "name": "Execution Test Workflow",
        "description": "Execution testing",
        "version": 1,
        "nodes": [
            {
                "id": "start-1",
                "node_type": {
                    "Start": {
                        "trigger": "Manual"
                    }
                },
                "label": "Start Node",
                "config": {},
                "position": { "x": 100.0, "y": 100.0 }
            }
        ],
        "edges": [],
        "variables": {},
        "created_at": "2026-06-13T00:00:00Z",
        "updated_at": "2026-06-13T00:00:00Z"
    });

    let resp = server
        .post("/api/v1/workflows")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    // ワークフローの実行をリクエスト
    let resp = server
        .post(&format!("/api/v1/workflows/{}/execute", workflow_id))
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body = resp.json::<serde_json::Value>();
    let execution_id = body["execution_id"]
        .as_str()
        .expect("execution_id should be returned in JSON response");
    assert!(!execution_id.is_empty());

    // 実行履歴が作成されたか検証
    let resp = server
        .get(&format!("/api/v1/workflows/{}/executions", workflow_id))
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let executions = resp.json::<serde_json::Value>();
    assert!(!executions.as_array().unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn test_workflow_fork_api() {
    let (server, _state, _tmp) = create_test_server_with_limit(100).await;
    let bearer = test_bearer(); // test_user
    let bearer_other =
        "Bearer mock_valid_token_other_user:00000000-0000-0000-0000-000000000009".to_string();

    let workflow_id = Uuid::new_v4().to_string();
    let payload = serde_json::json!({
        "id": workflow_id,
        "name": "Original Workflow",
        "description": "Fork test source",
        "version": 1,
        "nodes": [
            {
                "id": "start-1",
                "node_type": {
                    "Start": {
                        "trigger": "Manual"
                    }
                },
                "label": "Start Node",
                "config": {},
                "position": { "x": 100.0, "y": 100.0 }
            }
        ],
        "edges": [],
        "variables": {},
        "created_at": "2026-06-13T00:00:00Z",
        "updated_at": "2026-06-13T00:00:00Z"
    });

    // 1. ワークフロー作成 (POST)
    let resp = server
        .post("/api/v1/workflows")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 2. 他のユーザーが private なワークフローをフォークしようとすると 403 Forbidden
    let fork_resp = server
        .post(&format!("/api/v1/workflows/{}/fork", workflow_id))
        .add_header(axum::http::header::AUTHORIZATION, &bearer_other)
        .await;
    assert_eq!(fork_resp.status_code(), StatusCode::FORBIDDEN);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 3. 所有者自身が private なワークフローをフォークすると 200 OK
    let fork_resp_owner = server
        .post(&format!("/api/v1/workflows/{}/fork", workflow_id))
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(fork_resp_owner.status_code(), StatusCode::OK);

    // レスポンスから新ワークフローのIDを取得
    let fork_json = fork_resp_owner.json::<serde_json::Value>();
    let forked_id = fork_json["id"]
        .as_str()
        .expect("Forked ID should be returned");
    assert_ne!(forked_id, workflow_id);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 4. 新しくフォークされたワークフローの詳細を取得して検証
    let detail_resp = server
        .get(&format!("/api/v1/workflows/{}", forked_id))
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(detail_resp.status_code(), StatusCode::OK);
    let detail_json = detail_resp.json::<serde_json::Value>();
    assert_eq!(detail_json["name"], "Fork of Original Workflow");

    let fork_source = detail_json["fork_source_id"]
        .as_str()
        .expect("fork_source_id should be string");
    assert_eq!(fork_source, workflow_id);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 5. 存在しないワークフローをフォークしようとすると 404 Not Found
    let invalid_fork_resp = server
        .post(&format!("/api/v1/workflows/{}/fork", Uuid::new_v4()))
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(invalid_fork_resp.status_code(), StatusCode::NOT_FOUND);
}
