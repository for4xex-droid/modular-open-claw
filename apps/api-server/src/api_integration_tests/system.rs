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

    // Check Support Incidents stats (S-5)
    let si = json
        .get("support_incidents")
        .expect("support_incidents field missing");
    assert!(si.get("total_incidents_7d").is_some());
    assert!(si.get("distinct_users").is_some());
    assert!(si.get("unresolved").is_some());
}
#[serial]
#[tokio::test]
async fn test_rate_limiting_per_agent() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    for _ in 0..5 {
        let resp = server
            .get("/api/commune/status")
            .add_header(axum::http::header::AUTHORIZATION, &bearer)
            .await;
        assert_eq!(resp.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    let resp = server
        .get("/api/commune/status")
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
async fn test_diagnostics_summary_api() {
    let (server, state, _tmp) = create_test_server().await;

    // Arrange: Insert some test diagnoses with categories
    let db_pool = state.db_pool.get_inner().get_sqlite_pool_or_err().unwrap();

    // Insert parent jobs first to satisfy FK constraint
    let insert_job_sql = "INSERT INTO jobs (id, category, topic, style_name, karma_directives, status) VALUES (?, 'test_cat', 'test_topic', 'test_style', '[]', 'Failed')";
    sqlx::query(insert_job_sql)
        .bind("job1")
        .execute(db_pool)
        .await
        .unwrap();
    sqlx::query(insert_job_sql)
        .bind("job2")
        .execute(db_pool)
        .await
        .unwrap();
    sqlx::query(insert_job_sql)
        .bind("job3")
        .execute(db_pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO agent_diagnoses (job_id, critical_failure_step, failure_category, root_cause, evidence, self_repair_hint, diagnosed_at) 
         VALUES (?, 1, 'PlanAdherenceFailure', 'r1', 'e1', 'h1', '2026-05-13T00:00:00Z')"
    ).bind("job1").execute(db_pool).await.unwrap();

    sqlx::query(
        "INSERT INTO agent_diagnoses (job_id, critical_failure_step, failure_category, root_cause, evidence, self_repair_hint, diagnosed_at) 
         VALUES (?, 2, 'PlanAdherenceFailure', 'r2', 'e2', 'h2', '2026-05-13T00:00:00Z')"
    ).bind("job2").execute(db_pool).await.unwrap();

    sqlx::query(
        "INSERT INTO agent_diagnoses (job_id, critical_failure_step, failure_category, root_cause, evidence, self_repair_hint, diagnosed_at) 
         VALUES (?, 3, 'SystemFailure', 'r3', 'e3', 'h3', '2026-05-13T00:00:00Z')"
    ).bind("job3").execute(db_pool).await.unwrap();

    let resp = server
        .get("/api/v1/audit/diagnostics/summary")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer mock_valid_token_admin:{}", uuid::Uuid::new_v4()),
        )
        .await;

    if resp.status_code() != StatusCode::OK {
        let err_text = resp.text();
        panic!(
            "Diagnostics summary API failed with status: {}, body: {:?}",
            resp.status_code(),
            err_text
        );
    }
    let json = resp.json::<serde_json::Value>();
    assert_eq!(json.get("total_diagnoses").unwrap().as_i64().unwrap(), 3);

    let categories = json.get("categories").unwrap().as_array().unwrap();
    assert_eq!(categories.len(), 2);

    let mut plan_count = 0;
    let mut sys_count = 0;
    for cat in categories {
        let name = cat.get("failure_category").unwrap().as_str().unwrap();
        let count = cat.get("count").unwrap().as_i64().unwrap();
        if name == "PlanAdherenceFailure" {
            plan_count = count;
        }
        if name == "SystemFailure" {
            sys_count = count;
        }
    }
    assert_eq!(plan_count, 2);
    assert_eq!(sys_count, 1);
}

#[serial]
#[tokio::test]
async fn test_diagnostics_summary_api_forbidden() {
    let (server, _state, _tmp) = create_test_server().await;

    // Non-admin user should be rejected
    let other_id = uuid::Uuid::new_v4();
    let other_auth = format!("Bearer mock_valid_token_testuser:{}", other_id);
    let resp = server
        .get("/api/v1/audit/diagnostics/summary")
        .add_header(axum::http::header::AUTHORIZATION, &other_auth)
        .await;
    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::FORBIDDEN,
        "Non-admin users should not access diagnostics summary"
    );
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
        fast_provider: Component::new(router.clone()),
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

#[tokio::test]
async fn test_cortex_god_nodes() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let res = server
        .get("/api/v1/cortex/god-nodes?limit=5")
        .add_header(axum::http::header::AUTHORIZATION, bearer)
        .await;

    res.assert_status_ok();

    let json = res.json::<serde_json::Value>();
    assert!(json.is_array(), "Expected God Nodes to be an array");
}

