use crate::{
    AppState, QuotaState, Router, auth_middleware, get, handle_llm_complete, handle_llm_embed,
    handle_llm_stream, post,
};
use axum::http::StatusCode;
use axum_test::TestServer;
use secrecy::SecretString;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

fn create_test_state() -> AppState {
    AppState {
        gemini_key: Arc::new(SecretString::from("test_key".to_string())),
        vault_secret: Arc::new(SecretString::from("test_vault_secret".to_string())),
        client: reqwest::Client::new(),
        state: Arc::new(RwLock::new(QuotaState::default())),
        auth_manager: Arc::new(infrastructure::auth::MockAuthManager::new()),
        persistence_path: std::path::PathBuf::from("/tmp/key_proxy_test.json"),
        caller_quotas: {
            let mut q = HashMap::new();
            q.insert("test-caller".into(), 100);
            Arc::new(q)
        },
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
        .json(&serde_json::json!({"prompt": "hello"}))
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
        .json(&serde_json::json!({"input": "hello"}))
        .await;
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}
