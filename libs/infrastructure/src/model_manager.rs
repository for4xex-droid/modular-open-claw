/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::error::AiomeError;
use std::path::{Path, PathBuf};
use tracing::info;

#[cfg(feature = "native-inference")]
use hf_hub::api::tokio::ApiBuilder;

/// モデルのダウンロードとキャッシュ管理を行うユーティリティ
#[derive(Debug, Clone)]
pub struct ModelManager {
    cache_dir: PathBuf,
}

impl ModelManager {
    /// 新規インスタンスを生成
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
        }
    }

    /// デフォルトのキャッシュディレクトリ (~/.aiome/models) を使用して生成
    pub fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        Self::new(Path::new(&home).join(".aiome").join("models"))
    }

    /// 指定された HF Repo からファイルをダウンロード（またはキャッシュから取得）
    pub async fn get_model_file(
        &self,
        repo_id: &str,
        filename: &str,
    ) -> Result<PathBuf, AiomeError> {
        #[cfg(not(feature = "native-inference"))]
        return Err(AiomeError::Infrastructure {
            reason: "Native inference feature is disabled".into(),
        });

        #[cfg(feature = "native-inference")]
        {
            info!("🔍 [ModelManager] Checking model: {}/{}", repo_id, filename);
            let api = ApiBuilder::new()
                .with_cache_dir(self.cache_dir.clone())
                .build()
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to initialize HF Hub API: {}", e),
                })?;

            let repo = api.model(repo_id.to_string());
            let path = repo
                .get(filename)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!(
                        "Failed to download model {} from {}: {}",
                        filename, repo_id, e
                    ),
                })?;

            Ok(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_model_manager_path_resolution() {
        let manager = ModelManager::new("/tmp/aiome-models-test");
        assert_eq!(manager.cache_dir, PathBuf::from("/tmp/aiome-models-test"));
    }

    #[tokio::test]
    #[cfg(feature = "native-inference")]
    async fn test_model_manager_download_skeleton() {
        let manager = ModelManager::new("/tmp/aiome-models-test");
        // 実際にはネットワーク通信が発生するため、ここではインターフェースの疎通のみ確認
        // (テスト用ダミーリポジトリを指定して失敗することを確認するなど)
        let res = manager.get_model_file("invalid/repo", "config.json").await;
        assert!(res.is_err());
    }
}
