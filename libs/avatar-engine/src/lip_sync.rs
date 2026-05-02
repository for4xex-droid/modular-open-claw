/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Viseme {
    Closed,
    AA,
    IH,
    OU,
    EE,
    OH,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LipSyncFrame {
    pub timestamp_ms: u64,
    pub mouth_open: f32,
    pub viseme: Viseme,
}

use aiome_core_contracts::traits::TranscriptionSegment;

impl LipSyncFrame {
    /// 文字起こしセグメントから LipSync フレームを生成する (簡易版)
    pub fn from_segment(segment: &TranscriptionSegment) -> Self {
        let first_char = segment.text.trim().chars().next().unwrap_or(' ');
        let viseme = match first_char.to_ascii_lowercase() {
            'a' | 'あ' => Viseme::AA,
            'i' | 'い' => Viseme::IH,
            'u' | 'う' => Viseme::OU,
            'e' | 'え' => Viseme::EE,
            'o' | 'お' => Viseme::OH,
            _ => Viseme::AA, // Default to AA for simplicity
        };

        Self {
            timestamp_ms: (segment.start * 1000.0) as u64,
            mouth_open: segment.confidence, // 信頼度を口の開き具合に反映 (仮)
            viseme,
        }
    }
}

use aiome_core_contracts::error::AiomeError;
use async_trait::async_trait;

#[async_trait]
pub trait LipSyncProvider: Send + Sync {
    /// 与えられた音声データ (PCM等の一定フォーマット想定) からリップシンクフレームシーケンスを生成する
    async fn generate_frames(&self, audio_data: &[u8]) -> Result<Vec<LipSyncFrame>, AiomeError>;

    /// 指定された文字起こし結果からフレームを補完する
    fn generate_from_transcription(&self, segments: &[TranscriptionSegment]) -> Vec<LipSyncFrame> {
        segments.iter().map(LipSyncFrame::from_segment).collect()
    }
}

/// シンプルなリップシンクエンジン
/// 実際のオーディオデコードが不要なダミー分析、あるいは将来のWav2Vec連携に使用
#[derive(Debug, Default)]
pub struct SimpleLipSyncEngine;

#[async_trait]
impl LipSyncProvider for SimpleLipSyncEngine {
    async fn generate_frames(&self, audio_data: &[u8]) -> Result<Vec<LipSyncFrame>, AiomeError> {
        // 仮実装: バイトサイズから大まかな音声長を推測し（16kHz 16bit想定 = 32KB/sec）、100ms毎にフレームを生成
        let simulated_duration_ms = (audio_data.len() as f64 / 32_000.0 * 1000.0) as u64;
        let duration_ms = simulated_duration_ms.max(500); // 最低500ms

        let mut frames = Vec::new();
        for t in (0..duration_ms).step_by(100) {
            frames.push(LipSyncFrame {
                timestamp_ms: t,
                mouth_open: if (t / 100) % 2 == 0 { 0.8 } else { 0.2 }, // 開閉を繰り返す
                viseme: if (t / 100) % 2 == 0 {
                    Viseme::AA
                } else {
                    Viseme::EE
                },
            });
        }

        Ok(frames)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_lip_sync_engine_frames() {
        let engine = SimpleLipSyncEngine;
        // 64KB (約2秒分のダミー音声)
        let dummy_audio = vec![0u8; 64000];
        let frames = engine.generate_frames(&dummy_audio).await.unwrap(); // allow-anti-pattern

        assert!(!frames.is_empty(), "Frames should be generated");
        assert_eq!(
            frames.len(),
            20,
            "Should generate ~20 frames for 2 seconds (100ms interval)"
        );

        let first_frame = &frames[0];
        assert_eq!(first_frame.timestamp_ms, 0);
        assert_eq!(first_frame.mouth_open, 0.8);
    }

    #[test]
    fn test_lip_sync_from_transcription() {
        let engine = SimpleLipSyncEngine;
        let segments = vec![
            TranscriptionSegment {
                start: 0.0,
                end: 0.5,
                text: "あ".to_string(),
                confidence: 0.9,
            },
            TranscriptionSegment {
                start: 0.5,
                end: 1.0,
                text: "い".to_string(),
                confidence: 0.85,
            },
        ];

        let frames = engine.generate_from_transcription(&segments);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].timestamp_ms, 0);
        assert!(matches!(frames[0].viseme, Viseme::AA));
        assert_eq!(frames[1].timestamp_ms, 500);
        assert!(matches!(frames[1].viseme, Viseme::IH));
    }
}
