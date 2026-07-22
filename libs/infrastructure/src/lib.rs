/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![cfg_attr(test, allow(clippy::unwrap_used))]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::new_without_default)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::map_identity)]
#![allow(clippy::double_ended_iterator_last)]
#![allow(clippy::needless_update)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::assertions_on_constants)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::module_inception)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::default_constructed_unit_structs)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::unnecessary_to_owned)]
#![allow(clippy::redundant_closure)]
#![deny(unsafe_code)]
// dead_code: 個別 #[allow] への段階移行中。クレート全体 allow は pre-existing 17件の dead_code 解消まで暫定維持。
#![allow(dead_code)]
#![allow(unused_imports, unused_variables, unused_mut)]

//! # Infrastructure — I/O実装層
//!
//! `core` で定義されたトレイトの具体実装を提供する。
//!
//! モジュールは論理セクション（ADR-056）でグループ化する。物理クレート分割は行わない。
#![allow(missing_docs)]

pub use aiome_core::error::AiomeError;

// =============================================================================
// Platform — DB / 共通 I/O / サポート
// =============================================================================
pub mod a2ui;
pub mod blob_storage;
pub mod db;
pub mod disk_quota;
pub mod html_report;
pub mod repair_strategy;
pub mod spec_provider;
pub mod sql_helpers;
pub mod support;
#[cfg(any(test, debug_assertions))]
pub mod test_utils;
#[cfg(test)]
pub(crate) mod testing;
pub mod workspace_manager;
#[cfg(test)]
mod workspace_manager_tests;

// =============================================================================
// Security — 免疫・境界・認証隣接・出力フィルタ
// =============================================================================
/// Aegis Nervous System
pub mod aegis;
/// 認証・JWTトークン検証モジュール (Phase 8.2)
pub mod auth;
/// 安全なワークスペース環境スキャン（Phase F）
pub mod auto_profile;
/// 信念整合性ゲート (Phase 49: BeliefShift)
pub mod belief_consistency_gate;
#[cfg(test)]
mod belief_consistency_gate_tests;
/// タスクキュー・非同期実行・リトライ
#[macro_use]
pub mod boundary_verifier;
#[cfg(test)]
mod boundary_verifier_tests;
/// 認知健全性モニター (Phase 2A)
pub mod cognitive_sentinel;
/// コンプライアンス・eKYC
pub mod compliance;
/// AgentRx 行動制約チェッカー
pub mod constraint_checker;
/// 脅威シグネチャ監視・遮断
pub mod immune_system;
pub mod invariant_dag;
#[cfg(test)]
mod invariant_dag_tests;
pub mod output_filter;
/// エージェント別のレート制限 (G-2)
pub mod rate_limiter;
/// ネットワークセキュリティポリシー
pub mod security;
pub mod security_zombie;
/// 入出力データの検証
pub mod validator;

// =============================================================================
// Economy — Gig / 生成ゲートウェイ（Commerce 本体は別クレート）
// =============================================================================
pub mod buzz;
pub mod generative_engine;
/// 外部からの安全なタスク受理ゲートウェイ（Phase F）
pub mod gig_gateway;
/// 定期診断・プロアクティブ発火
pub mod gig_metadata_updater;

// =============================================================================
// Soul-adapters — Soul トレイト具象・永続・転生
// =============================================================================
/// Soul転生（L3）ロジック
pub mod samsara_engine;
/// イベント→Experience変換アダプタ
pub mod soul_adapter;
/// AgentSoulのSQLite永続化
pub mod soul_experience_crdt;
/// 経験に基づくSOUL.md動的書換え
pub mod soul_mutator;
pub mod soul_store;
#[cfg(test)]
mod soul_store_tests;
pub mod soul_sync_transport;

// =============================================================================
// Skills / Tools — WASM・Capability・Arena
// =============================================================================
/// スキルの並列実行と評価
pub mod arena_battle;
pub mod browser_conductor;
/// Capabilities and extensions registry
pub mod capability_registry;
/// WASMスキルのロード・サンドボックス管理
pub mod skills;
/// Tool カタログ → CapabilityProvider アダプタ (OP-092 / ADR-020)
pub mod tool_catalog_capability;

