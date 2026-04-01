/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use crate::slm_bridge::{SlmBackend, SlmMemoryEntry, SlmRecallResult};
use aiome_contracts::error::AiomeError;
use aiome_contracts::llm::NativeModelConfig;
use async_trait::async_trait;

#[cfg(feature = "native-inference")]
use candle_core::{Device, Tensor};

/// Candle ベースのインプロセス SLM バックエンド
#[derive(Debug)]
pub struct NativeSlmBackend {
    config: NativeModelConfig,
    // Phase 2: Reserve memory for Candle Device/Tensor integration
    memory_store: std::sync::RwLock<Vec<SlmMemoryEntry>>,
}

impl NativeSlmBackend {
    /// 新規インスタンスを生成
    pub fn new(config: NativeModelConfig) -> Self {
        Self {
            config,
            memory_store: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// モデルの初期化 (Lazy Loading)
    pub async fn init(&self) -> Result<(), AiomeError> {
        info!(
            "🚀 [NativeSlm] Initializing model: {}",
            self.config.model_name
        );
        // Phase 2: ModelManager を使用してファイルをロードし、Candle モデルを構築
        Ok(())
    }
}

#[async_trait]
impl SlmBackend for NativeSlmBackend {
    async fn store(&self, entry: SlmMemoryEntry) -> Result<(), AiomeError> {
        // Phase 2: インメモリ・ベクトルデータベースへの保存
        match self.memory_store.write() {
            Ok(mut store) => store.push(entry),
            Err(e) => {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Native backend memory lock poisoned: {}", e),
                });
            }
        }
        Ok(())
    }

    async fn recall(&self, _query: &str, _limit: i64) -> Result<Vec<SlmRecallResult>, AiomeError> {
        // Phase 2: KNN 検索
        Ok(vec![])
    }

    async fn detect_contradictions(&self, _text: &str) -> Result<f64, AiomeError> {
        // Phase 2: NLI モデルによる矛盾スコアリング
        Ok(0.0)
    }

    async fn calculate_importance(&self, _query: &str) -> Result<f64, AiomeError> {
        // Phase 2: トークン分布に基づく重要度
        Ok(0.5)
    }

    async fn calculate_importance_batch(
        &self,
        queries: &[String],
    ) -> Result<Vec<(String, f64)>, AiomeError> {
        let mut results = Vec::with_capacity(queries.len());
        for q in queries {
            results.push((q.clone(), 0.5));
        }
        Ok(results)
    }
}

use tracing::{info, warn};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_native_backend_skeleton() {
        let config = NativeModelConfig {
            model_name: "test-model".into(),
            model_path: "path/to/model".into(),
            tokenizer_path: "path/to/tokenizer".into(),
            context_size: 512,
            device: "cpu".into(),
            quantization: None,
            embedding_dim: Some(768),
        };
        let backend = NativeSlmBackend::new(config);
        let res = backend.calculate_importance("test").await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_native_backend_guardrail_silent_data_loss() {
        // This test ensures that developers in Phase 2 realize that store() doesn't actually
        // save to disk yet, and recall() always returns empty!
        let config = aiome_contracts::llm::NativeModelConfig {
            model_name: "test-model".into(),
            model_path: "path/to/model".into(),
            tokenizer_path: "path/to/tokenizer".into(),
            context_size: 512,
            device: "cpu".into(),
            quantization: None,
            embedding_dim: Some(768),
        };
        let backend = NativeSlmBackend::new(config);

        let entry = crate::slm_bridge::SlmMemoryEntry {
            timestamp: 12345,
            action: "TEST".to_string(),
            belief_affected: None,
            source_job_id: None,
        };

        // Storing currently pushes to an in-memory Vec but doesn't persist
        let _ = backend.store(entry).await;

        // Because Phase 2 KNN search is not implemented, recall is ALWAYS empty.
        // Once Phase 2 is implemented, THIS TEST WILL FAIL AND MUST BE REWRITTEN.
        // It acts as a tripwire to ensure we don't deploy half-baked SLM memory.
        let results = backend
            .recall("query", 10)
            .await
            .expect("Native recall failed");
        assert_eq!(
            results.len(),
            0,
            "CRITICAL: If recall() is now working, you must remove this Guardrail test and implement proper disk-backed persistence!"
        );
    }
}
