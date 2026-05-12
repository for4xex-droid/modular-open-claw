/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use crate::slm_bridge::{SlmBackend, SlmMemoryEntry, SlmRecallResult};
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::NativeModelConfig;
use async_trait::async_trait;

#[cfg(feature = "native-inference")]
use candle_core::{Device, Tensor};

#[cfg(feature = "native-inference")]
use crate::llm::native_embedding::NativeEmbeddingProvider;
use crate::polar_quant::PolarQuantEncoder;
use aiome_core::llm_provider::EmbeddingProvider;

/// Candle ベースのインプロセス SLM バックエンド
#[derive(Clone)]
pub struct NativeSlmBackend {
    config: NativeModelConfig,
    memory_store: std::sync::Arc<tokio::sync::RwLock<Vec<(SlmMemoryEntry, Vec<f64>, Vec<u8>)>>>,
    #[cfg(feature = "native-inference")]
    embedder: NativeEmbeddingProvider,
    encoder: std::sync::Arc<PolarQuantEncoder>,
}

impl std::fmt::Debug for NativeSlmBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeSlmBackend").finish()
    }
}

impl NativeSlmBackend {
    /// 新規インスタンスを生成
    pub fn new(config: NativeModelConfig) -> Result<Self, AiomeError> {
        #[cfg(feature = "native-inference")]
        let embedder = NativeEmbeddingProvider::new().map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to init native embedder: {}", e),
        })?;

        Ok(Self {
            config,
            memory_store: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
            #[cfg(feature = "native-inference")]
            embedder,
            encoder: std::sync::Arc::new(PolarQuantEncoder::new(4, 32)),
        })
    }

    /// モデルの初期化 (Lazy Loading)
    pub async fn init(&self) -> Result<(), AiomeError> {
        info!(
            "🚀 [NativeSlm] Initializing model: {}",
            self.config.model_name
        );
        Ok(())
    }
}

#[async_trait]
impl SlmBackend for NativeSlmBackend {
    async fn store(&self, entry: SlmMemoryEntry) -> Result<(), AiomeError> {
        #[cfg(feature = "native-inference")]
        {
            let text = format!("{} {}", entry.category, entry.content);
            let embedding_f32 = self.embedder.embed(&text, false).await?;
            let embedding: Vec<f64> = embedding_f32.into_iter().map(|v| v as f64).collect();
            let encoded = self.encoder.encode(&embedding);

            let mut store = self.memory_store.write().await;
            store.push((entry, embedding, encoded));
        }
        Ok(())
    }

    async fn recall(&self, query: &str, limit: i64) -> Result<Vec<SlmRecallResult>, AiomeError> {
        #[cfg(feature = "native-inference")]
        {
            let query_emb_f32 = self.embedder.embed(query, true).await?;
            let query_emb: Vec<f64> = query_emb_f32.into_iter().map(|v| v as f64).collect();

            let store = self.memory_store.read().await;

            // Phase 1: PolarQuant Top-50
            let mut polar_scores: Vec<(usize, f64)> = store
                .iter()
                .enumerate()
                .map(|(i, (_, _, encoded))| {
                    let decoded = self.encoder.decode(encoded, query_emb.len());
                    let score = query_emb
                        .iter()
                        .zip(decoded.iter())
                        .map(|(a, b)| a * b)
                        .sum::<f64>();
                    (i, score)
                })
                .collect();

            polar_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            polar_scores.truncate(50);

            // Phase 2: NativeEmbedding Rerank
            let mut results = Vec::new();
            for (idx, _polar_score) in polar_scores {
                let (entry, emb, _) = &store[idx];
                let score = query_emb
                    .iter()
                    .zip(emb.iter())
                    .map(|(a, b)| a * b)
                    .sum::<f64>();
                results.push(SlmRecallResult {
                    content: entry.content.clone(),
                    score,
                });
            }

            results.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            results.truncate(limit as usize);

            Ok(results)
        }
        #[cfg(not(feature = "native-inference"))]
        Ok(vec![])
    }

