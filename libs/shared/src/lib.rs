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
pub mod auth;
pub mod cleaner;
/// アプリケーション設定の管理
pub mod config;
/// 暗号化・ハッシュ処理共有ロジック
pub mod crypto;
/// CSAM (Child Safety & Compliance)
pub mod csam;

pub mod guardrails;
/// システムヘルスモニタリング
pub mod health;
pub mod os_utils;
pub mod output_validator;
pub mod sandbox;
/// ネットワークセキュリティポリシー
pub mod security;
pub mod watchtower;
