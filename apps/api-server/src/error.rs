/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use axum::response::{IntoResponse, Response};

pub struct AppError(pub AiomeError);

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Debug for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AppError({:?})", self.0)
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl From<AiomeError> for AppError {
    fn from(err: AiomeError) -> Self {
        Self(err)
    }
}

impl AppError {
    /// Unauthorized (401 Unauthorized)
    pub fn unauthorized(reason: impl Into<String>) -> Self {
        Self(AiomeError::Unauthorized {
            reason: reason.into(),
        })
    }

    /// SEC-2: IDOR/ Authorization error (403 Forbidden)
    pub fn forbidden(reason: impl Into<String>) -> Self {
        Self(AiomeError::SecurityViolation {
            reason: reason.into(),
        })
    }

    /// Validation / Bad Request (400 Bad Request)
    pub fn bad_request(reason: impl Into<String>) -> Self {
        Self(AiomeError::Validation {
            reason: reason.into(),
        })
    }

    /// Not Found (404 Not Found)
    pub fn not_found(reason: impl Into<String>) -> Self {
        Self(AiomeError::NotFound {
            reason: reason.into(),
        })
    }

    /// Internal Server Error (500)
    pub fn internal(reason: impl Into<String>) -> Self {
        Self(AiomeError::Infrastructure {
            reason: reason.into(),
        })
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast::<AiomeError>() {
            Ok(aiome_err) => Self(aiome_err),
            Err(err) => Self(AiomeError::OsError { source: err }),
        }
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for AppError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self(AiomeError::OsError {
            source: anyhow::anyhow!("{}", err),
        })
    }
}

impl From<shared::bootstrap_detector::FactoryResetError> for AppError {
    fn from(err: shared::bootstrap_detector::FactoryResetError) -> Self {
        Self(AiomeError::Infrastructure {
            reason: format!("Factory reset failed: {}", err),
        })
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}
