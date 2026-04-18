/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! # A2UI — Generative UI Protocol Layer (v0.9 Draft)
//!
//! LLM が出力した A2UI JSON を安全にパース・検証し、
//! フロントエンドのリアクティブレンダラーに渡すための型定義・カタログ・バリデーターを提供する。
//!
//! ## モジュール構成
//! - [`schema`]: `A2uiEnvelope`, `Surface`, `Component` の構造体定義
//! - [`catalog`]: LLM プロンプトに注入する UI コンポーネントカタログ
//! - [`validator`]: ホワイトリスト型コンポーネント検疫・SSRF/XSS 防止

pub mod catalog;
pub mod schema;
pub mod validator;

// ── re-exports ──────────────────────────────────────────
// 利用側が `use infrastructure::a2ui::{A2uiEnvelope, AiomeCatalog, A2uiValidator};` で済むようにする
pub use catalog::AiomeCatalog;
pub use schema::{A2uiEnvelope, Component, Surface};
pub use validator::A2uiValidator;
