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
    pub trend_items: Vec<TrendItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptResponse {
    pub title: String,
    pub script: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResponse {
    pub final_video_path: String,
    pub concept: ConceptResponse,
}
