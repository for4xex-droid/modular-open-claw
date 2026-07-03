/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
//! F-1 Agent Playbooks の API 統合テスト。
//! list / install / import / export の契約（認証・依存欠落 422・
//! 不正マニフェスト 400・部分適用禁止）を固定する。

use super::common::*;
use axum::http::StatusCode;
use serial_test::serial;
use uuid::Uuid;

fn minimal_workflow_json(name: &str) -> serde_json::Value {
    serde_json::json!({
        "id": Uuid::new_v4().to_string(),
        "name": name,
        "description": "playbook integration test workflow",
        "version": 1,
        "nodes": [
            {
                "id": "start-1",
                "node_type": { "Start": { "trigger": "Manual" } },
                "label": "Start Node",
                "config": {},
                "position": { "x": 0.0, "y": 0.0 }
            }
        ],
        "edges": [],
        "variables": {},
        "created_at": "2026-07-03T00:00:00Z",
        "updated_at": "2026-07-03T00:00:00Z"
    })
}

fn valid_manifest_json() -> serde_json::Value {
    serde_json::json!({
        "playbook_version": 1,
        "id": "integration-test-playbook",
        "name": "統合テスト Playbook",
        "description": "test manifest",
        "tags": ["test"],
        "required_skills": [],
        "required_mcp_servers": [],
        "workflows": [minimal_workflow_json("imported-wf")]
    })
}

async fn count_workflows(server: &axum_test::TestServer, bearer: &str) -> usize {
    let resp = server
        .get("/api/v1/workflows")
        .add_header(axum::http::header::AUTHORIZATION, bearer)
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    resp.json::<serde_json::Value>()
        .as_array()
        .expect("workflow list should be an array")
        .len()
}

#[serial]
#[tokio::test]
async fn test_playbook_routes_auth() {
    let (server, _state, _tmp) = create_test_server().await;

    let resp = server.get("/api/v1/playbooks").await;
    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let resp = server
        .post("/api/v1/playbooks/import")
        .json(&serde_json::json!({}))
        .await;
    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let resp = server
        .post("/api/v1/playbooks/seo-operations/install")
        .await;
    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let resp = server
        .get(&format!("/api/v1/workflows/{}/export", Uuid::new_v4()))
        .await;
    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);
}

#[serial]
#[tokio::test]
async fn test_playbook_list_returns_bundled_four() {
    let (server, _state, _tmp) = create_test_server_with_limit(100).await;
    let bearer = test_bearer();

    let resp = server
        .get("/api/v1/playbooks")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let list = resp.json::<serde_json::Value>();
    let arr = list.as_array().expect("playbook list should be an array");
    assert_eq!(arr.len(), 4, "four official playbooks must be bundled");

    let ids: Vec<&str> = arr.iter().filter_map(|p| p["id"].as_str()).collect();
    for expected in [
        "seo-operations",
        "sns-operations",
        "competitor-research",
        "support-triage",
    ] {
        assert!(ids.contains(&expected), "missing playbook {expected}");
    }
}

#[serial]
#[tokio::test]
async fn test_playbook_install_roundtrip() {
    let (server, _state, _tmp) = create_test_server_with_limit(100).await;
    let bearer = test_bearer();

    let resp = server
        .post("/api/v1/playbooks/seo-operations/install")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body = resp.json::<serde_json::Value>();
    assert_eq!(body["playbook_id"], "seo-operations");
    let created = body["created_workflow_ids"]
        .as_array()
        .expect("created_workflow_ids should be an array");
    assert!(!created.is_empty());

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    for id in created {
        let id = id.as_str().expect("workflow id should be a string");
        let detail = server
            .get(&format!("/api/v1/workflows/{}", id))
            .add_header(axum::http::header::AUTHORIZATION, &bearer)
            .await;
        assert_eq!(detail.status_code(), StatusCode::OK);
        let detail_json = detail.json::<serde_json::Value>();
        assert_eq!(detail_json["visibility"], "private");
    }
}

#[serial]
#[tokio::test]
async fn test_playbook_install_unknown_id_returns_404() {
    let (server, _state, _tmp) = create_test_server_with_limit(100).await;
    let bearer = test_bearer();

    let resp = server
        .post("/api/v1/playbooks/no-such-playbook/install")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(resp.status_code(), StatusCode::NOT_FOUND);
}

#[serial]
#[tokio::test]
async fn test_playbook_import_rejects_invalid_manifest() {
    let (server, _state, _tmp) = create_test_server_with_limit(100).await;
    let bearer = test_bearer();

    // パストラバーサルを含む id
    let mut bad_id = valid_manifest_json();
    bad_id["id"] = serde_json::json!("../etc");
    let resp = server
        .post("/api/v1/playbooks/import")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&bad_id)
        .await;
    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 空の workflows
    let mut empty_wf = valid_manifest_json();
    empty_wf["workflows"] = serde_json::json!([]);
    let resp = server
        .post("/api/v1/playbooks/import")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&empty_wf)
        .await;
    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
}

#[serial]
#[tokio::test]
async fn test_playbook_import_rejects_missing_deps() {
    let (server, _state, _tmp) = create_test_server_with_limit(100).await;
    let bearer = test_bearer();

    let before = count_workflows(&server, &bearer).await;

    let mut manifest = valid_manifest_json();
    manifest["required_skills"] = serde_json::json!(["no-such-skill"]);
    let resp = server
        .post("/api/v1/playbooks/import")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&manifest)
        .await;
    assert_eq!(resp.status_code(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = resp.json::<serde_json::Value>();
    let missing = body["missing_skills"]
        .as_array()
        .expect("missing_skills should be an array");
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0], "no-such-skill");

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // サイレント部分適用禁止: workflows 一覧が増えていないこと
    let after = count_workflows(&server, &bearer).await;
    assert_eq!(before, after, "no workflow must be created on 422");
}

#[serial]
#[tokio::test]
async fn test_workflow_export_roundtrip() {
    let (server, _state, _tmp) = create_test_server_with_limit(100).await;
    let bearer = test_bearer();

    // 1. ワークフローを作成
    let wf = minimal_workflow_json("export-source");
    let wf_id = wf["id"].as_str().expect("id").to_string();
    let resp = server
        .post("/api/v1/workflows")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&wf)
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 2. export → マニフェスト v1 が返る
    let resp = server
        .get(&format!("/api/v1/workflows/{}/export", wf_id))
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let manifest = resp.json::<serde_json::Value>();
    assert_eq!(manifest["playbook_version"], 1);
    assert_eq!(
        manifest["workflows"]
            .as_array()
            .expect("workflows array")
            .len(),
        1
    );

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 3. そのまま import できる（import/export の対称性）
    let resp = server
        .post("/api/v1/playbooks/import")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&manifest)
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);
}
