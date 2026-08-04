/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AiomeError;
use crate::task_tier::TaskTier;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::pin::Pin;
use tokio_stream::Stream;

/// Metadata key: explicit route tier override (`Fast` / `Smart`).
pub const ROUTE_TIER_KEY: &str = "route_tier";
/// Metadata key: machine-readable routing reason code.
pub const ROUTE_REASON_KEY: &str = "route_reason";
/// Metadata key: active routing mode (`legacy` / `rules`).
pub const ROUTE_MODE_KEY: &str = "route_mode";
/// Metadata key: provider that actually served the request.
pub const RESOLVED_PROVIDER_KEY: &str = "resolved_provider";
/// Metadata key: model that actually served the request.
pub const RESOLVED_MODEL_KEY: &str = "resolved_model";
/// Metadata key: sticky tier lock for EntropyGate re-asks.
pub const ROUTE_TIER_LOCKED_KEY: &str = "route_tier_locked";

/// Chat cache scope key (must be present for CachingLlmProvider to store/hit).
pub const CACHE_SCOPE_CHANNEL_KEY: &str = "channel_id";

/// Result of synchronous route-rule evaluation (ADR-058).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmRouteDecision {
    pub tier: TaskTier,
    pub reason_code: String,
    pub reason_detail: String,
}

/// LLMの停止理由
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// 正常終了（モデルが回答を完了）
    #[default]
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
    /// プロバイダー固有のメタデータ (例: "previous_interaction_id")
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

/// トークンごとの対数確率 (P8a: EntropyGate 基盤)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenLogprob {
    pub token: String,
    pub logprob: f64,
    pub top_logprobs: Option<Vec<(String, f64)>>,
}

/// LLMからの構造化レスポンス
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmResponse {
    /// 生成されたテキスト内容
    pub content: String,
    /// 停止理由
    pub stop_reason: StopReason,
    /// モデルの推論理由（思考プロセス）
    pub reasoning: Option<String>,
    /// プロバイダー固有のメタデータ (例: "interaction_id")
    pub metadata: Option<std::collections::HashMap<String, String>>,
    /// トークンごとの対数確率（取得可能なプロバイダーのみ）
    pub logprobs: Option<Vec<TokenLogprob>>,
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

    /// 使用するモデルの埋め込み次元数を取得 (Phase 2-C: Hardcode Elimination)
    fn embedding_dim(&self) -> usize;

    /// 接続テスト
    async fn test_connection(&self) -> Result<(), AiomeError>;

    fn name(&self) -> &str;
}

/// ネイティブ推論モデルの設定 (Phase 2: Native Integration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeModelConfig {
    /// モデル名 (例: "phi-3-mini", "nomic-embed-text")
    pub model_name: String,
    /// モデルファイルのパス (GGUF or Safetensors)
    pub model_path: String,
    /// トークナイザー設定のパス
    pub tokenizer_path: String,
    /// コンテキスト長
    pub context_size: usize,
    /// デバイス設定 ("cpu", "cuda", "metal")
    pub device: String,
    /// 量子化タイプ
    pub quantization: Option<String>,
    /// 埋め込み次元数 (Embeddingモデルの場合)
    pub embedding_dim: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_response_default() {
        let response = LlmResponse::default();
        assert_eq!(response.content, "");
        assert_eq!(response.stop_reason, StopReason::EndTurn);
        assert_eq!(response.reasoning, None);
        assert_eq!(response.metadata, None);
    }

    #[test]
    fn test_token_logprob_field() {
        let logprob = TokenLogprob {
            token: "hello".to_string(),
            logprob: -0.1,
            top_logprobs: None,
        };
        let response = LlmResponse {
            content: "test".to_string(),
            logprobs: Some(vec![logprob]),
            ..Default::default()
        };
        assert!(response.logprobs.is_some());
    }
}
