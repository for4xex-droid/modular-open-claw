/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use aiome_core::llm_provider::EmbeddingProvider;
use async_trait::async_trait;

#[cfg(feature = "native-inference")]
use candle_core::{DType, Device, Tensor};
#[cfg(feature = "native-inference")]
use candle_nn::VarBuilder;
#[cfg(feature = "native-inference")]
use candle_transformers::models::bert::{BertModel, Config, HiddenAct};
#[cfg(feature = "native-inference")]
use hf_hub::api::sync::Api;
#[cfg(feature = "native-inference")]
use std::sync::Arc;
#[cfg(feature = "native-inference")]
use tokenizers::{PaddingParams, Tokenizer};
#[cfg(feature = "native-inference")]
use tokio::sync::Mutex;

#[cfg(feature = "native-inference")]
pub struct NativeModelInner {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

#[cfg(not(feature = "native-inference"))]
pub struct NativeModelInner {} // Dummy for compilation

/// ローカル環境で実行される高速なネイティブ埋め込み (all-MiniLM-L6-v2)
#[derive(Clone)]
pub struct NativeEmbeddingProvider {
    inner: Arc<tokio::sync::Mutex<Option<NativeModelInner>>>,
}

impl std::fmt::Debug for NativeEmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeEmbeddingProvider").finish()
    }
}

impl NativeEmbeddingProvider {
    pub fn new() -> Result<Self, AiomeError> {
        Ok(Self {
            inner: Arc::new(tokio::sync::Mutex::new(None)),
        })
    }

    #[allow(unsafe_code)]
    #[cfg(feature = "native-inference")]
    async fn get_or_init_model(
        &self,
    ) -> Result<tokio::sync::MutexGuard<'_, Option<NativeModelInner>>, AiomeError> {
        let mut guard = self.inner.lock().await;
        if guard.is_none() {
            tracing::info!(
                "🚀 [NativeEmbedding] Loading all-MiniLM-L6-v2 model from HuggingFace Hub..."
            );
            let inner = tokio::task::spawn_blocking(|| {
                let api = Api::new().map_err(|e| AiomeError::Infrastructure {
                    reason: format!("HF Api Error: {}", e),
                })?;
                let repo = api.model("sentence-transformers/all-MiniLM-L6-v2".to_string());

                let config_filename =
                    repo.get("config.json")
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: format!("Fetch config.json error: {}", e),
                        })?;
                let tokenizer_filename =
                    repo.get("tokenizer.json")
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: format!("Fetch tokenizer.json error: {}", e),
                        })?;
                let weights_filename =
                    repo.get("model.safetensors")
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: format!("Fetch model.safetensors error: {}", e),
                        })?;

                let config = std::fs::read_to_string(config_filename).map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: format!("Read config.json error: {}", e),
                    }
                })?;
                let mut config: Config =
                    serde_json::from_str(&config).map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Parse config.json error: {}", e),
                    })?;

                let mut tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: format!("Load tokenizer error: {}", e),
                    }
                })?;

                if let Some(pp) = tokenizer.get_padding_mut() {
                    pp.strategy = tokenizers::PaddingStrategy::BatchLongest;
                } else {
                    let pp = PaddingParams {
                        strategy: tokenizers::PaddingStrategy::BatchLongest,
                        ..Default::default()
                    };
                    tokenizer.with_padding(Some(pp));
                }

                let device = Device::Cpu; // Use CPU by default for stability
                let vb = unsafe {
                    VarBuilder::from_mmaped_safetensors(&[weights_filename], DType::F32, &device)
                }
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Load safetensors error: {}", e),
                })?;

                let model =
                    BertModel::load(vb, &config).map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Load BertModel error: {}", e),
                    })?;

                tracing::info!("✅ [NativeEmbedding] Model loaded successfully");
                Ok::<NativeModelInner, AiomeError>(NativeModelInner {
                    model,
                    tokenizer,
                    device,
                })
            })
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("JoinError: {}", e),
            })??;

            *guard = Some(inner);
        }
        Ok(guard)
    }

    #[cfg(feature = "native-inference")]
    pub async fn calculate_entropy(&self, text: &str) -> Result<f64, AiomeError> {
        let mut guard = self.get_or_init_model().await?;
        let inner = guard.as_mut().ok_or_else(|| AiomeError::Infrastructure {
            reason: "ModelInner is uninitialized".to_string(),
        })?;

        let tokens =
            inner
                .tokenizer
                .encode(text, true)
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Tokenization error: {}", e),
                })?;
        let ids = tokens.get_ids();

        if ids.is_empty() {
            return Ok(0.0);
        }

        let mut counts = std::collections::HashMap::new();
        for &id in ids {
            *counts.entry(id).or_insert(0) += 1;
        }

        let total = ids.len() as f64;
        let mut entropy = 0.0;
        for &count in counts.values() {
            let p = (count as f64) / total;
            entropy -= p * p.log2();
        }

        // Normalize entropy by max possible entropy for this length (log2(N))
        let max_entropy = total.log2();
        let normalized = if max_entropy > 0.0 {
            entropy / max_entropy
        } else {
            0.0
        };

        Ok(normalized)
    }
}

