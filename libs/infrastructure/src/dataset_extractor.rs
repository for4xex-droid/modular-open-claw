/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::traits::SoulStore;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// DatasetExtractor: Agentの記憶（Soul）からLoRA学習用データを抽出・変換するETLコンポーネント。
pub struct DatasetExtractor {
    output_dir: PathBuf,
}

impl DatasetExtractor {
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }

    /// `soul_id` に紐づく履歴を取得し、MLX用の JSONL (`{"text": "..."}`) ファイルとして書き出す。
    /// 返り値はそのファイルのパス。
    pub async fn extract_to_jsonl<T: SoulStore + ?Sized>(
        &self,
        soul_store: &T,
        soul_id: &str,
        job_id: &str,
    ) -> Result<PathBuf, AiomeError> {
        let soul_data = soul_store.load_soul(soul_id).await?;

        let soul_json = match soul_data {
            Some(v) => v,
            None => {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Soul data not found for ID: {}", soul_id),
                })
            }
        };

        // "experiences" を安全に抽出
        let experiences = match soul_json.get("experiences").and_then(|e| e.as_array()) {
            Some(arr) if !arr.is_empty() => arr,
            _ => {
                return Err(AiomeError::Infrastructure {
                    reason: format!("No experiences found for giving soul {}", soul_id),
                })
            }
        };

        // フォルダ作成
        tokio::fs::create_dir_all(&self.output_dir)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to create output directory: {}", e),
            })?;

        // jsonl フルのパス (競合を避けるため job_id で一意に)
        let output_file = self.output_dir.join(format!("{}_dataset.jsonl", job_id));
        let mut file = tokio::fs::File::create(&output_file).await.map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("Failed to create output file: {}", e),
            }
        })?;

        // MLXのファインチューニング用に、一連の会話を単一のブロック文脈として構築する
        let mut conversation_block = String::new();

        for exp in experiences {
            let role = exp
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("unknown");
            let content = exp.get("content").and_then(|c| c.as_str()).unwrap_or("");

            conversation_block.push_str(&format!("{}: {}\n", role, content));
        }

        let jsonl_line = serde_json::json!({
            "text": conversation_block.trim_end()
        });

        let line_str =
            serde_json::to_string(&jsonl_line).map_err(|e| AiomeError::Infrastructure {
                reason: format!("JSONL serialization failed: {}", e),
            })?;

        file.write_all(format!("{}\n", line_str).as_bytes())
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to write to dataset file: {}", e),
            })?;

        file.flush().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to flush dataset file: {}", e),
        })?;

        Ok(output_file)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    /// SoulStoreの単純なモック
    #[derive(Clone)]
    struct MockSoulStore {
        pub return_data: Option<serde_json::Value>,
    }

    #[async_trait]
    impl SoulStore for MockSoulStore {
        async fn load_soul(&self, _id: &str) -> Result<Option<serde_json::Value>, AiomeError> {
            Ok(self.return_data.clone())
        }

        async fn store_soul_fragment(&self, _yaml: &str, _hash: &str) -> Result<(), AiomeError> {
            Ok(())
        }

        async fn fetch_latest_soul_fragment(&self) -> Result<Option<(String, String)>, AiomeError> {
            Ok(None)
        }

        async fn archive_lora_model(
            &self,
            _soul_id: &str,
            _gen: u32,
            _hash: &str,
            _path: &str,
            _base: &str,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_extract_to_jsonl_success() {
        // Arrange
        let mock_json = serde_json::json!({
            "experiences": [
                { "role": "user", "content": "Hello\nWorld" },
                { "role": "agent", "content": "Hi there!" }
            ]
        });
        let store = Arc::new(MockSoulStore {
            return_data: Some(mock_json),
        });

        let dump_dir = std::env::temp_dir().join("aiome_dataset_extractor_test");
        fs::create_dir_all(&dump_dir).await.unwrap(); // allow-anti-pattern

        let extractor = DatasetExtractor::new(dump_dir.clone());

        // Act
        let result = extractor
            .extract_to_jsonl(&*store, "test_soul_123", "job_123")
            .await;

        // Assert
        assert!(result.is_ok(), "Needs to successfully write file");
        let path = result.unwrap(); // allow-anti-pattern
        assert!(path.exists());

        // ファイルの中身をパースしてJSONL形式か確認
        let content = fs::read_to_string(&path).await.unwrap(); // allow-anti-pattern
        let lines: Vec<&str> = content.lines().collect();

        // 期待値: 会話ブロック全体が1つの文脈文字列として "\n" (改行コード) 結合された1行のJSONが出力されること
        assert_eq!(
            lines.len(),
            1,
            "Should contain exactly 1 line representing the whole conversation block"
        );

        let block_line: serde_json::Value = serde_json::from_str(lines[0]).unwrap(); // allow-anti-pattern
        let expected_text = "user: Hello\nWorld\nagent: Hi there!";
        assert_eq!(block_line["text"].as_str().unwrap(), expected_text); // allow-anti-pattern

        // Cleanup
        fs::remove_dir_all(&dump_dir).await.unwrap_or(());
    }

    #[tokio::test]
    async fn test_extract_to_jsonl_empty() {
        // Arrange
        let mock_json = serde_json::json!({
            "experiences": []
        });
        let store = Arc::new(MockSoulStore {
            return_data: Some(mock_json),
        });

        let dump_dir = std::env::temp_dir().join("aiome_dataset_extractor_test_empty");
        fs::create_dir_all(&dump_dir).await.unwrap(); // allow-anti-pattern

        let extractor = DatasetExtractor::new(dump_dir.clone());

        // Act
        let result = extractor
            .extract_to_jsonl(&*store, "test_soul_444", "job_444")
            .await;

        // Assert
        assert!(
            result.is_err(),
            "Extraction should fail if no data available"
        );

        // Cleanup
        fs::remove_dir_all(&dump_dir).await.unwrap_or(());
    }
}
