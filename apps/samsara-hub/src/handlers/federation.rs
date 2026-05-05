/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use crate::handlers::verify_bearer;
use crate::mdns_listener::AgentInfo;
use crate::models::*;
use crate::state::HubState;
use aiome_core::contracts::ImmuneRule;
use aiome_core::contracts::{
    ApprovalState, FederatedKarma, FederatedMetrics, FederationPushRequest, FederationPushResponse,
    FederationSyncRequest, FederationSyncResponse, HubMessage,
};
use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde_json::Value;
use shared::sql_fetch_all;
use std::sync::Arc;
use tracing::{error, info, warn};

pub async fn sync_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Json(payload): Json<FederationSyncRequest>,
) -> impl IntoResponse {
    // Auth Wall
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let mut authenticated = false;
    if auth_header.starts_with("Bearer ") {
        let token = auth_header.trim_start_matches("Bearer ");
        if let Ok(claims) = state.auth_manager.validate_token(token).await {
            if claims.agent_id != uuid::Uuid::nil() {
                authenticated = true;
            }
        }
    }

    if !authenticated && verify_bearer(auth_header, &state.secret) {
        authenticated = true;
    }

    if !authenticated {
        warn!(
            "🔒 Unauthorized sync attempt from node: {}",
            payload.node_id
        );
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized"})),
        )
            .into_response();
    }

    // BFT: BAN Check
    // BFT: BAN Check
    let ban_check_query = format!(
        "SELECT is_banned FROM node_reputation WHERE node_id = {}",
        state.pool.ph(0)
    );
    let is_banned =
        shared::sql_fetch_optional!(&state.pool, (bool,), &ban_check_query, &payload.node_id)
            .unwrap_or(Some((false,)))
            .unwrap_or((false,))
            .0;

    if is_banned {
        warn!(
            "🛡️ [BFT] Rejecting sync from BANNED node: {}",
            payload.node_id
        );
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Node is banned"})),
        )
            .into_response();
    }

    info!(
        "🌐 Node {} pulling approved updates since {:?}",
        payload.node_id, payload.since
    );

    let since = payload
        .since
        .unwrap_or_else(|| "1970-01-01T00:00:00".to_string());

    // Fetch ONLY approved data with Pagination (Flaw 2: OOM Defense)
    // Fetch approved AND quarantined data for synchronization (Phase 31 reliability)
    let karma_sync_query = format!(
        "SELECT id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, node_id, signature, clone_origin_id, generation, somatic_valence FROM (
            SELECT id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, node_id, signature, clone_origin_id, generation, somatic_valence, approved_at as ts FROM approved_karma
            UNION ALL
            SELECT id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, node_id, signature, clone_origin_id, generation, somatic_valence, received_at as ts FROM quarantined_karma
         ) WHERE ts > {} ORDER BY ts ASC LIMIT 500",
         state.pool.ph(0)
    );
    let karmas: Vec<FederatedKarmaRecord> =
        shared::sql_fetch_all!(&state.pool, FederatedKarmaRecord, &karma_sync_query, &since)
            .unwrap_or_default();
    let rule_sync_query = format!(
        "SELECT id, pattern, severity, action, created_at, lamport_clock, node_id, signature FROM approved_rules
         WHERE approved_at > {} ORDER BY approved_at ASC LIMIT 500",
         state.pool.ph(0)
    );
    let rules: Vec<ImmuneRuleRecord> =
        shared::sql_fetch_all!(&state.pool, ImmuneRuleRecord, &rule_sync_query, &since)
            .unwrap_or_default();
    let has_more = karmas.len() == 500 || rules.len() == 500;

    let arena_sync_query = format!("SELECT id, skill_a, skill_b, topic, output_a, output_b, winner, reasoning, created_at FROM approved_arena_matches WHERE approved_at > {} ORDER BY approved_at ASC LIMIT 500", state.pool.ph(0));
    let arena_rows: Vec<ArenaMatchRecord> =
        shared::sql_fetch_all!(&state.pool, ArenaMatchRecord, &arena_sync_query, &since)
            .unwrap_or_default();

    // Fetch latest Automerge Snapshot for this node if it exists
    let snapshot_query = format!(
        "SELECT snapshot_blob FROM timeline_snapshots WHERE node_id = {}",
        state.pool.ph(0)
    );
    let snapshot_blob: Option<Vec<u8>> =
        shared::sql_fetch_optional!(&state.pool, (Vec<u8>,), &snapshot_query, &payload.node_id)
            .unwrap_or_else(|_| Some((Vec::new(),)))
            .map(|t| t.0);

    let _next_cursor: Option<String> = if has_more {
        // Find the latest approved_at for pagination (Keyset Pagination)
        // For simplicity, we just use the last item's timestamp if we hit the limit
        // In a real high-perf system, we'd query for the max timestamp in the results.
        None // Placeholder: will be refined if needed, but since is enough for now.
    } else {
        None
    };

    let response = FederationSyncResponse {
        new_karmas: karmas
            .into_iter()
            .map(|k| FederatedKarma {
                id: k.id,
                job_id: None,
                karma_type: k.karma_type,
                related_skill: k.related_skill,
                lesson: k.lesson,
                weight: k.weight as i32,
                last_applied_at: Some(k.created_at.clone()),
                created_at: k.created_at,
                soul_version_hash: k.soul_version_hash,
                lamport_clock: k.lamport_clock as u64,
                node_id: k.node_id,
                signature: k.signature,
                clone_origin_id: k.clone_origin_id,
                generation: k.generation.map(|g| g as u32),
                somatic_valence: k.somatic_valence,
                score: 0.0,
            })
            .collect(),
        new_immune_rules: rules
            .into_iter()
            .map(|r| ImmuneRule {
                id: r.id,
                pattern: r.pattern,
                severity: r.severity as u8,
                action: r.action,
                created_at: r.created_at,
                approval_status: ApprovalState::Approved,
                input_constraints: None,
                lamport_clock: r.lamport_clock as u64,
                node_id: r.node_id,
                signature: r.signature,
            })
            .collect(),
        new_arena_matches: arena_rows
            .into_iter()
            .map(|a| aiome_core::contracts::ArenaMatch {
                id: a.id,
                skill_a: a.skill_a,
                skill_b: a.skill_b,
                topic: a.topic,
                output_a: a.output_a,
                output_b: a.output_b,
                winner: a.winner,
                reasoning: a.reasoning,
                created_at: a.created_at,
            })
            .collect(),
        server_time: chrono::Utc::now().to_rfc3339(),
        next_cursor: None,
        has_more,
        automerge_snapshot: snapshot_blob,
    };

    (StatusCode::OK, Json(response)).into_response()
}

