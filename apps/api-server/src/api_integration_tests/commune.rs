/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Version 2.0.
 */

use super::common::*;
use axum::http::StatusCode;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_commune_routes_auth() {
    let (server, _state, _tmp) = create_test_server().await;

    let resp_no_auth = server.get("/api/commune/status").await;
    assert_eq!(resp_no_auth.status_code(), StatusCode::UNAUTHORIZED);

    let resp_auth = server
        .get("/api/commune/status")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;
    assert_eq!(resp_auth.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[serial]
#[tokio::test]
async fn test_commune_send_message_content_length() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let content = "a".repeat(8001);
    let payload = serde_json::json!({
        "recipient_pubkey": "dummy",
        "topic_id": "dummy_topic",
        "content": content
    });

    let resp = server
        .post("/api/commune/send")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
    let json = resp.json::<serde_json::Value>();
    assert!(json["error"].as_str().unwrap().contains("8000 bytes"));
}

#[serial]
#[tokio::test]
async fn test_commune_send_message_binary_data() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let content = "Hello data:image/png;base64,iVBORw0KGgo=";
    let payload = serde_json::json!({
        "recipient_pubkey": "dummy",
        "topic_id": "dummy_topic",
        "content": content
    });

    let resp = server
        .post("/api/commune/send")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
    let json = resp.json::<serde_json::Value>();
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("Binary data embedding"));
}

#[serial]
#[tokio::test]
async fn test_commune_autonomous_clamp() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let payload = serde_json::json!({
        "topic_id": "dummy_topic",
        "peer_pubkey": "dummy_peer",
        "interval_secs": 0, // Too small, should clamp to 10
        "max_rounds": 0 // Too small, should clamp to 1
    });

    let resp = server
        .post("/api/commune/autonomous/start")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);

    let status_resp = server
        .get("/api/commune/autonomous/status")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(status_resp.status_code(), StatusCode::OK);
    let json = status_resp.json::<serde_json::Value>();
    assert_eq!(json["running"].as_bool(), Some(true));
    let config = &json["config"];
    assert_eq!(config["interval_secs"].as_u64(), Some(10));
    assert_eq!(config["max_rounds"].as_u64(), Some(1));
}

#[serial]
#[tokio::test]
async fn test_commune_hub_unreachable_error() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let resp = server
        .get("/api/commune/topics")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    // Fails because the external Hub endpoint is not mocked and fails to connect
    assert_eq!(resp.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[serial]
#[tokio::test]
async fn test_commune_genome_sharing_roundtrip() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let payload = serde_json::json!({
        "genome_json": "{\"genes\": [1.0, 2.0, 3.0]}"
    });

    // 1. Post shared genome
    let share_resp = server
        .post("/api/commune/test_topic/genome")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    assert_eq!(share_resp.status_code(), StatusCode::OK);
    let share_json = share_resp.json::<serde_json::Value>();
    assert_eq!(share_json["status"], "success");

    // 2. Get shared genomes
    let get_resp = server
        .get("/api/commune/test_topic/genomes")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(get_resp.status_code(), StatusCode::OK);
    let genomes = get_resp.json::<serde_json::Value>();
    let genomes_arr = genomes.as_array().unwrap();
    assert_eq!(genomes_arr.len(), 1);
    assert_eq!(
        genomes_arr[0]["blueprint_json"],
        "{\"genes\": [1.0, 2.0, 3.0]}"
    );
}
