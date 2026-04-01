/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_contracts::error::AiomeError;
use async_trait::async_trait;
use ed25519_dalek::pkcs8::{DecodePrivateKey, EncodePrivateKey};
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// JWT Custom Claims definition for Aiome internal token validation.
/// Based on OAuth 2.1 / OIDC specs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiomeCustomClaims {
    /// Subject identifier (User ID)
    pub sub: String,

    /// eKYC verification flag (Stripe Identity checked)
    #[serde(default)]
    pub ekyc_verified: bool,

    /// Agent identifier target for this session (Scope-based RBAC)
    #[serde(default = "uuid::Uuid::nil")]
    pub agent_id: uuid::Uuid,

    /// User roles for RBAC
    #[serde(default)]
    pub roles: Vec<String>,

    /// Expiration time (unix timestamp)
    pub exp: usize,

    /// Issued at (unix timestamp)
    #[serde(default)]
    pub iat: usize,

    /// Issuer identifier
    #[serde(default)]
    pub iss: String,
}

/// 認証マネージャの基盤トレイト
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

        let pkcs8_der = signing_key
            .to_pkcs8_der()
            .map_err(|e| AiomeError::Infrastructure {
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
    pub fn export_private_key_b64(&self) -> secrecy::SecretString {
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
#[cfg(any(test, debug_assertions))]
pub struct MockAuthManager;

#[cfg(any(test, debug_assertions))]
impl MockAuthManager {
    /// MockAuthManager を作成
    pub fn new() -> Self {
        Self
    }
}

#[cfg(any(test, debug_assertions))]
impl Default for MockAuthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, debug_assertions))]
#[async_trait]
impl AuthManager for MockAuthManager {
    #[instrument(skip_all)]
    async fn validate_token(&self, token: &str) -> anyhow::Result<AiomeCustomClaims, AiomeError> {
        if token.starts_with("mock_valid_token_") {
            let part = token.replace("mock_valid_token_", "");
            let components: Vec<&str> = part.split(':').collect();

            let (sub, agent_id_str) = if components.len() >= 2 {
                (components[0].to_string(), Some(components[1]))
            } else {
                (part, None)
            };

            let ekyc_verified = sub.starts_with("ekyc");
            let clean_sub = sub.replace("ekyc", "");

            let agent_id = if let Some(id_str) = agent_id_str {
                uuid::Uuid::parse_str(id_str).map_err(|e| AiomeError::SecurityViolation {
                    reason: format!("Invalid Agent ID in mock token: {}", e),
                })?
            } else {
                uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
            };

            Ok(AiomeCustomClaims {
                sub: clean_sub,
                ekyc_verified,
                agent_id,
                roles: vec!["user".to_string()],
                exp: 9999999999,
                iat: 1600000000,
                iss: "mock_issuer".to_string(),
            })
        } else if token == "mock_token" {
            Ok(AiomeCustomClaims {
                sub: "dev".to_string(),
                ekyc_verified: true,
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

    #[test]
    fn test_deserialize_claims() {
        let json_str = r#"{
            "sub": "user_001",
            "ekyc_verified": true,
            "roles": ["admin"],
            "exp": 1700000000,
            "iat": 1600000000,
            "iss": "https://auth.aiome.network"
        }"#;

        let claims: AiomeCustomClaims =
            serde_json::from_str(json_str).expect("Valid test claims JSON");
        assert_eq!(claims.sub, "user_001");
        assert!(claims.ekyc_verified);
        assert_eq!(claims.roles, vec!["admin"]);
        assert_eq!(claims.exp, 1700000000);
    }

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
}
