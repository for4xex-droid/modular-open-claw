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
    pub input_image: Option<String>,
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
pub struct CustomStyle {
    // --- 視覚演出 (Cameraman) ---
    pub zoom_speed: Option<f64>,
    pub pan_intensity: Option<f64>,
    
    // --- 音響演出 (SoundMixer) ---
    pub bgm_volume: Option<f32>,
    pub ducking_threshold: Option<f32>,
    pub ducking_ratio: Option<f32>,
    pub fade_duration: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRequest {
    pub category: String,
    pub topic: String,
    /// Remix 対象の動画ID (None の場合は新規作成)
    pub remix_id: Option<String>,
    /// スキップ先のステップ (None の場合はフル実行)
    pub skip_to_step: Option<String>,
    
    // --- Phase 8.5 Remix Lab Extensions ---
    /// 適用するスタイル名 (styles.toml のキー)
    #[serde(default)]
    pub style_name: String,
    /// ユーザーによるカスタム調整 (None の場合はプリセット通り)
    pub custom_style: Option<CustomStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResponse {
    pub final_video_path: String,
    pub concept: ConceptResponse,
}

// --- Phase 10-F: The JSON Contract (KarmaDirectives) ---

/// LLM Structured Output の厳密な型定義。
/// Samsara の合成フェーズで LLM が出力し、`jobs.karma_directives` にJSON文字列として格納される。
/// `CHECK(json_valid(karma_directives))` と連携し、不正なJSONをDBレイヤーで物理的に弾く。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KarmaDirectives {
    /// 今回生成する動画のトピック (例: "最新のAIニュースまとめ")
    pub topic: String,

    /// skills.md 内の最適なワークフロー/スタイル名 (例: "tech_news_v1")
    pub style: String,

    /// ポジティブプロンプトへの追加指示 (Karmaから導出)
    pub positive_prompt_additions: Option<String>,

    /// ネガティブプロンプトへの追加指示 (例: "ネオンカラーは使わないこと")
    pub negative_prompt_additions: Option<String>,

    /// 全般的な実行ノート・注意事項 (Karmaから導出)
    pub execution_notes: Option<String>,

    /// LLM 自身のこの生成に対する自信度 (0-100)。weight 計算の基準値。
    pub confidence_score: u8,
}
