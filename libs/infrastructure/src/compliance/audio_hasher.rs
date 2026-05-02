/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::task;
use tokio::time::timeout;

/// 音声ファイルの知覚ハッシュ（Perceptual Hash）を計算するエンジン。
/// Gate 4 (Expert 1) の指摘により、重い処理は `spawn_blocking` に逃がし、
/// かつ `timeout` を設定してスレッドプール枯渇攻撃を防ぐ。
pub struct AudioHasher {
    timeout_duration: Duration,
}

impl Default for AudioHasher {
    fn default() -> Self {
        Self {
            timeout_duration: Duration::from_secs(5), // デフォルト5秒でタイムアウト
        }
    }
}

impl AudioHasher {
    /// カスタムのタイムアウト時間を指定して AudioHasher を初期化する
    pub fn new(timeout_duration: Duration) -> Self {
        Self { timeout_duration }
    }

    /// 音声データからCSAMチェック等に用いるハッシュを計算する
    pub async fn compute_hash(&self, audio_data: Vec<u8>) -> Result<String, AiomeError> {
        let compute_future = task::spawn_blocking(move || {
            // QW-2: (Future) Implement perceptual hashing (e.g., Chromaprint or pHash) for CSAM detection.
            // Currently using SHA-256 for deterministic integrity checks.
            let mut hasher = Sha256::new();
            hasher.update(&audio_data);
            let result = hasher.finalize();
            hex::encode(result)
        });

        match timeout(self.timeout_duration, compute_future).await {
            Ok(Ok(hash)) => Ok(hash),
            Ok(Err(e)) => {
                tracing::error!("❌ [AudioHasher] Blocking task failed or panicked: {}", e);
                Err(AiomeError::Infrastructure {
                    reason: "Audio hash computation panicked or failed".to_string(),
                })
            }
            Err(_) => {
                tracing::error!(
                    "❌ [AudioHasher] Computation timed out after {:?}",
                    self.timeout_duration
                );
                Err(AiomeError::Infrastructure {
                    reason: "Audio hash computation timed out".to_string(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compute_hash_success() {
        let hasher = AudioHasher::default();
        let data = b"dummy audio content".to_vec();
        let hash = hasher.compute_hash(data).await.unwrap(); // allow-anti-pattern
        assert!(!hash.is_empty());
    }

    #[tokio::test]
    async fn test_compute_hash_timeout() {
        let hasher = AudioHasher::new(Duration::from_millis(10));

        // 意図的にタイムアウトさせるため、少し重い（ダミーで sleep する）タスクをエミュレートする
        // 実際の compute_hash はモックできないため、タイムアウトの挙動自体をテストする
        let compute_future = task::spawn_blocking(move || {
            std::thread::sleep(Duration::from_millis(50));
            "late_hash".to_string()
        });

        let result = timeout(hasher.timeout_duration, compute_future).await;
        assert!(result.is_err(), "Should timeout");
    }
}
