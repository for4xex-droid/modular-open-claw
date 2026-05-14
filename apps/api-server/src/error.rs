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
        let reason = reason.into();
        Self(AiomeError::Unauthorized {
            reason: if reason.is_empty() {
                "Unauthorized".to_string()
            } else {
                reason
            },
        })
    }

    /// SEC-2: IDOR/ Authorization error (403 Forbidden)
    pub fn forbidden(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self(AiomeError::SecurityViolation {
            reason: if reason.is_empty() {
                "Forbidden".to_string()
            } else {
                reason
            },
        })
    }

    /// Validation / Bad Request (400 Bad Request)
    pub fn bad_request(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self(AiomeError::Validation {
            reason: if reason.is_empty() {
                "Bad Request".to_string()
            } else {
                reason
            },
        })
    }

    /// Not Found (404 Not Found)
    pub fn not_found(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self(AiomeError::NotFound {
            reason: if reason.is_empty() {
                "Not Found".to_string()
            } else {
                reason
            },
        })
    }

    /// Internal Server Error (500)
    pub fn internal(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self(AiomeError::Infrastructure {
            reason: if reason.is_empty() {
                "Internal Server Error".to_string()
            } else {
                reason
            },
        })
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast::<AiomeError>() {
            Ok(aiome_err) => Self(aiome_err),
            Err(err) => {
                // CWE-532: Log only the root cause to prevent credential leakage
                // from anyhow error chains. Full chain is available at DEBUG level.
                tracing::warn!(
                    "anyhow::Error could not be downcast to AiomeError, wrapping as Infrastructure"
                );
                tracing::debug!(error = %err, "Full anyhow error chain for diagnostics");
                // CWE-209: Generic message for client; full details logged at DEBUG above.
                Self(AiomeError::Infrastructure {
                    reason: "An unexpected internal error occurred.".to_string(),
                })
            }
        }
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for AppError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        // Attempt to downcast to AiomeError to preserve domain-specific HTTP status codes.
        // Without this, errors like NotFound (404) or Validation (400) would be silently
        // converted to Infrastructure (500), losing critical client-facing semantics.
        match err.downcast::<AiomeError>() {
            Ok(aiome_err) => Self(*aiome_err),
            Err(err) => {
                // CWE-532: Log only the root cause to prevent credential leakage.
                // Full error details are available at DEBUG level.
                tracing::warn!(
                    "Box<dyn Error> could not be downcast to AiomeError, wrapping as Infrastructure"
                );
                tracing::debug!(error = %err, "Full Box<dyn Error> details for diagnostics");
                // CWE-209: Generic message for client; full details logged at DEBUG above.
                Self(AiomeError::Infrastructure {
                    reason: "An unexpected internal error occurred.".to_string(),
                })
            }
        }
    }
}

impl From<shared::bootstrap_detector::FactoryResetError> for AppError {
    fn from(err: shared::bootstrap_detector::FactoryResetError) -> Self {
        Self(AiomeError::Infrastructure {
            reason: format!("Factory reset failed: {}", err),
        })
    }
}

impl From<soul::error::SoulError> for AppError {
    fn from(err: soul::error::SoulError) -> Self {
        match err {
            soul::error::SoulError::Internal(reason) => AppError::internal(reason),
            soul::error::SoulError::InvalidTransition(reason) => AppError::bad_request(reason),
            soul::error::SoulError::DistillationFailed(reason) => {
                AppError::internal(format!("DistillationFailed: {}", reason))
            }
            soul::error::SoulError::RebirthFailed(reason) => {
                AppError::internal(format!("RebirthFailed: {}", reason))
            }
            soul::error::SoulError::AdapterError(reason) => {
                AppError::internal(format!("AdapterError: {}", reason))
            }
        }
    }
}

impl From<avatar_engine::loader::LoaderError> for AppError {
    fn from(err: avatar_engine::loader::LoaderError) -> Self {
        Self(err.into())
    }
}

impl From<avatar_engine::proportions::ProportionError> for AppError {
    fn from(err: avatar_engine::proportions::ProportionError) -> Self {
        Self(err.into())
    }
}

impl From<infrastructure::security_zombie::ProcessError> for AppError {
    fn from(err: infrastructure::security_zombie::ProcessError) -> Self {
        Self(err.into())
    }
}

impl From<shared::csam::image_hash::CsamError> for AppError {
    fn from(err: shared::csam::image_hash::CsamError) -> Self {
        Self(err.into())
    }
}

