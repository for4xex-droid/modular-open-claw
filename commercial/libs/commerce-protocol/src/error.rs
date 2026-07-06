/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 *
 * Usage of this software is subject to the BSL 1.1 terms.
 * Commercial use requires a separate license agreement.
 */

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum NurtureError {
    #[error("残高不足: 必要 {required}, 利用可能 {available}")]
    InsufficientBalance { required: u64, available: u64 },

    #[error("日次上限超過: 上限 {limit}, 現在 {current}")]
    DailyLimitExceeded { limit: u64, current: u64 },

    #[error("月次上限超過: 上限 {limit}, 現在 {current}")]
    MonthlyLimitExceeded { limit: u64, current: u64 },

    #[error("アイテムが見つからない: {0}")]
    ItemNotFound(Uuid),

    #[error("ポリシー違反: {0}")]
    PolicyViolation(String),

    #[error("決済エラー: {0}")]
    SettlementFailed(String),

    #[error("インターセプタブロック: {0}")]
    InterceptorBlocked(String),

    #[error("認証エラー: {0}")]
    Unauthorized(String),

    #[error("インフラエラー: {0}")]
    Infrastructure(String),

    #[error("Commerceエラー: {reason}")]
    Commerce { reason: String },

    #[error("Ledgerエラー: {reason}")]
    Ledger { reason: String },

    #[error("返金エラー: {0}")]
    Refund(String),

    #[error("CSAM Rejected: item {item_id} — {reason}")]
    CsamRejected { item_id: Uuid, reason: String },

    #[error("冪等性キー重複: {key}")]
    IdempotencyConflict { key: String },

    #[error("楽観的ロック衝突: {entity}")]
    OptimisticLockConflict { entity: String },
}

#[cfg(feature = "axum")]
impl axum::response::IntoResponse for NurtureError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;

        let status = match &self {
            NurtureError::InsufficientBalance { .. } => StatusCode::PAYMENT_REQUIRED,
            NurtureError::DailyLimitExceeded { .. } => StatusCode::FORBIDDEN,
            NurtureError::MonthlyLimitExceeded { .. } => StatusCode::FORBIDDEN,
            NurtureError::ItemNotFound(_) => StatusCode::NOT_FOUND,
            NurtureError::PolicyViolation(_) => StatusCode::FORBIDDEN,
            NurtureError::InterceptorBlocked(_) => StatusCode::FORBIDDEN,
            NurtureError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            NurtureError::IdempotencyConflict { .. } => StatusCode::CONFLICT,
            NurtureError::OptimisticLockConflict { .. } => StatusCode::CONFLICT,
            NurtureError::CsamRejected { .. } => StatusCode::FORBIDDEN,
            NurtureError::SettlementFailed(_)
            | NurtureError::Infrastructure(_)
            | NurtureError::Commerce { .. }
            | NurtureError::Ledger { .. }
            | NurtureError::Refund(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        // CWE-209: Error Masking
        let mask = cfg!(not(debug_assertions));
        let error_message = if status.is_server_error() && mask {
            let error_id = uuid::Uuid::new_v4();
            tracing::error!(
                error.id = %error_id,
                error.detail = %self,
                "Internal server error masked for client"
            );
            format!(
                "An internal server error occurred. Reference ID: {}",
                error_id
            )
        } else {
            self.to_string()
        };

        let body = Json(serde_json::json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

#[cfg(all(test, feature = "axum"))]
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    fn get_status(err: NurtureError) -> StatusCode {
        let resp = err.into_response();
        resp.status()
    }

    #[test]
    fn test_error_mapping_status_codes() {
        assert_eq!(
            get_status(NurtureError::InsufficientBalance {
                required: 10,
                available: 5
            }),
            StatusCode::PAYMENT_REQUIRED
        );
        assert_eq!(
            get_status(NurtureError::DailyLimitExceeded {
                limit: 100,
                current: 150
            }),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            get_status(NurtureError::MonthlyLimitExceeded {
                limit: 500,
                current: 600
            }),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            get_status(NurtureError::ItemNotFound(Uuid::new_v4())),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            get_status(NurtureError::PolicyViolation("test".to_string())),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            get_status(NurtureError::SettlementFailed("test".to_string())),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            get_status(NurtureError::InterceptorBlocked("test".to_string())),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            get_status(NurtureError::Unauthorized("test".to_string())),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            get_status(NurtureError::Infrastructure("test".to_string())),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            get_status(NurtureError::Commerce {
                reason: "test".to_string()
            }),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            get_status(NurtureError::Ledger {
                reason: "test".to_string()
            }),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            get_status(NurtureError::Refund("test".to_string())),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            get_status(NurtureError::CsamRejected {
                item_id: Uuid::new_v4(),
                reason: "test".to_string()
            }),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            get_status(NurtureError::IdempotencyConflict {
                key: "test".to_string()
            }),
            StatusCode::CONFLICT
        );
        assert_eq!(
            get_status(NurtureError::OptimisticLockConflict {
                entity: "test".to_string()
            }),
            StatusCode::CONFLICT
        );
    }

    // ─── CWE-209: Error masking tests ───

    // IntoResponse generates a JSON body, we need a helper to extract the error string
    async fn get_error_message(err: NurtureError) -> String {
        let resp = err.into_response();
        let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        json["error"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn test_internal_error_masked_when_flag_true() {
        let err = NurtureError::Infrastructure("DB pool exhausted: max 10 connections".to_string());
        let message = get_error_message(err).await;

        let mask_internals = cfg!(not(debug_assertions));
        if mask_internals {
            assert!(
                !message.contains("DB pool"),
                "Internal detail leaked: {}",
                message
            );
            assert!(
                message.contains("Reference ID:"),
                "Missing reference ID: {}",
                message
            );
        } else {
            assert!(
                message.contains("DB pool exhausted"),
                "Debug mode should show details: {}",
                message
            );
        }
    }

    #[tokio::test]
    async fn test_client_error_never_masked() {
        let err = NurtureError::InsufficientBalance {
            required: 100,
            available: 5,
        };
        let message = get_error_message(err).await;
        assert!(
            message.contains("残高不足"),
            "Client error should be transparent: {}",
            message
        );
    }
}
