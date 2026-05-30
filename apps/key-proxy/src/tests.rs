/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::{
    auth::auth_middleware,
    config::{AppState, EmbedResponse, ProxyResponse, QuotaState},
    handlers::llm::{handle_llm_complete, handle_llm_embed, handle_llm_stream},
};
use axum::{
    Router,
    http::StatusCode,
    routing::{get, post},
};
use axum_test::TestServer;
use secrecy::SecretString;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

fn create_test_state() -> AppState {
    AppState {
        gemini_key: Arc::new(SecretString::from("test_key".to_string())),
        vault_secret: Arc::new(SecretString::from("test_vault_secret".to_string())),
        client: aiome_core::http::get_http_client().clone(),
        state: Arc::new(RwLock::new(QuotaState::default())),
        auth_manager: Arc::new(infrastructure::auth::MockAuthManager::new()),
        persistence_path: std::path::PathBuf::from("/tmp/key_proxy_test.json"),
        caller_quotas: {
            let mut q = HashMap::new();
            q.insert("test-caller".into(), 100);
            Arc::new(q)
        },
        wp_api_url: None,
        wp_api_token: None,
        gemini_model: "gemini-2.0-flash".to_string(),
        gemini_embed_model: "text-embedding-004".to_string(),
    }
}

fn build_test_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/llm/complete", post(handle_llm_complete))
        .route("/api/v1/llm/stream", post(handle_llm_stream))
        .route("/api/v1/llm/embed", post(handle_llm_embed))
        .route("/api/v1/health", get(|| async { StatusCode::OK }))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

#[tokio::test]
async fn test_health_check() {
    let state = create_test_state();
    let app = build_test_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/api/v1/health").await;
    // Note: auth_middleware is applied to health too currently
    // Let's verify it requires auth
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_health_check_authorized() {
    let state = create_test_state();
    let app = build_test_router(state.clone());
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/api/v1/health")
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer test_vault_secret",
        )
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn test_llm_complete_unauthorized() {
    let state = create_test_state();
    let app = build_test_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/api/v1/llm/complete")
        .json(&serde_json::json!({"prompt": "hello", "caller_id": "test-caller", "endpoint": "gemini"}))
        .await;
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_llm_embed_unauthorized() {
    let state = create_test_state();
    let app = build_test_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/api/v1/llm/embed")
        .json(&serde_json::json!({"prompt": "hello", "caller_id": "test-caller", "endpoint": "gemini-embed"}))
        .await;
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

// --- Migrated from main.rs (inlined tests) ---

#[test]
fn test_gemini_payload_serialization_without_system_prompt() {
    let payload_prompt = "Hello";
    let payload_system: Option<String> = None;

    let mut gemini_payload = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": payload_prompt
            }]
        }]
    });

    if let Some(s) = payload_system {
        if let Some(obj) = gemini_payload.as_object_mut() {
            obj.insert(
                "system_instruction".to_string(),
                serde_json::json!({ "parts": [{ "text": s }] }),
            );
        }
    }

    assert_eq!(
        gemini_payload.get("system_instruction"),
        None,
        "Should omit system_instruction when system prompt is absent"
    );
}

#[test]
fn test_gemini_passthrough_url_construction() {
    let path = "v1beta/models/gemini-2.0-flash:generateContent".to_string();
    let query_string = Some("key=TEST_DUMMY_KEY".to_string());
    let expected = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key=TEST_DUMMY_KEY";

    let constructed =
        crate::handlers::passthrough::build_gemini_passthrough_url(&path, query_string.as_deref());
    assert_eq!(constructed, expected);
}

#[test]
fn test_gemini_payload_serialization_with_system_prompt() {
    let payload_prompt = "Hello";
    let payload_system: Option<String> = Some("You are a helpful assistant".to_string());

    let mut gemini_payload = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": payload_prompt
            }]
        }]
    });

    if let Some(s) = payload_system {
        if let Some(obj) = gemini_payload.as_object_mut() {
            obj.insert(
                "system_instruction".to_string(),
                serde_json::json!({ "parts": [{ "text": s }] }),
            );
        }
    }

    assert!(
        gemini_payload.get("system_instruction").is_some(),
        "Should include system_instruction when system prompt is present"
    );
}

#[test]
fn test_proxy_response_includes_telemetry() {
    let resp = ProxyResponse {
        content: "test".into(),
        stop_reason: "end_turn".into(),
        total_tokens: Some(42),
        response_time_ms: Some(150),
    };
    assert_eq!(resp.total_tokens, Some(42));
    assert_eq!(resp.response_time_ms, Some(150));
}

#[tokio::test]
async fn test_proxy_embed_response_contains_telemetry() {
    let resp = EmbedResponse {
        embedding: vec![1.0, 2.0],
        response_time_ms: Some(123),
    };
    assert_eq!(resp.response_time_ms, Some(123));
}
