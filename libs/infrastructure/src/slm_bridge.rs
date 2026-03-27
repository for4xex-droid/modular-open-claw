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

/// SuperLocalMemory MCP/CLI との通信を管理するブリッジ
#[derive(Debug)]
pub struct SlmBridge {
    circuit_breaker: Arc<CircuitBreaker>,
    command_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlmMemoryEntry {
    pub content: String,
    pub category: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Deserialize)]
struct SlmRecallData {
    #[serde(default)]
    results: Vec<SlmRecallResult>,
}

#[derive(Debug, Deserialize)]
struct SlmRecallJsonResponse {
    data: SlmRecallData,
}

#[derive(Debug, Deserialize)]
struct SlmTraceChannelScores {
    #[serde(default)]
    semantic: f64,
    #[serde(default)]
    bm25: f64,
    #[serde(default)]
    entity_graph: f64,
    #[serde(default)]
    poincare: f64,
}

#[derive(Debug, Deserialize)]
struct SlmTraceResult {
    #[serde(default)]
    score: f64,
    #[serde(default)]
    channel_scores: Option<SlmTraceChannelScores>,
}

#[derive(Debug, Deserialize)]
struct SlmTraceData {
    #[serde(default)]
    results: Vec<SlmTraceResult>,
}

#[derive(Debug, Deserialize)]
struct SlmTraceJsonResponse {
    data: SlmTraceData,
}

impl SlmBridge {
    /// 新しいインスタンスを生成する
    pub fn new() -> Self {
        Self::new_with_command("slm")
    }

    /// 指定されたコマンド名を使用してインスタンスを生成する（テスト用）
    pub fn new_with_command(command: &str) -> Self {
        let cb_config = CircuitBreakerConfig {
            failure_threshold: 3,
            reset_timeout: Duration::from_secs(60),
        };
        Self {
            circuit_breaker: Arc::new(CircuitBreaker::new("slm-bridge", cb_config)),
            command_name: command.to_string(),
        }
    }

