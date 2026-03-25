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

use aiome_contracts::error::AiomeError;
use async_trait::async_trait;

#[async_trait]
pub trait LipSyncProvider: Send + Sync {
    /// 与えられた音声データ (PCM等の一定フォーマット想定) からリップシンクフレームシーケンスを生成する
    async fn generate_frames(&self, audio_data: &[u8]) -> Result<Vec<LipSyncFrame>, AiomeError>;
}
