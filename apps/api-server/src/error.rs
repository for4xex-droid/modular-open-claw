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
        let (status, error_message) = match &self.0 {
            AiomeError::PromptBlocked { reason } => (StatusCode::FORBIDDEN, reason.clone()),
            AiomeError::ArtifactNotFound { path: _ } => (
                StatusCode::NOT_FOUND,
                "Artifact not found".to_string(),
            ),
            AiomeError::SecurityViolation { reason } => (
                StatusCode::FORBIDDEN,
                format!("Security violation: {}", reason),
            ),
            AiomeError::BudgetExhausted(e) => (
                StatusCode::TOO_MANY_REQUESTS,
                format!("Budget exhausted: {}", e),
            ),
            AiomeError::RemoteServiceTimeout { timeout_secs } => (
                StatusCode::GATEWAY_TIMEOUT,
                format!("Remote service timeout after {}s", timeout_secs),
            ),
            AiomeError::StorageFull { threshold } => (
                StatusCode::INSUFFICIENT_STORAGE,
                format!("Storage is full (leveled at {}%)", threshold),
            ),
            AiomeError::ContextFetch { source: _ }
            | AiomeError::LlmResponse { source: _ }
            | AiomeError::OsError { source: _ } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
            AiomeError::ConfigLoad { source: _ } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Configuration error".to_string(),
            ),
            AiomeError::Infrastructure { reason: _ } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Infrastructure error".to_string(),
            ),
            AiomeError::RemoteServiceError { url: _, source: _ } => (
                StatusCode::BAD_GATEWAY,
                "Remote service error".to_string(),
            ),
            AiomeError::RemoteServiceExecutionFailed { reason: _ } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Execution failed".to_string(),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unexpected error occurred".to_string(),
            ),
        };

        let body = Json(json!({
            "error": error_message,
            "code": format!("{:?}", self.0).split('(').next().unwrap_or("Unknown").trim(),
        }));

        (status, body).into_response()
    }
}
