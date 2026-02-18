//! # The Contract — アクター間通信契約
//!
//! 憲法第2条に基づき、アクター間のやり取りを型安全に定義する。

use serde::{Deserialize, Serialize};
use crate::traits::TrendItem;

/// 監査用メタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMeta {
    pub trace_id: String,
    pub sender_id: String,
}

/// メッセージの基本構造
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<T> {
    pub meta: MessageMeta,
    pub payload: T,
}

// --- Trend クラスター ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendRequest {
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendResponse {
    pub items: Vec<TrendItem>,
}

// --- Concept クラスター ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptRequest {
    pub topic: String,
    pub category: String,
    pub trend_items: Vec<TrendItem>,
    /// 利用可能な演出スタイルの一覧
    pub available_styles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptResponse {
    pub title: String,
    /// 導入部
    pub script_intro: String,
    /// 本編
    pub script_body: String,
    /// 結末
    pub script_outro: String,
    /// 全体共通の画風、ライティング、特定のキャラクター指定 (Subject/Style)
    pub common_style: String,
    /// 採択された演出スタイル (styles.toml のキー)
    pub style_profile: String,
    /// 各シーン固有の描写 (Action/Background) - 必ず3件
    pub visual_prompts: Vec<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

// --- Video クラスター ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoRequest {
    pub prompt: String,
    pub workflow_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoResponse {
    pub output_path: String,
}

// --- Voice クラスター ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceRequest {
    pub text: String,
    pub speaker_id: i32,
    pub style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceResponse {
    pub audio_path: String,
}

// --- Media クラスター ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaRequest {
    pub video_path: String,
    pub audio_path: String,
    pub subtitle_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaResponse {
    pub final_path: String,
}

// --- Workflow クラスター (Phase 5) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRequest {
    pub category: String,
    pub topic: String,
    /// Remix 対象の動画ID (None の場合は新規作成)
    pub remix_id: Option<String>,
    /// スキップ先のステップ (None の場合はフル実行)
    pub skip_to_step: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResponse {
    pub final_video_path: String,
    pub concept: ConceptResponse,
}