impl From<aiome_commerce::x402::X402Error> for AppError {
    fn from(err: aiome_commerce::x402::X402Error) -> Self {
        Self(err.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_aiome_error_preserves_variant() {
        let err = AiomeError::NotFound {
            reason: "User not found".to_string(),
        };
        let app_err = AppError::from(err);
        assert!(
            matches!(app_err.0, AiomeError::NotFound { .. }),
            "Expected NotFound variant, got: {:?}",
            app_err.0
        );
    }

    #[test]
    fn test_from_anyhow_with_aiome_error_preserves_variant() {
        let aiome_err = AiomeError::Validation {
            reason: "Invalid email".to_string(),
        };
        let anyhow_err: anyhow::Error = aiome_err.into();
        let app_err = AppError::from(anyhow_err);
        assert!(
            matches!(app_err.0, AiomeError::Validation { .. }),
            "Expected Validation variant after downcast, got: {:?}",
            app_err.0
        );
    }

    #[test]
    fn test_from_anyhow_without_aiome_error_falls_back_to_infrastructure() {
        let generic_err = anyhow::anyhow!("something went wrong");
        let app_err = AppError::from(generic_err);
        match &app_err.0 {
            AiomeError::Infrastructure { reason } => {
                assert!(
                    reason.contains("unexpected internal error"),
                    "Expected generic error message, got: {}",
                    reason
                );
            }
            other => panic!("Expected Infrastructure variant, got: {:?}", other),
        }
    }

    #[test]
    fn test_from_box_dyn_error_with_aiome_error_preserves_variant() {
        let aiome_err = AiomeError::NotFound {
            reason: "resource missing".to_string(),
        };
        let boxed: Box<dyn std::error::Error + Send + Sync> = Box::new(aiome_err);
        let app_err = AppError::from(boxed);
        assert!(
            matches!(app_err.0, AiomeError::NotFound { .. }),
            "Expected NotFound variant after Box downcast, got: {:?}",
            app_err.0
        );
    }

    #[test]
    fn test_from_box_dyn_error_non_aiome_falls_back_to_infrastructure() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let boxed: Box<dyn std::error::Error + Send + Sync> = Box::new(io_err);
        let app_err = AppError::from(boxed);
        match &app_err.0 {
            AiomeError::Infrastructure { reason } => {
                assert!(
                    reason.contains("unexpected internal error"),
                    "Expected generic error message, got: {}",
                    reason
                );
            }
            other => panic!("Expected Infrastructure variant, got: {:?}", other),
        }
    }

    #[test]
    fn test_into_response_bad_request() {
        let err = AppError::bad_request("missing field");
        let response = err.into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::BAD_REQUEST,
            "Expected 400 Bad Request"
        );
    }

