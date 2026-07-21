/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
#![allow(clippy::unwrap_used)]

//! OP-020-F5 S-4: two-node Soul Sync — Experience on A reaches B's soul_store within 60s.
//!
//! Transport: E2E encrypt → opaque hub bus (mpsc, ciphertext only) → decrypt → apply CRDT.
//! Pattern aligned with federation chaos tests (isolated in-memory nodes + timed assertion).

use ed25519_dalek::SigningKey;
use infrastructure::db::DatabasePool;
use infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore;
use infrastructure::job_queue::UniversalJobQueue;
use infrastructure::soul_experience_crdt::experience_set_hash;
use infrastructure::soul_store::UniversalSoulStore;
use infrastructure::soul_sync_transport::{open_experience_diff, seal_experience_diff};
use rand::{thread_rng, RngCore};
use soul::{AgentSoul, Experience};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

async fn node_store() -> UniversalSoulStore {
    let pool = DatabasePool::new_sqlite("sqlite::memory:").await.unwrap();
    let ts = Arc::new(SqliteTrajectoryStore::new(pool.clone()));
    let _jq = UniversalJobQueue::new(pool.clone(), None, ts)
        .await
        .expect("job queue init (migrations)");
    UniversalSoulStore::new(pool)
}

#[tokio::test]
async fn test_soul_sync_two_node_experience_within_60s() {
    const CANARY: &str = "EXPERIENCE_FROM_DEVICE_A_CANARY";
    let deadline = Duration::from_secs(60);
    let started = Instant::now();

    // Device B keypair (A encrypts to B).
    let mut b_seed = [0u8; 32];
    thread_rng().fill_bytes(&mut b_seed);
    let b_pubkey = SigningKey::from_bytes(&b_seed).verifying_key().to_bytes();

    let store_a = node_store().await;
    let store_b = node_store().await;
    let soul_id = "soul-two-node";

    let mut soul_a = AgentSoul::new(soul_id.to_string());
    soul_a.experience_buffer.push(Experience {
        id: "exp-from-a".into(),
        domain: "sync".into(),
        content: CANARY.into(),
        outcome_valence: 0.8,
        timestamp: "2026-07-22T04:00:00Z".into(),
        original_prediction: 0.1,
        is_core_memory: false,
        embedding: None,
    });
    store_a.save_soul(&soul_a).await.unwrap();

    let soul_b = AgentSoul::new(soul_id.to_string());
    store_b.save_soul(&soul_b).await.unwrap();

    let parent_hash = experience_set_hash(&soul_a.experience_buffer).unwrap();
    let envelope = seal_experience_diff(
        "sess-two-node",
        soul_id,
        Some(parent_hash),
        &soul_a.experience_buffer,
        &b_pubkey,
    )
    .unwrap();

    // Opaque hub bus: must not carry Soul plaintext in the wire field.
    assert!(
        !envelope.ciphertext.contains(CANARY),
        "hub-visible ciphertext must not contain Soul plaintext"
    );

    let (hub_tx, mut hub_rx) = mpsc::unbounded_channel();
    hub_tx.send(envelope).unwrap();

    // Node B receives within 60s (acceptance criterion 1).
    let received = tokio::time::timeout(deadline, hub_rx.recv())
        .await
        .expect("timed out waiting for hub relay (>60s)")
        .expect("hub bus closed");

    let (_payload, blob) = open_experience_diff(&received, &b_seed).unwrap();
    let (synced_b, _) = store_b
        .apply_experience_sync_diff(soul_id, &blob)
        .await
        .unwrap();

    assert!(
        started.elapsed() < deadline,
        "sync exceeded 60s: {:?}",
        started.elapsed()
    );
    assert!(
        synced_b
            .experience_buffer
            .iter()
            .any(|e| e.id == "exp-from-a" && e.content == CANARY),
        "device B soul_store missing A's Experience"
    );

    // Negative: wrong device seed cannot open the envelope.
    let mut wrong_seed = [0u8; 32];
    thread_rng().fill_bytes(&mut wrong_seed);
    assert!(
        open_experience_diff(&received, &wrong_seed).is_err(),
        "wrong recipient must fail decrypt"
    );
}

#[tokio::test]
async fn test_soul_sync_two_node_idempotent_over_hub_bus() {
    let mut b_seed = [0u8; 32];
    thread_rng().fill_bytes(&mut b_seed);
    let b_pubkey = SigningKey::from_bytes(&b_seed).verifying_key().to_bytes();

    let store_a = node_store().await;
    let store_b = node_store().await;
    let soul_id = "soul-idem-bus";

    let mut soul_a = AgentSoul::new(soul_id.to_string());
    soul_a.experience_buffer.push(Experience {
        id: "once".into(),
        domain: "sync".into(),
        content: "once-only".into(),
        outcome_valence: 0.0,
        timestamp: "2026-07-22T04:10:00Z".into(),
        original_prediction: 0.0,
        is_core_memory: false,
        embedding: None,
    });
    store_a.save_soul(&soul_a).await.unwrap();
    store_b
        .save_soul(&AgentSoul::new(soul_id.to_string()))
        .await
        .unwrap();

    let env = seal_experience_diff(
        "sess-idem",
        soul_id,
        None,
        &soul_a.experience_buffer,
        &b_pubkey,
    )
    .unwrap();
    let (_, blob) = open_experience_diff(&env, &b_seed).unwrap();

    let (b1, _) = store_b
        .apply_experience_sync_diff(soul_id, &blob)
        .await
        .unwrap();
    let (b2, _) = store_b
        .apply_experience_sync_diff(soul_id, &blob)
        .await
        .unwrap();
    assert_eq!(b1.experience_buffer.len(), 1);
    assert_eq!(b2.experience_buffer.len(), 1);
}
