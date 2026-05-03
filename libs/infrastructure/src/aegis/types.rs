/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IncidentStatus {
    Open,
    Analyzing,
    PatchGenerated,
    KaniVerifying,
    KaniSuccess,
    HotSwapped,
    Resolved,
    WontFix,
}

impl std::fmt::Display for IncidentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            IncidentStatus::Open => "Open",
            IncidentStatus::Analyzing => "Analyzing",
            IncidentStatus::PatchGenerated => "PatchGenerated",
            IncidentStatus::KaniVerifying => "KaniVerifying",
            IncidentStatus::KaniSuccess => "KaniSuccess",
            IncidentStatus::HotSwapped => "HotSwapped",
            IncidentStatus::Resolved => "Resolved",
            IncidentStatus::WontFix => "WontFix",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for IncidentStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Open" => Ok(IncidentStatus::Open),
            "Analyzing" => Ok(IncidentStatus::Analyzing),
            "PatchGenerated" => Ok(IncidentStatus::PatchGenerated),
            "KaniVerifying" => Ok(IncidentStatus::KaniVerifying),
            "KaniSuccess" => Ok(IncidentStatus::KaniSuccess),
            "HotSwapped" => Ok(IncidentStatus::HotSwapped),
            "Resolved" => Ok(IncidentStatus::Resolved),
            "WontFix" => Ok(IncidentStatus::WontFix),
            _ => Err(format!("Unknown IncidentStatus: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentRecord {
    pub id: String,
    pub skill_name: String,
    pub wasm_hash: String,
    pub input_payload: String,
    pub stack_trace: String,
    pub status: IncidentStatus,
    pub retry_count: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct WeeklyStats {
    pub total_incidents_7d: i64,
    pub distinct_skills: i64,
    pub unresolved: i64,
    pub top_failing_skill: Option<String>,
}
