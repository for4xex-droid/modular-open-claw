/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 *
 * Licensed under the Version 2.0.
 */

use super::common::*;
use axum::http::StatusCode;
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_biome_routes_auth() {
    let (server, _state, _tmp) = create_test_server().await;

    // 1. 認証なしの場合は UNAUTHORIZED が返ることを確認
    let resp = server
        .post("/api/v1/biome/runs")
        .json(&serde_json::json!({}))
        .await;
    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);

    let resp = server.get("/api/v1/biome/runs").await;
    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);
}

#[serial]
#[tokio::test]
async fn test_biome_runs_roundtrip() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let run_id = Uuid::new_v4().to_string();
    let agent_id = Uuid::new_v4().to_string();

    let payload = serde_json::json!({
        "id": run_id,
        "agent_id": agent_id,
        "generation": 15,
        "score": 89.2,
        "max_generation": 200,
        "cell_count": 256,
        "is_dendou": 0
    });

    // 1. Run 情報を POST
    let post_resp = server
        .post("/api/v1/biome/runs")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;
    assert_eq!(post_resp.status_code(), StatusCode::OK);

    // 2. Run 情報の一覧を GET
    let get_resp = server
        .get("/api/v1/biome/runs")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(get_resp.status_code(), StatusCode::OK);

    let list = get_resp.json::<serde_json::Value>();
    let arr = list.as_array().unwrap();
    assert!(arr.iter().any(|r| r["id"] == run_id));
}

#[serial]
#[tokio::test]
async fn test_biome_specimens_and_analytics() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let run_id = Uuid::new_v4().to_string();
    let agent_id = Uuid::new_v4().to_string();

    // 外部キー制約を満たすためにあらかじめ Run をインサート
    let run_payload = serde_json::json!({
        "id": run_id,
        "agent_id": agent_id,
        "generation": 10,
        "score": 50.0,
        "max_generation": 100,
        "cell_count": 100,
        "is_dendou": 1
    });
    server
        .post("/api/v1/biome/runs")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&run_payload)
        .await;

    // 1. Specimen を POST
    let specimen_id = Uuid::new_v4().to_string();
    let specimen_payload = serde_json::json!({
        "id": specimen_id,
        "run_id": run_id,
        "specimen_name": "TestHelix",
        "genome_data": "{\"sequence\": [4, 5, 6]}",
        "rarity": "common"
    });
    let spec_post_resp = server
        .post("/api/v1/biome/specimens")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&specimen_payload)
        .await;
    assert_eq!(spec_post_resp.status_code(), StatusCode::OK);

    // 2. Specimen を GET
    let spec_get_resp = server
        .get("/api/v1/biome/specimens")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(spec_get_resp.status_code(), StatusCode::OK);
    let specimens = spec_get_resp.json::<serde_json::Value>();
    let spec_arr = specimens.as_array().unwrap();
    assert!(spec_arr.iter().any(|s| s["id"] == specimen_id));

    // 3. Analytics を GET (POSTはシミュレーション中に裏で行われるため、GETのエンドポイントから取得)
    let analytics_resp = server
        .get(&format!("/api/v1/biome/analytics/{}", run_id))
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    // 初期状態はデータ無しでも 200 OK で空配列が返る
    assert_eq!(analytics_resp.status_code(), StatusCode::OK);
}
