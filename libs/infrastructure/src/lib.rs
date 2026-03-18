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
/// 自動補完モジュール
pub mod artifact_store;
/// 自動補完モジュール
pub mod channel_bridge;
pub mod circuit_breaker;
/// 自動補完モジュール
pub mod commerce_mock;
/// 自動補完モジュール
pub mod concept_manager;
/// 自動補完モジュール
pub mod constraint_checker;
/// 自動補完モジュール
pub mod context_engine;
/// 自動補完モジュール
pub mod diagnostics;
/// 自動補完モジュール
pub mod dream_state;
/// 自動補完モジュール
pub mod heartbeat_wakeup;
/// 自動補完モジュール
pub mod immune_system;
/// 自動補完モジュール
pub mod job_queue;
/// 自動補完モジュール
pub mod knowledge_indexer;
/// 自動補完モジュール
pub mod llm;
/// 自動補完モジュール
pub mod memory_crystallizer;
/// 自動補完モジュール
pub mod oracle;
/// 自動補完モジュール
pub mod publisher;
/// 自動補完モジュール
pub mod samsara_engine;
/// 自動補完モジュール
pub mod security;
pub mod security_zombie;
/// 自動補完モジュール
pub mod skill_arena;
/// 自動補完モジュール
pub mod skills;
pub mod slo_engine;
/// 自動補完モジュール
pub mod soul_adapter;
/// 自動補完モジュール
pub mod soul_mutator;
/// 自動補完モジュール
pub mod soul_store;
pub mod trend_sonar;
/// 自動補完モジュール
pub mod user_learner;
/// 自動補完モジュール
pub mod validator;
pub mod workspace_manager;
mod workspace_manager_tests;
