/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::*;
use axum_test::TestServer;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{AllowOrigin, CorsLayer};

#[derive(Debug)]
struct DummyLlm;
#[async_trait::async_trait]
impl aiome_core::llm_provider::LlmProvider for DummyLlm {
    async fn complete(
        &self,
        _prompt: &str,
        _sys: Option<&str>,
    ) -> Result<aiome_contracts::LlmResponse, aiome_core::error::AiomeError> {
        Ok(aiome_contracts::LlmResponse {
            content: "Dummy Output".to_string(),
            stop_reason: aiome_contracts::StopReason::EndTurn,
        })
    }
    async fn test_connection(&self) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }
    fn name(&self) -> &str {
        "Dummy"
    }

    async fn stream_complete(
        &self,
        _prompt: &str,
        _sys: Option<&str>,
    ) -> Result<
        std::pin::Pin<
            Box<
                dyn tokio_stream::Stream<Item = Result<String, aiome_core::error::AiomeError>>
                    + Send,
            >,
        >,
        aiome_core::error::AiomeError,
    > {
        let s = async_stream::stream! {
            yield Ok("Dummy Stream".to_string());
        };
        Ok(Box::pin(s))
    }
}

async fn create_test_server() -> (TestServer, tempfile::TempDir) {
    let tmp_dir = tempfile::TempDir::new().expect("tmp dir creation failed");
    let db_path = tmp_dir.path().join("test.db");

    let job_queue = Arc::new(
        infrastructure::job_queue::SqliteJobQueue::new(&format!(
            "sqlite://{}",
            db_path.to_str().unwrap()
        ))
        .await
        .expect("Failed to create test job queue"),
    );

    let provider = Arc::new(DummyLlm);

    let skills_dir = tmp_dir.path().join("skills");
    let forge_dir = tmp_dir.path().join("forge");
    let sandbox_dir = tmp_dir.path().join("sandbox");
    let artifacts_dir = tmp_dir.path().join("artifacts");

    std::fs::create_dir_all(&skills_dir).unwrap();
    std::fs::create_dir_all(&forge_dir).unwrap();
    std::fs::create_dir_all(&sandbox_dir).unwrap();
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let wasm_skill_manager = Arc::new(
        infrastructure::skills::WasmSkillManager::new(
            skills_dir.to_str().unwrap(),
            sandbox_dir.to_str().unwrap(),
        )
        .unwrap(),
    );
    let skill_forge = Arc::new(infrastructure::skills::forge::SkillForge::new(
        forge_dir.to_str().unwrap(),
        skills_dir.to_str().unwrap(),
    ));
    let artifact_store = Arc::new(infrastructure::artifact_store::SqliteArtifactStore::new(
        job_queue.get_pool().clone(),
        artifacts_dir,
    ));
    let context_engine = Arc::new(infrastructure::context_engine::ContextEngine::new(
        provider.clone(),
        job_queue.clone(),
        Arc::new(tokio::sync::Semaphore::new(1)),
    ));
    let soul_mutator = Arc::new(infrastructure::soul_mutator::SoulMutator::new(
        provider.clone(),
        tmp_dir.path().join("SOUL.md"),
    ));
    let autonomous_running = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let autonomous_config = Arc::new(tokio::sync::RwLock::new(None));

    let state = AppState {
        health_monitor: Arc::new(Mutex::new(HealthMonitor::new())),
        job_queue: job_queue.clone(),
        wasm_skill_manager,
        skill_forge,
        docs_path: tmp_dir.path().to_str().unwrap().to_string(),
        llm_semaphore: Arc::new(tokio::sync::Semaphore::new(1)),
        forge_semaphore: Arc::new(tokio::sync::Semaphore::new(1)),
        mcp_sessions: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        mcp_manager: Arc::new(mcp::client::McpProcessManager::new()),
        artifact_store,
        event_sender: tokio::sync::broadcast::channel(10).0,
        context_engine,
        soul_mutator,
        soul_store: Arc::new(infrastructure::soul_store::SqliteSoulStore::new(Arc::new(
            job_queue.get_pool().clone(),
        ))),
        autonomous_running,
        autonomous_config,
        provider,
        docker_failures: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        http_client: aiome_core::http::get_http_client().clone(),
        security_policy: shared::security::SecurityPolicy::default(),
        commerce_engine: None,
        circuit_breaker: Arc::new(infrastructure::circuit_breaker::CircuitBreaker::new(
            "integration-test",
            infrastructure::circuit_breaker::CircuitBreakerConfig {
                failure_threshold: 5,
                reset_timeout: std::time::Duration::from_secs(60),
            },
        )),
        slo_engine: Arc::new(infrastructure::slo_engine::SloEngine::new(
            infrastructure::slo_engine::SloConfig {
                error_budget_max: 100,
                warning_threshold: 80,
            },
            chrono::Duration::hours(24),
        )),
        api_server_secret: Arc::new(secrecy::SecretString::from("test_secret".to_string())),
        federation_secret: Some(Arc::new(secrecy::SecretString::from(
            "test_fed_secret".to_string(),
        ))),
        config: Arc::new(shared::config::AiomeConfig::load().unwrap_or_else(|_| {
            shared::config::AiomeConfig {
                db_path: db_path.to_str().unwrap().to_string(),
                log_level: "info".to_string(),
                ollama_host: "".to_string(),
                ollama_model: "".to_string(),
                gemini_api_key: None,
                openai_api_key: None,
                anthropic_api_key: None,
                api_server_port: 0,
                key_proxy_url: "".to_string(),
                samsara_hub_url: "".to_string(),
                allowed_origins: vec![],
                abyss_vault_path: "".to_string(),
                tremendous_api_key: None,
                master_email: None,
            }
        })),
        gift_engine: Arc::new(infrastructure::commerce::gift::TremendousGiftEngine::new(
            "".to_string(),
            true,
        )),
        ekyc_engine: Arc::new(infrastructure::compliance::ekyc::MockEkycEngine),
        quarantine_store: Arc::new(infrastructure::compliance::quarantine::MockQuarantineStore),
        auth_manager: Arc::new(infrastructure::auth::MockAuthManager::new()),
        system_agent_id: uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
    };

    let cors_layer = CorsLayer::new().allow_origin(AllowOrigin::any());

    // In testing, multiple test_servers share the process. Prometheus only allows one global recorder.
    // Hack: We use a static cell to hold the handle created by the first test.
    static METRICS_HANDLE: once_cell::sync::Lazy<metrics_exporter_prometheus::PrometheusHandle> =
        once_cell::sync::Lazy::new(|| {
            metrics_exporter_prometheus::PrometheusBuilder::new()
                .install_recorder()
                .expect("Failed to install global prometheus recorder in tests")
        });

    let plugin_registry = plugin_loader::PluginRegistry::new();
    let metrics_handle = METRICS_HANDLE.clone();
    
    let app = build_app(state, cors_layer, "static", plugin_registry, metrics_handle);

    (TestServer::new(app).unwrap(), tmp_dir)
}

