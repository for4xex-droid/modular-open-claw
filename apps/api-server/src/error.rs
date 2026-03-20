/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::error::AiomeError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub struct AppError(pub AiomeError);

impl AppError {
    /// SEC-2: IDOR/ Authorization error (403 Forbidden)
    pub fn forbidden(reason: impl Into<String>) -> Self {
        Self(AiomeError::SecurityViolation {
            reason: reason.into(),
        })
    }

    /// Validation / Bad Request (400 Bad Request)
    pub fn bad_request(reason: impl Into<String>) -> Self {
        Self(AiomeError::Infrastructure {
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

impl From<AiomeError> for AppError {
    fn from(err: AiomeError) -> Self {
        Self(err)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self(AiomeError::OsError { source: err })
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for AppError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self(AiomeError::OsError {
            source: anyhow::anyhow!("{}", err),
        })
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}
