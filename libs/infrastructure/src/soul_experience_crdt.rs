/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! OP-020-F5 S-3: Automerge CRDT for Soul `experience_buffer`.
//!
//! Each Experience is a ROOT scalar keyed by `e:{id}` so concurrent docs merge by
//! key union (nested `put_object` maps conflict across independently created docs).
//! Pattern mirrors hub `timeline.rs` / `job_queue/crdt.rs` (AutoCommit load → merge → save).

use aiome_core::error::AiomeError;
use automerge::{transaction::Transactable, AutoCommit, ReadDoc, Value, ROOT};
use sha2::{Digest, Sha256};
use soul::Experience;

/// Hard limit aligned with timeline / hub CRDT (1 MiB).
pub const SOUL_SYNC_CRDT_MAX_BYTES: usize = 1024 * 1024;

const EXP_KEY_PREFIX: &str = "e:";

fn experience_key(id: &str) -> String {
    format!("{EXP_KEY_PREFIX}{id}")
}

/// Encode experiences into an Automerge document blob (ROOT keys `e:{id}`).
pub fn experiences_to_automerge(experiences: &[Experience]) -> Result<Vec<u8>, AiomeError> {
    let mut doc = AutoCommit::new();

    for exp in experiences {
        let json = serde_json::to_string(exp).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Experience serialize failed: {e}"),
        })?;
        doc.put(ROOT, experience_key(&exp.id), json)
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Soul Sync Automerge put failed: {e}"),
            })?;
    }

    Ok(doc.save())
}

/// Decode experiences from an Automerge blob.
pub fn experiences_from_automerge(blob: &[u8]) -> Result<Vec<Experience>, AiomeError> {
    if blob.is_empty() {
        return Ok(Vec::new());
    }
    let doc = AutoCommit::load(blob).map_err(|e| AiomeError::Infrastructure {
        reason: format!("Soul Sync Automerge load failed: {e}"),
    })?;

    let mut out = Vec::new();
    for key in doc.keys(ROOT) {
        if !key.starts_with(EXP_KEY_PREFIX) {
            continue;
        }
        let Some((val, _)) =
            doc.get(ROOT, key.as_str())
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Soul Sync Automerge get key failed: {e}"),
                })?
        else {
            continue;
        };
        let json = match val {
            Value::Scalar(s) => s.to_string().trim_matches('"').to_string(),
            other => {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Soul Sync experience value is not scalar: {other:?}"),
                });
            }
        };
        let exp: Experience =
            serde_json::from_str(&json).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Experience deserialize failed: {e}"),
            })?;
        out.push(exp);
    }

    out.sort_by(|a, b| a.timestamp.cmp(&b.timestamp).then(a.id.cmp(&b.id)));
    Ok(out)
}

/// Merge local and remote Automerge blobs (remote size-capped). Returns merged blob.
pub fn merge_experience_blobs(
    local_blob: &[u8],
    remote_blob: &[u8],
) -> Result<Vec<u8>, AiomeError> {
    if remote_blob.len() > SOUL_SYNC_CRDT_MAX_BYTES {
        return Err(AiomeError::SecurityViolation {
            reason: format!(
                "Soul Sync CRDT remote blob exceeds maximum allowed size of {} bytes",
                SOUL_SYNC_CRDT_MAX_BYTES
            ),
        });
    }

    let mut local = if local_blob.is_empty() {
        AutoCommit::new()
    } else {
        AutoCommit::load(local_blob).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Soul Sync local Automerge load failed: {e}"),
        })?
    };

    if !remote_blob.is_empty() {
        let mut remote = AutoCommit::load(remote_blob).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Soul Sync remote Automerge load failed: {e}"),
        })?;
        local
            .merge(&mut remote)
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Soul Sync Automerge merge failed: {e}"),
            })?;
    }

    Ok(local.save())
}

/// Merge remote experience CRDT into a local buffer; same Experience.id applied twice is idempotent.
pub fn merge_experiences_idempotent(
    local: &[Experience],
    remote_blob: &[u8],
) -> Result<Vec<Experience>, AiomeError> {
    let local_blob = experiences_to_automerge(local)?;
    let merged_blob = merge_experience_blobs(&local_blob, remote_blob)?;
    experiences_from_automerge(&merged_blob)
}

/// Stable SHA-256 over sorted Experience JSON (for `record_version` lineage).
pub fn experience_set_hash(experiences: &[Experience]) -> Result<String, AiomeError> {
    let mut parts: Vec<(String, String)> = Vec::with_capacity(experiences.len());
    for exp in experiences {
        let json = serde_json::to_string(exp).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Experience serialize failed: {e}"),
        })?;
        parts.push((exp.id.clone(), json));
    }
    parts.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (_, json) in parts {
        hasher.update(json.as_bytes());
        hasher.update([0xff]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use soul::Experience;

    fn sample_exp(id: &str, content: &str) -> Experience {
        Experience {
            id: id.to_string(),
            domain: "test".into(),
            content: content.into(),
            outcome_valence: 0.5,
            timestamp: "2026-07-22T00:00:00Z".into(),
            original_prediction: 0.0,
            is_core_memory: false,
            embedding: None,
        }
    }

    #[test]
    fn roundtrip_experiences_automerge() {
        let exps = vec![sample_exp("e1", "hello"), sample_exp("e2", "world")];
        let blob = experiences_to_automerge(&exps).unwrap();
        let back = experiences_from_automerge(&blob).unwrap();
        assert_eq!(back.len(), 2);
        assert!(back.iter().any(|e| e.id == "e1" && e.content == "hello"));
        assert!(back.iter().any(|e| e.id == "e2" && e.content == "world"));
    }

    #[test]
    fn double_apply_same_experience_is_idempotent() {
        let local = vec![sample_exp("local-1", "on-a")];
        let remote_only = vec![sample_exp("remote-1", "from-b")];
        let remote_blob = experiences_to_automerge(&remote_only).unwrap();

        let once = merge_experiences_idempotent(&local, &remote_blob).unwrap();
        assert_eq!(once.len(), 2);

        // Negative / acceptance #3: re-apply identical remote blob must not duplicate.
        let twice = merge_experiences_idempotent(&once, &remote_blob).unwrap();
        assert_eq!(twice.len(), 2, "duplicate Experience must not be appended");
        let ids: Vec<_> = twice.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&"local-1"));
        assert!(ids.contains(&"remote-1"));
        assert_eq!(ids.iter().filter(|id| **id == "remote-1").count(), 1);
    }

    #[test]
    fn reject_oversized_remote_blob() {
        let giant = vec![0u8; SOUL_SYNC_CRDT_MAX_BYTES + 1];
        let err = merge_experience_blobs(&[], &giant).unwrap_err();
        match err {
            AiomeError::SecurityViolation { reason } => {
                assert!(reason.contains("maximum allowed size"));
            }
            other => panic!("expected SecurityViolation, got {other:?}"),
        }
    }
}
