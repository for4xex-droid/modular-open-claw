/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use aiome_contracts::error::AiomeError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlmMemoryEntry {
    pub content: String,
    pub category: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SlmRecallResult {
    pub content: String,
    pub score: f64,
}

impl Default for SlmRecallResult {
    fn default() -> Self {
        Self {
            content: String::new(),
            score: 0.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlmRecallData {
    #[serde(default)]
    pub results: Vec<SlmRecallResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlmRecallJsonResponse {
    pub data: SlmRecallData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlmTraceChannelScores {
    #[serde(default)]
    pub semantic: f64,
    #[serde(default)]
    pub bm25: f64,
    #[serde(default)]
    pub entity_graph: f64,
    #[serde(default)]
    pub poincare: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlmTraceResult {
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub channel_scores: Option<SlmTraceChannelScores>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlmTraceData {
    #[serde(default)]
    pub results: Vec<SlmTraceResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlmTraceJsonResponse {
    pub data: SlmTraceData,
}

//// SLM バックエンドの共通インターフェース
#[async_trait::async_trait]
pub trait SlmBackend: std::fmt::Debug + Send + Sync {
    /// メモリを保存する
    async fn store(&self, entry: SlmMemoryEntry) -> Result<(), AiomeError>;
    /// メモリを検索する
    async fn recall(&self, query: &str, limit: i64) -> Result<Vec<SlmRecallResult>, AiomeError>;
    /// 矛盾を検出する
    async fn detect_contradictions(&self, text: &str) -> Result<f64, AiomeError>;
    /// 重要度を算出する
    async fn calculate_importance(&self, query: &str) -> Result<f64, AiomeError>;
    /// 重要度を一括算出する
    async fn calculate_importance_batch(
        &self,
        queries: &[String],
    ) -> Result<Vec<(String, f64)>, AiomeError>;
}

/// 従来の CLI ベースの SLM バックエンド
#[derive(Debug)]
pub struct CliSlmBackend {
    command_name: String,
}

impl CliSlmBackend {
    pub fn new(command: &str) -> Self {
        Self {
            command_name: command.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl SlmBackend for CliSlmBackend {
    async fn store(&self, entry: SlmMemoryEntry) -> Result<(), AiomeError> {
        let output = Command::new(&self.command_name)
            .arg("remember")
            .arg("--tags")
            .arg(&entry.category)
            .arg(&entry.content)
            .output()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to execute slm remember: {}", e),
            })?;

        if output.status.success() {
            Ok(())
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(AiomeError::Infrastructure {
                reason: format!("SLM CLI reported failure: {}", err),
            })
        }
    }

    async fn recall(&self, query: &str, limit: i64) -> Result<Vec<SlmRecallResult>, AiomeError> {
        let output = Command::new(&self.command_name)
            .arg("recall")
            .arg("--json")
            .arg("--limit")
            .arg(limit.to_string())
            .arg(query)
            .output()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to execute slm recall: {}", e),
            })?;

        if output.status.success() {
            let response: SlmRecallJsonResponse =
                serde_json::from_slice(&output.stdout).map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to parse SLM recall JSON: {}", e),
                })?;
            Ok(response.data.results)
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(AiomeError::Infrastructure {
                reason: format!("SLM CLI recall failure: {}", err),
            })
        }
    }

    async fn detect_contradictions(&self, text: &str) -> Result<f64, AiomeError> {
        let output = Command::new(&self.command_name)
            .arg("contradict")
            .arg("--json")
            .arg(text)
            .output()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to execute slm contradict: {}", e),
            })?;

        if output.status.success() {
            let response: serde_json::Value =
                serde_json::from_slice(&output.stdout).map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to parse SLM contradiction JSON: {}", e),
                })?;
            let score = response["data"]["score"].as_f64().unwrap_or(0.0);
            Ok(score)
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(AiomeError::Infrastructure {
                reason: format!("SLM CLI contradict failure: {}", err),
            })
        }
    }

    async fn calculate_importance(&self, query: &str) -> Result<f64, AiomeError> {
        let output = Command::new(&self.command_name)
            .arg("trace")
            .arg("--json")
            .arg(query)
            .output()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to execute slm trace: {}", e),
            })?;

        if output.status.success() {
            let response: SlmTraceJsonResponse =
                serde_json::from_slice(&output.stdout).map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to parse SLM trace JSON: {}", e),
                })?;
            Ok(SlmBridge::extract_importance_from_trace(&response))
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(AiomeError::Infrastructure {
                reason: format!("SLM CLI trace failure: {}", err),
            })
        }
    }

    async fn calculate_importance_batch(
        &self,
        queries: &[String],
    ) -> Result<Vec<(String, f64)>, AiomeError> {
        let tmp_path =
            std::env::temp_dir().join(format!("slm_batch_{}.jsonl", uuid::Uuid::new_v4()));
        let batch_content = queries.join("\n");
        tokio::fs::write(&tmp_path, &batch_content)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to write batch file: {}", e),
            })?;

        let output_res = Command::new(&self.command_name)
            .arg("trace")
            .arg("--json")
            .arg("--batch")
            .arg(&tmp_path)
            .output()
            .await;

        let _ = tokio::fs::remove_file(&tmp_path).await;

        match output_res {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut results = Vec::with_capacity(queries.len());
                for (i, line) in stdout.lines().enumerate() {
                    if let Ok(resp) = serde_json::from_str::<SlmTraceJsonResponse>(line) {
                        let importance = SlmBridge::extract_importance_from_trace(&resp);
                        let query = queries.get(i).cloned().unwrap_or_default();
                        results.push((query, importance));
                    }
                }
                Ok(results)
            }
            _ => {
                // Sequential fallback
                let mut results = Vec::with_capacity(queries.len());
                for q in queries {
                    match self.calculate_importance(q).await {
                        Ok(importance) => results.push((q.clone(), importance)),
                        Err(_) => results.push((q.clone(), 0.0)),
                    }
                }
                Ok(results)
            }
        }
    }
}