    async fn detect_contradictions(&self, text: &str) -> Result<f64, AiomeError> {
        #[cfg(feature = "native-inference")]
        {
            // TDD GREEN: Minimal proxy for NLI using embeddings
            let text_emb_f32 = self.embedder.embed(text, true).await?;
            let text_emb: Vec<f64> = text_emb_f32.into_iter().map(|v| v as f64).collect();

            let store = self.memory_store.read().await;
            if store.is_empty() {
                return Ok(0.0);
            }

            let mut max_contradiction = 0.0;
            for (_, emb, _) in store.iter() {
                let similarity = text_emb
                    .iter()
                    .zip(emb.iter())
                    .map(|(a, b)| a * b)
                    .sum::<f64>();
                // High similarity means entailment. Low or negative means contradiction.
                // Contradiction score = 1.0 - similarity
                let contradiction = 1.0 - similarity;
                if contradiction > max_contradiction {
                    max_contradiction = contradiction;
                }
            }
            Ok(max_contradiction)
        }
        #[cfg(not(feature = "native-inference"))]
        Ok(0.5)
    }

    async fn calculate_importance(&self, text: &str) -> Result<f64, AiomeError> {
        #[cfg(feature = "native-inference")]
        {
            self.embedder.calculate_entropy(text).await
        }
        #[cfg(not(feature = "native-inference"))]
        Ok(0.8)
    }

    async fn calculate_importance_batch(
        &self,
        queries: &[String],
    ) -> Result<Vec<(String, f64)>, AiomeError> {
        let mut results = Vec::with_capacity(queries.len());
        for q in queries {
            results.push((q.clone(), 0.8));
        }
        Ok(results)
    }
}

use tracing::{info, warn};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_native_backend_skeleton() -> Result<(), Box<dyn std::error::Error>> {
        let config = NativeModelConfig {
            model_name: "test-model".into(),
            model_path: "path/to/model".into(),
            tokenizer_path: "path/to/tokenizer".into(),
            context_size: 512,
            device: "cpu".into(),
            quantization: None,
            embedding_dim: Some(768),
        };
        let backend = NativeSlmBackend::new(config)?;
        let res = backend.calculate_importance("test").await;
        assert!(res.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_native_backend_full_flow() -> Result<(), Box<dyn std::error::Error>> {
        let config = aiome_core_contracts::llm::NativeModelConfig {
            model_name: "test-model".into(),
            model_path: "path/to/model".into(),
            tokenizer_path: "path/to/tokenizer".into(),
            context_size: 512,
            device: "cpu".into(),
            quantization: None,
            embedding_dim: Some(768),
        };
        let backend = NativeSlmBackend::new(config)?;

        let entry = crate::slm_bridge::SlmMemoryEntry {
            content: "I went to the store to buy an apple.".to_string(),
            category: "TEST".to_string(),
            metadata: None,
        };

        backend.store(entry).await.unwrap();

        let results = backend.recall("buy apple", 10).await.unwrap();
        assert_eq!(results.len(), 1, "Should recall the stored memory");
        assert_eq!(results[0].content, "I went to the store to buy an apple.");

        let importance_low = backend
            .calculate_importance("apple apple apple apple")
            .await
            .unwrap();
        let importance_high = backend
            .calculate_importance("This is a highly critical event that changes everything!")
            .await
            .unwrap();
        assert!(
            importance_high > importance_low,
            "High entropy text should have higher importance"
        );

        let contradiction_high = backend
            .detect_contradictions("I did not buy any apples at all.")
            .await
            .unwrap();
        let contradiction_low = backend
            .detect_contradictions("I bought an apple.")
            .await
            .unwrap();

        assert!(
            contradiction_high > contradiction_low,
            "Contradictory statement should have higher score"
        );
        Ok(())
    }
}
