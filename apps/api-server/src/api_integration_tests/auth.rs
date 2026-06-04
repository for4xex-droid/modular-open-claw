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
async fn test_settings_unauthorized() {
    let (server, _state, _tmp) = create_test_server().await;
    let response = server.get("/api/v1/settings").await;
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}
#[serial]
#[tokio::test]
async fn test_settings_authorized_and_crud() {
    let (server, _state, _tmp) = create_test_server().await;

    // Get initial settings
    let get_resp = server
        .get("/api/v1/settings")
        .add_header(axum::http::header::AUTHORIZATION, test_bearer())
        .await;
    assert_eq!(get_resp.status_code(), StatusCode::OK);
    let settings = get_resp.json::<serde_json::Value>();
    let settings_array = settings.as_array().unwrap();

    // DB migration inserts feature flag automatically.
    // Assert that the array is not empty and contains the flag.
    assert!(!settings_array.is_empty());
    let expected_flag = format!(
        "feature_flag.{}",
        shared::feature_flags::FEDERATION_V1_5_FLAG
    );
    let has_federation_flag = settings_array.iter().any(|s| s["key"] == expected_flag);
    assert!(has_federation_flag);

    // Put a valid setting (ollama_model is allowed)
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
    let settings_array2 = get_resp2.json::<Vec<serde_json::Value>>();

    // Check that the explicitly added setting is present
    let has_ollama_model = settings_array2
        .iter()
        .any(|s| s["key"] == "ollama_model" && s["value"] == "qwen2");
    assert!(has_ollama_model);

    // Ensure the initial federation flag is still present
    let has_federation_flag2 = settings_array2.iter().any(|s| s["key"] == expected_flag);
    assert!(has_federation_flag2);
}
#[serial]
#[tokio::test]
async fn test_settings_ssrf_protection() {
    std::env::set_var("AIOME_DEV_MODE", "1");
    let (server, _state, _tmp) = create_test_server().await;

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
#[serial]
#[tokio::test]
async fn test_oauth2_endpoints_stub() {
    let (server, _state, _tmp) = create_test_server().await;

    // Generate a real PKCE challenge/verifier pair
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use sha2::{Digest, Sha256};
    let verifier = "my_super_secret_verifier_for_testing_purposes";
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    // 1. Authorize: GET with PKCE
    let authorize_url = format!("/api/v1/auth/authorize?client_id=test&response_type=code&code_challenge={}&code_challenge_method=S256", challenge);
    let resp = server.get(&authorize_url).await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);

    let json: serde_json::Value = resp.json();
    let auth_code = json["code"].as_str().unwrap();

    // 2. Token: POST with matching verifier
    let resp = server
        .post("/api/v1/auth/token")
        .json(&json!({"grant_type": "authorization_code", "code": auth_code, "code_verifier": verifier}))
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);
}
#[serial]
#[tokio::test]
async fn test_security_regression_sentinel_block() {
    let (_server, mut state, _tmp) = create_test_server().await;

    // We mock the LLM to return a Sentinel block response.
    #[derive(Debug)]
    struct SentinelLlm;
    #[async_trait::async_trait]
    impl aiome_core::llm_provider::LlmProvider for SentinelLlm {
        async fn complete(
            &self,
            _prompt: &str,
            _sys: Option<&str>,
        ) -> Result<aiome_core_contracts::llm::LlmResponse, aiome_core::error::AiomeError> {
            Ok(aiome_core_contracts::llm::LlmResponse {
                content: r#"{"status": "blocked", "reason": "malicious code execution detected", "violated_pattern": "rm -rf"} "#.into(),

                stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
                ..Default::default()
            })
        }
        async fn test_connection(&self) -> Result<(), aiome_core::error::AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "SentinelLlm"
        }
    }

    // Use the sentinel LLM
    state.provider = Component::new(std::sync::Arc::new(SentinelLlm));

    let reply = r#"malicious_tool { "cmd": "rm -rf /" }"#;
    let mut steps = 0;

    let results =
        crate::tool_call_processor::process_generated_tool_calls(reply, &state, &mut steps, None)
            .await;

    assert_eq!(results.len(), 1);
    let msg = &results[0];
    assert!(
        msg.contains("[SENTINEL BLOCK]") || msg.contains("[GUARDRAIL BLOCK]"),
        "Expected rm -rf to be blocked by sentinel, got: {}",
        msg
    );
}
#[serial]
#[tokio::test]
async fn test_security_regression_path_traversal() {
    let (_server, state, _tmp) = create_test_server().await;

    // Attempt to parse tool calls with path traversal
    let reply = r#"../../etc/passwd { "data": "exploit" }"#;
    let _calls = crate::tool_call_processor::parse_tool_calls(reply);

    // Test that the tool parser actually ignores or fails to parse invalid skill names
    // We expect the parser to drop it, or process_generated_tool_calls to block it

    let mut steps = 0;
    let results =
        crate::tool_call_processor::process_generated_tool_calls(reply, &state, &mut steps, None)
            .await;

    // The parse_tool_calls function safely drops tool names with invalid characters (like `/` or `.`).
    // If it dropped it, results is empty, which means it safely blocked the traversal.
    // If it somehow parsed it, it MUST have blocked it via Sentinel/Guardrail.
    if results.is_empty() {
        // Success condition: the parser refused to parse the exploit
        // Successfully passed Watchtower DR rules
    } else {
        let msg = &results[0];
        assert!(
            msg.contains("Error")
                || msg.contains("not found")
                || msg.contains("Invalid")
                || msg.contains("[SENTINEL BLOCK]")
                || msg.contains("Failed to evaluate")
                || msg.contains("Unknown"),
            "Expected explicit failure for path traversal, got: {}",
            msg
        );
    }
}
#[serial]
#[tokio::test]
async fn test_auth_full_oauth_workflow() {
    let (server, state, _tmp) = create_test_server().await;

    // Generate real PKCE challenge/verifier pair
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use sha2::{Digest, Sha256};
    let verifier = "another_secret_verifier_for_full_oauth_workflow";
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    // 1. Authorize (Request Code)
    // We expect a valid JSON auth code, not a plain string Mock.
    let authorize_res = server
        .get("/api/v1/auth/authorize")
        .add_query_param("client_id", "aiome_test_client")
        .add_query_param("response_type", "code")
        .add_query_param("code_challenge", &challenge)
        .add_query_param("code_challenge_method", "S256")
        .await;

    assert_eq!(authorize_res.status_code(), reqwest::StatusCode::OK);
    let authorize_json: serde_json::Value = authorize_res.json();
    let auth_code = authorize_json["code"]
        .as_str()
        .expect("Must return auth code");

    // 2. Token Exchange
    let token_payload = serde_json::json!({
        "grant_type": "authorization_code",
        "code": auth_code,
        "client_id": "aiome_test_client",
        "code_verifier": verifier
    });

    let token_res = server.post("/api/v1/auth/token").json(&token_payload).await;

    assert_eq!(token_res.status_code(), reqwest::StatusCode::OK);
    let token_json: serde_json::Value = token_res.json();

    assert!(token_json.get("access_token").is_some());
    let access_token = token_json["access_token"].as_str().unwrap();

    // The returned token MUST be signed by AuthManager (which in tests is MockAuthManager)
    assert!(
        access_token.starts_with("eyJ") || access_token.starts_with("mock_valid_token_"),
        "Token must be a valid JWT or mock token"
    );

    // Validate the token via inner AuthManager
    let claim = state
        .auth_manager
        .validate_token(access_token)
        .await
        .expect("Token must be validly signed");
    assert_eq!(claim.roles, vec![shared::auth::Role::Agent]);
}
#[serial]
#[tokio::test]
async fn test_auth_pkce_rejection_workflow() {
    let (server, _state, _tmp) = create_test_server().await;

    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use sha2::{Digest, Sha256};
    let verifier = "correct_secret_verifier_for_rejection_test";
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    // 1. Authorize (Request Code) with PKCE
    let authorize_res = server
        .get("/api/v1/auth/authorize")
        .add_query_param("client_id", "aiome_test_client_rej")
        .add_query_param("response_type", "code")
        .add_query_param("code_challenge", &challenge)
        .add_query_param("code_challenge_method", "S256")
        .await;

    assert_eq!(authorize_res.status_code(), reqwest::StatusCode::OK);
    let authorize_json: serde_json::Value = authorize_res.json();
    let auth_code = authorize_json["code"].as_str().unwrap();

    // 2. Token Exchange with WRONG verifier
    let token_payload = serde_json::json!({
        "grant_type": "authorization_code",
        "code": auth_code,
        "client_id": "aiome_test_client_rej",
        "code_verifier": "wrong_verifier_should_fail"
    });

    let token_res = server.post("/api/v1/auth/token").json(&token_payload).await;

    // This should fail with 400 Bad Request
    assert_eq!(token_res.status_code(), reqwest::StatusCode::BAD_REQUEST);
}
#[serial]
#[tokio::test]
#[serial]
async fn test_aegis_sentinel_integration() {
    let (_server, state, _tmp) = create_test_server().await;

    // 1. Get the incident repo from the DB pool
    let db_pool = state.db_pool.get_inner().as_ref().clone();
    let repo = infrastructure::aegis::incident_repo::IncidentRepository::new(db_pool.clone());

    // 2. Insert dummy incidents to trigger Aegis Alert
    for i in 0..15 {
        let _ = repo
            .insert_incident(
                "failing_test_skill",
                "hash123",
                &format!("payload_{}", i),
                "panic at src/lib.rs:42",
            )
            .await
            .unwrap();
    }

    // 3. Subscribe to CoreEvents to catch the AegisSentinel alert
    let mut rx = state.event_sender.get_inner().subscribe();

    // 4. Manually run the Aegis Sentinel dream logic to simulate background dream
    let dream_state =
        infrastructure::dream_state::DreamState::new(state.provider.get_inner().clone())
            .with_incident_repo(std::sync::Arc::new(repo))
            .with_event_sender(state.event_sender.get_inner().clone());

    let res = dream_state.aegis_sentinel_dream().await.unwrap();
    let result = res.expect("Should return a DreamResult");

    // Check that we got an insight warning about the 15 incidents
    let insight = result.insight.expect("Should have an insight message");
    assert!(insight.contains("Aegis Warning Alert: 15 total incidents"));

    // 5. Verify the event was broadcasted
    let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("Timeout waiting for AegisSentinel event")
        .expect("Failed to receive event");

    match event {
        aiome_core_contracts::events::CoreEvent::AegisSentinel {
            level,
            message,
            total_incidents,
            top_skill,
        } => {
            assert_eq!(level, "Warning");
            assert!(message.contains("Aegis Warning Alert: 15 total incidents"));
            assert_eq!(total_incidents, 15);
            assert_eq!(top_skill, Some("failing_test_skill".to_string()));
        }
        _ => panic!("Expected AegisSentinel event, got another event"),
    }
}

#[serial]
#[tokio::test]
async fn test_auth_password_grant_with_argon2id() {
    let (server, state, _tmp) = create_test_server().await;

    // 1. Manually insert the argon2id hash into DB (like setup/init would)
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(b"db_secure_password", &salt)
        .unwrap()
        .to_string();

    use aiome_core_contracts::SettingsOps;
    state
        .job_queue
        .get_inner()
        .update_setting("admin_password_hash", &password_hash, "auth", true)
        .await
        .unwrap();

    // 2. Test login with correct DB password
    let payload_ok = json!({
        "grant_type": "password",
        "client_secret": "db_secure_password"
    });

    let resp = server.post("/api/v1/auth/token").json(&payload_ok).await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);

    // 3. Test login with wrong password
    let payload_err = json!({
        "grant_type": "password",
        "client_secret": "wrong_password"
    });
    let resp = server.post("/api/v1/auth/token").json(&payload_err).await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::FORBIDDEN);
}
