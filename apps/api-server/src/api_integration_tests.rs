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
    };

    let cors_layer = CorsLayer::new().allow_origin(AllowOrigin::any());
    let plugin_registry = plugin_loader::PluginRegistry::new();
    let app = build_app(state, cors_layer, "static", plugin_registry);

    (TestServer::new(app).unwrap(), tmp_dir)
}

#[tokio::test]
async fn test_health_check() {
    let (server, _tmp) = create_test_server().await;
    let response = server
        .get("/api/health")
        .add_header(axum::http::header::AUTHORIZATION, "Bearer test_secret")
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
        .add_header(axum::http::header::AUTHORIZATION, "Bearer test_secret")
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
        .add_header(axum::http::header::AUTHORIZATION, "Bearer test_secret")
        .json(&put_req)
        .await;

    assert_eq!(put_resp.status_code(), StatusCode::OK);

    // Check if it got saved
    let get_resp2 = server
        .get("/api/v1/settings")
        .add_header(axum::http::header::AUTHORIZATION, "Bearer test_secret")
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
        .add_header(axum::http::header::AUTHORIZATION, "Bearer test_secret")
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
        .add_header(axum::http::header::AUTHORIZATION, "Bearer test_secret")
        .await;
    assert_eq!(resp_auth.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn test_ollama_models() {
    let (server, _tmp) = create_test_server().await;

    // Test hitting the ollama models endpoint
    let resp = server
        .get("/api/v1/ollama/models")
        .add_header(axum::http::header::AUTHORIZATION, "Bearer test_secret")
        .await;

    // Without a real ollama server running to mock, it will fail to connect and return 500 or 502/503 depending on impl.
    // We just verify it's responsive and authorized, not hanging.
    assert!(
        resp.status_code() == StatusCode::SERVICE_UNAVAILABLE
            || resp.status_code() == StatusCode::INTERNAL_SERVER_ERROR
            || resp.status_code() == StatusCode::OK
    );
}
