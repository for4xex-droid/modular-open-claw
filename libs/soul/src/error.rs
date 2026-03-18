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

impl From<SoulError> for aiome_contracts::error::AiomeError {
    fn from(e: SoulError) -> Self {
        aiome_contracts::error::AiomeError::Infrastructure {
            reason: e.to_string(),
        }
    }
}
