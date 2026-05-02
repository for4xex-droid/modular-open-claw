/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![allow(clippy::unwrap_used)]

//! # Chaos Engineering Primitives
//!
//! テスト専用のフォルトインジェクション provider。
//! `tests/` 内に配置することで、本番バイナリからの完全な隔離を保証する。

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{LlmProvider, LlmRequest, LlmResponse, StopReason};
use async_trait::async_trait;

// ─────────────────────────────────────────────────
//  ChaosMode: 注入可能な障害モードの型安全な列挙
// ─────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub enum ChaosMode {
    /// 正常動作（フォルトなし）
    #[default]
    Normal,
    /// LLM が空文字列を返す
    EmptyResponse,
    /// LLM がタイムアウトする（指定秒数後に AiomeError を返す）
    Timeout(Duration),
    /// LLM が不正な JSON を返す
    MalformedJson,
    /// LLM が巨大な出力を返す（指定バイト数）
    GiantOutput(usize),
    /// 全操作で Err を返す
    AlwaysFail,
}

// ─────────────────────────────────────────────────
//  ChaosLlmProvider: 障害注入 LLM ラッパー
// ─────────────────────────────────────────────────

pub struct ChaosLlmProvider {
    pub inner: Arc<dyn LlmProvider>,
    pub mode: ChaosMode,
}

impl fmt::Debug for ChaosLlmProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChaosLlmProvider")
            .field("mode", &self.mode)
            .field("inner", &self.inner.name())
            .finish()
    }
}

#[async_trait]
impl LlmProvider for ChaosLlmProvider {
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        match &self.mode {
            ChaosMode::Normal => self.inner.complete(prompt, system).await,
            ChaosMode::EmptyResponse => Ok(LlmResponse {
                content: String::new(),
                stop_reason: StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            }),
            ChaosMode::Timeout(dur) => {
                tokio::time::sleep(*dur).await;
                Err(AiomeError::Infrastructure {
                    reason: "Chaos: LLM timeout".into(),
                })
            }
            ChaosMode::MalformedJson => Ok(LlmResponse {
                content: "{invalid json///".into(),
                stop_reason: StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            }),
            ChaosMode::GiantOutput(size) => Ok(LlmResponse {
                content: "x".repeat(*size),
                stop_reason: StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            }),
            ChaosMode::AlwaysFail => Err(AiomeError::Infrastructure {
                reason: "Chaos: Forced failure".into(),
            }),
        }
    }

    /// `complete_with_cache` を `complete` に委譲する。
    /// ChaosMode は `complete` 内でインターセプトされるため、
    /// キャッシュ付きリクエストにも障害が正しく注入される。
    /// 注意: 複数の system メッセージがある場合、最後の1つが採用される。
    async fn complete_with_cache(&self, req: LlmRequest) -> Result<LlmResponse, AiomeError> {
        let mut system = None;
        let mut prompt = String::new();
        for m in &req.messages {
            if m.role == "system" {
                system = Some(m.content.as_str());
            } else if m.role == "user" {
                prompt = m.content.clone();
            }
        }
        self.complete(&prompt, system).await
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.inner.test_connection().await
    }

    fn name(&self) -> &str {
        "ChaosLlmProvider"
    }
}

// ─────────────────────────────────────────────────
//  MockLlm: テスト用の基本 LLM スタブ
//  (test_utils::GlobalMockLlm は #[cfg(test)] 内で
//   integration tests から不可視なため、独自に定義)
// ─────────────────────────────────────────────────

#[derive(Debug)]
pub struct MockLlm {
    pub content: String,
    pub should_fail: bool,
}

impl MockLlm {
    pub fn ok(content: &str) -> Self {
        Self {
            content: content.to_string(),
            should_fail: false,
        }
    }

    pub fn failing() -> Self {
        Self {
            content: String::new(),
            should_fail: true,
        }
    }
}

#[async_trait]
impl LlmProvider for MockLlm {
    async fn complete(
        &self,
        _prompt: &str,
        _system: Option<&str>,
    ) -> Result<LlmResponse, AiomeError> {
        if self.should_fail {
            Err(AiomeError::Infrastructure {
                reason: "mock failure".into(),
            })
        } else {
            Ok(LlmResponse {
                content: self.content.clone(),
                stop_reason: StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            })
        }
    }

    async fn complete_with_cache(&self, req: LlmRequest) -> Result<LlmResponse, AiomeError> {
        // メッセージからプロンプトを抽出して complete に委譲
        let prompt = req
            .messages
            .iter()
            .rfind(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .unwrap_or("");
        let system = req
            .messages
            .iter()
            .rfind(|m| m.role == "system")
            .map(|m| m.content.as_str());
        self.complete(prompt, system).await
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        Ok(())
    }

    fn name(&self) -> &str {
        "MockLlm"
    }
}
