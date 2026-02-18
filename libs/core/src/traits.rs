//! # ドメイントレイト定義
//!
//! ShortsFactory の4つのツールモジュールのインターフェースを定義する。
//! 具体実装は `libs/infrastructure` に配置する（依存性逆転の原則）。

use crate::error::FactoryError;
use async_trait::async_trait;
use std::path::PathBuf;

/// トレンド調査ツール (TrendSonar)
///
/// X, Google Trends, 5ch 等から今バズっているテーマを取得する。
#[async_trait]
pub trait TrendSource: Send + Sync {
    /// 指定カテゴリのトレンドキーワードを取得
    async fn get_trends(&self, category: &str) -> Result<Vec<TrendItem>, FactoryError>;
}

/// トレンド情報の1件分
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrendItem {
    /// キーワード
    pub keyword: String,
    /// ソース (例: "X", "GoogleTrends", "5ch")
    pub source: String,
    /// スコア (高いほど注目度が高い)
    pub score: f64,
}

/// 動画生成ツール (ComfyBridge)
///
/// ComfyUI API を通じて画像/動画を生成する。
#[async_trait]
pub trait VideoGenerator: Send + Sync {
    /// ワークフローを実行し、生成されたファイルのパスを返す
    async fn generate_video(
        &self,
        prompt: &str,
        workflow_id: &str,
    ) -> Result<PathBuf, FactoryError>;

    /// ComfyUI の接続状態を確認
    async fn health_check(&self) -> Result<bool, FactoryError>;
}

/// メディア編集ツール (MediaForge)
///
/// FFmpeg を使って動画・音声・字幕を合成する。
#[async_trait]
pub trait MediaEditor: Send + Sync {
    /// 動画、音声、字幕を合成して最終出力を生成
    async fn combine_assets(
        &self,
        video: &PathBuf,
        audio: &PathBuf,
        subtitle: Option<&PathBuf>,
    ) -> Result<PathBuf, FactoryError>;

    /// 動画をショート用にリサイズ (9:16, 1080x1920)
    async fn resize_for_shorts(&self, input: &PathBuf) -> Result<PathBuf, FactoryError>;
}

/// ログ・通知ツール (FactoryLog)
///
/// 稼働ログをSQLiteに記録し、必要に応じてSlack/Discordに通知する。
#[async_trait]
pub trait FactoryLogger: Send + Sync {
    /// 動画生成成功をログに記録
    async fn log_success(&self, video_id: &str, output_path: &PathBuf) -> Result<(), FactoryError>;

    /// エラーをログに記録
    async fn log_error(&self, reason: &str) -> Result<(), FactoryError>;

    /// 日次サマリーを生成
    async fn daily_summary(&self) -> Result<String, FactoryError>;
}
