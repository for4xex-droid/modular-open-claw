/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

pub use aiome_core_contracts::biome::*;

/// 自律的なP2P対話エンジンの実装
pub mod autonomous;
/// 対話履歴、ターン管理およびペナルティ処理
pub mod dialogue;

// Re-export autonomous engine and config to maintain compatibility with existing code
pub use autonomous::{AutonomousBiomeEngine, AutonomousConfig};
