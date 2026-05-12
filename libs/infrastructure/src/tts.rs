/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::traits::TtsProvider;
use async_trait::async_trait;
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use std::time::Duration;

/// Phase 13.3: OpenAI をバックエンドとした TTS プロバイダー
pub struct OpenAiTtsProvider {
    client: reqwest::Client,
    api_key: SecretString,
    model: String,
    endpoint: String,
}

impl std::fmt::Debug for OpenAiTtsProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiTtsProvider")
            .field("model", &self.model)
            .field("endpoint", &self.endpoint)
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}

impl OpenAiTtsProvider {
    pub fn new(api_key: SecretString, model: String, endpoint: Option<String>) -> Self {
        Self {
            client: aiome_core::http::get_http_client().clone(),
            api_key,
            model,
            endpoint: endpoint
                .unwrap_or_else(|| "https://api.openai.com/v1/audio/speech".to_string()),
        }
    }

    /// Shared HTTP request builder for synthesize and synthesize_stream
    async fn send_tts_request(
        &self,
        text: &str,
        voice_id: &str,
    ) -> Result<reqwest::Response, AiomeError> {
        let payload = serde_json::json!({
            "model": self.model,
            "input": text,
            "voice": voice_id
        });

        let resp = self
            .client
            .post(&self.endpoint)
            .header(
                "Authorization",
                format!("Bearer {}", self.api_key.expose_secret()),
            )
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("OpenAI TTS request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("OpenAI TTS API error: {}", resp.status()),
            });
        }

        Ok(resp)
    }
}

#[async_trait]
impl TtsProvider for OpenAiTtsProvider {
    async fn synthesize(&self, text: &str, voice_id: &str) -> Result<Vec<u8>, AiomeError> {
        let resp = self.send_tts_request(text, voice_id).await?;
        let bytes = resp.bytes().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to read OpenAI TTS response: {}", e),
        })?;
        Ok(bytes.to_vec())
    }

    async fn synthesize_stream(
        &self,
        text: &str,
        voice_id: &str,
    ) -> Result<
        std::pin::Pin<
            Box<
                dyn tokio_stream::Stream<
                        Item = Result<aiome_core_contracts::traits::TtsStreamEvent, AiomeError>,
                    > + Send,
            >,
        >,
        AiomeError,
    > {
        let resp = self.send_tts_request(text, voice_id).await?;

        use tokio_stream::StreamExt;
        // NOTE: OpenAI TTS API does not provide Viseme/phoneme data.
        // This stream emits Audio-only events. Viseme generation for lip-sync
        // must be performed downstream (e.g., via phoneme analysis on audio chunks).
        let stream = resp.bytes_stream().map(|chunk_res| {
            chunk_res
                .map(|b| aiome_core_contracts::traits::TtsStreamEvent::Audio(b.to_vec()))
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Stream read error: {}", e),
                })
        });

        Ok(Box::pin(stream))
    }

    async fn health_check(&self) -> Result<bool, AiomeError> {
        Ok(!self.api_key.expose_secret().is_empty())
    }
}

/// Phase 13.3: XTTS (Local) をバックエンドとした TTS プロバイダー
#[derive(Debug)]
pub struct XttsProvider {
    client: reqwest::Client,
    endpoint: String,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl XttsProvider {
    pub fn new(endpoint: String) -> Self {
        let cb_config = CircuitBreakerConfig {
            failure_threshold: 2, // Fails after 2 consecutive errors
            reset_timeout: Duration::from_secs(30),
        };
        Self {
            client: aiome_core::http::get_http_client().clone(),
            endpoint,
            circuit_breaker: Arc::new(CircuitBreaker::new("xtts_provider", cb_config)),
        }
    }
}

#[async_trait]
impl TtsProvider for XttsProvider {
    async fn synthesize(&self, text: &str, voice_id: &str) -> Result<Vec<u8>, AiomeError> {
        if let Err(msg) = self.circuit_breaker.check_state().await {
            return Err(AiomeError::Infrastructure {
                reason: format!("XTTS Circuit Breaker blocked request: {}", msg),
            });
        }

        let payload = serde_json::json!({
            "text": text,
            "speaker_id": voice_id,
            "language": "ja"
        });

        let url = format!("{}/tts_to_audio", self.endpoint.trim_end_matches('/'));

        let resp = match self
            .client
            .post(&url)
            .timeout(Duration::from_secs(10))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                return Err(AiomeError::Infrastructure {
                    reason: format!("XTTS request failed: {}", e),
                });
            }
        };

        if !resp.status().is_success() {
            self.circuit_breaker.record_failure().await;
            return Err(AiomeError::Infrastructure {
                reason: format!("XTTS API error: {}", resp.status()),
            });
        }

