/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Zero-Trust OxiLean Proof Certificate
///
/// Aiome のエッジノード (`api-server`) が Nurture（商用クラウド）に対して決済などを要求する際、
/// OXP スコアが数学的・暗号学的に妥当であることを証明するための証明書。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OxiLeanProofCertificate {
    pub subject_id: String,
    pub oxp_score: u32,
    pub timestamp: String,
    pub signature: String, // HMAC-SHA256 signature of (subject_id + oxp_score + timestamp) using Nurture Shared Secret
}

impl OxiLeanProofCertificate {
    /// 新しい証明書を生成する
    pub fn generate(subject_id: String, oxp_score: u32, timestamp: String, secret: &str) -> Self {
        let payload = format!("{}:{}:{}", subject_id, oxp_score, timestamp);
        let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
            Ok(m) => m,
            Err(_) => unreachable!("HMAC can take key of any size"), // allow-anti-pattern: unreachable is safe here
        };
        mac.update(payload.as_bytes());
        let result = mac.finalize();
        let signature = hex::encode(result.into_bytes());

        Self {
            subject_id,
            oxp_score,
            timestamp,
            signature,
        }
    }

    /// 署名を検証する
    pub fn verify(&self, secret: &str) -> bool {
        let payload = format!("{}:{}:{}", self.subject_id, self.oxp_score, self.timestamp);
        let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
            Ok(m) => m,
            Err(_) => return false,
        };
        mac.update(payload.as_bytes());

        let sig_bytes = match hex::decode(&self.signature) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        mac.verify_slice(&sig_bytes).is_ok()
    }

    /// 証明書を生成し、`X-OxiLean-Proof-Certificate` ヘッダ値（Base64 JSON）に直列化する
    pub fn generate_header(node_id: &str, oxp: u32, secret: &str) -> Option<String> {
        let ts = chrono::Utc::now().to_rfc3339();
        let cert = Self::generate(node_id.to_string(), oxp, ts, secret);
        let cert_json = serde_json::to_string(&cert).ok()?;
        use base64::Engine;
        Some(base64::engine::general_purpose::STANDARD.encode(cert_json))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_certificate_validation() {
        let secret = "super_secret_nurture_key";
        let cert = OxiLeanProofCertificate::generate(
            "agent-123".to_string(),
            950,
            "2026-04-23T12:00:00Z".to_string(),
            secret,
        );

        // Validation should succeed
        assert!(cert.verify(secret));

        // Tampering with score should fail
        let mut tampered_cert = cert.clone();
        tampered_cert.oxp_score = 999;
        assert!(!tampered_cert.verify(secret));

        // Wrong secret should fail
        assert!(!cert.verify("wrong_secret"));
    }

    #[test]
    fn test_generate_header_roundtrip() {
        let secret = "super_secret_nurture_key";
        let header = OxiLeanProofCertificate::generate_header("aiome-edge-node", 950, secret)
            .expect("header generation should succeed");
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&header)
            .expect("header should be valid base64");
        let cert: OxiLeanProofCertificate =
            serde_json::from_slice(&decoded).expect("header should be valid JSON");
        assert_eq!(cert.subject_id, "aiome-edge-node");
        assert_eq!(cert.oxp_score, 950);
        assert!(cert.verify(secret));
    }
}