#[serial]
#[tokio::test]
async fn test_alert_manager_di_and_alerting() {
    let (_server, state, _tmp) = create_test_server().await;

    // 1. AppState に alert_manager フィールドが存在し、アクセス可能であることを検証
    // （この時点では AppState にフィールドが存在しないため、コンパイルエラー（RED）となります）
    let alert_manager = state.alert_manager.get_inner().clone();

    // 2. AlertManager の基本的なアラート発火が機能するか検証
    // テスト用の Notifier を登録し、アラートが通知されることを検証
    #[derive(Debug)]
    struct MockNotifier {
        received: std::sync::Arc<tokio::sync::Mutex<Vec<(String, String)>>>,
    }

    #[async_trait::async_trait]
    impl infrastructure::alerts::AlertNotifier for MockNotifier {
        async fn send_alert(
            &self,
            title: &str,
            message: &str,
            _level: infrastructure::alerts::AlertLevel,
        ) -> Result<(), aiome_core::error::AiomeError> {
            self.received
                .lock()
                .await
                .push((title.to_string(), message.to_string()));
            Ok(())
        }
    }

    let received = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let mock_notifier = std::sync::Arc::new(MockNotifier {
        received: received.clone(),
    });

    alert_manager.register_notifier(mock_notifier).await;
    let _ = alert_manager
        .trigger_alert(
            "Test Title",
            "Test Message",
            infrastructure::alerts::AlertLevel::Critical,
        )
        .await;

    // Wait for async spawn to execute the notifier
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let results = received.lock().await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "Test Title");
    assert_eq!(results[0].1, "Test Message");
}

#[serial]
#[tokio::test]
async fn test_fast_provider_fallback_policy_behavior() {
    let tmp_dir = tempfile::TempDir::new().expect("tmp dir creation failed");
    let db_path = tmp_dir.path().join("test_llm_policy.db");
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

    let db = crate::bootstrap::DatabaseResult {
        db_pool: pool.clone(),
        job_queue,
        eval_logger: Arc::new(
            infrastructure::llm::evaluation_logger::EvaluationLogger::new(Arc::new(
                infrastructure::llm::evaluation_logger::SqlEvalLogRepository::new(pool.clone()),
            )),
        ),
        audit_logger: Arc::new(infrastructure::audit_logger::AsyncAuditLogger::new(
            pool.clone().into(),
            10,
        )),
        system_agent_id: uuid::Uuid::new_v4(),
        circuit_breaker: Arc::new(infrastructure::circuit_breaker::CircuitBreaker::new(
            "test-cb",
            infrastructure::circuit_breaker::CircuitBreakerConfig {
                failure_threshold: 5,
                reset_timeout: std::time::Duration::from_secs(60),
            },
        )),
        rate_limiter: infrastructure::rate_limiter::AgentRateLimiter::new(10).unwrap(),
        slo_engine: Arc::new(infrastructure::slo_engine::SloEngine::new(
            infrastructure::slo_engine::SloConfig {
                error_budget_max: 100,
                warning_threshold: 80,
            },
            chrono::Duration::hours(24),
        )),
        http_client: reqwest::Client::new(),
        sandbox: Arc::new({
            let sandbox_dir = tmp_dir.path().join("sandbox");
            std::fs::create_dir_all(&sandbox_dir).expect("failed to create sandbox dir");
            shared::sandbox::PathSandbox::new(sandbox_dir).expect("sandbox creation failed")
        }),
        hook_manager: Arc::new(infrastructure::security::hook_manager::HookManager::new()),
        alert_manager: Arc::new(infrastructure::alerts::AlertManager::new()),
    };

    // Case 1: local_fallback_policy == "local_only"
    let mut config = shared::config::AiomeConfig::default();
    config.local_fallback_policy = shared::config::LocalFallbackPolicy::LocalOnly;
    config.ollama_host = "http://localhost:9999".to_string();
    let config_arc = Arc::new(config);

    let providers = crate::bootstrap::init_llm_providers(&config_arc, &db, None)
        .await
        .expect("Failed to init llm providers");

    assert_eq!(providers.fast_provider.name(), "BackgroundLlm");

    // Case 2: local_fallback_policy == "auto_switch"
    let mut config = shared::config::AiomeConfig::default();
    config.local_fallback_policy = shared::config::LocalFallbackPolicy::AutoSwitch;
    config.ollama_host = "http://localhost:9999".to_string();
    let config_arc = Arc::new(config);

    let providers = crate::bootstrap::init_llm_providers(&config_arc, &db, None)
        .await
        .expect("Failed to init llm providers");

    assert_eq!(providers.fast_provider.name(), "FallbackRouter");
}
