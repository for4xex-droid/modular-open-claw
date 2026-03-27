/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::error::AiomeError;
use aiome_contracts::traits::TtsProvider;
use async_trait::async_trait;

/// Phase 13.3: OpenAI をバックエンドとした TTS プロバイダー
#[derive(Debug)]
pub struct OpenAiTtsProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl OpenAiTtsProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
        }
    }
}

#[async_trait]
impl TtsProvider for OpenAiTtsProvider {
    async fn synthesize(&self, text: &str, voice_id: &str) -> Result<Vec<u8>, AiomeError> {
        let payload = serde_json::json!({
            "model": self.model,
            "input": text,
            "voice": voice_id
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/audio/speech")
            .header("Authorization", format!("Bearer {}", self.api_key))
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

        let bytes = resp.bytes().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to read OpenAI TTS response: {}", e),
        })?;

        Ok(bytes.to_vec())
    }

    async fn health_check(&self) -> Result<bool, AiomeError> {
        Ok(!self.api_key.is_empty())
    }
}

/// Phase 13.3: XTTS (Local) をバックエンドとした TTS プロバイダー
#[derive(Debug)]
pub struct XttsProvider {
    client: reqwest::Client,
    endpoint: String,
}

impl XttsProvider {
    pub fn new(endpoint: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint,
        }
    }
}

#[async_trait]
impl TtsProvider for XttsProvider {
    async fn synthesize(&self, text: &str, voice_id: &str) -> Result<Vec<u8>, AiomeError> {
        let payload = serde_json::json!({
            "text": text,
            "speaker_id": voice_id,
            "language": "ja"
        });

        let url = format!("{}/tts_to_audio", self.endpoint.trim_end_matches('/'));

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("XTTS request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("XTTS API error: {}", resp.status()),
            });
        }

        let bytes = resp.bytes().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to read XTTS response: {}", e),
        })?;

        Ok(bytes.to_vec())
    }

    async fn health_check(&self) -> Result<bool, AiomeError> {
        let url = format!("{}/health", self.endpoint.trim_end_matches('/'));
        let resp = self.client.get(url).send().await;
        Ok(resp.is_ok() && resp.unwrap().status().is_success())
    }
}

/// [Phase 13.3] Mock TTS Provider for Testing
#[derive(Debug, Default)]
pub struct MockTtsProvider;

#[async_trait]
impl TtsProvider for MockTtsProvider {
    async fn synthesize(&self, _text: &str, _voice_id: &str) -> Result<Vec<u8>, AiomeError> {
        // Return 100 bytes of "audio" (wav header mock)
        Ok(vec![0u8; 100])
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
    async fn test_xtts_provider_health_check_offline() {
        let provider = XttsProvider::new("http://invalid.local:18020".into());
        let res = provider.health_check().await;
        assert!(res.is_ok()); // Should return Ok(false) if server unreachable
        assert_eq!(res.unwrap(), false);
    }
}
