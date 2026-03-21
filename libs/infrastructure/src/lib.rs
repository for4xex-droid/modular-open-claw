/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
#![forbid(unsafe_code)]
#![allow(unused_imports, unused_variables, dead_code, unused_mut)]

//! # Infrastructure — I/O実装層
//!
//! `core` で定義されたトレイトの具体実装を提供する。
#![warn(missing_docs)]

pub mod aiome_log;
/// 成果物の永続化・管理
pub mod artifact_store;
/// 認証・JWTトークン検証モジュール (Phase 8.2)
pub mod auth;
/// 外部チャットプラットフォームとの通信ブリッジ
pub mod channel_bridge;
pub mod circuit_breaker;
/// 商用連携実装（ギフト送信等）
pub mod commerce;
/// 決済フローのモック実装
pub mod commerce_mock;
/// コンプライアンス・eKYC
pub mod compliance;
/// コンセプト（概念）のベクター管理
pub mod concept_manager;
/// AgentRx 行動制約チェッカー
pub mod constraint_checker;
/// LLM向けコンテキスト生成エンジン
pub mod context_engine;
/// AgentRx 自己診断・軌跡分析
pub mod diagnostics;
/// アイドル時の自律思考管理
pub mod dream_state;
/// 定期診断・プロアクティブ発火
pub mod heartbeat_wakeup;
/// 脅威シグネチャ監視・遮断
pub mod immune_system;
/// タスクキュー・非同期実行・リトライ
pub mod job_queue;
/// ドキュメント・Karma検索インデックス
pub mod knowledge_indexer;
/// LLMプロバイダの動的選択・プロキシ
pub mod llm;
/// 短期記憶→長期Karma結晶化
pub mod memory_crystallizer;
/// 高度な論理推論エンジン
pub mod oracle;
/// 成果物のSNS自動投稿
pub mod publisher;
/// デジタルアセットのレジストリ・所有権管理モジュール
pub mod registry;
/// Soul転生（L3）ロジック
pub mod samsara_engine;
/// ネットワークセキュリティポリシー
pub mod security;
pub mod security_zombie;
/// スキルの並列実行と評価
pub mod skill_arena;
/// WASMスキルのロード・サンドボックス管理
pub mod skills;
pub mod slo_engine;
/// イベント→Experience変換アダプタ
pub mod soul_adapter;
/// 経験に基づくSOUL.md動的書換え
pub mod soul_mutator;
/// AgentSoulのSQLite永続化
pub mod soul_store;
pub mod trend_sonar;
/// ユーザー行動パターン学習
pub mod user_learner;
/// 入出力データの検証
pub mod validator;
pub mod workspace_manager;

mod soul_store_tests;
#[cfg(test)]
pub mod test_utils;
mod workspace_manager_tests;
