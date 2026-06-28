/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use serde::{Deserialize, Serialize};

/// [A-1] Docker Agent ↔ Karma Feedback Loop
/// Represents the result of an execution inside the Docker sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

impl DelegationResult {
    /// Determines if the execution was successful based on exit code.
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// High-level classification of the failure type.
    pub fn failure_kind(&self) -> DelegationFailureKind {
        if self.is_success() {
            return DelegationFailureKind::None;
        }

        // Common exit codes and stderr patterns
        if self.exit_code == 124 || self.stderr.contains("timeout") {
            DelegationFailureKind::Timeout
        } else if self.exit_code == 137
            || self.stderr.contains("OOM")
            || self.stderr.contains("Out of memory")
        {
            DelegationFailureKind::Oom
        } else if self.stderr.contains("Module not found") || self.stderr.contains("ImportError") {
            DelegationFailureKind::DependencyMissing
        } else if self.stderr.contains("SyntaxError") {
            DelegationFailureKind::SyntaxError
        } else {
            DelegationFailureKind::UnknownRuntime
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DelegationFailureKind {
    None,
    Timeout,
    Oom,
    DependencyMissing,
    SyntaxError,
    UnknownRuntime,
}
