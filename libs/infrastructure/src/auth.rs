/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::error::AiomeError;
use async_trait::async_trait;
use ed25519_dalek::pkcs8::DecodePrivateKey;
use ed25519_dalek::pkcs8::EncodePrivateKey;
use shared::auth::AiomeCustomClaims;
use tracing::instrument;

#[async_trait]
pub trait AuthManager: Send + Sync {
    /// 与えられた Bearer トークンを検証し、正当であれば Custom Claims を返す
    async fn validate_token(&self, token: &str) -> anyhow::Result<AiomeCustomClaims, AiomeError>;

    /// 署名用のトークンを発行する (内部用)
    async fn issue_token(&self, claims: AiomeCustomClaims) -> anyhow::Result<String, AiomeError>;
}

/// Ed25519 を用いた JWT マネージャの実装
pub struct JwtAuthManager {
    encoding_key: jsonwebtoken::EncodingKey,
    decoding_key: jsonwebtoken::DecodingKey,
    private_key_b64: secrecy::SecretString,
}

impl JwtAuthManager {
    /// 新規に Ed25519 鍵ペアを生成してマネージャを構築する
    pub fn try_new_generated() -> Result<Self, AiomeError> {
        let signing_key = ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng);
        let verifying_key = signing_key.verifying_key();

        let pkcs8_der = signing_key.to_pkcs8_der().map_err(|e| AiomeError::Infrastructure {
            reason: format!("PKCS8 derivation failed: {}", e),
        })?;

        let encoding_key = jsonwebtoken::EncodingKey::from_ed_der(pkcs8_der.as_bytes());
        let decoding_key = jsonwebtoken::DecodingKey::from_ed_der(verifying_key.as_bytes());

        Ok(Self {
            encoding_key,
            decoding_key,
            private_key_b64: secrecy::SecretString::from(base64::Engine::encode(
                &base64::prelude::BASE64_STANDARD,
                pkcs8_der.as_bytes(),
            )),
        })
    }

    /// 保存された鍵（Base64 PKCS#8 DER）からマネージャを復元する
    pub fn from_private_key_b64(b64: &str) -> anyhow::Result<Self, AiomeError> {
        let der = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, b64).map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("Base64 decode error: {}", e),
            }
        })?;

        let signing_key = ed25519_dalek::SigningKey::from_pkcs8_der(&der).map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("Invalid PKCS8: {}", e),
            }
        })?;
        let verifying_key = signing_key.verifying_key();

        let encoding_key = jsonwebtoken::EncodingKey::from_ed_der(&der);
        let decoding_key = jsonwebtoken::DecodingKey::from_ed_der(verifying_key.as_bytes());

        Ok(Self {
            encoding_key,
            decoding_key,
            private_key_b64: secrecy::SecretString::from(b64.to_string()),
        })
    }

    /// 秘密鍵を Base64 PKCS#8 形式でエクスポートする
    pub(crate) fn export_private_key_b64(&self) -> secrecy::SecretString {
        self.private_key_b64.clone()
    }
}

#[async_trait]
impl AuthManager for JwtAuthManager {
    async fn validate_token(&self, token: &str) -> anyhow::Result<AiomeCustomClaims, AiomeError> {
        let validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::EdDSA);
        let token_data =
            jsonwebtoken::decode::<AiomeCustomClaims>(token, &self.decoding_key, &validation)
                .map_err(|e| AiomeError::SecurityViolation {
                    reason: format!("JWT Validation Error: {}", e),
                })?;
        Ok(token_data.claims)
    }

    async fn issue_token(&self, claims: AiomeCustomClaims) -> anyhow::Result<String, AiomeError> {
        let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::EdDSA);
        jsonwebtoken::encode(&header, &claims, &self.encoding_key).map_err(|e| {
            AiomeError::Infrastructure {
                reason: format!("JWT Encoding Error: {}", e),
            }
        })
    }
}

/// テスト用のモックマネージャ
pub struct MockAuthManager;

impl MockAuthManager {
    /// MockAuthManager を作成
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockAuthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuthManager for MockAuthManager {
    #[instrument(skip_all)]
    async fn validate_token(&self, token: &str) -> anyhow::Result<AiomeCustomClaims, AiomeError> {
        if token.starts_with("mock_valid_token_") {
            let user_id = token.replace("mock_valid_token_", "");
            let ekyc_verified = user_id.starts_with("ekyc_");
            let clean_id = user_id.replace("ekyc_", "");

            Ok(AiomeCustomClaims {
                sub: clean_id,
                ekyc_verified,
                // A-4: For testing, provide a non-nil agent_id to pass the Authenticated guard
                agent_id: uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                roles: vec!["user".to_string()],
                exp: 9999999999,
                iat: 1600000000,
                iss: "mock_issuer".to_string(),
            })
        } else {
            Err(AiomeError::SecurityViolation {
                reason: "Invalid Mock Token".to_string(),
            })
        }
    }

    async fn issue_token(&self, claims: AiomeCustomClaims) -> anyhow::Result<String, AiomeError> {
        Ok(format!("mock_valid_token_{}", claims.sub))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_auth_manager_valid() {
        let manager = MockAuthManager::new();
        let claims = manager
            .validate_token("mock_valid_token_user_123")
            .await
            .expect("Valid mock token should pass");
        assert_eq!(claims.sub, "user_123");
    }

    #[tokio::test]
    async fn test_mock_auth_manager_invalid() {
        let manager = MockAuthManager::new();
        let res = manager.validate_token("invalid_token").await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_jwt_key_persistence() {
        use secrecy::ExposeSecret;
        let manager1 = JwtAuthManager::try_new_generated().unwrap();
        let key_b64 = manager1.export_private_key_b64().expose_secret().clone();

        let claims = AiomeCustomClaims {
            sub: "user_p1".to_string(),
            ekyc_verified: true,
            agent_id: uuid::Uuid::new_v4(),
            roles: vec!["admin".to_string()],
            exp: 9999999999,
            iat: 1600000000,
            iss: "aiome".to_string(),
        };
        let token = manager1.issue_token(claims.clone()).await.unwrap();

        let manager2 = JwtAuthManager::from_private_key_b64(&key_b64).unwrap();
        let verified_claims = manager2.validate_token(&token).await.unwrap();
        assert_eq!(verified_claims.sub, claims.sub);
        assert_eq!(verified_claims.agent_id, claims.agent_id);
    }

    #[tokio::test]
    async fn test_jwt_expiration() {
        let manager = JwtAuthManager::try_new_generated().unwrap();
        let claims = AiomeCustomClaims {
            sub: "old_user".to_string(),
            ekyc_verified: true,
            agent_id: uuid::Uuid::new_v4(), // Ensure agent_id is not nil for testing
            roles: vec!["user".to_string()],
            exp: 1000, // Very long ago
            iat: 900,
            iss: "aiome".to_string(),
        };
        let token = manager.issue_token(claims).await.unwrap();
        let res = manager.validate_token(&token).await;

        assert!(res.is_err(), "Expired token must be rejected");
        if let Err(AiomeError::SecurityViolation { reason }) = res {
            assert!(
                reason.contains("ExpiredSignature"),
                "Error should be ExpiredSignature: {}",
                reason
            );
        } else {
            panic!("Wrong error type: {:?}", res.err());
        }
    }
}
