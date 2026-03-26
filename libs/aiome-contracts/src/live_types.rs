/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LiveSessionState {
    Connecting,
    Active,
    Interrupted,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveEvent {
    AudioChunk(Vec<u8>),
    TextDelta(String),
    ToolCall(LiveToolCall),
    Transcript(String),
    TurnEnd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveToolCall {
    pub function_calls: Vec<LiveFunctionCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveFunctionCall {
    pub name: String,
    pub args: serde_json::Value,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveToolResponse {
    pub function_responses: Vec<LiveFunctionResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveFunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
    pub id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingLevel {
    Minimal,
    Low,
    Medium,
    High,
}

impl Default for ThinkingLevel {
    fn default() -> Self {
        Self::Minimal
    }
}
