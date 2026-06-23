/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::{
    auth::auth_middleware,
    config::{AppState, EmbedResponse, ProxyResponse, QuotaState},
    handlers::{
        llm::{handle_llm_complete, handle_llm_embed, handle_llm_stream},
        secrets::handle_get_secrets,
        vault_admin::VaultStatusResponse,
    },
};
use axum::{
    Router,
    http::StatusCode,
    routing::{delete, get, post, put},
};
use axum_test::TestServer;
use secrecy::SecretString;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

async fn create_test_state() -> AppState {
    let db_pool = {
        let pool = infrastructure::db::DatabasePool::new_sqlite(":memory:")
            .await
            .unwrap();
        let schema = "CREATE TABLE IF NOT EXISTS vault_secrets (key TEXT PRIMARY KEY, encrypted_value BLOB NOT NULL)";
        infrastructure::sql_exec!(&pool, schema).unwrap();
        pool
    };

    let master_key_bytes = vec![0u8; 32];
    let vault_backend = Arc::new(
        infrastructure::security::sqlite_vault_backend::UniversalVaultBackend::new_with_master_key(
            db_pool,
            master_key_bytes,
        ),
    );

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
        vault_backend,
    }
}

fn build_test_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/llm/complete", post(handle_llm_complete))
        .route("/api/v1/llm/stream", post(handle_llm_stream))
        .route("/api/v1/llm/embed", post(handle_llm_embed))
        .route("/api/v1/secrets", post(handle_get_secrets))
        .route(
            "/api/v1/admin/status",
            get(crate::handlers::vault_admin::handle_vault_status),
        )
        .route(
            "/api/v1/admin/secrets",
            put(crate::handlers::vault_admin::handle_vault_store),
        )
        .route(
            "/api/v1/admin/secrets/:key",
            delete(crate::handlers::vault_admin::handle_vault_delete),
        )
        .route("/api/v1/health", get(|| async { StatusCode::OK }))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

#[tokio::test]
async fn test_health_check() {
    let state = create_test_state().await;
    let app = build_test_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/api/v1/health").await;
    // Note: auth_middleware is applied to health too currently
    // Let's verify it requires auth
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_health_check_authorized() {
    let state = create_test_state().await;
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
    let state = create_test_state().await;
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
    let state = create_test_state().await;
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

#[tokio::test]
async fn test_get_secrets_endpoint_unauthorized() {
    let state = create_test_state().await;
    let app = build_test_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/api/v1/secrets")
        .json(&serde_json::json!({"keys": ["GEMINI_API_KEY"]}))
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_secrets_endpoint_authorized() {
    let state = create_test_state().await;
    state
        .vault_backend
        .store_secret("GEMINI_API_KEY", "super_gemini_secret")
        .await
        .unwrap();

    let app = build_test_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/api/v1/secrets")
        .json(&serde_json::json!({"keys": ["GEMINI_API_KEY"]}))
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer test_vault_secret",
        )
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let json: HashMap<String, String> = response.json();
    assert_eq!(
        json.get("GEMINI_API_KEY"),
        Some(&"super_gemini_secret".to_string())
    );
}

#[tokio::test]
async fn test_get_secrets_endpoint_whitelist_violation() {
    let state = create_test_state().await;
    let app = build_test_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/api/v1/secrets")
        .json(&serde_json::json!({"keys": ["INVALID_KEY_NOT_IN_WHITELIST"]}))
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer test_vault_secret",
        )
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_secrets_endpoint_partial_success() {
    let state = create_test_state().await;
    state
        .vault_backend
        .store_secret("GEMINI_API_KEY", "super_gemini_secret")
        .await
        .unwrap();

    let app = build_test_router(state);
    let server = TestServer::new(app).unwrap();

    // OPENAI_API_KEY is in whitelist but not stored in vault
    let response = server
        .post("/api/v1/secrets")
        .json(&serde_json::json!({"keys": ["GEMINI_API_KEY", "OPENAI_API_KEY"]}))
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer test_vault_secret",
        )
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let json: HashMap<String, String> = response.json();
    assert_eq!(
        json.get("GEMINI_API_KEY"),
        Some(&"super_gemini_secret".to_string())
    );
    assert!(!json.contains_key("OPENAI_API_KEY"));
}

#[tokio::test]
async fn test_vault_status_endpoint() {
    let state = create_test_state().await;
    let app = build_test_router(state.clone());
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/api/v1/admin/status")
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer test_vault_secret",
        )
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let res: VaultStatusResponse = response.json();
    assert_eq!(res.total, 18);
    assert_eq!(res.configured, 0);

    // GEMINI_API_KEY を保存して status が変化するか確認
    state
        .vault_backend
        .store_secret("GEMINI_API_KEY", "test_gemini")
        .await
        .unwrap();

    let response2 = server
        .get("/api/v1/admin/status")
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer test_vault_secret",
        )
        .await;
    assert_eq!(response2.status_code(), StatusCode::OK);
    let res2: VaultStatusResponse = response2.json();
    assert_eq!(res2.configured, 1);
    let gemini_item = res2
        .secrets
        .iter()
        .find(|item| item.key == "GEMINI_API_KEY")
        .unwrap();
    assert!(gemini_item.is_set);
    assert_eq!(gemini_item.category, "ai");
}

