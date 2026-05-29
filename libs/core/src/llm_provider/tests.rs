/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::error::AiomeError;
    use crate::llm_provider::*;
    use aiome_core_contracts::llm::{LlmMessage, LlmProvider, LlmRequest};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_provider_initialization_and_names() {
        let client = crate::http::get_http_client().clone();

        let ollama =
            OllamaProvider::new("http://localhost:11434".to_string(), "llama3".to_string());
        assert_eq!(LlmProvider::name(&ollama), "Ollama");

        let gemini = GeminiProvider::new(
            client.clone(),
            secrecy::SecretString::from("key".to_string()),
            "gemini".to_string(),
        );
        assert_eq!(LlmProvider::name(&gemini), "Gemini");

        let openai = OpenAiProvider::new(
            client.clone(),
            secrecy::SecretString::from("key".to_string()),
            "gpt-4".to_string(),
        );
        assert_eq!(openai.name(), "OpenAI");

        let claude = ClaudeProvider::new(
            client.clone(),
            secrecy::SecretString::from("key".to_string()),
            "claude".to_string(),
        );
        assert_eq!(claude.name(), "Claude");

        let lmstudio = LmStudioProvider::new(
            client.clone(),
            "http://localhost:1234".to_string(),
            "local".to_string(),
        );
        assert_eq!(lmstudio.name(), "LMStudio");
    }

    #[tokio::test]
    async fn test_lmstudio_complete_success() {
        let mock_server = MockServer::start().await;
        let mock_response = serde_json::json!({
            "choices": [{
                "message": {
                    "content": "Hello from mock LM Studio"
                }
            }]
        });

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        let client = crate::http::get_http_client().clone();
        let provider = LmStudioProvider::new(client, mock_server.uri(), "test-model".to_string());

        let result = provider.complete("Say hello", Some("System prompt")).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "Hello from mock LM Studio");
    }

    #[tokio::test]
    async fn test_ollama_complete_json_format_and_options() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .and(wiremock::matchers::body_json(serde_json::json!({
                "model": "arrowcanaria",
                "messages": [{
                    "role": "user",
                    "content": "Give JSON"
                }],
                "stream": false,
                "think": false,
                "format": "json",
                "options": {
                    "num_predict": 300,
                    "temperature": 0.5
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": {
                    "role": "assistant",
                    "content": "{\"status\": \"ok\"}"
                },
                "done_reason": "stop"
            })))
            .mount(&mock_server)
            .await;

        let provider = OllamaProvider::new(mock_server.uri(), "arrowcanaria".to_string());

        let request = LlmRequest {
            messages: vec![LlmMessage {
                role: "user".into(),
                content: "Give JSON".into(),
                cache: false,
            }],
            format: Some("json".into()),
            temperature: Some(0.5),
            ..Default::default()
        };
        let result = provider.complete_with_cache(request).await;

        assert!(
            result.is_ok(),
            "Request should succeed if matched, but will fail (404) if logic is missing: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().content, "{\"status\": \"ok\"}");
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq)]
    struct TestData {
        name: String,
        score: i32,
    }

    #[tokio::test]
    async fn test_ollama_complete_structured_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": { "role": "assistant", "content": "{\"name\": \"Alice\", \"score\": 100}" },
                "done_reason": "stop"
            })))
            .mount(&mock_server)
            .await;

        let provider = OllamaProvider::new(mock_server.uri(), "test".to_string());
        let result: Result<TestData, AiomeError> =
            provider.complete_structured("give data", None).await;

        assert!(
            result.is_ok(),
            "Should successfully parse valid JSON: {:?}",
            result.err()
        );
        assert_eq!(
            result.unwrap(),
            TestData {
                name: "Alice".into(),
                score: 100
            }
        );
    }

    #[tokio::test]
    async fn test_ollama_complete_structured_retry_on_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": { "role": "assistant", "content": "{\"name\": \"Bob\", \"score\": " },
                "done_reason": "stop"
            })))
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": { "role": "assistant", "content": "{\"name\": \"Bob\", \"score\": 200}" },
                "done_reason": "stop"
            })))
            .mount(&mock_server)
            .await;

        let provider = OllamaProvider::new(mock_server.uri(), "test".to_string());
        let result: Result<TestData, AiomeError> =
            provider.complete_structured("give data", None).await;

        assert!(
            result.is_ok(),
            "Should succeed after retry: {:?}",
            result.err()
        );
        assert_eq!(
            result.unwrap(),
            TestData {
                name: "Bob".into(),
                score: 200
            }
        );
    }

    #[tokio::test]
    async fn test_ollama_complete_with_cache_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .and(wiremock::matchers::body_json(serde_json::json!({
                "model": "test",
                "messages": [
                    { "role": "system", "content": "You are helpful" },
                    { "role": "user", "content": "Hello" }
                ],
                "stream": false,
                "think": false,
                "format": "json",
                "options": { "num_predict": 300, "temperature": 0.5 }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": { "role": "assistant", "content": "Hi!" },
                "done_reason": "stop"
            })))
            .mount(&mock_server)
            .await;

        let provider = OllamaProvider::new(mock_server.uri(), "test".to_string());
        let mut request = LlmRequest {
            temperature: Some(0.5),
            format: Some("json".into()),
            ..Default::default()
        };
        request.messages.push(LlmMessage {
            role: "system".into(),
            content: "You are helpful".into(),
            cache: true,
        });
        request.messages.push(LlmMessage {
            role: "user".into(),
            content: "Hello".into(),
            cache: false,
        });

        let result = provider.complete_with_cache(request).await;

        assert!(
            result.is_ok(),
            "Expected wiremock match but likely 404: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_gemini_complete_with_cache_sends_full_history() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/v1beta/models/test-model:generateContent".to_string(),
            ))
            .and(wiremock::matchers::body_json(serde_json::json!({
                "contents": [
                    { "role": "user", "parts": [{ "text": "Hello" }] },
                    { "role": "model", "parts": [{ "text": "Hi there!" }] },
                    { "role": "user", "parts": [{ "text": "Who are you?" }] }
                ],
                "system_instruction": { "parts": [{ "text": "You are a helpful assistant." }] }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "candidates": [{
                    "content": { "parts": [{ "text": "I am Aiome." }] },
                    "finishReason": "STOP"
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = crate::http::get_http_client().clone();
        let provider = GeminiProvider::with_base_url(
            client,
            secrecy::SecretString::from("test-key".to_string()),
            "test-model".into(),
            mock_server.uri(),
        );

        let request = LlmRequest {
            messages: vec![
                LlmMessage {
                    role: "system".into(),
                    content: "You are a helpful assistant.".into(),
                    cache: false,
                },
                LlmMessage {
                    role: "user".into(),
                    content: "Hello".into(),
                    cache: false,
                },
                LlmMessage {
                    role: "assistant".into(),
                    content: "Hi there!".into(),
                    cache: false,
                },
                LlmMessage {
                    role: "user".into(),
                    content: "Who are you?".into(),
                    cache: false,
                },
            ],
            ..Default::default()
        };

        let result = provider.complete_with_cache(request).await;

        assert!(result.is_ok(), "Request failed: {:?}", result.err());
        assert_eq!(result.unwrap().content, "I am Aiome.");
    }
}