#[async_trait]
impl EmbeddingProvider for NativeEmbeddingProvider {
    #[cfg(feature = "native-inference")]
    async fn embed(&self, text: &str, _is_query: bool) -> Result<Vec<f32>, AiomeError> {
        let mut guard = self.get_or_init_model().await?;
        let inner = guard.as_mut().ok_or_else(|| AiomeError::Infrastructure {
            reason: "ModelInner is uninitialized".to_string(),
        })?;

        let tokens =
            inner
                .tokenizer
                .encode(text, true)
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Tokenization error: {}", e),
                })?;
        let token_ids_raw = tokens.get_ids().to_vec();

        let token_ids = Tensor::new(token_ids_raw.as_slice(), &inner.device)
            .and_then(|t| t.unsqueeze(0)) // Batch size 1
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Tensor creation error: {}", e),
            })?;

        let token_type_ids = token_ids
            .zeros_like()
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("TokenTypeIds creation error: {}", e),
            })?;

        // Run forward
        let embeddings = inner
            .model
            .forward(&token_ids, &token_type_ids, None)
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Model forward error: {}", e),
            })?;

        // Apply Mean Pooling
        let dims = embeddings.dims3().map_err(|e| AiomeError::Infrastructure {
            reason: format!("Tensor dims3 error: {}", e),
        })?;
        let _b_size = dims.0;

        let attention_mask = token_ids
            .ones_like()
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Mask creation error: {}", e),
            })?;

        let sum_embeddings = embeddings.sum(1).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Sum embeddings error: {}", e),
        })?;

        let sum_mask = attention_mask
            .to_dtype(DType::F32)
            .and_then(|m| m.sum(1))
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Sum mask error: {}", e),
            })?;

        let pooled =
            sum_embeddings
                .broadcast_div(&sum_mask)
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Pooling div error: {}", e),
                })?;

        // L2 Normalization
        let normalized = pooled
            .sqr()
            .and_then(|p| p.sum_keepdim(1))
            .and_then(|s| s.sqrt())
            .and_then(|norm| pooled.broadcast_div(&norm))
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("L2 Normalization error: {}", e),
            })?;

        let vec_embeddings: Vec<Vec<f32>> =
            normalized
                .to_vec2()
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("To Vec error: {}", e),
                })?;

        if let Some(first) = vec_embeddings.into_iter().next() {
            Ok(first)
        } else {
            Err(AiomeError::Infrastructure {
                reason: "Empty embedding generated".to_string(),
            })
        }
    }

    #[cfg(not(feature = "native-inference"))]
    async fn embed(&self, _text: &str, _is_query: bool) -> Result<Vec<f32>, AiomeError> {
        Err(AiomeError::Infrastructure {
            reason: "Feature native-inference is disabled".to_string(),
        })
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        Ok(())
    }

    fn name(&self) -> &str {
        "Native(all-MiniLM-L6-v2)"
    }

    fn embedding_dim(&self) -> usize {
        384
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(feature = "native-inference")]
    async fn test_native_embedding_green() {
        // GREEN phase test
        let provider = NativeEmbeddingProvider::new().expect("Should initialize");
        assert_eq!(provider.embedding_dim(), 384);
        assert_eq!(provider.name(), "Native(all-MiniLM-L6-v2)");

        // Ignore test if running without internet access or avoiding long downloads in simple CI
        if std::env::var("CI").is_err() {
            let embed_res = provider
                .embed("hello world", false)
                .await
                .expect("Should generate embedding");
            assert_eq!(embed_res.len(), 384);

            // Check that it's somewhat normalized
            let norm: f32 = embed_res.iter().map(|v| v * v).sum();
            assert!(
                (norm - 1.0).abs() < 0.01,
                "Embedding should be L2 normalized"
            );
        }
    }
}