#[tokio::test]
async fn test_vault_store_endpoint() {
    let state = create_test_state().await;
    let app = build_test_router(state.clone());
    let server = TestServer::new(app).unwrap();

    let response = server
        .put("/api/v1/admin/secrets")
        .json(&serde_json::json!({
            "key": "GEMINI_API_KEY",
            "value": "new_gemini_key_val"
        }))
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer test_vault_secret",
        )
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    // 実際に保存されたか確認
    let val = state
        .vault_backend
        .get_secret("GEMINI_API_KEY")
        .await
        .unwrap();
    assert_eq!(&*val, "new_gemini_key_val");
}

#[tokio::test]
async fn test_vault_store_invalid_key() {
    let state = create_test_state().await;
    let app = build_test_router(state.clone());
    let server = TestServer::new(app).unwrap();

    let response = server
        .put("/api/v1/admin/secrets")
        .json(&serde_json::json!({
            "key": "INVALID_KEY_NAME_HERE",
            "value": "some_value"
        }))
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer test_vault_secret",
        )
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_vault_delete_endpoint() {
    let state = create_test_state().await;
    state
        .vault_backend
        .store_secret("GEMINI_API_KEY", "val")
        .await
        .unwrap();

    let app = build_test_router(state.clone());
    let server = TestServer::new(app).unwrap();

    let response = server
        .delete("/api/v1/admin/secrets/GEMINI_API_KEY")
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer test_vault_secret",
        )
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    // 実際に削除されたか（取得できないこと）を確認
    let res = state.vault_backend.get_secret("GEMINI_API_KEY").await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_vault_delete_invalid_key() {
    let state = create_test_state().await;
    let app = build_test_router(state.clone());
    let server = TestServer::new(app).unwrap();

    let response = server
        .delete("/api/v1/admin/secrets/INVALID_KEY_NAME_HERE")
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer test_vault_secret",
        )
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_build_auth_manager_with_invalid_base64_in_dev_falls_back_to_mock() {
    let result = crate::build_auth_manager(Some("invalid_base64_invalid_base64".to_string()));
    #[cfg(debug_assertions)]
    assert!(result.is_ok());
    #[cfg(not(debug_assertions))]
    assert!(result.is_err());
}

#[test]
fn test_build_auth_manager_with_placeholder_falls_back_to_mock() {
    let result = crate::build_auth_manager(Some("<YOUR_KEY_HERE>".to_string()));
    #[cfg(debug_assertions)]
    assert!(result.is_ok());
    #[cfg(not(debug_assertions))]
    assert!(result.is_err());
}

#[test]
fn test_build_auth_manager_none_falls_back_to_mock() {
    let result = crate::build_auth_manager(None);
    #[cfg(debug_assertions)]
    assert!(result.is_ok());
    #[cfg(not(debug_assertions))]
    assert!(result.is_err());
}
