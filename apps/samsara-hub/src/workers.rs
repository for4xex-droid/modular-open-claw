/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::models::{FederatedKarmaRecord, ImmuneRuleRecord};
use shared::db::DatabasePool;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

pub async fn approval_worker(pool: DatabasePool, token: CancellationToken) {
    info!("⚙️ [ApprovalWorker] Starting quarantine validation thread.");

    loop {
        if token.is_cancelled() {
            break;
        }

        // 1. Process Quarantined Karma
        let karma_fetch_query = "SELECT * FROM quarantined_karma LIMIT 50";
        let karmas: Vec<FederatedKarmaRecord> =
            match shared::sql_fetch_all!(&pool, FederatedKarmaRecord, karma_fetch_query) {
                Ok(k) => k,
                Err(e) => {
                    tracing::error!("Failed to fetch quarantined karma: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

        for k in &karmas {
            let mut valid = false;
            if let Some(ref sig_b64) = k.signature {
                let payload = format!("{}:{}:{}", k.id, k.lesson, k.lamport_clock);
                valid = crate::auth::verify_ed25519_signature(&k.node_id, sig_b64, &payload);
            }

            if valid {
                match pool.begin().await {
                    Ok(mut tx) => {
                        let approved_at_dt = chrono::Utc::now();
                        let approve_karma_query = format!(
                            "INSERT INTO approved_karma (id, node_id, karma_type, related_skill, lesson, weight, soul_version_hash, lamport_clock, signature, created_at, approved_at, clone_origin_id, generation, somatic_valence)
                             VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}) ON CONFLICT(id) DO NOTHING ",
                             pool.ph(0), pool.ph(1), pool.ph(2), pool.ph(3), pool.ph(4), pool.ph(5), pool.ph(6), pool.ph(7), pool.ph(8), pool.ph(9), pool.ph(10), pool.ph(11), pool.ph(12), pool.ph(13)
                        );
                        let res = match &mut tx {
                            shared::db::DatabaseTransaction::Sqlite(t) => {
                                sqlx::query::<sqlx::Sqlite>(&approve_karma_query)
                                    .bind(&k.id)
                                    .bind(&k.node_id)
                                    .bind(&k.karma_type)
                                    .bind(&k.related_skill)
                                    .bind(&k.lesson)
                                    .bind(k.weight as i64)
                                    .bind(&k.soul_version_hash)
                                    .bind(k.lamport_clock as i64)
                                    .bind(&k.signature)
                                    .bind(&k.created_at)
                                    .bind(&approved_at_dt)
                                    .bind(&k.clone_origin_id)
                                    .bind(k.generation.map(|v| v as i64))
                                    .bind(k.somatic_valence)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                            shared::db::DatabaseTransaction::Postgres(t) => {
                                sqlx::query::<sqlx::Postgres>(&approve_karma_query)
                                    .bind(&k.id)
                                    .bind(&k.node_id)
                                    .bind(&k.karma_type)
                                    .bind(&k.related_skill)
                                    .bind(&k.lesson)
                                    .bind(k.weight as i64)
                                    .bind(&k.soul_version_hash)
                                    .bind(k.lamport_clock as i64)
                                    .bind(&k.signature)
                                    .bind(&k.created_at)
                                    .bind(&approved_at_dt)
                                    .bind(&k.clone_origin_id)
                                    .bind(k.generation.map(|v| v as i64))
                                    .bind(k.somatic_valence)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                        };
                        if let Err(e) = res {
                            warn!(
                                "🛡️ [ApprovalWorker] Failed to insert approved karma {}: {}",
                                k.id, e
                            );
                        }

                        let delete_quarantine_query =
                            format!("DELETE FROM quarantined_karma WHERE id = {}", pool.ph(0));
                        let res_del = match &mut tx {
                            shared::db::DatabaseTransaction::Sqlite(t) => {
                                sqlx::query::<sqlx::Sqlite>(&delete_quarantine_query)
                                    .bind(&k.id)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                            shared::db::DatabaseTransaction::Postgres(t) => {
                                sqlx::query::<sqlx::Postgres>(&delete_quarantine_query)
                                    .bind(&k.id)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                        };
                        if let Err(e) = res_del {
                            warn!(
                                "🛡️ [ApprovalWorker] Failed to delete quarantined karma {}: {}",
                                k.id, e
                            );
                        }
                        if let Err(e) = tx.commit().await {
                            error!(
                                "❌ [ApprovalWorker] Failed to commit karma approval for {}: {}",
                                k.id, e
                            );
                        } else {
                            info!("✅ [ApprovalWorker] Approved Karma: {}", k.id);
                        }
                    }
                    Err(e) => error!("❌ [ApprovalWorker] Failed to start transaction: {:?}", e),
                }
            } else {
                warn!(
                    "🛡️ [ApprovalWorker] Rejecting invalid Karma (Signature Mismatch): {}",
                    k.id
                );
                // BFT Slashing
                let slash_query = format!("UPDATE node_reputation SET reputation_score = reputation_score - 10 WHERE node_id = {}", pool.ph(0));
                let _ = shared::sql_exec!(&pool, &slash_query, &k.node_id);
                let delete_malformed_query =
                    format!("DELETE FROM quarantined_karma WHERE id = {}", pool.ph(0));
                let _ = shared::sql_exec!(&pool, &delete_malformed_query, &k.id);
            }
        }

        // 2. Process Quarantined Rules
        let rule_fetch_query = "SELECT * FROM quarantined_rules LIMIT 50";
        let rules: Vec<ImmuneRuleRecord> =
            match shared::sql_fetch_all!(&pool, ImmuneRuleRecord, rule_fetch_query) {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Failed to fetch quarantined rules: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

        for r in &rules {
            let mut valid = false;
            if let Some(ref sig_b64) = r.signature {
                let payload = format!("{}:{}:{}", r.id, r.pattern, r.lamport_clock);
                valid = crate::auth::verify_ed25519_signature(&r.node_id, sig_b64, &payload);
            }

            if valid {
                match pool.begin().await {
                    Ok(mut tx) => {
                        let approved_at_dt = chrono::Utc::now();
                        let approve_rule_query = format!(
                            "INSERT INTO approved_rules (id, pattern, severity, action, node_id, lamport_clock, signature, created_at, approved_at)
                             VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}) ON CONFLICT(id) DO NOTHING ",
                             pool.ph(0), pool.ph(1), pool.ph(2), pool.ph(3), pool.ph(4), pool.ph(5), pool.ph(6), pool.ph(7), pool.ph(8)
                        );
                        let res = match &mut tx {
                            shared::db::DatabaseTransaction::Sqlite(t) => {
                                sqlx::query::<sqlx::Sqlite>(&approve_rule_query)
                                    .bind(&r.id)
                                    .bind(&r.pattern)
                                    .bind(r.severity)
                                    .bind(&r.action)
                                    .bind(&r.node_id)
                                    .bind(r.lamport_clock)
                                    .bind(&r.signature)
                                    .bind(&r.created_at)
                                    .bind(&approved_at_dt)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                            shared::db::DatabaseTransaction::Postgres(t) => {
                                sqlx::query::<sqlx::Postgres>(&approve_rule_query)
                                    .bind(&r.id)
                                    .bind(&r.pattern)
                                    .bind(r.severity)
                                    .bind(&r.action)
                                    .bind(&r.node_id)
                                    .bind(r.lamport_clock)
                                    .bind(&r.signature)
                                    .bind(&r.created_at)
                                    .bind(&approved_at_dt)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                        };
                        if let Err(e) = res {
                            warn!(
                                "🛡️ [ApprovalWorker] Failed to insert approved rule {}: {}",
                                r.id, e
                            );
                        }

                        let delete_quarantine_rule_query =
                            format!("DELETE FROM quarantined_rules WHERE id = {}", pool.ph(0));
                        let res_del = match &mut tx {
                            shared::db::DatabaseTransaction::Sqlite(t) => {
                                sqlx::query::<sqlx::Sqlite>(&delete_quarantine_rule_query)
                                    .bind(&r.id)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                            shared::db::DatabaseTransaction::Postgres(t) => {
                                sqlx::query::<sqlx::Postgres>(&delete_quarantine_rule_query)
                                    .bind(&r.id)
                                    .execute(&mut **t)
                                    .await
                                    .map(|_| ())
                            }
                        };
                        if let Err(e) = res_del {
                            warn!(
                                "🛡️ [ApprovalWorker] Failed to delete quarantined rule {}: {}",
                                r.id, e
                            );
                        }
                        if let Err(e) = tx.commit().await {
                            error!(
                                "❌ [ApprovalWorker] Failed to commit rule approval for {}: {}",
                                r.id, e
                            );
                        } else {
                            info!("✅ [ApprovalWorker] Approved Rule: {}", r.id);
                        }
                    }
                    Err(e) => error!("❌ [ApprovalWorker] Failed to start transaction: {:?}", e),
                }
            } else {
                warn!(
                    "🛡️ [ApprovalWorker] Rejecting invalid Rule (Signature Mismatch): {}",
                    r.id
                );
                // BFT Slashing
                let slash_rule_query = format!("UPDATE node_reputation SET reputation_score = reputation_score - 10 WHERE node_id = {}", pool.ph(0));
                let _ = shared::sql_exec!(&pool, &slash_rule_query, &r.node_id);
                let delete_malformed_rule_query =
                    format!("DELETE FROM quarantined_rules WHERE id = {}", pool.ph(0));
                let _ = shared::sql_exec!(&pool, &delete_malformed_rule_query, &r.id);
            }
        }

        // 3. Data Eviction (Flaw 3: Disk Exhaustion Defense)
        // Keep ONLY the last 1,000,000 Records
        let karma_evict_query = "DELETE FROM approved_karma WHERE id NOT IN (SELECT id FROM approved_karma ORDER BY approved_at DESC LIMIT 1000000)";
        let rule_evict_query = "DELETE FROM approved_rules WHERE id NOT IN (SELECT id FROM approved_rules ORDER BY approved_at DESC LIMIT 1000000)";

        let q_karma_prune = "DELETE FROM quarantined_karma WHERE id NOT IN (SELECT id FROM quarantined_karma ORDER BY received_at DESC LIMIT 100000)";
        let q_rules_prune = "DELETE FROM quarantined_rules WHERE id NOT IN (SELECT id FROM quarantined_rules ORDER BY received_at DESC LIMIT 100000)";

        let res_k = shared::sql_exec!(&pool, karma_evict_query);
        if let Err(e) = res_k {
            warn!("⚠️ [SamsaraHub] Karma eviction failed: {}", e);
        }

        let res_r = shared::sql_exec!(&pool, rule_evict_query);
        if let Err(e) = res_r {
            warn!("⚠️ [SamsaraHub] Rule eviction failed: {}", e);
        }

        let res_qk = shared::sql_exec!(&pool, q_karma_prune);
        if let Err(e) = res_qk {
            warn!("⚠️ [SamsaraHub] Quarantined Karma prune failed: {}", e);
        }

        let res_qr = shared::sql_exec!(&pool, q_rules_prune);
        if let Err(e) = res_qr {
            warn!("⚠️ [SamsaraHub] Quarantined Rule prune failed: {}", e);
        }

        // Dynamic Polling (Component 2: Backpressure Tuning)
        let total_processed = karmas.len() + rules.len();
        if total_processed >= 100 {
            // High load: Don't sleep, keep processing quarantine
            tokio::task::yield_now().await;
        } else {
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}
