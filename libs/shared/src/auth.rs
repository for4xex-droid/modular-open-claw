/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use serde::{Deserialize, Serialize};

/// JWT Custom Claims definition for Aiome internal token validation.
/// Based on OAuth 2.1 / OIDC specs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiomeCustomClaims {
    /// Subject identifier (User ID)
    pub sub: String,

    /// eKYC verification flag (Stripe Identity checked)
    #[serde(default)]
    pub ekyc_verified: bool,

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

        let claims: AiomeCustomClaims = serde_json::from_str(json_str).unwrap();
        assert_eq!(claims.sub, "user_001");
        assert!(claims.ekyc_verified);
        assert_eq!(claims.roles, vec!["admin"]);
        assert_eq!(claims.exp, 1700000000);
    }

    #[test]
    fn test_deserialize_claims_default_values() {
        let json_str = r#"{
            "sub": "user_002",
            "exp": 1700000000
        }"#;

        let claims: AiomeCustomClaims = serde_json::from_str(json_str).unwrap();
        assert_eq!(claims.sub, "user_002");
        assert!(!claims.ekyc_verified); // default is false
        assert!(claims.roles.is_empty());
        assert_eq!(claims.exp, 1700000000);
    }
}
