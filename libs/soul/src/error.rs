/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SoulError {
    #[error("Distillation failed: {0}")]
    DistillationFailed(String),

    #[error("Rebirth failed: {0}")]
    RebirthFailed(String),

    #[error("Invalid state transition: {0}")]
    InvalidTransition(String),

    #[error("Adapter error: {0}")]
    AdapterError(String),

    #[error("Internal engine error: {0}")]
    Internal(String),
}

impl From<SoulError> for aiome_core_contracts::error::AiomeError {
    fn from(e: SoulError) -> Self {
        match e {
            SoulError::DistillationFailed(r) => {
                aiome_core_contracts::error::AiomeError::Infrastructure {
                    reason: format!("[SoulDistill] {}", r),
                }
            }
            SoulError::RebirthFailed(r) => {
                aiome_core_contracts::error::AiomeError::Infrastructure {
                    reason: format!("[SoulRebirth] {}", r),
                }
            }
            SoulError::AdapterError(r) => aiome_core_contracts::error::AiomeError::Infrastructure {
                reason: format!("[SoulAdapter] {}", r),
            },
            // Boundary: invalid lifecycle transition is a client/domain 400, not 500.
            SoulError::InvalidTransition(r) => {
                aiome_core_contracts::error::AiomeError::Validation {
                    reason: format!("[SoulTransition] {}", r),
                }
            }
            SoulError::Internal(r) => aiome_core_contracts::error::AiomeError::Infrastructure {
                reason: format!("[SoulInternal] {}", r),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core_contracts::error::AiomeError;

    #[test]
    fn soul_error_into_aiome_preserves_boundary_semantics() {
        let internal: AiomeError = SoulError::Internal("db timeout".into()).into();
        assert!(matches!(
            internal,
            AiomeError::Infrastructure { reason } if reason.contains("db timeout")
        ));

        let invalid: AiomeError = SoulError::InvalidTransition("state mismatch".into()).into();
        assert!(matches!(
            invalid,
            AiomeError::Validation { reason } if reason.contains("state mismatch")
        ));
    }
}
