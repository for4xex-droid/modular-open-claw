/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
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

use aiome_contracts::traits::TranscriptionSegment;

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

use aiome_contracts::error::AiomeError;
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