pub async fn push_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Json(mut payload): Json<FederationPushRequest>,
) -> impl IntoResponse {
    // 🛡️ [GlassWorm Shield] Sanitize all inbound text fields to prevent Federation Worm Attack
    payload.node_id = shared::guardrails::strip_invisible_unicode(&payload.node_id).into_owned();
    for k in &mut payload.karmas {
        k.id = shared::guardrails::strip_invisible_unicode(&k.id).into_owned();
        k.job_id = k
            .job_id
            .take()
            .map(|s| shared::guardrails::strip_invisible_unicode(&s).into_owned());
        k.karma_type = shared::guardrails::strip_invisible_unicode(&k.karma_type).into_owned();
        k.lesson = shared::guardrails::strip_invisible_unicode(&k.lesson).into_owned();
        k.related_skill =
            shared::guardrails::strip_invisible_unicode(&k.related_skill).into_owned();
        k.node_id = shared::guardrails::strip_invisible_unicode(&k.node_id).into_owned();
        k.signature = k
            .signature
            .take()
            .map(|s| shared::guardrails::strip_invisible_unicode(&s).into_owned());
        k.clone_origin_id = k
            .clone_origin_id
            .take()
            .map(|s| shared::guardrails::strip_invisible_unicode(&s).into_owned());
    }
    for r in &mut payload.rules {
        r.id = shared::guardrails::strip_invisible_unicode(&r.id).into_owned();
        r.pattern = shared::guardrails::strip_invisible_unicode(&r.pattern).into_owned();
        r.action = shared::guardrails::strip_invisible_unicode(&r.action).into_owned();
        r.node_id = shared::guardrails::strip_invisible_unicode(&r.node_id).into_owned();
        r.signature = r
            .signature
            .take()
            .map(|s| shared::guardrails::strip_invisible_unicode(&s).into_owned());
    }
    for m in &mut payload.arena_matches {
        m.id = shared::guardrails::strip_invisible_unicode(&m.id).into_owned();
        m.skill_a = shared::guardrails::strip_invisible_unicode(&m.skill_a).into_owned();
        m.skill_b = shared::guardrails::strip_invisible_unicode(&m.skill_b).into_owned();
        m.topic = shared::guardrails::strip_invisible_unicode(&m.topic).into_owned();
        m.output_a = m
            .output_a
            .take()
            .map(|s| shared::guardrails::strip_invisible_unicode(&s).into_owned());
        m.output_b = m
            .output_b
            .take()
            .map(|s| shared::guardrails::strip_invisible_unicode(&s).into_owned());
        m.winner = m
            .winner
            .take()
            .map(|s| shared::guardrails::strip_invisible_unicode(&s).into_owned());
        m.reasoning = shared::guardrails::strip_invisible_unicode(&m.reasoning).into_owned();
    }

    // Auth Wall
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let mut authenticated = false;
    if auth_header.starts_with("Bearer ") {
        let token = auth_header.trim_start_matches("Bearer ");
        if let Ok(claims) = state.auth_manager.validate_token(token).await {
            if claims.agent_id != uuid::Uuid::nil() {
                authenticated = true;
            }
        }
    }

    if !authenticated && verify_bearer(auth_header, &state.secret) {
        authenticated = true;
    }

    if !authenticated {
        warn!(
            "🔒 Unauthorized push attempt from node: {}",
            payload.node_id
        );
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized"})),
        )
            .into_response();
    }

    // BFT: BAN Check
    let ban_check_query = format!(
        "SELECT is_banned FROM node_reputation WHERE node_id = {}",
        state.pool.ph(0)
    );
    let is_banned =
        shared::sql_fetch_optional!(&state.pool, (bool,), &ban_check_query, &payload.node_id)
            .unwrap_or(Some((false,)))
            .unwrap_or((false,))
            .0;

    if is_banned {
        warn!(
            "🛡️ [BFT] Rejecting push from BANNED node: {}",
            payload.node_id
        );
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Node is banned"})),
        )
            .into_response();
    }

    let karma_count = payload.karmas.len();
    let rule_count = payload.rules.len();
    info!(
        "📥 Received push from node {}: {} Karmas, {} Rules. Sending to Quarantine.",
        payload.node_id, karma_count, rule_count
    );

    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal error"})),
            )
                .into_response()
        }
    };

    let received_at_dt = chrono::Utc::now();
    for k in &payload.karmas {
        // BFT: Equivocation Check (Double-Signing)
        let equiv_check_query = format!(
            "SELECT COUNT(*) FROM (
                SELECT id FROM approved_karma WHERE node_id = {} AND lamport_clock = {} AND (lesson != {} OR weight != {})
                UNION ALL
                SELECT id FROM quarantined_karma WHERE node_id = {} AND lamport_clock = {} AND (lesson != {} OR weight != {})
             ) AS equiv LIMIT 1",
             state.pool.ph(0), state.pool.ph(1), state.pool.ph(2), state.pool.ph(3),
             state.pool.ph(4), state.pool.ph(5), state.pool.ph(6), state.pool.ph(7)
        );

        let equiv_exists = shared::sql_fetch_optional!(
            &state.pool,
            (i64,),
            &equiv_check_query,
            &k.node_id,
            &(k.lamport_clock as i64),
            &k.lesson,
            &(k.weight as i64),
            &k.node_id,
            &(k.lamport_clock as i64),
            &k.lesson,
            &(k.weight as i64)
        )
        .unwrap_or(Some((0,)))
        .unwrap_or((0,))
        .0 > 0;
        if equiv_exists {
            warn!(
                "🛡️ [BFT] EQUIVOCATION detected from node: {}. Slashing node.",
                k.node_id
            );
            let slash_query = format!("UPDATE node_reputation SET is_banned = 1, reputation_score = -1000 WHERE node_id = {}", state.pool.ph(0));
            let _ = shared::sql_exec!(&state.pool, &slash_query, &k.node_id);
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "Equivocation detected"})),
            )
                .into_response();
        }

        let quarantine_karma_query = format!(
            "INSERT INTO quarantined_karma (id, node_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, signature, received_at, clone_origin_id, generation, somatic_valence)
             VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})
             ON CONFLICT(id) DO NOTHING "
