/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
//! Aiome共有ライブラリ — 設定、ヘルス、セキュリティ等の横断的機能を提供
#![forbid(unsafe_code)]
#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
#![warn(missing_docs)]

/// 認証・認可関連のロジック
pub mod app_data;
/// 認証モジュール
pub mod auth;
/// クリーナー機能
pub mod cleaner;
/// アプリケーション設定の管理
pub mod config;
/// 暗号化・ハッシュ処理共有ロジック
pub mod crypto;
/// CSAM (Child Safety & Compliance)
pub mod csam;
/// データベース・共通マクロ
pub mod db;

/// ガードレール機能
pub mod guardrails;
/// システムヘルスモニタリング
pub mod health;
/// OS依存ユーティリティ
pub mod os_utils;
/// 出力バリデーター機能
pub mod output_validator;
/// サンドボックス環境機能
pub mod sandbox;
/// ネットワークセキュリティポリシー
pub mod security;
/// ウォッチタワー機能
pub mod watchtower;

/// Macros use these re-exports to avoid requiring dependencies in caller crates.
pub mod reexport {
    pub use aiome_contracts::error::AiomeError;
}
