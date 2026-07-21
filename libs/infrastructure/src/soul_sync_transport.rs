/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! OP-020-F5 S-4: client-side seal/open for Soul Sync diffs over opaque hub relay.

use aiome_core::error::AiomeError;
use aiome_core_contracts::soul_sync::{EncryptedEnvelope, SoulSyncDiffPayload};
use base64::Engine;
use soul::Experience;

use crate::soul_experience_crdt::experiences_to_automerge;

/// Build an opaque hub envelope containing an encrypted Automerge experience diff.
pub fn seal_experience_diff(
    session_id: &str,
    soul_id: &str,
    parent_hash: Option<String>,
    experiences: &[Experience],
    recipient_ed25519_pubkey: &[u8; 32],
) -> Result<EncryptedEnvelope, AiomeError> {
    let blob = experiences_to_automerge(experiences)?;
    let payload = SoulSyncDiffPayload {
        soul_id: soul_id.to_string(),
        parent_hash,
        automerge_blob_b64: base64::engine::general_purpose::STANDARD.encode(&blob),
    };
    let plaintext = serde_json::to_string(&payload).map_err(|e| AiomeError::Infrastructure {
        reason: format!("SoulSyncDiffPayload serialize failed: {e}"),
    })?;
    let ciphertext = shared::crypto::encrypt_message(&plaintext, recipient_ed25519_pubkey)
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Soul Sync encrypt failed: {e}"),
        })?;
    Ok(EncryptedEnvelope {
        session_id: session_id.to_string(),
        ciphertext,
    })
}

/// Decrypt a hub envelope into the CRDT blob (hub never sees this plaintext).
pub fn open_experience_diff(
    envelope: &EncryptedEnvelope,
    recipient_ed25519_seed: &[u8; 32],
) -> Result<(SoulSyncDiffPayload, Vec<u8>), AiomeError> {
    let plaintext = shared::crypto::decrypt_message(&envelope.ciphertext, recipient_ed25519_seed)
        .map_err(|e| AiomeError::Infrastructure {
        reason: format!("Soul Sync decrypt failed: {e}"),
    })?;
    let payload: SoulSyncDiffPayload =
        serde_json::from_str(&plaintext).map_err(|e| AiomeError::Infrastructure {
            reason: format!("SoulSyncDiffPayload deserialize failed: {e}"),
        })?;
    let blob = base64::engine::general_purpose::STANDARD
        .decode(&payload.automerge_blob_b64)
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Soul Sync automerge_blob_b64 decode failed: {e}"),
        })?;
    Ok((payload, blob))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::{thread_rng, RngCore};
    use soul::Experience;

    #[test]
    fn seal_open_roundtrip() {
        let mut seed = [0u8; 32];
        thread_rng().fill_bytes(&mut seed);
        let pub_key = SigningKey::from_bytes(&seed).verifying_key().to_bytes();

        let exps = vec![Experience {
            id: "e-seal".into(),
            domain: "test".into(),
            content: "sealed".into(),
            outcome_valence: 0.2,
            timestamp: "2026-07-22T03:00:00Z".into(),
            original_prediction: 0.0,
            is_core_memory: false,
            embedding: None,
        }];
        let env = seal_experience_diff("sess", "soul-1", None, &exps, &pub_key).unwrap();
        assert!(!env.ciphertext.contains("sealed"));
        let (payload, blob) = open_experience_diff(&env, &seed).unwrap();
        assert_eq!(payload.soul_id, "soul-1");
        assert!(!blob.is_empty());
    }
}
