/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

/// `background` モジュール
pub mod background;
/// `cost` モジュール
pub mod cost;
/// `cost_breaker` モジュール
pub mod cost_breaker;
/// `dynamic` モジュール
pub mod dynamic;
/// `fallback_router` モジュール
pub mod entropy_gate;
pub mod evaluation_logger;
pub mod fallback_router;

#[cfg(feature = "native-inference")]
pub mod native_embedding;

/// `proxy` モジュール
pub mod proxy;
pub mod rlm_client;
/// `semantic_cache` モジュール
pub mod semantic_cache;
pub mod utils;

/// `humanizer_filter` モジュール
pub mod humanizer_filter;
/// `humanizer_rules` モジュール
pub mod humanizer_rules;
/// `whisper_middleware` モジュール
pub mod whisper_middleware;
/// `writing_context` モジュール
pub mod writing_context;