    #[test]
    fn test_into_response_unauthorized() {
        let err = AppError::unauthorized("no token");
        let response = err.into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::UNAUTHORIZED,
            "Expected 401 Unauthorized"
        );
    }

    #[test]
    fn test_into_response_forbidden() {
        let err = AppError::forbidden("access denied");
        let response = err.into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::FORBIDDEN,
            "Expected 403 Forbidden"
        );
    }

    #[test]
    fn test_into_response_not_found() {
        let err = AppError::not_found("resource missing");
        let response = err.into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::NOT_FOUND,
            "Expected 404 Not Found"
        );
    }

    #[test]
    fn test_into_response_internal_server_error() {
        let err = AppError::internal("db crashed");
        let response = err.into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Expected 500 Internal Server Error"
        );
    }

    #[test]
    fn test_helper_methods_produce_correct_variants() {
        assert!(matches!(
            AppError::unauthorized("no token").0,
            AiomeError::Unauthorized { .. }
        ));
        assert!(matches!(
            AppError::forbidden("access denied").0,
            AiomeError::SecurityViolation { .. }
        ));
        assert!(matches!(
            AppError::not_found("missing").0,
            AiomeError::NotFound { .. }
        ));
        assert!(matches!(
            AppError::internal("crash").0,
            AiomeError::Infrastructure { .. }
        ));
        assert!(matches!(
            AppError::bad_request("invalid").0,
            AiomeError::Validation { .. }
        ));
    }

    #[test]
    fn test_helper_methods_empty_reason_fallback() {
        if let AiomeError::Infrastructure { reason } = AppError::internal("").0 {
            assert_eq!(reason, "Internal Server Error");
        } else {
            panic!("Expected Infrastructure variant");
        }

        if let AiomeError::Validation { reason } = AppError::bad_request("").0 {
            assert_eq!(reason, "Bad Request");
        } else {
            panic!("Expected Validation variant");
        }

        if let AiomeError::NotFound { reason } = AppError::not_found("").0 {
            assert_eq!(reason, "Not Found");
        } else {
            panic!("Expected NotFound variant");
        }

        if let AiomeError::SecurityViolation { reason } = AppError::forbidden("").0 {
            assert_eq!(reason, "Forbidden");
        } else {
            panic!("Expected SecurityViolation variant");
        }

        if let AiomeError::Unauthorized { reason } = AppError::unauthorized("").0 {
            assert_eq!(reason, "Unauthorized");
        } else {
            panic!("Expected Unauthorized variant");
        }
    }

    #[test]
    fn test_display_delegates_to_inner() {
        let err = AppError::bad_request("bad input");
        let display = format!("{}", err);
        assert!(
            display.contains("bad input"),
            "Display should contain inner message, got: {}",
            display
        );
    }

    #[test]
    fn test_source_returns_inner_error() {
        let err = AppError::internal("db down");
        let source = std::error::Error::source(&err);
        assert!(
            source.is_some(),
            "source() should return the inner AiomeError"
        );
    }

    #[test]
    fn test_from_factory_reset_error() {
        let err = shared::bootstrap_detector::FactoryResetError::DirectoryNotFound(
            "/dummy/path".to_string(),
        );
        let app_err = AppError::from(err);
        match &app_err.0 {
            AiomeError::Infrastructure { reason } => {
                assert!(
                    reason.contains("Factory reset failed"),
                    "Expected original message in reason, got: {}",
                    reason
                );
            }
            other => panic!("Expected Infrastructure variant, got: {:?}", other),
        }
    }

    #[test]
    fn test_from_soul_error_preserves_semantics() {
        let soul_internal = soul::error::SoulError::Internal("db timeout".to_string());
        let app_err_internal = AppError::from(soul_internal);

        match &app_err_internal.0 {
            AiomeError::Infrastructure { reason } => {
                assert!(reason.contains("db timeout"));
            }
            _ => panic!("Expected Infrastructure variant"),
        }

        let soul_invalid = soul::error::SoulError::InvalidTransition("state mismatch".to_string());
        let app_err_invalid = AppError::from(soul_invalid);

        match &app_err_invalid.0 {
            AiomeError::Validation { reason } => {
                assert!(reason.contains("state mismatch"));
            }
            _ => panic!("Expected Validation variant, got {:?}", app_err_invalid.0),
        }
    }

    #[test]
    fn test_from_loader_error_preserves_semantics() {
        let loader_invalid = avatar_engine::loader::LoaderError::InvalidHeader;
        let app_err_invalid = AppError::from(loader_invalid);

        match &app_err_invalid.0 {
            AiomeError::Validation { reason } => {
                assert!(reason.contains("Invalid INX header"));
            }
            _ => panic!("Expected Validation variant"),
        }
    }

    #[test]
    fn test_from_proportion_error_preserves_semantics() {
        let prop_err = avatar_engine::proportions::ProportionError::TooYoung(4.0);
        let app_err = AppError::from(prop_err);

        match &app_err.0 {
            AiomeError::SecurityViolation { reason } => {
                assert!(reason.contains("young"));
            }
            _ => panic!("Expected SecurityViolation variant"),
        }
    }

    #[test]
    fn test_from_process_error_preserves_semantics() {
        let proc_err = infrastructure::security_zombie::ProcessError::TimedOut {
            command: "test".into(),
            timeout_secs: 5,
        };
        let app_err = AppError::from(proc_err);

        match &app_err.0 {
            AiomeError::RemoteServiceTimeout { timeout_secs } => {
                assert_eq!(*timeout_secs, 5);
            }
            _ => panic!("Expected RemoteServiceTimeout variant"),
        }
    }

    #[test]
    fn test_from_csam_error_preserves_semantics() {
        let csam_err = shared::csam::image_hash::CsamError::HashError;
        let app_err = AppError::from(csam_err);

        match &app_err.0 {
            AiomeError::Infrastructure { reason } => {
                assert!(reason.contains("CSAM") || reason.contains("Hash"));
            }
            _ => panic!("Expected Infrastructure variant"),
        }
    }

    #[test]
    fn test_from_x402_error_preserves_semantics() {
        let x402_err = aiome_commerce::x402::X402Error::MissingHeaders;
        let app_err = AppError::from(x402_err);

        match &app_err.0 {
            AiomeError::Infrastructure { reason } => {
                assert!(reason.contains("Missing") || reason.contains("headers"));
            }
            _ => panic!("Expected Infrastructure variant"),
        }
    }
}