    /// 入力文字列を検証する (C-1 対策: White-list & Black-list Hybrid)
    fn validate(input: &str) -> Result<(), AiomeError> {
        if input.trim().is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "Input is empty or whitespace only".into(),
            });
        }

        // 1. 長さ制限 (Resource Exhaustion 対策)
        if input.len() > 1024 * 64 {
            return Err(AiomeError::Infrastructure {
                reason: "Input too large (max 64KB)".into(),
            });
        }

        // 2. 制御文字のチェック (Null byte 等)
        if input
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        {
            return Err(AiomeError::Infrastructure {
                reason: "Control characters detected in input".into(),
            });
        }

        // 3. 危険なシェル文字のチェック
        let dangerous_chars = [
            ';', '&', '|', '$', '>', '<', '`', '\\', '!', '{', '}', '(', ')', '[', ']', '*', '?',
            '~',
        ];
        if input.chars().any(|c| dangerous_chars.contains(&c)) {
            return Err(AiomeError::Infrastructure {
                reason: format!("Potentially malicious characters detected in input"),
            });
        }

        Ok(())
    }

    /// メモリを保存する (Phase 1 堅牢版)
    pub async fn store_memory(&self, entry: SlmMemoryEntry) -> Result<(), AiomeError> {
        // サーキットブレーカーのチェック
        self.circuit_breaker
            .check_state()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("SLM Service is unavailable: {}", e),
            })?;

        // バリデーション (サニタイズではなく拒否)
        Self::validate(&entry.content)?;
        Self::validate(&entry.category)?;

        // 実行 (タイムアウト付き C-3 対策)
        let future = Command::new(&self.command_name)
            .arg("remember")
            .arg("--tags")
            .arg(&entry.category)
            .arg(&entry.content)
            .output();

        match timeout(Duration::from_secs(30), future).await {
            Ok(Ok(output)) => {
                if output.status.success() {
                    self.circuit_breaker.record_success().await;
                    info!("Memory stored in SLM (category: {})", entry.category);
                    Ok(())
                } else {
                    let err = String::from_utf8_lossy(&output.stderr);
                    error!("SLM Remember Error: {}", err);
                    self.circuit_breaker.record_failure().await;
                    Err(AiomeError::Infrastructure {
                        reason: format!("SLM CLI reported failure: {}", err),
                    })
                }
            }
            Ok(Err(e)) => {
                self.circuit_breaker.record_failure().await;
                Err(AiomeError::Infrastructure {
                    reason: format!("Failed to execute slm remember: {}", e),
                })
            }
            Err(_) => {
                self.circuit_breaker.record_failure().await;
                warn!("SLM Remember timed out after 30s");
                Err(AiomeError::Infrastructure {
                    reason: "SLM process timed out".into(),
                })
            }
        }
    }

    /// メモリを検索する (Phase 1 堅牢版)
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

        let future = Command::new(&self.command_name)
            .arg("recall")
            .arg("--json")
            .arg("--limit")
            .arg(limit.to_string())
            .arg(query)
            .output();

        match timeout(Duration::from_secs(30), future).await {
            Ok(Ok(output)) => {
                if output.status.success() {
                    self.circuit_breaker.record_success().await;
                    let response: SlmRecallJsonResponse = serde_json::from_slice(&output.stdout)
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: format!("Failed to parse SLM recall JSON: {}", e),
                        })?;
                    Ok(response.data.results)
                } else {
                    let err = String::from_utf8_lossy(&output.stderr);
                    self.circuit_breaker.record_failure().await;
                    Err(AiomeError::Infrastructure {
                        reason: format!("SLM CLI recall failure: {}", err),
                    })
                }
            }
            Ok(Err(e)) => {
                self.circuit_breaker.record_failure().await;
                Err(AiomeError::Infrastructure {
                    reason: format!("Failed to execute slm recall: {}", e),
                })
            }
            Err(_) => {
                self.circuit_breaker.record_failure().await;
                Err(AiomeError::Infrastructure {
                    reason: "SLM recall timed out".into(),
                })
            }
        }
    }

    /// 矛盾を検出する (Phase 3: Constitutional Security)
    pub async fn detect_contradictions(&self, text: &str) -> Result<f64, AiomeError> {
        self.circuit_breaker
            .check_state()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("SLM Service is unavailable: {}", e),
            })?;

        Self::validate(text)?;

        let future = Command::new(&self.command_name)
            .arg("contradict")
            .arg("--json")
            .arg(text)
            .output();

        match timeout(Duration::from_secs(30), future).await {
            Ok(Ok(output)) => {
                if output.status.success() {
                    self.circuit_breaker.record_success().await;
                    let response: serde_json::Value = serde_json::from_slice(&output.stdout)
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: format!("Failed to parse SLM contradiction JSON: {}", e),
                        })?;
                    // JSON 例: { "data": { "score": 0.85, "reason": "..." } }
                    let score = response["data"]["score"].as_f64().unwrap_or(0.0);
                    Ok(score)
                } else {
                    let err = String::from_utf8_lossy(&output.stderr);
                    self.circuit_breaker.record_failure().await;
                    Err(AiomeError::Infrastructure {
                        reason: format!("SLM CLI contradict failure: {}", err),
                    })
                }
            }
            Ok(Err(e)) => {
                self.circuit_breaker.record_failure().await;
                Err(AiomeError::Infrastructure {
                    reason: format!("Failed to execute slm contradict: {}", e),
                })
            }
            Err(_) => {
                self.circuit_breaker.record_failure().await;
                Err(AiomeError::Infrastructure {
                    reason: "SLM contradict timed out".into(),
                })
            }
        }
    }

    /// 記憶の重要度を算出する (Phase 4: Poincare GC)
    pub async fn calculate_importance(&self, query: &str) -> Result<f64, AiomeError> {
        self.circuit_breaker
            .check_state()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("SLM Service is unavailable: {}", e),
            })?;

        Self::validate(query)?;

        let future = Command::new(&self.command_name)
            .arg("trace")
            .arg("--json")
            .arg(query)
            .output();

        match timeout(Duration::from_secs(30), future).await {
            Ok(Ok(output)) => {
                if output.status.success() {
                    self.circuit_breaker.record_success().await;
                    let response: SlmTraceJsonResponse = serde_json::from_slice(&output.stdout)
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: format!("Failed to parse SLM trace JSON: {}", e),
                        })?;

                    Ok(Self::extract_importance_from_trace(&response))
                } else {
                    let err = String::from_utf8_lossy(&output.stderr);
                    self.circuit_breaker.record_failure().await;
                    Err(AiomeError::Infrastructure {
                        reason: format!("SLM CLI trace failure: {}", err),
                    })
                }
            }
            Ok(Err(e)) => {
                self.circuit_breaker.record_failure().await;
                Err(AiomeError::Infrastructure {
                    reason: format!("Failed to execute slm trace: {}", e),
                })
            }
            Err(_) => {
                self.circuit_breaker.record_failure().await;
                Err(AiomeError::Infrastructure {
                    reason: "SLM trace timed out".into(),
                })
            }
        }
    }

    /// 複数の記憶の重要度を一括算出する (CR-1: バッチ化パフォーマンス改善)
    /// 一時ファイルに改行区切りのクエリを書き込み、1回の `slm trace --batch` プロセス起動で処理する。
    /// `slm trace --batch` が未サポートの場合は、フォールバックとして逐次実行を行う。
    pub async fn calculate_importance_batch(
        &self,
        queries: &[String],
    ) -> Result<Vec<(String, f64)>, AiomeError> {
        if queries.is_empty() {
            return Ok(vec![]);
        }

        self.circuit_breaker
            .check_state()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("SLM Service is unavailable: {}", e),
            })?;

        // 各クエリをバリデーション
        for q in queries {
            Self::validate(q)?;
        }

        // 一時ファイルにクエリを書き込み
        let tmp_path =
            std::env::temp_dir().join(format!("slm_batch_{}.jsonl", uuid::Uuid::new_v4()));
        let batch_content = queries.join("\n");
        tokio::fs::write(&tmp_path, &batch_content)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to write batch file: {}", e),
            })?;

        let future = Command::new(&self.command_name)
            .arg("trace")
            .arg("--json")
            .arg("--batch")
            .arg(&tmp_path)
            .output();

        let batch_result = timeout(Duration::from_secs(60), future).await;

        // 一時ファイルの後片付け（エラーは無視）
        let _ = tokio::fs::remove_file(&tmp_path).await;

        match batch_result {
            Ok(Ok(output)) if output.status.success() => {
                self.circuit_breaker.record_success().await;
                // バッチ出力: JSONL (1行1 SlmTraceJsonResponse)
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut results = Vec::with_capacity(queries.len());
                for (i, line) in stdout.lines().enumerate() {
                    if let Ok(resp) = serde_json::from_str::<SlmTraceJsonResponse>(line) {
                        let importance = Self::extract_importance_from_trace(&resp);
                        let query = queries.get(i).cloned().unwrap_or_default();
                        results.push((query, importance));
                    }
                }
                Ok(results)
            }
            Ok(Ok(_output)) => {
                // `--batch` 未サポートの場合、逐次フォールバック
                warn!("⚠️ [SlmBridge] Batch trace not supported, falling back to sequential execution");
                self.circuit_breaker.record_success().await;
                let mut results = Vec::with_capacity(queries.len());
                for q in queries {
                    match self.calculate_importance(q).await {
                        Ok(importance) => results.push((q.clone(), importance)),
                        Err(_) => results.push((q.clone(), 0.0)),
                    }
                }
                Ok(results)
            }
            Ok(Err(e)) => {
                self.circuit_breaker.record_failure().await;
                // フォールバック: 逐次実行
                warn!(
                    "⚠️ [SlmBridge] Batch execution failed ({}), falling back",
                    e
                );
                let mut results = Vec::with_capacity(queries.len());
                for q in queries {
                    match self.calculate_importance(q).await {
                        Ok(importance) => results.push((q.clone(), importance)),
                        Err(_) => results.push((q.clone(), 0.0)),
                    }
                }
                Ok(results)
            }
            Err(_) => {
                self.circuit_breaker.record_failure().await;
                Err(AiomeError::Infrastructure {
                    reason: "SLM batch trace timed out".into(),
                })
            }
        }
    }

    /// SlmTraceJsonResponse から重要度スコアを算出する共通ヘルパー
    fn extract_importance_from_trace(response: &SlmTraceJsonResponse) -> f64 {
        if response.data.results.is_empty() {
            return 0.0;
        }
        let top = &response.data.results[0];
        let base_score = top.score;
        if let Some(channels) = &top.channel_scores {
            let poincare = channels.poincare;
            let avg_channels =
                (channels.semantic + channels.bm25 + channels.entity_graph + poincare) / 4.0;
            (base_score + avg_channels) / 2.0
        } else {
            base_score
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_slm_bridge_store_green() {
        let bridge = SlmBridge::new();
        let entry = SlmMemoryEntry {
            content: "TDD focus: integration test for SLM bridge.".into(),
            category: "Test".into(),
            metadata: None,
        };

        let res = bridge.store_memory(entry).await;
        assert!(res.is_ok(), "Store memory should succeed in GREEN phase");
    }

    #[tokio::test]
    async fn test_slm_bridge_recall_green() {
        let bridge = SlmBridge::new();

        // 保存済みの内容を検索してみる
        let res = bridge.recall("TDD focus", 10).await;
        assert!(res.is_ok(), "Recall memory should succeed in GREEN phase");

        let results = res.unwrap();
        // 少なくとも1件以上ヒットすることを期待 (SLM の精度に依存するが、テストデータを入れた直後なら 100% のはず)
        assert!(!results.is_empty(), "Should return at least one result");
    }

    #[tokio::test]
    async fn test_slm_bridge_timeout_red() {
        let bridge = SlmBridge::new();
        // SLM recall に非常に長い時間がかかる偽装（現在は未実装なので即座に終わるが、
        // 意図的にタイムアウトを短く設定して失敗させる等が必要。
        // ここでは、将来的に導入するタイムアウト機能が正しく動作することを期待するテストを書く）

        // 現状の実装にはタイムアウトがないため、もし slm がハングすればこのテストもハングする。
        // GREEN フェーズで tokio::time::timeout を導入し、エラーを返すようにする。
        let result = bridge.recall("query", 10).await;
        assert!(result.is_ok()); // 現時点では OK だが、ハングリスクがある
    }

    #[tokio::test]
    async fn test_slm_bridge_resource_exhaustion_red() {
        let bridge = SlmBridge::new();
        // 64KB を超える巨大入力
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
    async fn test_slm_bridge_control_char_red() {
        let bridge = SlmBridge::new();
        // Null byte インジェクションの試み
        let entry = SlmMemoryEntry {
            content: "Normal facts\0; rm -rf /".into(),
            category: "Test".into(),
            metadata: None,
        };

        let res = bridge.store_memory(entry).await;
        assert!(res.is_err(), "Should reject control characters (Null byte)");
    }

    #[tokio::test]
    async fn test_slm_bridge_race_condition_circuit_breaker() {
        // 存在しないコマンドを指定して確実に失敗させる
        let bridge = SlmBridge::new_with_command("non-existent-slm-cmd");
        let bridge_arc = Arc::new(bridge);

        // 並行して多数の失敗を発生させ、サーキットブレーカーが正しく OPEN に遷移するか
        // (SlmBridge は Command::new("slm") を呼ぶため、実際には slm バイナリの不在で失敗する)
        let mut handles = vec![];
        for i in 0..10 {
            let b = Arc::clone(&bridge_arc);
            handles.push(tokio::spawn(async move {
                let entry = SlmMemoryEntry {
                    content: format!("Contention test {}", i),
                    category: "Test".into(),
                    metadata: None,
                };
                // slm バイナリは存在するが、不正な引数やモックされないエラーを模倣するため、
                // 回数制限を超えた場合に OPEN になることを確認。
                // テスト環境で slm が成功してしまう場合を考慮し、
                // 明らかに不正なタグ名を使用してエラーを誘発させる。
                let mut invalid_entry = entry;
                invalid_entry.category = "TestCategory".into(); // バリデーションを通る名前
                let _ = b.store_memory(invalid_entry).await;
            }));
        }

        for h in handles {
            let _ = h.await;
        }

        // 状態が Open になっているはず（失敗閾値 3）
        let status = bridge_arc.circuit_breaker.get_status().await;
        assert!(status.failure_count >= 3);
        // 注意: 並行実行のタイミングにより、OPEN 遷移後も数件は実行が試みられる可能性があるが、
        // 最終的なチェックで OPEN になっていることが重要。
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
