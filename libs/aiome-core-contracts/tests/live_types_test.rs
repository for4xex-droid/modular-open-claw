/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![allow(clippy::unwrap_used)]

use aiome_core_contracts::*;

#[test]
fn test_live_types_existence() {
    // これらの型が存在し、インポートできることを確認（コンパイルエラーになれば成功）
    let _state: LiveSessionState = LiveSessionState::Closed;
    let _event: LiveEvent = LiveEvent::TurnEnd;
    let _level: ThinkingLevel = ThinkingLevel::Minimal;
}
