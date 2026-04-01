/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

pub use aiome_contracts::expression::Expression;

/// 感情テキストとメタデータを生成する推論アルゴリズム
pub mod engine;
/// Phase 10.1a: TTS非同期処理ワーカー
pub mod tts_worker;
