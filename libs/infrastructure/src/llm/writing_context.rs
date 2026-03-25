/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use serde::{Deserialize, Serialize};

/// AiomeのLLM出力コンテキスト。出力先に応じて適用するヒューマナイザールールを切り替えるために使用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WritingContext {
    /// ユーザーとの直接対話（Discord Watchtower人格など）
    Chat,
    /// 内部的な独白や日記（MANIFESTO.mdなど）
    Manifesto,
    /// 技術ログやAPIレスポンス
    TechLog,
    /// 創造的なコンテンツ（DreamStateなど）
    Dream,
    /// デフォルト（コンテキスト指定なし）
    #[default]
    Default,
}
