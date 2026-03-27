/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
#![deny(unsafe_code)]
#![allow(unused_imports, unused_variables, dead_code, unused_mut)]

//! # Infrastructure — I/O実装層
//!
//! `core` で定義されたトレイトの具体実装を提供する。
#![warn(missing_docs)]

pub mod aiome_log;
/// 成果物の永続化・管理
pub mod artifact_store;
/// 非同期監査ログ用 MPSC キュー (Phase 35 Step 7)
pub mod audit_logger;
/// 認証・JWTトークン検証モジュール (Phase 8.2)
pub mod auth;
/// Capabilities and extensions registry
pub mod capability_registry;
/// 外部チャットプラットフォームとの通信ブリッジ
pub mod channel_bridge;
pub mod circuit_breaker;
/// 商用連携実装（ギフト送信等）
pub mod commerce;
/// 決済フローのモック実装
#[cfg(any(test, debug_assertions))]
pub mod commerce_mock;
/// コンプライアンス・eKYC
pub mod compliance;
/// コンセプト（概念）のベクター管理
pub mod concept_manager;
/// AgentRx 行動制約チェッカー
pub mod constraint_checker;
/// LLM向けコンテキスト生成エンジン
pub mod context_engine;
/// タスクキュー・非同期実行・リトライ
#[macro_use]
pub mod db;
/// AgentRx 自己診断・軌跡分析
pub mod diagnostics;
pub mod docker_conductor;
/// アイドル時の自律思考管理
pub mod dream_state;
/// AIギグ・エコノミー受発注エンジン
pub mod gig_engine;
/// 定期診断・プロアクティブ発火
pub mod heartbeat_wakeup;
/// 脅威シグネチャ監視・遮断
pub mod immune_system;
/// Intent の生成・サニタイズ (AgentSense Phase AS-1)
pub mod intent;
/// Universal Job Queue backend interface
pub mod job_queue;
/// ドキュメント・Karma検索インデックス
pub mod knowledge_indexer;
/// LLMプロバイダの動的選択・プロキシ
pub mod llm;
/// 短期記憶→長期Karma結晶化
pub mod memory_crystallizer;
/// 高度な論理推論エンジン
pub mod oracle;
pub mod oss_ast_analyzer;
pub mod oss_orchestrator;
/// 外部リポジトリの自動クローン・RAGインデックス化
pub mod oss_repository_indexer;
pub mod oss_type_matcher;
/// TurboQuant PolarQuant エンコーダ (Phase 39)
pub mod polar_quant;
/// 成果物のSNS自動投稿
pub mod publisher;
/// エージェント別のレート制限 (G-2)
pub mod rate_limiter;
/// デジタルアセットのレジストリ・所有権管理モジュール
pub mod registry;
/// RSSベースのトレンド収集
pub mod rss_collector;
/// Soul転生（L3）ロジック
pub mod samsara_engine;
/// ネットワークセキュリティポリシー
pub mod security;
pub mod security_zombie;
/// スキルの並列実行と評価
pub mod skill_arena;
/// WASMスキルのロード・サンドボックス管理
pub mod skills;
pub mod slm_bridge;
pub mod slo_engine;
/// イベント→Experience変換アダプタ
pub mod soul_adapter;
/// 経験に基づくSOUL.md動的書換え
pub mod soul_mutator;
/// AgentSoulのSQLite永続化
pub mod soul_store;
pub mod syndicate_store;
pub mod task_orchestrator;
pub mod trend_sonar;
pub mod tts;
/// ユーザー行動パターン学習
pub mod user_learner;
/// 入出力データの検証
pub mod validator;
/// ベクトル演算の統一インターフェース (Phase 39)
pub mod vector_ops;
pub mod whisper_transcription;
pub mod workspace_manager;

mod artifact_store_tests;
mod soul_store_tests;
#[cfg(any(test, debug_assertions))]
pub mod test_utils;
mod workspace_manager_tests;
