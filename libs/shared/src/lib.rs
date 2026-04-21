/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
//! Aiome共有ライブラリ — 設定、ヘルス、セキュリティ等の横断的機能を提供
//!
//! # Unsafe Code Policy
//!
//! 本クレートは `#![deny(unsafe_code)]` で保護されている。
//! `#[allow(unsafe_code)]` の使用は [`security::scrub_env`] の1関数のみに限定し、
//! 新規追加にはコードレビューでの承認を必須とすること。
//!
//! `#![forbid(unsafe_code)]` を使用しない理由:
//! `std::env::remove_var` が Rust 2024 Edition で `unsafe fn` に昇格したため、
//! シークレット消去ヘルパー (`scrub_env`) が unsafe を必要とする。
//! `forbid` は子モジュールからのオーバーライドが不可能 (E0453) なため `deny` を使用。
#![deny(unsafe_code)]
#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
#![warn(missing_docs)]

/// 認証・認可関連のロジック
pub mod app_data;
/// 認証モジュール
pub mod auth;
/// Bootstrap検出 & Factory Reset (Phase 2B-CORE)
pub mod bootstrap_detector;
/// クリーナー機能
pub mod cleaner;
/// アプリケーション設定の管理
pub mod config;
/// コンテナランタイム検出 (Podman / Docker) の SSOT
pub mod container_runtime;
/// 暗号化・ハッシュ処理共有ロジック
pub mod crypto;
/// CSAM (Child Safety & Compliance)
pub mod csam;
/// データベース・共通マクロ
pub mod db;
/// ファイルの Magic Bytes 検証
pub mod file_validator;

/// ガードレール機能
pub mod guardrails;
/// システムヘルスモニタリング
pub mod health;
/// MCP の セキュリティ定数
pub mod mcp_constants;
/// OS依存ユーティリティ
pub mod os_utils;
/// 出力バリデーター機能
pub mod output_validator;
/// サンドボックス環境機能
pub mod sandbox;
/// ネットワークセキュリティポリシー + シークレット消去
pub mod security;
/// 安全な文字列操作・文字列表現ユーティリティ
pub mod strings;
/// ウォッチタワー機能
pub mod watchtower;

/// Macros use these re-exports to avoid requiring dependencies in caller crates.
pub mod reexport {
    pub use aiome_core_contracts::error::AiomeError;
}
