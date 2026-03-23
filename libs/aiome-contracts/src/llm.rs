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

/// LLMへのメッセージ (ADR-021)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    /// ロール ("system", "user", "assistant")
    pub role: String,
    /// 内容
    pub content: String,
    /// キャッシュ制御フラグ (ADR-021: Prompt Caching)
    /// true の場合、このメッセージまでのプレフィックスをキャッシュ対象とする。
    pub cache: bool,
}

/// LLMリクエストの構造体 (ADR-021)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmRequest {
    /// メッセージ履歴
    pub messages: Vec<LlmMessage>,
    /// 温度パラメータ
    pub temperature: Option<f32>,
    /// 最大トークン数
    pub max_tokens: Option<i32>,
    /// 停止シーケンス
    pub stop_sequences: Option<Vec<String>>,
    /// 出力フォーマット (例: "json")
    pub format: Option<String>,
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

    /// 詳細なリクエスト（キャッシュ制御を含む） (ADR-021)
    async fn complete_with_cache(&self, request: LlmRequest) -> Result<LlmResponse, AiomeError> {
        let mut system = None;
        let mut prompt = String::new();
        for m in &request.messages {
            if m.role == "system" {
                system = Some(m.content.as_str());
            } else if m.role == "user" {
                prompt = m.content.clone();
            }
        }
        self.complete(&prompt, system).await
    }

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
