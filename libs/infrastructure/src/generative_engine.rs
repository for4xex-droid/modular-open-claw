use aiome_contracts::contracts::ArtifactResponse;
use aiome_contracts::traits::GenerativeEngine;
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use secrecy::ExposeSecret;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// ComfyUI (ローカルGPU) を使用する GenerativeEngine 実装。
/// SSRF防御付きの共有HTTPクライアントを内部で使用する。
pub struct ComfyUiGenerativeEngine {
    base_url: String,
    /// GPU排他制御用セマフォ。LoRA学習との並走を防止する。
    compute_semaphore: Option<Arc<tokio::sync::Semaphore>>,
}

impl ComfyUiGenerativeEngine {
    pub fn new(base_url: String, compute_semaphore: Option<Arc<tokio::sync::Semaphore>>) -> Self {
        Self {
            base_url,
            compute_semaphore,
        }
    }
}

#[async_trait]
impl GenerativeEngine for ComfyUiGenerativeEngine {
    async fn generate_artifact(
        &self,
        prompt: &str,
        _workflow_id: &str,
        _input_artifact: Option<&Path>,
    ) -> Result<ArtifactResponse, AiomeError> {
        // F-02: Acquire compute semaphore to prevent OOM when running alongside LoRA training
        let _permit = if let Some(sem) = &self.compute_semaphore {
            Some(
                sem.acquire()
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to acquire compute semaphore: {}", e),
                    })?,
            )
        } else {
            None
        };

        let client = aiome_core::http::get_http_client();
        let url = format!("{}/prompt", self.base_url);
        let res = client
            .post(&url)
            .json(&serde_json::json!({
                "prompt": { "text": prompt },
                "client_id": "aiome-engine"
            }))
            .send()
            .await;

        match res {
            Ok(response) if response.status().is_success() => {
                let json = response.json::<serde_json::Value>().await.map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: format!("Failed to parse ComfyUI JSON: {}", e),
                    }
                })?;
                let prompt_id =
                    json["prompt_id"]
                        .as_str()
                        .ok_or_else(|| AiomeError::Infrastructure {
                            reason: "Missing prompt_id in ComfyUI response".into(),
                        })?;
                let output_path = shared::app_data::AppDataResolver::new()
                    .resolve("artifacts")
                    .join(format!("comfy_output_{}.png", prompt_id));
                Ok(ArtifactResponse {
                    output_path: output_path.to_string_lossy().to_string(),
                    job_id: prompt_id.to_string(),
                })
            }
            Ok(resp) => Err(AiomeError::Infrastructure {
                reason: format!("ComfyUI error: HTTP {}", resp.status()),
            }),
            Err(e) => Err(AiomeError::Infrastructure {
                reason: format!("ComfyUI connection error: {}", e),
            }),
        }
    }

    async fn health_check(&self) -> Result<bool, AiomeError> {
        let client = aiome_core::http::get_http_client();
        match client
            .get(format!("{}/system_stats", self.base_url))
            .send()
            .await
        {
            Ok(res) => Ok(res.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

/// Fal.ai (クラウドAPI) を使用する GenerativeEngine 実装。
/// APIキーは `secrecy::SecretString` で保護され、Debug出力やログに漏洩しない。
pub struct FalAiGenerativeEngine {
    api_key: secrecy::SecretString,
}

impl FalAiGenerativeEngine {
    pub fn new(api_key: secrecy::SecretString) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl GenerativeEngine for FalAiGenerativeEngine {
    async fn generate_artifact(
        &self,
        prompt: &str,
        workflow_id: &str,
        _input_artifact: Option<&Path>,
    ) -> Result<ArtifactResponse, AiomeError> {
        let client = aiome_core::http::get_http_client();
        let url = format!("https://fal.run/{}", workflow_id);
        let res = client
            .post(&url)
            .header(
                "Authorization",
                format!("Key {}", self.api_key.expose_secret()),
            )
            .json(&serde_json::json!({ "prompt": prompt }))
            .send()
            .await;

        match res {
            Ok(response) if response.status().is_success() => {
                let json = response.json::<serde_json::Value>().await.map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: format!("Failed to parse Fal.ai JSON: {}", e),
                    }
                })?;
                let url = json["images"]
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|img| img["url"].as_str())
                    .ok_or_else(|| AiomeError::Infrastructure {
                        reason: "Missing images[0].url in Fal.ai response".into(),
                    })?;
                Ok(ArtifactResponse {
                    output_path: url.to_string(),
                    job_id: format!("fal_{}", Uuid::new_v4()),
                })
            }
            Ok(resp) => Err(AiomeError::Infrastructure {
                reason: format!("Fal.ai error: HTTP {}", resp.status()),
            }),
            Err(e) => Err(AiomeError::Infrastructure {
                reason: format!("Fal.ai connection error: {}", e),
            }),
        }
    }

    async fn health_check(&self) -> Result<bool, AiomeError> {
        if self.api_key.expose_secret().is_empty() {
            Err(AiomeError::Infrastructure {
                reason: "Missing Fal.ai API key".into(),
            })
        } else {
            Ok(true)
        }
    }
}

#[cfg(any(test, debug_assertions))]
pub mod mock {
    use super::*;

    pub struct MockGenerativeEngine {
        pub fail_health_check: bool,
        pub mock_output_path: String,
    }

    impl Default for MockGenerativeEngine {
        fn default() -> Self {
            Self {
                fail_health_check: false,
                mock_output_path: "/tmp/mock_generate_output.png".into(),
            }
        }
    }

    #[async_trait]
    impl GenerativeEngine for MockGenerativeEngine {
        async fn generate_artifact(
            &self,
            _prompt: &str,
            _workflow_id: &str,
            _input_artifact: Option<&Path>,
        ) -> Result<ArtifactResponse, AiomeError> {
            Ok(ArtifactResponse {
                output_path: self.mock_output_path.clone(),
                job_id: "mock-job-123".into(),
            })
        }

        async fn health_check(&self) -> Result<bool, AiomeError> {
            if self.fail_health_check {
                Err(AiomeError::Infrastructure {
                    reason: "Mock health check failed".into(),
                })
            } else {
                Ok(true)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_comfyui_engine_generate_artifact_green() {
        let engine = ComfyUiGenerativeEngine::new("http://localhost:8188".into(), None);
        let result = engine
            .generate_artifact("test prompt", "sdxl_workflow", None)
            .await;
        assert!(result.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = result {
            assert!(reason.contains("ComfyUI connection error"));
        } else {
            panic!("Expected Infrastructure error");
        }
    }

    #[tokio::test]
    async fn test_falai_engine_generate_artifact_green() {
        let engine = FalAiGenerativeEngine::new("dummy_key".to_string().into());
        let result = engine
            .generate_artifact("test prompt", "fast_lora", None)
            .await;
        assert!(result.is_err());
        if let Err(AiomeError::Infrastructure { reason }) = result {
            assert!(reason.contains("401 Unauthorized") || reason.contains("Fal.ai error: HTTP"));
        } else {
            panic!("Expected Infrastructure error for HTTP error");
        }
    }

    #[tokio::test]
    async fn test_mock_engine_green() {
        let engine = mock::MockGenerativeEngine::default();
        let result = engine.health_check().await;
        assert!(result.is_ok());
        assert_eq!(result.expect("health_check failed in test"), true);
    }
}