        let bytes = match resp.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                return Err(AiomeError::Infrastructure {
                    reason: format!("Failed to read XTTS response: {}", e),
                });
            }
        };

        self.circuit_breaker.record_success().await;
        Ok(bytes.to_vec())
    }

    async fn health_check(&self) -> Result<bool, AiomeError> {
        let url = format!("{}/health", self.endpoint.trim_end_matches('/'));
        // Use a short timeout for health checks
        let resp = self
            .client
            .get(url)
            .timeout(Duration::from_secs(3))
            .send()
            .await;
        Ok(resp.map(|r| r.status().is_success()).unwrap_or(false))
    }
}

/// [Phase 13.3] Mock TTS Provider for Testing
#[cfg(any(test, debug_assertions))]
#[derive(Debug, Default)]
pub struct MockTtsProvider;

#[cfg(any(test, debug_assertions))]
#[async_trait]
impl TtsProvider for MockTtsProvider {
    async fn synthesize(&self, _text: &str, _voice_id: &str) -> Result<Vec<u8>, AiomeError> {
        // Return 100 bytes of "audio" (wav header mock)
        Ok(vec![0u8; 100])
    }

    async fn synthesize_stream(
        &self,
        _text: &str,
        _voice_id: &str,
    ) -> Result<
        std::pin::Pin<
            Box<
                dyn tokio_stream::Stream<
                        Item = Result<aiome_core_contracts::traits::TtsStreamEvent, AiomeError>,
                    > + Send,
            >,
        >,
        AiomeError,
    > {
        let stream = async_stream::stream! {
            yield Ok(aiome_core_contracts::traits::TtsStreamEvent::Audio(vec![0u8; 50]));
            yield Ok(aiome_core_contracts::traits::TtsStreamEvent::Viseme {
                viseme: "A".to_string(),
                timestamp_ms: 0,
                duration_ms: 100,
            });
            yield Ok(aiome_core_contracts::traits::TtsStreamEvent::Audio(vec![0u8; 50]));
            yield Ok(aiome_core_contracts::traits::TtsStreamEvent::Viseme {
                viseme: "O".to_string(),
                timestamp_ms: 100,
                duration_ms: 150,
            });
        };
        Ok(Box::pin(stream))
    }

    async fn health_check(&self) -> Result<bool, AiomeError> {
        Ok(true)
    }
}

/// Disabled TTS Provider for Production without credentials
#[derive(Debug, Default)]
pub struct DisabledTtsProvider;

#[async_trait]
impl TtsProvider for DisabledTtsProvider {
    async fn synthesize(&self, _text: &str, _voice_id: &str) -> Result<Vec<u8>, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "TTS feature is disabled in production".into(),
        })
    }

    async fn health_check(&self) -> Result<bool, AiomeError> {
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_tts_provider_green() {
        let provider = MockTtsProvider::default();
        let res = provider.synthesize("こんにちは", "p225").await;

        assert!(res.is_ok());
        assert_eq!(res.unwrap().len(), 100);
    }

    #[tokio::test]
    async fn test_mock_tts_stream_with_visemes() {
        let provider = MockTtsProvider::default();
        let stream_res = provider.synthesize_stream("hello", "p225").await;
        assert!(stream_res.is_ok(), "Mock should support streaming");

        let mut stream = stream_res.unwrap();
        let mut has_audio = false;
        let mut has_viseme = false;

        use tokio_stream::StreamExt;
        while let Some(event) = stream.next().await {
            match event.unwrap() {
                aiome_core_contracts::traits::TtsStreamEvent::Audio(_) => has_audio = true,
                aiome_core_contracts::traits::TtsStreamEvent::Viseme { viseme, .. } => {
                    assert!(!viseme.is_empty());
                    has_viseme = true;
                }
            }
        }

        assert!(has_audio, "Should yield audio chunks");
        assert!(has_viseme, "Should yield viseme events");
    }

    #[tokio::test]
    async fn test_xtts_provider_health_check_offline() {
        let provider = XttsProvider::new("http://invalid.local:18020".into());
        let res = provider.health_check().await;
        assert!(res.is_ok()); // Should return Ok(false) if server unreachable
        assert_eq!(res.unwrap(), false);
    }

    #[tokio::test]
    async fn test_xtts_provider_circuit_breaker_trips() {
        let provider = XttsProvider::new("http://invalid.local:18020".into());

        let res1 = provider.synthesize("test", "p225").await;
        assert!(res1.is_err()); // 1st failure

        let res2 = provider.synthesize("test", "p225").await;
        assert!(res2.is_err()); // 2nd failure

        let res3 = provider.synthesize("test", "p225").await;
        // 3rd failure should not even try to connect, but fail fast due to circuit breaker
        let err_msg = res3.unwrap_err().to_string();
        assert!(
            err_msg.contains("CircuitBreaker is OPEN"),
            "Expected CircuitBreaker to trip, got: {}",
            err_msg
        );
    }
}
