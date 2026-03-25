/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::error::AiomeError;
use aiome_contracts::traits::{TranscriptionEngine, TranscriptionResult, TranscriptionSegment};
use async_trait::async_trait;
use std::path::Path;

use aiome_core::security::RuntimeJail;
use std::sync::Arc;
use tracing::{info, warn};

// CLI constants for insanely-fast-whisper
const CLI_BINARY: &str = "insanely-fast-whisper";
const FLAG_FILE: &str = "--file-name";
const FLAG_DEVICE: &str = "--device-id";
const FLAG_OUTPUT: &str = "--transcript-path";

/// insanely-fast-whisper を使用した STT アダプタ
pub struct WhisperTranscriptionAdapter {
    jail: Arc<dyn RuntimeJail>,
    enabled: bool,
}

impl WhisperTranscriptionAdapter {
    pub fn new(jail: Arc<dyn RuntimeJail>, enabled: bool) -> Self {
        Self { jail, enabled }
    }
}

#[derive(Debug, serde::Deserialize)]
struct WhisperOutputSegment {
    text: String,
    timestamp: (f32, f32),
}

#[derive(Debug, serde::Deserialize)]
struct WhisperOutput {
    text: String,
    chunks: Vec<WhisperOutputSegment>,
}

#[async_trait]
impl TranscriptionEngine for WhisperTranscriptionAdapter {
    async fn transcribe(&self, audio_path: &Path) -> Result<TranscriptionResult, AiomeError> {
        if !self.enabled {
            return Err(AiomeError::Infrastructure {
                reason: "STT is disabled".into(),
            });
        }

        let output_json = audio_path.with_extension("json");
        let output_json_str = output_json.to_string_lossy();
        let audio_path_str = audio_path.to_string_lossy();

        // 1. insanely-fast-whisper CLI の実行
        // GPU デバイスの指定 (Mac = mps, NVIDIA = 0,1,...)
        let device = if cfg!(target_os = "macos") {
            "mps"
        } else {
            "0"
        };

        let cmd = format!(
            "{} {} {} {} {} {} {}",
            CLI_BINARY,
            FLAG_FILE,
            audio_path_str,
            FLAG_DEVICE,
            device,
            FLAG_OUTPUT,
            output_json_str
        );

        info!("🎙️ [WhisperTranscription] Executing: {}", cmd);

        // PythonForge プロファイルで実行 (ネットワーク禁止)
        self.jail
            .safe_exec_with_profile(&cmd, aiome_core::security::SandboxProfile::PythonForge)
            .await?;

        // 2. 出力 JSON のパース
        let json_content =
            std::fs::read_to_string(&output_json).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to read transcription output: {}", e),
            })?;

        let whisper_res: WhisperOutput =
            serde_json::from_str(&json_content).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to parse transcription JSON: {}", e),
            })?;

        // 3. TranscriptionResult への変換
        let segments = whisper_res
            .chunks
            .into_iter()
            .map(|c| {
                TranscriptionSegment {
                    text: c.text,
                    start: c.timestamp.0,
                    end: c.timestamp.1,
                    confidence: 1.0, // insanely-fast-whisper 0.1.x では confidence が出ない場合があるため 1.0 固定
                }
            })
            .collect();

        // クリーンアップ
        let _ = std::fs::remove_file(output_json);

        Ok(TranscriptionResult {
            text: whisper_res.text,
            language: "unknown".into(), // Whisper は自動検知するが JSON に含まれない場合がある
            segments,
        })
    }

    async fn health_check(&self) -> Result<bool, AiomeError> {
        if !self.enabled {
            return Ok(false);
        }

        // 実際にバイナリが実行可能かチェック
        match self
            .jail
            .safe_exec_with_profile(
                &format!("{} --help", CLI_BINARY),
                aiome_core::security::SandboxProfile::PythonForge,
            )
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("🎙️ [WhisperTranscription] Health check failed: CLI binary not found or not executable. Error: {:?}", e);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::BastionGuard;
    use aiome_core::security::PermissionManifest;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_whisper_transcribe_initialization() {
        let manifest = PermissionManifest::default();
        let jail = Arc::new(BastionGuard::new_internal(manifest));
        let adapter = WhisperTranscriptionAdapter::new(jail, true);

        // CI 環境やバイナリ未インストール環境では false になるのが正しい挙動
        let _ = adapter.health_check().await;
    }

    #[tokio::test]
    async fn test_health_check_behavior() {
        let manifest = PermissionManifest::default();
        let jail = Arc::new(BastionGuard::new_internal(manifest));

        // 1. Enabled=true But Binary Missing -> should be false
        let adapter = WhisperTranscriptionAdapter::new(jail.clone(), true);
        let status = adapter.health_check().await.unwrap();
        // ここでは false か true かは環境に依存するが、panic しないことを確認
        info!("Health status (Enabled=true): {}", status);

        // 2. Enabled=false -> Must be false
        let adapter_disabled = WhisperTranscriptionAdapter::new(jail, false);
        assert!(!adapter_disabled.health_check().await.unwrap());
    }

    // Note: 実機テスト (test_whisper_transcribe_live) は
    // insanely-fast-whisper がインストールされた環境でのみ実行可能とする。
}
