/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Default,
    strum_macros::Display,
    strum_macros::EnumString,
)]
#[strum(serialize_all = "PascalCase")]
pub enum TtsStatus {
    #[default]
    NotRequested,
    Generating,
    Ready,
    Failed,
}

impl TtsStatus {
    pub fn from_string(s: &str) -> Self {
        use std::str::FromStr;
        Self::from_str(s).unwrap_or(Self::NotRequested)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Expression {
    pub id: String,
    pub content: String,                          // 生成されたテキスト
    pub emotion: String, // 推定された感情 ("curious", "reflective", "excited", etc.)
    pub karma_refs: Vec<String>, // 参照したKarmaのID (JSON array serialized in DB)
    pub audio_path: Option<String>, // DP-9: 音声ファイルのパス
    pub duration_ms: Option<i32>, // DP-9: 音声の長さ(ms)
    pub tts_status: TtsStatus, // Phase 10.1a: TTS生成ステータス
    pub avatar_params: Option<serde_json::Value>, // Phase 7: Inochi2D/VRM 感情パラメータ
    pub created_at: String,
}

/// DP-10: リソース使用量とコストの監視用ログ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageLog {
    pub id: Option<i64>,
    pub job_id: Option<String>,
    pub provider_name: String, // "Gemini", "OpenAI", "ElevenLabs" etc.
    pub model_name: String,
    pub usage_type: String, // "tokens", "chars", "seconds"
    pub amount: i64,
    pub estimated_cost_usd: f64,
    pub created_at: String,
}
