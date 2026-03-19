/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::error::AiomeError;
use async_trait::async_trait;
use shared::auth::AiomeCustomClaims;
use tracing::instrument;

/// 外部 IdP または内部での JWT トークン検証を行うマネージャ
#[async_trait]
pub trait AuthManager: Send + Sync {
    /// 与えられた Bearer トークンを検証し、正当であれば Custom Claims を返す
    async fn validate_token(&self, token: &str) -> anyhow::Result<AiomeCustomClaims, AiomeError>;
}

/// テスト用のモックマネージャ
/// 指定されたプレフィックスに基づくトークンで、対応するモック情報を返す
pub struct MockAuthManager;

impl MockAuthManager {
    /// Create a new instance of MockAuthManager.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AuthManager for MockAuthManager {
    #[instrument(skip_all)]
    async fn validate_token(&self, token: &str) -> anyhow::Result<AiomeCustomClaims, AiomeError> {
        if token.starts_with("mock_valid_token_") {
            let user_id = token.replace("mock_valid_token_", "");

            // "ekyc_" から始まる ID の場合は eKYC 完了とするテスト用ロジック
            let ekyc_verified = user_id.starts_with("ekyc_");
            let clean_id = user_id.replace("ekyc_", "");

            Ok(AiomeCustomClaims {
                sub: clean_id,
                ekyc_verified,
                roles: vec!["user".to_string()],
                exp: 9999999999, // 遠い未来
                iat: 1600000000,
                iss: "mock_issuer".to_string(),
            })
        } else {
            Err(AiomeError::SecurityViolation {
                reason: "Invalid Mock Token".to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_auth_manager_valid() {
        let manager = MockAuthManager::new();

        // 通常のユーザ
        let claims = manager
            .validate_token("mock_valid_token_user_123")
            .await
            .expect("Valid mock token should pass");
        assert_eq!(claims.sub, "user_123");
        assert!(!claims.ekyc_verified);

        // eKYC 完了のユーザ
        let ekyc_claims = manager
            .validate_token("mock_valid_token_ekyc_user_123")
            .await
            .expect("Valid ekyc mock token should pass");
        assert_eq!(ekyc_claims.sub, "user_123");
        assert!(ekyc_claims.ekyc_verified);
    }

    #[tokio::test]
    async fn test_mock_auth_manager_invalid() {
        let manager = MockAuthManager::new();
        let res = manager.validate_token("invalid_token").await;
        assert!(res.is_err());
    }
}
