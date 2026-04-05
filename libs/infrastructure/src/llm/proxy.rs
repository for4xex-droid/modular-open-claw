/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
/// `ProxyLlmProvider` 構造体
pub struct ProxyLlmProvider {
    /// proxy_url
    pub proxy_url: String,
    /// endpoint_tag
    pub endpoint_tag: String,
    /// caller_id
    pub caller_id: String,
    /// proxy_secret (VULN-65: used for HMAC signature)
    pub proxy_secret: Option<String>,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct ProxyRequest {
    caller_id: String,
    prompt: String,
    system: Option<String>,
    endpoint: String,
}

#[derive(Deserialize)]
struct ProxyResponse {
    content: String,
    stop_reason: aiome_core_contracts::llm::StopReason,
    reasoning: Option<String>,
    metadata: Option<std::collections::HashMap<String, String>>,
}

impl ProxyLlmProvider {
    /// 新しいインスタンスを生成する
    pub fn new(
        proxy_url: String,
        endpoint_tag: String,
        caller_id: String,
        proxy_secret: Option<String>,
    ) -> Self {
        Self {
            proxy_url,
            endpoint_tag,
            caller_id,
            proxy_secret,
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| aiome_core::http::get_http_client().clone()),
        }
    }
}

#[async_trait]
impl LlmProvider for ProxyLlmProvider {
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
        let url = format!("{}/api/v1/llm/complete", self.proxy_url);

        let payload = ProxyRequest {
            caller_id: self.caller_id.clone(),
            prompt: prompt.to_string(),
            system: system.map(|s| s.to_string()),
            endpoint: self.endpoint_tag.clone(),
        };

        let payload_json =
            serde_json::to_string(&payload).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to serialize proxy request (complete): {}", e),
            })?;
        let mut request_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        // VULN-65: Add HMAC Signature to headers to prevent LLM Proxy Integrity tampering
        if let Some(secret) = &self.proxy_secret {
            use hmac::{Hmac, Mac};
            use sha2::Sha256;
            type HmacSha256 = Hmac<Sha256>;
            if let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) {
                mac.update(payload_json.as_bytes());
                let result = mac.finalize();
                let signature = hex::encode(result.into_bytes());
                request_builder = request_builder.header("X-Proxy-Signature", signature);
            }
        }

        let res = request_builder
            .body(payload_json)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        if !res.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("KeyProxy returned error status: {}", res.status()),
            });
        }

        let body: ProxyResponse = res.json().await.map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        Ok(aiome_core::llm_provider::LlmResponse {
            content: body.content,
            stop_reason: body.stop_reason,
            reasoning: body.reasoning,
            metadata: body.metadata,
        })
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.complete("ping", None).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "KeyProxy"
    }
}

#[async_trait]
impl aiome_core::llm_provider::EmbeddingProvider for ProxyLlmProvider {
    async fn embed(&self, text: &str, _is_query: bool) -> Result<Vec<f32>, AiomeError> {
        let url = format!("{}/api/v1/llm/embed", self.proxy_url);

        let payload = ProxyRequest {
            caller_id: self.caller_id.clone(),
            prompt: text.to_string(),
            system: None,
            endpoint: "gemini-embed".to_string(),
        };

        let payload_json =
            serde_json::to_string(&payload).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to serialize proxy request (embed): {}", e),
            })?;
        let mut request_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        // VULN-65: Add HMAC Signature to headers to prevent LLM Proxy Integrity tampering
        if let Some(secret) = &self.proxy_secret {
            use hmac::{Hmac, Mac};
            use sha2::Sha256;
            type HmacSha256 = Hmac<Sha256>;
            if let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) {
                mac.update(payload_json.as_bytes());
                let result = mac.finalize();
                let signature = hex::encode(result.into_bytes());
                request_builder = request_builder.header("X-Proxy-Signature", signature);
            }
        }

        let res = request_builder
            .body(payload_json)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        if !res.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("KeyProxy (Embed) error: {}", res.status()),
            });
        }

        #[derive(Deserialize)]
        struct EmbedRes {
            embedding: Vec<f32>,
        }
        let body: EmbedRes = res.json().await.map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        Ok(body.embedding)
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.embed("ping", false).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "KeyProxy(Embed)"
    }

    fn embedding_dim(&self) -> usize {
        768 // Default for VaultProxy
    }
}
