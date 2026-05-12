use super::common::*;
use crate::app_state::Component;
use axum::http::StatusCode;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_a2ui_action_integration() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    // Enqueue a job to have something to approve
    state
        .job_queue
        .enqueue("Testing", "A2UI", "standard", None, None, None, 1)
        .await
        .unwrap();
    let jobs = state.job_queue.fetch_recent_jobs(1).await.unwrap();
    let target_id = jobs[0].id.clone();

    // 1. Submit approve action
    let resp = server
        .post("/api/v1/a2ui/action")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&json!({
            "surface_id": "surf_123",
            "action": format!("approve_job:{}", target_id)
        }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);

    // Verify job status changed to pending
    let job = state
        .job_queue
        .fetch_job(&target_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.status, aiome_core_contracts::traits::JobStatus::Pending);

    // 2. Submit cancel action on the same job
    let resp = server
        .post("/api/v1/a2ui/action")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&json!({
            "surface_id": "surf_123",
            "action": format!("cancel_job:{}", target_id)
        }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);

    // Verify job status changed to Cancelled (not Failed)
    let job = state
        .job_queue
        .fetch_job(&target_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        job.status,
        aiome_core_contracts::traits::JobStatus::Cancelled
    );

    // 3. Invalid UUID format → 400
    let resp = server
        .post("/api/v1/a2ui/action")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&json!({
            "surface_id": "surf_456",
            "action": "approve_job:not-a-valid-uuid"
        }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);

    // 4. Invalid action prefix → 400
    let resp = server
        .post("/api/v1/a2ui/action")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&json!({
            "surface_id": "surf_789",
            "action": format!("delete_job:{}", target_id)
        }))
        .await;

    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);

    // 5. Rate Limiting → 429
    // Tests 1-4 above already consumed 4 rate-limit tokens (limit=5 in test state).
    // The auth middleware checks rate limit BEFORE the handler runs, so even
    // requests that return 400 consume tokens. One more valid request should pass,
    // then the next must be rejected with 429.
    let resp = server
        .post("/api/v1/a2ui/action")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&json!({
            "surface_id": "surf_rl",
            "action": format!("approve_job:{}", target_id)
        }))
        .await;
    // 5th request (token #5) — still within limit
    assert!(resp.status_code() != StatusCode::TOO_MANY_REQUESTS);

    // 6th request → must be rate-limited (429)
    let resp_rl = server
        .post("/api/v1/a2ui/action")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&json!({
            "surface_id": "surf_rl",
            "action": format!("approve_job:{}", target_id)
        }))
        .await;

    assert_eq!(resp_rl.status_code(), StatusCode::TOO_MANY_REQUESTS);
}
#[serial]
#[tokio::test]
async fn test_expression_generation_plan_limits() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer =
        "Bearer mock_valid_token_test_user:00000000-0000-0000-0000-000000000002".to_string();
    use aiome_core::expression::Expression;
    use aiome_core_contracts::expression::TtsStatus;
    // Simulate 5 recently generated expressions
    for i in 0..5 {
        let mut expr = Expression::default();
        expr.id = format!("expr_{}", i);
        expr.content = "mock content".into();
        expr.tts_status = TtsStatus::NotRequested;
        expr.created_at = chrono::Utc::now().to_rfc3339();

        state.job_queue.store_expression(&expr).await.unwrap();
    }

    // 2. The generation attempt should be blocked by the Free plan rate limit
    let resp = server
        .post("/api/expression/generate")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(
        resp.status_code(),
        StatusCode::TOO_MANY_REQUESTS,
        "Free plan should be rate limited at 5 expressions per hour"
    );
}
#[serial]
#[tokio::test]
async fn test_ollama_models() {
    let (server, _state, _tmp) = create_test_server().await;

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
#[serial]
#[tokio::test]
async fn test_tts_synthesis() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let payload = json!({
        "text": "Hello from TDD",
        "voice_id": "p225"
    });

    let resp = server
        .post("/api/v1/voice/synthesize")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    // This is expected to FAIL (RED) with 404/405 initially
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body = resp.as_bytes();
    assert!(!body.is_empty());
}
#[serial]
#[tokio::test]
async fn test_lora_training_start() {
    let (server, _state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    let payload = json!({
        "base_model": "mistral-7b",
        "dataset_id": "user-123-chat",
        "params": {
            "epochs": 3,
            "lr": 1e-4
        }
    });

    let resp = server
        .post("/api/v1/lora/train")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .json(&payload)
        .await;

    // This is expected to FAIL (RED) with 404/405 initially
    assert_eq!(resp.status_code(), StatusCode::ACCEPTED);

    let json = resp.json::<serde_json::Value>();
    assert!(json.get("job_id").is_some());
}
#[serial]
#[tokio::test]
async fn test_lora_training_status() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    // Arrange: Insert a dummy job
    let mut job = aiome_core_contracts::traits::Job::default();
    job.id = "mock_lora_job_id".to_string();
    job.category = "LORA_TRAINING".to_string();
    job.status = aiome_core_contracts::traits::JobStatus::InProgress;

    // We must use enqueue to insert it if possible, but the JobQueue trait might not support inserting arbitrary jobs with arbitrary IDs.
    // However, JobQueue has `store_job` or similar in Mock, but let's just use `enqueue` and get the real job_id.
    let job_id = state
        .job_queue
        .enqueue(
            "LORA_TRAINING",
            "mock_lora_job",
            "training",
            None,
            None,
            None,
            0,
        )
        .await
        .expect("Failed to enqueue job");

    // We also need to update its status to something verifiable
    state
        .job_queue
        .update_job_status(&job_id, aiome_core_contracts::traits::JobStatus::Completed)
        .await
        .unwrap();

    let url = format!("/api/v1/lora/status/{}", job_id);
    let resp = server
        .get(&url)
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    // This is expected to FAIL (RED) with 404 Not Found initially
    assert_eq!(resp.status_code(), StatusCode::OK);

    let json = resp.json::<serde_json::Value>();
    assert_eq!(json.get("status").unwrap().as_str().unwrap(), "Completed");
}
#[serial]
#[tokio::test]
async fn test_avatar_upload_ekyc_enforcement() {
    let (server, _state, _tmp) = create_test_server().await;

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
    assert_eq!(
        resp.status_code(),
        StatusCode::FORBIDDEN,
        "Unverified user should be blocked from avatar upload"
    );
}
#[serial]
#[tokio::test]
async fn test_avatar_upload_limit_bypass() {
    let (server, _state, _tmp) = create_test_server().await;

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
    assert_ne!(
        resp.status_code(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "Avatar upload should bypass global 2MB limit up to 50MB"
    );
}
#[serial]
#[tokio::test]
#[serial]
async fn test_voice_drm_roundtrip() {
    let (server, state, tmp) = create_test_server().await;
    let registry = state.registry.clone();
    let token = test_bearer();

    let mut original_audio = b"RIFFAAAAWAVE".to_vec();
    original_audio.extend_from_slice(b"secret_ai_voice_model_data_12345");

    // 1. Upload
    let multipart = axum_test::multipart::MultipartForm::new().add_part(
        "file",
        axum_test::multipart::Part::bytes(original_audio.clone()).file_name("model.aivoice"),
    );

    let response = server
        .post("/api/v1/voice/upload")
        .add_header(axum::http::header::AUTHORIZATION, &token)
        .multipart(multipart)
        .await;

    assert_eq!(response.status_code(), axum::http::StatusCode::OK);
    let json: serde_json::Value = response.json();
    let asset_id_str = json["asset_id"].as_str().unwrap();
    let asset_id = uuid::Uuid::parse_str(asset_id_str).unwrap();

    let agent_id_str = json["creator_id"].as_str().unwrap();
    let agent_id = uuid::Uuid::parse_str(agent_id_str).unwrap();

    let db_path = tmp.path().join("test.db");
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&format!("sqlite:{}", db_path.to_str().unwrap()))
        .await
        .unwrap();

    // 2. Validate registry
    let owns = registry.check_ownership(agent_id, asset_id).await.unwrap();
    assert!(owns, "The uploader must own the uploaded asset");

    // 3. Verify file on disk
    let vault_dir = tmp.path().join(".abyss_vault");
    let file_path = vault_dir.join(format!("{}.aivoice", asset_id));
    assert!(
        file_path.exists(),
        "The encrypted voice asset should exist on disk"
    );

    // 4. Decrypt manually using AbyssVoiceVault
    let encrypted_data = tokio::fs::read(&file_path).await.unwrap();

    // Check if it's actually encrypted (not matching original)
    assert_ne!(encrypted_data, original_audio, "File must be encrypted");

    // Reconstruct vault
    let vault = infrastructure::security::abyss_voice_vault::AbyssVoiceVault::new(
        (*registry).clone(),
        infrastructure::db::DatabasePool::Sqlite(pool),
    );
    vault.restore_keys_from_db().await.unwrap();

    use aiome_core_contracts::voice_vault::VoiceKeyVault;
    let decrypted = vault
        .decrypt_stream(agent_id, asset_id, &encrypted_data)
        .await
        .unwrap();

    // 5. Assert equal
    assert_eq!(
        decrypted, original_audio,
        "Decrypted data must exactly match original uploaded audio"
    );
}
#[serial]
#[tokio::test]
async fn test_synergy_demo_routes_visibility() {
    let (server, _state, _tmp) = create_test_server().await;
    let auth = test_bearer();

    // 1. Synergy Test Routes
    let resp = server
        .post("/api/synergy/test/failure")
        .add_header(axum::http::header::AUTHORIZATION, &auth)
        .await;

    #[cfg(feature = "dev-routes")]
    assert_eq!(
        resp.status_code(),
        axum::http::StatusCode::OK,
        "dev-routes is enabled, should return 200 OK"
    );

    #[cfg(not(debug_assertions))]
    {
        assert_eq!(
            resp.status_code(),
            axum::http::StatusCode::NOT_FOUND,
            "Synergy test routes must be 404 in non-debug builds"
        );
    }

    // 2. Settings Test Connection Route
    let resp_settings = server
        .post("/api/v1/settings/test")
        .add_header(axum::http::header::AUTHORIZATION, &auth)
        .json(&json!({
            "service": "ollama",
            "url": "http://localhost:11434"
        }))
        .await;

    #[cfg(not(debug_assertions))]
    {
        assert_eq!(
            resp_settings.status_code(),
            axum::http::StatusCode::NOT_FOUND,
            "Settings test route must be 404 in non-debug builds"
        );
    }
}
#[serial]
#[tokio::test]
async fn test_voice_asset_list() {
    let (server, _state, _tmp) = create_test_server().await;

    // 1. Fetch public voice models
    let resp = server
        .get("/api/v1/voice/list?scope=public")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);
    let json: serde_json::Value = resp.json();
    assert!(json.as_array().is_some(), "Response should be a JSON array");
}
#[serial]
#[tokio::test]
async fn test_ekyc_session_creation() {
    let (server, _state, _tmp) = create_test_server().await;

    let resp = server
        .post("/api/v1/ekyc/session")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);
    let json: serde_json::Value = resp.json();
    assert!(json.get("session_url").is_some());
    assert!(json.get("session_id").is_some());
}
#[serial]
#[tokio::test]
async fn test_inochi2d_upload() {
    let (server, _state, _tmp) = create_test_server().await;

    // valid INX magic: INX\x02
    let mut payload = b"INX\x02".to_vec();
    payload.extend(vec![0u8; 100]); // dummy data

    let multipart = axum_test::multipart::MultipartForm::new().add_part(
        "file",
        axum_test::multipart::Part::bytes(payload.clone()).file_name("model.inx"),
    );

    let resp = server
        .post("/api/v1/avatar/inochi2d/upload")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .multipart(multipart)
        .await;

    let status = resp.status_code();
    println!("Inochi2D Upload Response: {:?}", resp.text());
    assert_eq!(status, axum::http::StatusCode::OK);
}
#[serial]
#[tokio::test]
async fn test_voice_upload_limit() {
    let (server, _state, _tmp) = create_test_server().await;

    // After relaxation to 500MB, 110MB should be ACCEPTED by the limit layer.
    let large_data = vec![0u8; 110 * 1024 * 1024];

    let resp = server
        .post("/api/v1/voice/upload")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .bytes(large_data.into())
        .await;

    // Expected GREEN: Should NOT be PayloadTooLarge anymore.
    assert_ne!(
        resp.status_code(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "110MB voice upload should be accepted by the new 500MB limit"
    );
}
#[serial]
#[tokio::test]
async fn test_tts_worker_flow_red() {
    use aiome_core::expression::tts_worker::TtsWorker;
    use aiome_core::expression::Expression;
    use aiome_core_contracts::expression::TtsStatus;

    let (server, state, tmp) = create_test_server().await;
    let jq = state.job_queue.clone();

    let artifacts_dir = tmp.path().join("artifacts");

    // 1. Create a pending expression
    let expr = Expression {
        id: "tts-test-1".into(),
        content: "Testing TTS worker flow".into(),
        emotion: "neutral".into(),
        karma_refs: vec![],
        audio_path: None,
        duration_ms: None,
        tts_status: TtsStatus::NotRequested,
        avatar_params: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    JobQueue::store_expression(&**jq, &expr).await.unwrap();

    // 2. Run TtsWorker (Expected to FAIL if XTTS is offline)
    // We use a dummy/invalid endpoint to guarantee RED if not handled or no server.
    let provider = infrastructure::tts::XttsProvider::new("http://invalid.local:18020".to_string());
    let res = TtsWorker::process_pending_tts(jq.as_ref(), &provider, "p225", &artifacts_dir).await;

    // In a real TDD Red, we expect an error or some indication of failure
    // because the server is not there.
    assert!(
        res.is_ok(),
        "Worker should handle connection errors internally and return Ok(processed_count)"
    );
    assert_eq!(
        res.unwrap(),
        0,
        "Processed count should be 0 because connection failed"
    );

    // 3. Verify status in DB (should be Failed or still Generating/NotRequested depending on retry logic)
    let fetched = JobQueue::fetch_expressions(&**jq, 1).await.unwrap();
    assert_eq!(
        fetched[0].tts_status,
        TtsStatus::Failed,
        "Status should be Failed after connection error"
    );
}
#[serial]
#[tokio::test]
async fn test_treasure_get_recommendations() {
    let (server, state, _tmp) = create_test_server().await;
    let agent_id = uuid::Uuid::new_v4();

    // Generate valid token for the test agent
    let token = state
        .auth_manager
        .issue_token(shared::auth::AiomeCustomClaims {
            sub: "test_user".to_string(),
            iss: "aiome-test".to_string(),
            roles: vec![shared::auth::Role::User],
            ekyc_verified: true,
            agent_id,
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            iat: chrono::Utc::now().timestamp() as usize,
        })
        .await
        .unwrap();

    // 1. Get recommendations (Treasure Box)
    let response = server
        .get("/api/v1/treasure")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    // This should fail initially (RED) as the GET route is not yet defined
    assert_eq!(response.status_code(), axum::http::StatusCode::OK);

    let items = response.json::<Vec<serde_json::Value>>();
    assert!(
        !items.is_empty(),
        "Should receive at least one recommendation"
    );
    assert!(
        items[0]["id"].as_str().is_some(),
        "Recommendations should have IDs"
    );

    // 2. Record feedback (AS-1.7: Karma Reward)
    let item_id = items[0]["id"].as_str().unwrap();
    let feedback_req = serde_json::json!({
        "item_id": item_id,
        "action": "click"
    });

    let response = server
        .post("/api/v1/treasure/feedback")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .json(&feedback_req)
        .await;

    assert_eq!(response.status_code(), axum::http::StatusCode::OK);

    // Check if resonance increased (via AgentStats)
    let stats: aiome_core_contracts::types::AgentStats = state
        .job_queue
        .get_agent_stats()
        .await
        .expect("Failed to get initialized stats");
    assert!(
        stats.resonance > 0,
        "Agent resonance should increase after feedback"
    );
}
#[serial]
#[tokio::test]
#[cfg(debug_assertions)]
async fn test_autonomous_demo_lifecycle() {
    let (server, _state, _tmp) = create_test_server().await;

    // Start the demo
    let resp = server
        .post("/api/v1/demo/start")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    // This should pass now (GREEN)
    assert_eq!(resp.status_code(), StatusCode::OK);
    let json = resp.json::<serde_json::Value>();
    assert_eq!(json["success"], true);
    assert!(json["message"]
        .as_str()
        .unwrap()
        .contains("Autonomous demo started"));
}
#[serial]
#[tokio::test]
async fn test_model_status_unauthorized() {
    let (server, _state, _tmp) = create_test_server().await;
    let response = server.get("/api/v1/models/status").await;
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}
#[serial]
#[tokio::test]
async fn test_model_status_authorized() {
    let (server, state, _tmp) = create_test_server().await;
    use aiome_core::traits::SettingsOps;
    let _ = state
        .job_queue
        .get_inner()
        .update_setting("ollama_host", "http://127.0.0.1:11434", "llm", false)
        .await;

    let bearer = test_bearer();

    let response = server
        .get("/api/v1/models/status")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    let status = response.status_code();
    println!("Response Body: {}", response.text());
    assert_eq!(status, StatusCode::OK);
}
#[serial]
#[tokio::test]
async fn test_seo_content_conductor_exists() {
    let (_server, state, _tmp) = create_test_server().await;

    // RED: genericllm_conductor is not yet registered with "seo_content"
    let dispatcher = state.task_dispatcher.get_inner();
    let conductor = dispatcher.get_conductor_for("seo_content");

    assert!(
        conductor.is_some(),
        "seo_content category must be handled by a registered conductor"
    );

    assert_eq!(
        conductor.expect("conductor is missing").conductor_name(),
        "SeoContentConductor",
        "seo_content should be handled by dedicated SeoContentConductor"
    );
}
#[serial]
#[tokio::test]
async fn test_inochi2d_asset_delivery_and_path_traversal() {
    let (server, _state, _tmp) = create_test_server().await;

    // 1. Try to download a valid asset (RED initially as endpoint missing)
    let valid_response = server.get("/api/v1/avatar/inochi2d/valid.inx").await;
    // We expect a 404 if the file isn't there, but currently the route doesn't exist at all so it might be 404 too.
    // Instead we can actually create the file in the mock Sandbox and fetch it.

    // 2. Try Path Traversal (MUST be blocked)
    let traversal_res = server
        .get("/api/v1/avatar/inochi2d/..%2f..%2fetc%2fpasswd")
        .await;
    assert_ne!(
        traversal_res.status_code(),
        reqwest::StatusCode::OK,
        "Path traversal must be rejected"
    );
}
#[serial]
#[tokio::test]
async fn test_whisper_monologue_api() {
    let (server, _state, _tmp) = create_test_server().await;

    // Hit the Whisper API (RED since it doesn't exist)
    let res = server
        .get("/api/v1/whisper/monologue")
        .add_query_param("limit", "10")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;

    assert_eq!(res.status_code(), reqwest::StatusCode::OK);
}
#[serial]
#[tokio::test]
async fn test_heartbeat_dpo_e2e_integration() -> Result<(), Box<dyn std::error::Error>> {
    let (_server, state, _tmp) = create_test_server().await;
    let pool = (*state.db_pool.get_inner()).clone();

    // 1. Prepare score_snapshots data simulating plateau
    let sqlite_pool = pool.get_sqlite_pool().unwrap();
    let _ = sqlx::query("CREATE TABLE IF NOT EXISTS score_snapshots (snapshot_date TEXT NOT NULL, metric_name TEXT NOT NULL, metric_value REAL NOT NULL, PRIMARY KEY (snapshot_date, metric_name))")
        .execute(sqlite_pool).await?;

    for i in 0..15 {
        let date = (chrono::Utc::now() - chrono::Duration::days(15 - i))
            .format("%Y-%m-%d")
            .to_string();
        let val = if i <= 10 {
            (i * 10) as f64
        } else {
            100.0 + ((i - 10) as f64 * 0.1)
        };
        sqlx::query("INSERT OR REPLACE INTO score_snapshots (snapshot_date, metric_name, metric_value) VALUES (?, ?, ?)")
            .bind(date)
            .bind("exp")
            .bind(val)
            .execute(sqlite_pool).await?;
    }

    // 2. Setup Trackers and Engines
    let forecast_provider = std::sync::Arc::new(E2eMockForecastProvider);
    let score_tracker = std::sync::Arc::new(infrastructure::score_tracker::ScoreTracker::new(
        Some(forecast_provider),
        (**state.db_pool.get_inner()).clone(),
    ));

    let mock_lora = std::sync::Arc::new(E2eMockLoraEngine::default());
    let train_called = mock_lora.train_called.clone();

    // Setup dummy HEARTBEAT.md in workspace so it doesn't fail
    let workspace = _tmp.path().to_path_buf();
    std::fs::write(workspace.join("HEARTBEAT.md"), "# Status\nSystem nominal.")?;

    let evolver: std::sync::Arc<dyn aiome_core_contracts::traits::AgentEvolver> =
        state.job_queue.get_inner().clone();

    let wakeup_service = infrastructure::heartbeat_wakeup::HeartbeatWakeupService::new(
        state.provider.get_inner().clone(),
        state.llm_semaphore.get_inner().clone(),
        workspace,
    )
    .with_evolution_tools(score_tracker.clone(), evolver, Some(mock_lora));

    // 3. Act
    let _ = wakeup_service.run_wakeup_ping().await;

    // 4. Assert
    assert!(
        train_called.load(std::sync::atomic::Ordering::SeqCst),
        "LoraEngine::train was not called during plateau in integration test!"
    );

    Ok(())
}

#[serial]
#[tokio::test]
async fn test_expression_generation_with_tts_stream() {
    let (server, state, _tmp) = create_test_server().await;
    let bearer = test_bearer();

    use aiome_core::traits::SettingsOps;
    use aiome_core_contracts::traits::{JobQueue, KarmaRegistry};

    // Setup Tts settings to use mock TTS
    let _ = state
        .job_queue
        .get_inner()
        .update_setting("tts_provider", "openai", "system", false)
        .await;
    let _ = state
        .job_queue
        .get_inner()
        .update_setting("llm_api_key", "dummy", "system", false)
        .await;

    // Create a job first to satisfy foreign key constraint
    state
        .job_queue
        .enqueue("Testing", "TTS Stream", "standard", None, None, None, 1)
        .await
        .unwrap();
    let jobs = state.job_queue.fetch_recent_jobs(1).await.unwrap();
    let job_id = jobs[0].id.clone();

    // Provide karma using correct signature
    state
        .job_queue
        .get_inner()
        .store_karma(
            &job_id,
            "skill-1",
            "test lesson",
            "Technical",
            "hash-1",
            None,
            None,
            None,
            false,
        )
        .await
        .unwrap();

    let resp = server
        .post("/api/expression/generate")
        .add_header(axum::http::header::AUTHORIZATION, &bearer)
        .await;

    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);
    let json: serde_json::Value = resp.json();
    let audio_path = json["audio_path"].as_str();
    assert!(audio_path.is_some(), "Expression should have an audio_path");

    // RED condition: The viseme file should be generated alongside the audio file
    let base_path = std::path::PathBuf::from(audio_path.unwrap());
    let viseme_path = base_path.with_extension("visemes.json");

    assert!(
        viseme_path.exists(),
        "Visemes file must be generated via synthesize_stream"
    );
}