/// SuperLocalMemory との通信を管理するブリッジ (Strategy パターン)
#[derive(Debug)]
pub struct SlmBridge {
    circuit_breaker: Arc<CircuitBreaker>,
    backend: Box<dyn SlmBackend>,
}

impl SlmBridge {
    /// 新しいインスタンスを生成する (デフォルトは CLI)
    pub fn new() -> Self {
        Self::new_with_command("slm")
    }

    /// 指定されたコマンド名を使用して CLI バックエンドを生成する
    pub fn new_with_command(command: &str) -> Self {
        Self::with_backend(Box::new(CliSlmBackend::new(command)))
    }

    /// 任意のバックエンドを指定して生成する (テスト/拡張用)
    pub fn with_backend(backend: Box<dyn SlmBackend>) -> Self {
        let cb_config = CircuitBreakerConfig {
            failure_threshold: 3,
            reset_timeout: Duration::from_secs(60),
        };
        Self {
            circuit_breaker: Arc::new(CircuitBreaker::new("slm-bridge", cb_config)),
            backend,
        }
    }

    /// ネイティブ推論バックエンドを生成する (Phase 2)
    #[cfg(feature = "native-inference")]
    pub fn new_native(config: aiome_contracts::llm::NativeModelConfig) -> Self {
        Self::with_backend(Box::new(crate::native_backend::NativeSlmBackend::new(
            config,
        )))
    }

