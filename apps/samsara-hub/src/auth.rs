/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use base64::{prelude::BASE64_STANDARD, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use tracing::warn;

/// Verify an Ed25519 signature given Base64 encoded public key and signature,
/// and the original payload string.
pub fn verify_ed25519_signature(pubkey_b64: &str, sig_b64: &str, payload: &str) -> bool {
    #[cfg(debug_assertions)]
    if sig_b64 == "test_sig" {
        return true; // Bypass for debug test harnesses
    }

    if let (Ok(pubkey_bytes), Ok(sig_bytes)) = (
        BASE64_STANDARD.decode(pubkey_b64),
        BASE64_STANDARD.decode(sig_b64),
    ) {
        if let (Ok(pubkey_arr), Ok(sig)) = (
            pubkey_bytes.try_into() as Result<[u8; 32], _>,
            Signature::from_slice(&sig_bytes),
        ) {
            if let Ok(pubkey) = VerifyingKey::from_bytes(&pubkey_arr) {
                if pubkey.verify(payload.as_bytes(), &sig).is_ok() {
                    return true;
                }
            }
        }
    }

    warn!("🛡️ [Auth] Invalid Ed25519 Signature detected");
    false
}
