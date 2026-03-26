/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::*;

#[test]
fn test_live_types_existence() {
    // これらの型が存在し、インポートできることを確認（コンパイルエラーになれば成功）
    let _state: LiveSessionState = LiveSessionState::Closed;
    let _event: LiveEvent = LiveEvent::TurnEnd;
    let _level: ThinkingLevel = ThinkingLevel::Minimal;
}
