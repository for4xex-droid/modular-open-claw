use super::common::*;
use crate::app_state::Component;
use axum::http::StatusCode;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_verify_skill_proof_endpoint_connected() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let payload = serde_json::json!({
        "skill_name": "test_skill",
        "proof_spec_b64": "ZHVtbXlfcHJvb2Y=",
        "wasm_hash": "hash123"
    });

    let resp = server
        .post("/api/skills/verify-proof")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::NOT_FOUND,
        "Handler should return 404 when WASM not found"
    );

    let json = resp.json::<serde_json::Value>();
    assert_eq!(json["code"], "NotFound");
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("Skill WASM not found"));
}
#[serial]
#[tokio::test]
async fn test_skill_import_oom_protection() {
    let (server, _state, _tmp) = create_test_server().await;

    // We need to mock a remote server that returns a huge response.
    // Since we use reqwest::Client in AppState, this is hard to mock without a real mock server.
    // However, we can at least test the 1MB limit check logic if we could mock the response.

    // For now, let's just test that a normal import is authorized
    let resp = server
        .post("/api/skills/import")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&json!({"url": "http://example.com/skill.yaml"}))
        .await;

    // It should fail with RemoteServiceError (because example.com/skill.yaml doesn't exist in test env),
    // but it should NOT be UNAUTHORIZED.
    assert_ne!(resp.status_code(), StatusCode::UNAUTHORIZED);
}
#[serial]
#[tokio::test]
async fn test_cortex_wiki_unauthorized() {
    let (server, _state, _tmp) = create_test_server().await;
    let response = server.get("/api/v1/cortex/wiki").await;
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}
#[serial]
#[tokio::test]
async fn test_cortex_wiki_authorized() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    // Insert dummy article directly into DB so we can test the API
    let pool = state.db_pool.get_inner().get_sqlite_pool_or_err().unwrap();
    sqlx::query(
        "INSERT INTO cortex_wiki_articles (id, title, content_md, concepts, backlinks, source_refs, content_hash, version) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind("test-article-id")
    .bind("Test Article")
    .bind("## Content")
    .bind("[\"Test\"]")
    .bind("[]")
    .bind("[\"doc1\"]")
    .bind("hash123")
    .bind(1)
    .execute(pool)
    .await.unwrap();

    let response = server
        .get("/api/v1/cortex/wiki")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let articles: Vec<serde_json::Value> = response.json();
    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0]["id"], "test-article-id");
    assert_eq!(articles[0]["title"], "Test Article");

    let response = server
        .get("/api/v1/cortex/wiki/test-article-id")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let article: serde_json::Value = response.json();
    assert_eq!(article["id"], "test-article-id");
    assert_eq!(article["title"], "Test Article");
    assert_eq!(article["content_md"], "## Content");
}
#[serial]
#[tokio::test]
async fn test_mcp_config_update_unauthorized() {
    let (server, _state, _tmp) = create_test_server().await;
    let payload = serde_json::json!({
        "mcp_servers": {}
    });
    let response = server.put("/api/skills/mcp/config").json(&payload).await;
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}
#[serial]
#[tokio::test]
async fn test_mcp_config_update_authorized_green() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();
    let payload = serde_json::json!({
        "mcp_servers": {}
    });
    let response = server
        .put("/api/skills/mcp/config")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
}
#[serial]
#[tokio::test]
async fn test_cortex_query_file_back() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let payload = serde_json::json!({
        "question": "What is AI?",
        "file_back": true
    });

    let response = server
        .post("/api/v1/cortex/query")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;
    let status = response.status_code();
    println!("Response Body: {}", response.text());
    assert_eq!(status, axum::http::StatusCode::OK);

    let pool = state.db_pool.get_inner().get_sqlite_pool_or_err().unwrap();

    let log_row =
        sqlx::query("SELECT COUNT(*) as count FROM cortex_activity_log WHERE event_type = 'query'")
            .fetch_one(pool)
            .await
            .unwrap();
    use sqlx::Row;
    let log_count: i64 = log_row.get("count");
    assert_eq!(log_count, 1, "Query activity log should be inserted");

    let doc_row =
        sqlx::query("SELECT COUNT(*) as count FROM cortex_documents WHERE source_type = 'query'")
            .fetch_one(pool)
            .await
            .unwrap();
    let doc_count: i64 = doc_row.get("count");
    assert_eq!(
        doc_count, 1,
        "File-back document should be inserted since mock confidence is 0.95"
    );
}
#[serial]
#[tokio::test]
async fn test_mcp_oauth_authorize_flow() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    // 1. Valid provider but NO credentials configured -> 400 Bad Request
    //    (Security: no more insecure 'dummy' fallback)
    let resp = server
        .get("/api/v1/mcp/oauth/authorize?provider=github")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::BAD_REQUEST,
        "Unconfigured provider credentials must be rejected (no dummy fallback)"
    );

    // 2. Invalid provider -> Should return 400 Bad Request (Negative Test)
    let resp_invalid = server
        .get("/api/v1/mcp/oauth/authorize?provider=invalid_provider")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(
        resp_invalid.status_code(),
        axum::http::StatusCode::BAD_REQUEST
    );
}
#[serial]
#[tokio::test]
async fn test_mcp_oauth_callback_flow() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    // 1. Valid provider + code but missing state param -> 400 (CSRF protection)
    let resp = server
        .get("/api/v1/mcp/oauth/callback?provider=github&code=dummy_auth_code")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::BAD_REQUEST,
        "Missing state parameter must be rejected for CSRF protection"
    );

    // 2. Valid provider + code + invalid state -> 400 (CSRF mismatch)
    let resp_bad_state = server
        .get("/api/v1/mcp/oauth/callback?provider=github&code=dummy_auth_code&state=invalid_state")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(
        resp_bad_state.status_code(),
        axum::http::StatusCode::BAD_REQUEST,
        "Invalid state parameter must be rejected as possible CSRF"
    );

    // 3. Missing code -> Should return 400 Bad Request (Negative Test)
    let resp_missing_code = server
        .get("/api/v1/mcp/oauth/callback?provider=github")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(
        resp_missing_code.status_code(),
        axum::http::StatusCode::BAD_REQUEST
    );

    // 4. Invalid provider -> Should return 400 Bad Request (Negative Test)
    let resp_invalid_provider = server
        .get("/api/v1/mcp/oauth/callback?provider=invalid_provider&code=dummy_auth_code")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(
        resp_invalid_provider.status_code(),
        axum::http::StatusCode::BAD_REQUEST
    );
}
#[serial]
#[tokio::test]
async fn test_mcp_oauth_token_exchange_success() {
    let mock_server = wiremock::MockServer::start().await;
    std::env::set_var(
        "TEST_OAUTH_TOKEN_URL_OVERRIDE",
        format!("{}/token", mock_server.uri()),
    );

    std::env::set_var("GITHUB_CLIENT_ID", "test_client_id");
    std::env::set_var("GITHUB_CLIENT_SECRET", "test_secret");

    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    state
        .pkce_cache
        .get_inner()
        .insert("valid_state_123".to_string(), (None, "dummy".to_string()))
        .await;

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "mock_access_token_123",
            "token_type": "bearer",
            "scope": "repo"
        })))
        .mount(&mock_server)
        .await;

    let resp = server
        .get("/api/v1/mcp/oauth/callback?provider=github&code=auth_code_xyz&state=valid_state_123")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::TEMPORARY_REDIRECT
    );

    std::env::remove_var("TEST_OAUTH_TOKEN_URL_OVERRIDE");
    std::env::remove_var("GITHUB_CLIENT_ID");
    std::env::remove_var("GITHUB_CLIENT_SECRET");
}
#[serial]
#[tokio::test]
async fn test_mcp_oauth_token_exchange_failure() {
    let mock_server = wiremock::MockServer::start().await;
    std::env::set_var(
        "TEST_OAUTH_TOKEN_URL_OVERRIDE",
        format!("{}/token", mock_server.uri()),
    );

    std::env::set_var("GITHUB_CLIENT_ID", "test_client_id");
    std::env::set_var("GITHUB_CLIENT_SECRET", "test_secret");

    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    state
        .pkce_cache
        .get_inner()
        .insert("valid_state_123".to_string(), (None, "dummy".to_string()))
        .await;

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": "invalid_client",
            "error_description": "The client authentication failed"
        })))
        .mount(&mock_server)
        .await;

    let resp = server
        .get("/api/v1/mcp/oauth/callback?provider=github&code=auth_code_xyz&state=valid_state_123")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    );

    std::env::remove_var("TEST_OAUTH_TOKEN_URL_OVERRIDE");
    std::env::remove_var("GITHUB_CLIENT_ID");
    std::env::remove_var("GITHUB_CLIENT_SECRET");
}
#[serial]
#[tokio::test]
async fn test_spec_export_endpoint() {
    let (server, _state, _tmp_dir) = create_test_server().await;

    // Use admin token to hit the endpoint
    let response = server
        .get("/api/v1/system/spec-export")
        .add_header(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_static("Bearer mock_valid_token_admin"),
        )
        .await;

    assert_eq!(response.status_code(), 200);
    let json: serde_json::Value = response.json();
    assert_eq!(json["status"], "success");
    assert_eq!(json["export_path"], ".specify-export-tmp");
}