fn test_bearer() -> String {
    // MockAuthManager accepts "mock_valid_token_<sub>"
    "Bearer mock_valid_token_test_user".to_string()
}

#[tokio::test]
async fn test_health_check() {
    let (server, _tmp) = create_test_server().await;
    let response = server
        .get("/api/health")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);

    // Check JSON structure: ResourceStatus fields
    let json = response.json::<serde_json::Value>();
    assert!(json.get("cpu_usage_percent").is_some());
    assert!(json.get("level").is_some());
}

#[tokio::test]
async fn test_settings_unauthorized() {
    let (server, _tmp) = create_test_server().await;
    let response = server.get("/api/v1/settings").await;
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_settings_authorized_and_crud() {
    let (server, _tmp) = create_test_server().await;

    // Get empty settings
    let get_resp = server
        .get("/api/v1/settings")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;
    assert_eq!(get_resp.status_code(), StatusCode::OK);
    let settings = get_resp.json::<serde_json::Value>();
    assert!(settings.as_array().unwrap().is_empty());

    // Put a valid setting (assuming theme is allowed)
    // Wait, the API checks whitelist. theme should be allowed.
    let put_req = json!({
        "key": "ollama_model",
        "value": "qwen2",
        "category": "llm"
    });

    let put_resp = server
        .put("/api/v1/settings")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&put_req)
        .await;

    assert_eq!(put_resp.status_code(), StatusCode::OK);

    // Check if it got saved
    let get_resp2 = server
        .get("/api/v1/settings")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;
    let settings_array = get_resp2.json::<Vec<serde_json::Value>>();
    assert_eq!(settings_array.len(), 1);
    assert_eq!(settings_array[0]["key"], "ollama_model");
    assert_eq!(settings_array[0]["value"], "qwen2");
}

#[tokio::test]
async fn test_settings_ssrf_protection() {
    let (server, _tmp) = create_test_server().await;

    let payload = json!({
        "service": "ollama",
        "url": "http://169.254.169.254",
        "model": "malicious"
    });

    let resp = server
        .post("/api/v1/settings/test")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&payload)
        .await;

    // Should block SSRF attempt with success: false and message containing "SSRF Blocked"
    assert_eq!(resp.status_code(), StatusCode::OK);
    let json = resp.json::<serde_json::Value>();
    assert_eq!(json["success"], false);
    assert!(json["message"].as_str().unwrap().contains("SSRF Blocked"));
}

