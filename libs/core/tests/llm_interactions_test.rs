/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use aiome_core::llm_provider::interactions::InteractionsGeminiProvider;
use aiome_core_contracts::llm::{LlmMessage, LlmProvider, LlmRequest};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[allow(clippy::unwrap_used)]
#[tokio::test]
async fn test_interactions_complete_with_cache() {
    // Arrange
    let mock_server = MockServer::start().await;

    let interaction_id = "v1_abc123";
    let mock_response = json!({
        "id": interaction_id,
        "model": "gemini-3-flash-preview",
        "status": "completed",
        "object": "interaction",
        "role": "model",
        "outputs": [
            {
                "type": "text",
                "text": "Hello, I am Gemini!"
            }
        ],
        "usage": {
            "total_tokens": 125
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1beta/interactions"))
        .and(wiremock::matchers::body_partial_json(
            json!({"input": "Hello!"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .expect(1) // allow-anti-pattern
        .mount(&mock_server)
        .await;

    let provider = InteractionsGeminiProvider::with_base_url(
        aiome_core::http::get_http_client().clone(),
        secrecy::SecretString::from("fake_key".to_string()),
        "gemini-3-flash-preview".to_string(),
        mock_server.uri(),
    );

    let request = LlmRequest {
        messages: vec![LlmMessage {
            role: "user".to_string(),
            content: "Hello!".to_string(),
            cache: false,
        }],
        ..Default::default()
    };

    // Act
    let response = provider
        .complete_with_cache(request)
        .await
        .expect("Request failed"); // allow-anti-pattern

    // Assert 1
    assert_eq!(response.content, "Hello, I am Gemini!");
    let id1 = response
        .metadata
        .as_ref()
        .unwrap() // allow-anti-pattern
        .get("interaction_id")
        .unwrap() // allow-anti-pattern
        .clone();
    assert_eq!(id1, interaction_id);

    // Turn 2
    let interaction_id2 = "v1_def456";
    let mock_response2 = json!({
        "id": interaction_id2,
        "model": "gemini-3-flash-preview",
        "status": "completed",
        "object": "interaction",
        "role": "model",
        "outputs": [
            {
                "type": "text",
                "text": "Your name is Phil."
            }
        ]
    });

    Mock::given(method("POST"))
        .and(path("/v1beta/interactions"))
        // body should contain previous_interaction_id
        .and(wiremock::matchers::body_partial_json(
            json!({"previous_interaction_id": id1}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response2))
        .mount(&mock_server)
        .await;

    let mut metadata2 = std::collections::HashMap::new();
    metadata2.insert("previous_interaction_id".to_string(), id1);

    let request2 = LlmRequest {
        messages: vec![LlmMessage {
            role: "user".to_string(),
            content: "What is my name?".to_string(),
            cache: false,
        }],
        metadata: Some(metadata2),
        ..Default::default()
    };

    // Act 2
    let response2 = provider
        .complete_with_cache(request2)
        .await
        .expect("Request 2 failed"); // allow-anti-pattern

    // Assert 2
    assert_eq!(response2.content, "Your name is Phil.");
    let id2 = response2
        .metadata
        .as_ref()
        .unwrap() // allow-anti-pattern
        .get("interaction_id")
        .unwrap(); // allow-anti-pattern
    assert_eq!(id2, interaction_id2);
}

#[allow(clippy::unwrap_used)]
#[tokio::test]
async fn test_interactions_failover() {
    // Arrange
    let mock_server = MockServer::start().await;

    // Gemini API consistently returns 500
    Mock::given(method("POST"))
        .and(path("/v1beta/interactions"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    // Fallback provider
    let fallback = std::sync::Arc::new(aiome_core::llm_provider::MockLlmProvider {
        response: "Fallback response".to_string(),
        ..Default::default()
    });

    let provider = InteractionsGeminiProvider::with_base_url(
        aiome_core::http::get_http_client().clone(),
        secrecy::SecretString::from("fake_key".to_string()),
        "gemini-3-flash-preview".to_string(),
        mock_server.uri(),
    )
    .with_fallback(fallback);

    let request = LlmRequest {
        messages: vec![LlmMessage {
            role: "user".to_string(),
            content: "Hello!".to_string(),
            cache: false,
        }],
        ..Default::default()
    };

    // Act
    let response = provider
        .complete_with_cache(request)
        .await
        .expect("Request should succeed via fallback"); // allow-anti-pattern

    // Assert
    assert_eq!(response.content, "Fallback response");
}
