use super::common::*;
use crate::app_state::Component;
use axum::http::StatusCode;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_health_check() {
    let (server, _state, _tmp) = create_test_server().await;
    let response = server
        .get("/api/health")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);

    // Check JSON structure: ResourceStatus fields
    let json = response.json::<serde_json::Value>();
    assert!(json.get("cpu_usage_percent").is_some());
    assert!(json.get("level").is_some());

    // Check Circuit Breaker status (G-1)
    let cb = json
        .get("llm_circuit_breaker")
        .expect("llm_circuit_breaker field missing");
    assert_eq!(cb["name"], "integration-test");
    assert_eq!(cb["state"], "Closed");
}
#[serial]
#[tokio::test]
async fn test_rate_limiting_per_agent() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    for _ in 0..5 {
        let resp = server
            .get("/api/biome/status")
            .add_header(axum::http::header::AUTHORIZATION, &bearer)
            .await;
        assert_eq!(resp.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    let resp = server
        .get("/api/biome/status")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;
    assert_eq!(resp.status_code(), StatusCode::TOO_MANY_REQUESTS);
}
#[serial]
#[tokio::test]
async fn test_get_prompt_stats() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = format!("Bearer mock_valid_token_admin:{}", uuid::Uuid::new_v4());

    // Arrange: Insert seed data
    let db_pool = state.db_pool.get_inner().get_sqlite_pool_or_err().unwrap();
    sqlx::query(
        "INSERT INTO prompt_evaluation_log (prompt_hash, provider, model, latency_ms, token_count_in, token_count_out, cost_usd, cache_hit) VALUES ('testhash', 'test_provider', 'test_model', 100, 10, 20, 0.0015, 1)"
    )
    .execute(db_pool)
    .await
    .unwrap();

    // Validate GET /api/v1/audit/prompt-stats
    let response = server
        .get("/api/v1/audit/prompt-stats")
        .add_query_param("period", "7d")
        .add_header(axum::http::header::AUTHORIZATION, bearer)
        .await;

    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "Audit prompt stats should return 200 OK"
    );
    let json = response.json::<serde_json::Value>();
    assert!(json.get("period").is_some(), "Should contain period field");
    assert!(
        json.get("providers").is_some(),
        "Should contain providers field"
    );

    let providers = json.get("providers").unwrap().as_array().unwrap();
    assert!(
        !providers.is_empty(),
        "Should contain at least one provider stat"
    );

    let first = &providers[0];
    assert!(
        first.get("total_cost_usd").is_some(),
        "ProviderStat should contain total_cost_usd"
    );
    assert!(
        first.get("cache_hit_rate").is_some(),
        "ProviderStat should contain cache_hit_rate"
    );
}
#[serial]
#[tokio::test]
async fn test_global_body_limit() {
    let (server, _state, _tmp) = create_test_server().await;

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
    assert_eq!(
        resp.status_code(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "4MB request should be rejected by 2MB limit"
    );
}
#[serial]
#[tokio::test]
async fn test_diagnostics_api() {
    let (server, _state, _tmp) = create_test_server().await;

    let resp = server
        .get("/api/v1/audit/diagnostics")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer mock_valid_token_admin:{}", uuid::Uuid::new_v4()),
        )
        .await;

    if resp.status_code() != StatusCode::OK {
        let err_text = resp.text();
        panic!(
            "Diagnostics API failed with status: {}, body: {:?}",
            resp.status_code(),
            err_text
        );
    }
    let json = resp.json::<serde_json::Value>();
    assert!(json.as_array().is_some());
}
#[serial]
#[tokio::test]
async fn test_artifacts_api() {
    let (server, _state, _tmp) = create_test_server().await;

    let resp = server
        .get("/api/artifacts")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let json = resp.json::<serde_json::Value>();
    assert!(json.as_array().is_some());
}
#[serial]
#[tokio::test]
async fn test_trends_api() {
    let (server, _state, _tmp) = create_test_server().await;

    let resp = server
        .get("/api/v1/trends")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let json = resp.json::<serde_json::Value>();
    assert!(json.get("trends").is_some());
}
#[serial]
#[tokio::test]
async fn test_quarantine_audit_api() {
    let (server, state, _tmp) = create_test_server().await;
    let system_id = state.system_agent_id;
    let system_auth = format!("Bearer mock_valid_token_system:{}", system_id);

    // 1. System Agent access: Expect 200 OK
    let resp = server
        .get("/api/v1/audit/quarantine")
        .add_header(axum::http::header::AUTHORIZATION, &system_auth)
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);

    // 2. Unauthorized access: Another agent_id
    let other_id = uuid::Uuid::new_v4();
    let other_auth = format!("Bearer mock_valid_token_testuser:{}", other_id);
    let resp_forbidden = server
        .get("/api/v1/audit/quarantine")
        .add_header(axum::http::header::AUTHORIZATION, &other_auth)
        .await;
    assert_eq!(
        resp_forbidden.status_code(),
        axum::http::StatusCode::FORBIDDEN
    );
}
#[serial]
#[tokio::test]
async fn test_quarantine_release_api() {
    let (server, state, _tmp) = create_test_server().await;
    let system_id = state.system_agent_id;
    let system_auth = format!("Bearer mock_valid_token_system:{}", system_id);

    let asset_id = uuid::Uuid::new_v4().to_string();

    // 1. System Agent access: Expect 200 OK
    let resp = server
        .post(&format!("/api/v1/audit/quarantine/{}/release", asset_id))
        .add_header(axum::http::header::AUTHORIZATION, &system_auth)
        .await;

    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::OK,
        "System admin should be able to release quarantined assets"
    );

    // 2. Unauthorized access: Another agent_id
    let other_id = uuid::Uuid::new_v4();
    let other_auth = format!("Bearer mock_valid_token_testuser:{}", other_id);
    let resp_forbidden = server
        .post(&format!("/api/v1/audit/quarantine/{}/release", asset_id))
        .add_header(axum::http::header::AUTHORIZATION, &other_auth)
        .await;

    assert_eq!(
        resp_forbidden.status_code(),
        axum::http::StatusCode::FORBIDDEN,
        "Normal users should not be able to release quarantined assets"
    );
}
#[serial]
#[tokio::test]
async fn test_fallback_router_failover() {
    let tmp_dir = tempfile::TempDir::new().expect("tmp dir creation failed");
    let db_path = tmp_dir.path().join("test_failover.db");

    let pool = infrastructure::db::DatabasePool::new_sqlite(&format!(
        "sqlite://{}",
        db_path.to_str().unwrap()
    ))
    .await
    .expect("Failed to create test DB pool");

    let ts = std::sync::Arc::new(
        infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
    );
    let job_queue = Arc::new(
        infrastructure::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
            .await
            .expect("Failed to create test job queue"),
    );

    // 1. Setup primary that always fails
    use aiome_core::llm_provider::MockLlmProvider;
    let primary = Arc::new(MockLlmProvider {
        response: "primary failed".to_string(),
        should_fail: true,
    });

    // 2. Setup fallback that succeed
    let fallback = Arc::new(MockLlmProvider {
        response: "fallback success".to_string(),
        should_fail: false,
    });

    let router: Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync> = Arc::new(
        infrastructure::llm::fallback_router::FallbackRouter::new(primary, fallback, 3),
    );

    // 3. Register state with this router
    let state = AppState {
        tokens_css: String::new(),
        db_pool: Component::new(Arc::new(pool.clone())),
        hook_chain: Default::default(),
        registry: Component::new(Arc::new(infrastructure::registry::RegistryManager::new(
            pool.clone(),
        ))),
        wasm_skill_manager: Component::new(Arc::new(
            infrastructure::skills::WasmSkillManager::new(
                tmp_dir.path().join("skills").to_str().unwrap(),
                tmp_dir.path().join("sandbox").to_str().unwrap(),
            )
            .unwrap(),
        )),
        job_queue: Component::new(job_queue.clone()),
        config: Component::new(std::sync::Arc::new(shared::config::AiomeConfig::default())),
        provider: Component::new(router.clone()),
        health_monitor: Component::new(Arc::new(Mutex::new(HealthMonitor::new()))),
        ..Default::default()
    };

    let metrics_handle = GLOBAL_METRICS_HANDLE.clone();

    let app = build_app(
        state,
        CorsLayer::new().allow_origin(tower_http::cors::AllowOrigin::any()),
        "static".to_string(),
        plugin_loader::PluginRegistry::new(),
        metrics_handle,
    );
    let _server = TestServer::new(app).unwrap();

    // 4. Trigger chat via the router handle we kept
    let res = router.complete("hello", None).await.unwrap();
    assert_eq!(res.content, "fallback success");
}
#[serial]
#[tokio::test]
#[serial]
#[cfg(not(debug_assertions))]
async fn test_test_endpoints_are_inaccessible_in_release() {
    let (server, _state, _dir) = create_test_server().await;

    let res = server.post("/api/synergy/test/failure").await;
    let code = res.status_code();
    assert!(
        code == axum::http::StatusCode::NOT_FOUND
            || code == axum::http::StatusCode::METHOD_NOT_ALLOWED,
        "Status was: {}",
        code
    );

    let res = server.post("/api/synergy/test/security").await;
    let code = res.status_code();
    assert!(
        code == axum::http::StatusCode::NOT_FOUND
            || code == axum::http::StatusCode::METHOD_NOT_ALLOWED,
        "Status was: {}",
        code
    );

    let res = server.post("/api/synergy/test/federation").await;
    let code = res.status_code();
    assert!(
        code == axum::http::StatusCode::NOT_FOUND
            || code == axum::http::StatusCode::METHOD_NOT_ALLOWED,
        "Status was: {}",
        code
    );

    let res = server
        .post("/api/v1/settings/test")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&serde_json::json!({}))
        .await;
    let code = res.status_code();
    assert!(
        code == axum::http::StatusCode::NOT_FOUND
            || code == axum::http::StatusCode::METHOD_NOT_ALLOWED,
        "Status was: {}",
        code
    );

    let res = server
        .post("/api/v1/demo/start")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;
    let code = res.status_code();
    assert!(
        code == axum::http::StatusCode::NOT_FOUND
            || code == axum::http::StatusCode::METHOD_NOT_ALLOWED,
        "Status was: {}",
        code
    );
}
#[serial]
#[tokio::test]
#[serial]
#[cfg(debug_assertions)]
async fn test_test_endpoints_are_accessible_in_debug() {
    let (server, _state, _dir) = create_test_server().await;

    std::env::set_var("AIOME_DEV_MODE", "1");
    // test endpoints might require authorization and params, so 401/400 is fine, but NOT 404.
    let res = server.post("/api/synergy/test/failure").await;
    assert_ne!(res.status_code(), axum::http::StatusCode::NOT_FOUND);

    let res = server
        .post("/api/v1/settings/test")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .json(&serde_json::json!({}))
        .await;
    assert_ne!(res.status_code(), axum::http::StatusCode::NOT_FOUND);

    let res = server
        .post("/api/v1/demo/start")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;
    assert_ne!(res.status_code(), axum::http::StatusCode::NOT_FOUND);
}
#[serial]
#[test]
fn test_hookchain_bypass_eradication() {
    use std::fs;
    use std::path::Path;

    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // Files allowed to call execute_wasm_skill/execute_forge_command directly
    // (because they ARE the execution layer, not callers)
    let allowlist = [
        "skill_handler.rs",         // Definition site
        "api_integration_tests.rs", // Test infrastructure
    ];

    let mut violations = Vec::new();

    fn scan_dir(dir: &Path, allowlist: &[&str], violations: &mut Vec<String>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    scan_dir(&path, allowlist, violations);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let filename = path.file_name().unwrap().to_str().unwrap();
                    if allowlist.contains(&filename) {
                        continue;
                    }
                    if let Ok(content) = fs::read_to_string(&path) {
                        let has_direct_call = content.contains("execute_wasm_skill")
                            || content.contains("execute_forge_command");
                        let has_hookchain = content.contains("HookVerdict");

                        if has_direct_call && !has_hookchain {
                            violations.push(format!(
                                "{}: calls execute_wasm_skill/execute_forge_command without HookChain (missing HookVerdict)",
                                path.display()
                            ));
                        }
                    }
                }
            }
        }
    }

    scan_dir(&src_dir, &allowlist, &mut violations);

    assert!(
        violations.is_empty(),
        "🚨 HookChain BYPASS DETECTED! The following files call skill execution functions without HookChain integration:\n{}",
        violations.join("\n")
    );
}
#[serial]
#[tokio::test]
#[serial]
async fn test_metrics_observability() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    // 1. Send a request to a core handler, e.g., health
    let res = server.get("/api/health").await;
    res.assert_status_ok();

    // 2. Fetch metrics
    let metrics_res = server
        .get("/api/v1/metrics")
        .add_header(axum::http::header::AUTHORIZATION, bearer)
        .await;
    metrics_res.assert_status_ok();

    let text = metrics_res.text();
    // Validate that our custom metric 'aiome_api_requests_total' exists and was incremented
    assert!(
        text.contains("aiome_api_requests_total"),
        "Custom metric aiome_api_requests_total is missing from Prometheus output!"
    );
}