// =============================================================================
// Observability — 診断・監査・SLO・アラート
// =============================================================================
pub mod aiome_log;
pub mod alerts;
/// 非同期監査ログ用 MPSC キュー (Phase 35 Step 7)
pub mod audit_logger;
pub mod circuit_breaker;
/// AgentRx 自己診断・軌跡分析
pub mod diagnostics;
pub mod score_tracker;
pub mod slo_engine;

// =============================================================================
// Cortex / Knowledge — 文書投影・索引・コンテキスト
// =============================================================================
/// LLM向けコンテキスト生成エンジン
pub mod context_engine;
pub mod cortex_compiler;
#[cfg(test)]
mod cortex_compiler_tests;
/// Agent-Native Document Discovery via File System Projection (ADR-025)
pub mod cortex_file_projector;
pub mod cortex_ingester;
#[cfg(test)]
mod cortex_ingester_tests;
pub mod cortex_query;
pub mod cortex_synth;
#[cfg(test)]
mod cortex_synth_tests;
pub mod dataset_extractor;
/// ドキュメント・Karma検索インデックス
pub mod knowledge_indexer;
#[cfg(test)]
mod knowledge_indexer_tests;
/// 短期記憶→長期Karma結晶化
pub mod memory_crystallizer;
pub mod oss_ast_analyzer;
pub mod oss_orchestrator;
/// 外部リポジトリの自動クローン・RAGインデックス化
pub mod oss_repository_indexer;
pub mod oss_type_matcher;
/// TurboQuant PolarQuant エンコーダ (Phase 39)
pub mod polar_quant;
pub mod prompt_registry;
pub mod trajectory_adapter;
pub mod trajectory_graph;
/// ベクトル演算の統一インターフェース (Phase 39)
pub mod vector_ops;

// =============================================================================
// Channels — 外部チャット・投稿・トレンド・メディア
// =============================================================================
/// 外部チャットプラットフォームとの通信ブリッジ
pub mod channel_bridge;
/// Intent の生成・サニタイズ (AgentSense Phase AS-1)
pub mod intent;
/// 成果物のSNS自動投稿
pub mod publisher;
/// RSSベースのトレンド収集
pub mod rss_collector;
pub mod trend_sonar;
pub mod tts;
pub mod whisper_transcription;
pub mod x_signal_probe;

// =============================================================================
// Workflow / JobQueue — タスク・ジョブ・監督
// =============================================================================
#[cfg(feature = "grpc")]
pub mod docker_conductor;
/// アイドル時の自律思考管理
pub mod dream_state;
#[cfg(feature = "grpc")]
pub mod grpc;
#[cfg(feature = "grpc")]
pub mod grpc_proof_gate;
pub mod heartbeat_wakeup;
pub mod hierarchical_router;
#[cfg(test)]
mod hierarchical_router_tests;
/// Universal Job Queue backend interface
pub mod job_queue;
pub mod quality_gate_store;
/// Let It Crash / Supervision Tree (Phase 1.5)
pub mod supervisor;
pub mod task_orchestrator;
pub mod workflow;

// =============================================================================
// Platform (compute) — LLM / LoRA / Oracle / 推論周辺
// =============================================================================
/// 成果物の永続化・管理
pub mod artifact_store;
#[cfg(test)]
mod artifact_store_tests;
pub mod forecast;
/// LLMプロバイダの動的選択・プロキシ
pub mod llm;
pub mod lora_autotuner;
/// LoRA アダプター取引マーケットプレイス
pub mod lora_marketplace;
pub mod lora_training;
/// HuggingFace Hub モデル管理 (Phase 2)
pub mod model_manager;
/// ネイティブ推論バックエンド (Phase 2)
#[cfg(feature = "native-inference")]
pub mod native_backend;
/// 高度な論理推論エンジン
pub mod oracle;
/// デジタルアセットのレジストリ・所有権管理モジュール
pub mod registry;
pub mod slm_bridge;
/// 思考の社会 (Evans et al, 2026) 熟議エンジン
pub mod society_of_thought;
/// ユーザー行動パターン学習
pub mod user_learner;
