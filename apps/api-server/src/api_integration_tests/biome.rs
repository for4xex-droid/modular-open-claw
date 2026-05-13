/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::common::*;
use crate::app_state::Component;
use axum::http::StatusCode;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_biome_routes_auth() {
    let (server, _state, _tmp) = create_test_server().await;

    let resp_no_auth = server.get("/api/biome/status").await;
    assert_eq!(resp_no_auth.status_code(), StatusCode::UNAUTHORIZED);

    let resp_auth = server
        .get("/api/biome/status")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;
    assert_eq!(resp_auth.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
}
#[serial]
#[tokio::test]
async fn test_biome_send_message_content_length() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let content = "a".repeat(8001);
    let payload = serde_json::json!({
        "recipient_pubkey": "dummy",
        "topic_id": "dummy_topic",
        "content": content
    });

    let resp = server
        .post("/api/biome/send")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
    let json = resp.json::<serde_json::Value>();
    assert!(json["error"].as_str().unwrap().contains("8000 bytes"));
}
#[serial]
#[tokio::test]
async fn test_biome_send_message_binary_data() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let content = "Hello data:image/png;base64,iVBORw0KGgo=";
    let payload = serde_json::json!({
        "recipient_pubkey": "dummy",
        "topic_id": "dummy_topic",
        "content": content
    });

    let resp = server
        .post("/api/biome/send")
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
async fn test_biome_autonomous_clamp() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let payload = serde_json::json!({
        "topic_id": "dummy_topic",
        "peer_pubkey": "dummy_peer",
        "interval_secs": 0, // Too small, should clamp to 10
        "max_rounds": 0 // Too small, should clamp to 1
    });

    let resp = server
        .post("/api/biome/autonomous/start")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);

    let status_resp = server
        .get("/api/biome/autonomous/status")
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
async fn test_biome_hub_unreachable_error() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let resp = server
        .get("/api/biome/topics")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    // Fails because the external Hub endpoint is not mocked and fails to connect
    assert_eq!(resp.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
}