,
             state.pool.ph(0), state.pool.ph(1), state.pool.ph(2), state.pool.ph(3), state.pool.ph(4),
             state.pool.ph(5), state.pool.ph(6), state.pool.ph(7), state.pool.ph(8), state.pool.ph(9),
             state.pool.ph(10), state.pool.ph(11), state.pool.ph(12), state.pool.ph(13)
        );
        let res = match &mut tx {
            shared::db::DatabaseTransaction::Sqlite(ref mut t) => {
                sqlx::query::<sqlx::Sqlite>(&quarantine_karma_query)
                    .bind(&k.id)
                    .bind(&k.node_id)
                    .bind(&k.karma_type)
                    .bind(&k.related_skill)
                    .bind(&k.lesson)
                    .bind(k.weight as i64)
                    .bind(&k.soul_version_hash)
                    .bind(&k.created_at)
                    .bind(k.lamport_clock as i64)
                    .bind(&k.signature)
                    .bind(&received_at_dt)
                    .bind(&k.clone_origin_id)
                    .bind(k.generation.map(|v| v as i64))
                    .bind(k.somatic_valence)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
            shared::db::DatabaseTransaction::Postgres(ref mut t) => {
                sqlx::query::<sqlx::Postgres>(&quarantine_karma_query)
                    .bind(&k.id)
                    .bind(&k.node_id)
                    .bind(&k.karma_type)
                    .bind(&k.related_skill)
                    .bind(&k.lesson)
                    .bind(k.weight as i64)
                    .bind(&k.soul_version_hash)
                    .bind(&k.created_at)
                    .bind(k.lamport_clock as i64)
                    .bind(&k.signature)
                    .bind(&received_at_dt)
                    .bind(&k.clone_origin_id)
                    .bind(k.generation.map(|v| v as i64))
                    .bind(k.somatic_valence)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
        };
        if let Err(e) = res {
            warn!("🛡️ [Push] Failed to quarantine karma {}: {}", k.id, e);
        }
    }

    for r in &payload.rules {
        // BFT: Equivocation Check (Double-Signing) for Rules
        let equiv_check_rule_query = format!(
            "SELECT COUNT(*) FROM (
                SELECT id FROM approved_rules WHERE node_id = {} AND lamport_clock = {} AND (pattern != {} OR severity != {} OR action != {})
                UNION ALL
                SELECT id FROM quarantined_rules WHERE node_id = {} AND lamport_clock = {} AND (pattern != {} OR severity != {} OR action != {})
             ) AS equiv LIMIT 1",
             state.pool.ph(0), state.pool.ph(1), state.pool.ph(2), state.pool.ph(3), state.pool.ph(4),
             state.pool.ph(5), state.pool.ph(6), state.pool.ph(7), state.pool.ph(8), state.pool.ph(9)
        );
        let exists = shared::sql_fetch_optional!(
            &state.pool,
            (i64,),
            &equiv_check_rule_query,
            &r.node_id,
            &(r.lamport_clock as i64),
            &r.pattern,
            &(r.severity as i64),
            &r.action,
            &r.node_id,
            &(r.lamport_clock as i64),
            &r.pattern,
            &(r.severity as i64),
            &r.action
        )
        .unwrap_or(Some((0,)))
        .unwrap_or((0,))
        .0;
        if exists > 0 {
            warn!(
                "🛡️ [BFT] EQUIVOCATION detected in RULE from node: {}. Slashing node.",
                r.node_id
            );
            let ban_query = format!("UPDATE node_reputation SET is_banned = 1, reputation_score = -1000 WHERE node_id = {}", state.pool.ph(0));
            let _ = shared::sql_exec!(&state.pool, &ban_query, &r.node_id);
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "Equivocation detected"})),
            )
                .into_response();
        }

        let quarantine_rule_query = format!(
            "INSERT INTO quarantined_rules (id, node_id, pattern, severity, action, created_at, lamport_clock, signature, received_at)
             VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {})
             ON CONFLICT(id) DO NOTHING ",
             state.pool.ph(0), state.pool.ph(1), state.pool.ph(2), state.pool.ph(3), state.pool.ph(4),
             state.pool.ph(5), state.pool.ph(6), state.pool.ph(7), state.pool.ph(8)
        );
        let res = match &mut tx {
            shared::db::DatabaseTransaction::Sqlite(t) => {
                sqlx::query::<sqlx::Sqlite>(&quarantine_rule_query)
                    .bind(&r.id)
                    .bind(&r.node_id)
                    .bind(&r.pattern)
                    .bind(r.severity as i64)
                    .bind(&r.action)
                    .bind(&r.created_at)
                    .bind(r.lamport_clock as i64)
                    .bind(&r.signature)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
            shared::db::DatabaseTransaction::Postgres(t) => {
                sqlx::query::<sqlx::Postgres>(&quarantine_rule_query)
                    .bind(&r.id)
                    .bind(&r.node_id)
                    .bind(&r.pattern)
                    .bind(r.severity as i64)
                    .bind(&r.action)
                    .bind(&r.created_at)
                    .bind(r.lamport_clock as i64)
                    .bind(&r.signature)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
        };
        if let Err(e) = res {
            warn!("🛡️ [Push] Failed to quarantine rule {}: {}", r.id, e);
        }
    }

    for a in &payload.arena_matches {
        let quarantine_arena_query = format!(
            "INSERT INTO quarantined_arena_matches (id, skill_a, skill_b, topic, output_a, output_b, winner, reasoning, created_at, received_at)
             VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {})
             ON CONFLICT(id) DO NOTHING ",
             state.pool.ph(0), state.pool.ph(1), state.pool.ph(2), state.pool.ph(3), state.pool.ph(4),
             state.pool.ph(5), state.pool.ph(6), state.pool.ph(7), state.pool.ph(8), state.pool.ph(9)
        );
        let res = match &mut tx {
            shared::db::DatabaseTransaction::Sqlite(t) => {
                sqlx::query::<sqlx::Sqlite>(&quarantine_arena_query)
                    .bind(&a.id)
                    .bind(&a.skill_a)
                    .bind(&a.skill_b)
                    .bind(&a.topic)
                    .bind(&a.output_a)
                    .bind(&a.output_b)
                    .bind(&a.winner)
                    .bind(&a.reasoning)
                    .bind(&a.created_at)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
            shared::db::DatabaseTransaction::Postgres(t) => {
                sqlx::query::<sqlx::Postgres>(&quarantine_arena_query)
                    .bind(&a.id)
                    .bind(&a.skill_a)
                    .bind(&a.skill_b)
                    .bind(&a.topic)
                    .bind(&a.output_a)
                    .bind(&a.output_b)
                    .bind(&a.winner)
                    .bind(&a.reasoning)
                    .bind(&a.created_at)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
        };
        if let Err(e) = res {
            warn!("🛡️ [Push] Failed to quarantine arena match {}: {}", a.id, e);
        }
    }

    // Store Automerge Snapshot (Binary Timeline)
    if let Some(snapshot) = &payload.automerge_snapshot {
        let snapshot_query = format!(
            "INSERT INTO timeline_snapshots (node_id, snapshot_blob, received_at) VALUES ({}, {}, {})
             ON CONFLICT(node_id) DO UPDATE SET snapshot_blob = excluded.snapshot_blob, received_at = excluded.received_at",
            state.pool.ph(0), state.pool.ph(1), state.pool.ph(2)
        );
        let res = match &mut tx {
            shared::db::DatabaseTransaction::Sqlite(t) => {
                sqlx::query::<sqlx::Sqlite>(&snapshot_query)
                    .bind(&payload.node_id)
                    .bind(snapshot)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
            shared::db::DatabaseTransaction::Postgres(t) => {
                sqlx::query::<sqlx::Postgres>(&snapshot_query)
                    .bind(&payload.node_id)
                    .bind(snapshot)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
        };
        if let Err(e) = res {
            warn!(
                "🛡️ [Push] Failed to store timeline snapshot from {}: {}",
                payload.node_id, e
            );
        }
    }

    if let Some(metrics) = &payload.metrics {
        let metrics_json = serde_json::to_string(metrics).unwrap_or_default();
        let metrics_query = format!(
            "INSERT INTO federated_metrics (node_id, metrics_json, received_at) VALUES ({}, {}, {})",
            state.pool.ph(0), state.pool.ph(1), state.pool.ph(2)
        );
        let res = match &mut tx {
            shared::db::DatabaseTransaction::Sqlite(t) => {
                sqlx::query::<sqlx::Sqlite>(&metrics_query)
                    .bind(&payload.node_id)
                    .bind(&metrics_json)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
            shared::db::DatabaseTransaction::Postgres(t) => {
                sqlx::query::<sqlx::Postgres>(&metrics_query)
                    .bind(&payload.node_id)
                    .bind(&metrics_json)
                    .bind(&received_at_dt)
                    .execute(&mut **t)
                    .await
                    .map(|_| ())
            }
        };
        if let Err(e) = res {
            warn!(
                "🛡️ [Push] Failed to store federated metrics from {}: {}",
                payload.node_id, e
            );
        }
    }

    if let Err(e) = tx.commit().await {
        error!("❌ Push commit failed: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Internal error"})),
        )
            .into_response();
    }

    let arenas_count = payload.arena_matches.len();

    // BFT: Update reputation / last_seen
    let reputation_query = format!(
        "INSERT INTO node_reputation (node_id, last_seen_at) VALUES ({}, {})
         ON CONFLICT(node_id) DO UPDATE SET last_seen_at = excluded.last_seen_at, reputation_score = node_reputation.reputation_score + 1",
         state.pool.ph(0), state.pool.ph(1)
    );
    let res = shared::sql_exec!(
        &state.pool,
        &reputation_query,
        &payload.node_id,
        &received_at_dt
    );
    if let Err(e) = res {
        warn!(
            "🛡️ [Push] Failed to update node reputation for {}: {}",
            payload.node_id, e
        );
    }

    // 📣 Real-time Broadcast to all connected nodes (Relay Sync)
    for r in &payload.rules {
        let _ = state.tx.send(HubMessage::NewImmuneRule(r.clone()));
    }
    for k in &payload.karmas {
        let _ = state.tx.send(HubMessage::NewKarma(k.clone()));
    }

    (
        StatusCode::OK,
        Json(FederationPushResponse {
            accepted_count: karma_count + rule_count + arenas_count,
            message: "Data received and placed in quarantine for validation. ".to_string(),
        }),
    )
        .into_response()
}
