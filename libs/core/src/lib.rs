/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![cfg_attr(test, allow(clippy::unwrap_used))]
#![forbid(unsafe_code)]
// #![allow(unused_imports, unused_variables, dead_code, unused_mut)]

//! # Core — ドメインロジック層
//!
//! Framework のビジネスロジックを定義する。
//! 具体的なI/O実装は `infrastructure` クレートに委譲する（依存性逆転의原則）。

#![warn(missing_docs)]

/// Biome Protocol関連のデータ型およびエンジン
pub mod biome;
/// LLM予算・コスト計算モジュール
pub mod budget;
pub mod commerce;
/// コアシステム全体で共有するデータ形式の定義
pub mod contracts;
pub mod error;
/// 表情・感情表現などに関するモジュール
pub mod expression;
/// 内部HTTPリクエストおよびクライアントに関するヘルパー
pub mod http;
/// 外部LLMAPI(Gemini, OpenAI等)とのインターフェース
pub mod llm_provider;
/// LoRAモデルの管理・推論支援（Phase 10.1b）
pub mod lora;
/// System security and validation logic
pub mod security;
/// JobQueue等、インフラ層に実装を依存させるためのTrait
pub mod traits;

/// 行動履歴やプラン（Trajectory）の管理
pub mod trajectory {
    pub use aiome_core_contracts::trajectory::*;
}
pub mod security_impl;

#[cfg(test)]
mod security_test;
