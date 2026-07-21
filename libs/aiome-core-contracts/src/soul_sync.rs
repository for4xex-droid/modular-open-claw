/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! Soul Sync (OP-020-F5) wire types.
//!
//! Hub relays opaque ciphertext only — it must never decrypt or persist Soul plaintext.

use serde::{Deserialize, Serialize};

/// E2E-encrypted Soul Sync envelope.
///
/// Hub treats this as an opaque blob: no decrypt API, no plaintext fields.
/// Pairing / CRDT payload lives inside `ciphertext` (client-side crypto).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedEnvelope {
    /// Opaque pairing/session id for routing (not Soul content).
    pub session_id: String,
    /// Base64 ciphertext. Hub must not log or store this as interpretable Soul data.
    pub ciphertext: String,
}

/// Mutual device pairing registration (pubkeys only — no Soul plaintext).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SoulSyncPairRequest {
    pub session_id: String,
    /// Device A public key (base64). Typically the offerer's key from the pairing code.
    pub device_a_pubkey: String,
    /// Device B public key (base64). Accepter's key after scanning A's code.
    pub device_b_pubkey: String,
}

/// Compact pairing code payload for QR / manual entry (client-rendered).
///
/// Encode with JSON then standard base64 for display. Hub never needs Soul content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SoulSyncPairingCode {
    pub session_id: String,
    pub device_pubkey: String,
}

/// Plaintext CRDT diff payload (encrypt before putting in [`EncryptedEnvelope::ciphertext`]).
///
/// `parent_hash` is the sender's pre-sync `soul_hash` (lamport-ish lineage via `record_version`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SoulSyncDiffPayload {
    pub soul_id: String,
    pub parent_hash: Option<String>,
    /// Standard base64 of Automerge document bytes (experience map keyed by Experience.id).
    pub automerge_blob_b64: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypted_envelope_has_no_plaintext_soul_fields() {
        let env = EncryptedEnvelope {
            session_id: "pair-abc".into(),
            ciphertext: "YmFzZTY0LWNpcGhlcnRleHQ=".into(),
        };
        let json = serde_json::to_string(&env).expect("serialize");
        assert!(json.contains("session_id"));
        assert!(json.contains("ciphertext"));
        assert!(
            !json.contains("experience") && !json.contains("soul_json"),
            "envelope schema must not expose Soul plaintext field names"
        );
    }
}