#[tokio::test]
async fn test_biome_routes_auth() {
    let (server, _tmp) = create_test_server().await;

    let resp_no_auth = server.get("/api/biome/status").await;
    assert_eq!(resp_no_auth.status_code(), StatusCode::UNAUTHORIZED);

    let resp_auth = server
        .get("/api/biome/status")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;
    assert_eq!(resp_auth.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn test_ollama_models() {
    let (server, _tmp) = create_test_server().await;

    // Test hitting the ollama models endpoint
    let resp = server
        .get("/api/v1/ollama/models")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    // Without a real ollama server running to mock, it will fail to connect and return 500 or 502/503 depending on impl.
    // We just verify it's responsive and authorized, not hanging.
    assert!(
        resp.status_code() == StatusCode::SERVICE_UNAVAILABLE
            || resp.status_code() == StatusCode::INTERNAL_SERVER_ERROR
            || resp.status_code() == StatusCode::OK
    );
}

#[tokio::test]
async fn test_avatar_upload_ekyc_enforcement() {
    let (server, _tmp) = create_test_server().await;

    // mock_valid_token_unverified does NOT start with 'ekyc_'
    let bearer = "Bearer mock_valid_token_unverified_user".to_string();

    let payload = json!({
        "name": "test_avatar",
        "content_base64": base64::engine::general_purpose::STANDARD.encode(vec![0u8; 100]),
        "head_height": 1.0,
        "total_height": 6.0
    });

    let resp = server
        .post("/api/avatar/upload")
        .add_header(axum::http::header::AUTHORIZATION, bearer)
        .json(&payload)
        .await;

    // Currently this returns OK because it only warns.
    // We WANT it to return FORBIDDEN or UNAUTHORIZED.
    assert_eq!(resp.status_code(), StatusCode::FORBIDDEN, "Unverified user should be blocked from avatar upload");
}

#[tokio::test]
async fn test_skill_import_oom_protection() {
    let (server, _tmp) = create_test_server().await;

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
#[tokio::test]
async fn test_global_body_limit() {
    let (server, _tmp) = create_test_server().await;

    // Send a 4MB JSON payload (limit should be 2MB)
    let large_string = "a".repeat(4 * 1024 * 1024);
    let payload = json!({
        "key": "theme",
        "value": large_string,
        "category": "ui"
    });

    let resp = server
        .post("/api/v1/settings")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&payload)
        .await;

    // This should fail (RED) because current limit is 512MB
    assert_eq!(resp.status_code(), StatusCode::PAYLOAD_TOO_LARGE, "4MB request should be rejected by 2MB limit");
}

#[tokio::test]
async fn test_avatar_upload_limit_bypass() {
    let (server, _tmp) = create_test_server().await;

    // Send a 10MB payload to /api/avatar/upload (permitted by 50MB layer)
    let large_base64 = "a".repeat(10 * 1024 * 1024);
    let payload = json!({
        "name": "large_avatar",
        "content_base64": large_base64,
        "head_height": 1.0,
        "total_height": 6.0
    });

    let bearer = "Bearer mock_valid_token_ekyc_verified_user".to_string();

    let resp = server
        .post("/api/avatar/upload")
        .add_header(axum::http::header::AUTHORIZATION, bearer)
        .json(&payload)
        .await;

    // Should NOT be 413. Should be 200 or 400.
    assert_ne!(resp.status_code(), StatusCode::PAYLOAD_TOO_LARGE, "Avatar upload should bypass global 2MB limit up to 50MB");
}

#[tokio::test]
async fn test_diagnostics_api() {
    let (server, _tmp) = create_test_server().await;

    let resp = server
        .get("/api/v1/audit/diagnostics")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let json = resp.json::<serde_json::Value>();
    assert!(json.as_array().is_some());
}

#[tokio::test]
async fn test_artifacts_api() {
    let (server, _tmp) = create_test_server().await;

    let resp = server
        .get("/api/artifacts")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let json = resp.json::<serde_json::Value>();
    assert!(json.as_array().is_some());
}

#[tokio::test]
async fn test_trends_api() {
    let (server, _tmp) = create_test_server().await;

    let resp = server
        .get("/api/v1/trends")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let json = resp.json::<serde_json::Value>();
    assert!(json.get("trends").is_some());
}
