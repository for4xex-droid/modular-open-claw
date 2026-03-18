use crate::error::AiomeError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::pin::Pin;
use tokio_stream::Stream;

/// LLMの停止理由
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// 正常終了（モデルが回答を完了）
    EndTurn,
    /// ツール使用リクエストによる停止
    ToolUse,
    /// 最大トークン数に達したための停止
    MaxTokens,
    /// ストップシーケンスによる停止
    StopSequence,
    /// その他（エラー、フィルタリング等）
    Other(String),
}

/// LLMからの構造化レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// 生成されたテキスト内容
    pub content: String,
    /// 停止理由
    pub stop_reason: StopReason,
}

/// LLMプロバイダーの共通インターフェース
#[async_trait]
pub trait LlmProvider: Send + Sync + Debug {
    /// テキスト生成リクエスト
    async fn complete(&self, prompt: &str, system: Option<&str>)
        -> Result<LlmResponse, AiomeError>;

    /// ストリーミング生成リクエスト
    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        let resp = self.complete(prompt, system).await?;
        let s = async_stream::stream! {
            yield Ok(resp.content);
        };
        Ok(Box::pin(s))
    }

    /// 接続テスト
    async fn test_connection(&self) -> Result<(), AiomeError>;

    /// プロバイダー名を取得（デバッグ用）
    fn name(&self) -> &str;
}

/// 埋め込み（Embedding）プロバイダーの共通インターフェース
#[async_trait]
pub trait EmbeddingProvider: Send + Sync + Debug {
    /// テキストをベクトルに変換
    /// is_query: trueの場合は検索クエリ用、falseの場合はドキュメント用として解釈する
    async fn embed(&self, text: &str, is_query: bool) -> Result<Vec<f32>, AiomeError>;

    /// 接続テスト
    async fn test_connection(&self) -> Result<(), AiomeError>;

    fn name(&self) -> &str;
}