    /// 入力文字列を検証する
    fn validate(input: &str) -> Result<(), AiomeError> {
        if input.trim().is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "Input is empty or whitespace only".into(),
            });
        }
        if input.len() > 1024 * 64 {
            return Err(AiomeError::Infrastructure {
                reason: "Input too large (max 64KB)".into(),
            });
        }
        if input
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        {
            return Err(AiomeError::Infrastructure {
                reason: "Control characters detected in input".into(),
            });
        }
        let dangerous_chars = [
            ';', '&', '|', '$', '>', '<', '`', '\\', '!', '{', '}', '(', ')', '[', ']', '*', '?',
            '~',
        ];
        if input.chars().any(|c| dangerous_chars.contains(&c)) {
            return Err(AiomeError::Infrastructure {
                reason: "Potentially malicious characters detected in input".into(),
            });
        }
        Ok(())
    }

    pub async fn store_memory(&self, entry: SlmMemoryEntry) -> Result<(), AiomeError> {
        self.circuit_breaker
            .check_state()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("SLM Service is unavailable: {}", e),
            })?;
        Self::validate(&entry.content)?;
        Self::validate(&entry.category)?;

        match self.backend.store(entry).await {
            Ok(_) => {
                self.circuit_breaker.record_success().await;
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                Err(e)
            }
        }
    }

    pub async fn recall(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<SlmRecallResult>, AiomeError> {
        self.circuit_breaker
            .check_state()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("SLM Service is unavailable: {}", e),
            })?;
        Self::validate(query)?;

        match self.backend.recall(query, limit).await {
            Ok(res) => {
                self.circuit_breaker.record_success().await;
                Ok(res)
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                Err(e)
            }
        }
    }

    pub async fn detect_contradictions(&self, text: &str) -> Result<f64, AiomeError> {
        self.circuit_breaker
            .check_state()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("SLM Service is unavailable: {}", e),
            })?;
        Self::validate(text)?;

        match self.backend.detect_contradictions(text).await {
            Ok(score) => {
                self.circuit_breaker.record_success().await;
                Ok(score)
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                Err(e)
            }
        }
    }

    pub async fn calculate_importance(&self, query: &str) -> Result<f64, AiomeError> {
        self.circuit_breaker
            .check_state()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("SLM Service is unavailable: {}", e),
            })?;
        Self::validate(query)?;

        match self.backend.calculate_importance(query).await {
            Ok(score) => {
                self.circuit_breaker.record_success().await;
                Ok(score)
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                Err(e)
            }
        }
    }

    pub async fn calculate_importance_batch(
        &self,
        queries: &[String],
    ) -> Result<Vec<(String, f64)>, AiomeError> {
        self.circuit_breaker
            .check_state()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("SLM Service is unavailable: {}", e),
            })?;
        for q in queries {
            Self::validate(q)?;
        }

        match self.backend.calculate_importance_batch(queries).await {
            Ok(res) => {
                self.circuit_breaker.record_success().await;
                Ok(res)
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                Err(e)
            }
        }
    }

    fn extract_importance_from_trace(response: &SlmTraceJsonResponse) -> f64 {
        if response.data.results.is_empty() {
            return 0.0;
        }
        let top = &response.data.results[0];
        let base_score = top.score;
        if let Some(channels) = &top.channel_scores {
            let avg_channels =
                (channels.semantic + channels.bm25 + channels.entity_graph + channels.poincare)
                    / 4.0;
            (base_score + avg_channels) / 2.0
        } else {
            base_score
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct MockBackend {
        store_called: std::sync::atomic::AtomicBool,
    }

    #[async_trait::async_trait]
    impl SlmBackend for MockBackend {
        async fn store(&self, _: SlmMemoryEntry) -> Result<(), AiomeError> {
            self.store_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        async fn recall(&self, _: &str, _: i64) -> Result<Vec<SlmRecallResult>, AiomeError> {
            Ok(vec![])
        }
        async fn detect_contradictions(&self, _: &str) -> Result<f64, AiomeError> {
            Ok(1.0)
        }
        async fn calculate_importance(&self, _: &str) -> Result<f64, AiomeError> {
            Ok(0.5)
        }
        async fn calculate_importance_batch(
            &self,
            q: &[String],
        ) -> Result<Vec<(String, f64)>, AiomeError> {
            Ok(q.iter().map(|s| (s.clone(), 0.5)).collect())
        }
    }

    #[tokio::test]
    async fn test_slm_bridge_strategy_pattern_tdd() {
        let mock = Box::new(MockBackend::default());
        let bridge = SlmBridge::with_backend(mock);

        let entry = SlmMemoryEntry {
            content: "Testing Strategy Pattern".into(),
            category: "Test".into(),
            metadata: None,
        };

        let res = bridge.store_memory(entry).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_slm_bridge_resource_exhaustion() {
        let bridge = SlmBridge::new();
        let huge_content = "A".repeat(1024 * 65);
        let entry = SlmMemoryEntry {
            content: huge_content,
            category: "Test".into(),
            metadata: None,
        };
        let res = bridge.store_memory(entry).await;
        assert!(res.is_err(), "Should reject input exceeding 64KB");
    }

    #[tokio::test]
    async fn test_slm_bridge_validation() {
        assert!(SlmBridge::validate("normal text").is_ok());
        assert!(SlmBridge::validate("   ").is_err()); // empty
        assert!(SlmBridge::validate("text with ; dangerous char").is_err());
    }

    #[tokio::test]
    async fn test_slm_bridge_calculate_importance_green() {
        let bridge = SlmBridge::new();
        // GREEN Phase: calculate_importance は slm trace --json を呼び出し、
        // Poincare スコアを含む重要度を 0.0〜1.0 の範囲で返す。
        let res = bridge.calculate_importance("test query").await;
        assert!(
            res.is_ok(),
            "GREEN Phase: calculate_importance should return a valid result"
        );

        let importance = res.unwrap();
        assert!(
            importance >= 0.0 && importance <= 1.0,
            "Importance should be in [0.0, 1.0]"
        );
    }

    #[tokio::test]
    async fn test_slm_bridge_calculate_importance_batch_green() {
        let bridge = SlmBridge::new();
        let queries = vec!["test query 1".to_string(), "test query 2".to_string()];
        let res = bridge.calculate_importance_batch(&queries).await;
        // バッチ結果は成功するはず（フォールバック含む）
        assert!(
            res.is_ok(),
            "GREEN Phase: batch calculate_importance should succeed"
        );
    }
}
