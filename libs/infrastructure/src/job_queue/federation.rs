/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::try_get_opt;
use super::UniversalJobQueue;
use aiome_core::contracts::{ArenaMatch, FederatedKarma, FederatedMetrics, ImmuneRule};
use aiome_core::error::AiomeError;
use aiome_core::traits::JobQueue;
use async_trait::async_trait;
use sqlx::Row;
use tracing::{info, warn};

#[async_trait]
/// フェデレーション（他のノードや Samsara Hub とのデータ同期）を行うためのオペレーションを定義するトレイト。
pub trait FederationOps {
    /// 指定された日時以降に更新されたカルマ、免疫ルール、アリーナの対戦履歴をエクスポートする。
    async fn do_export_federated_data(
        &self,
        since: Option<&str>,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>, Vec<ArenaMatch>), AiomeError>;

    /// 外部から受信したカルマ、免疫ルール、アリーナの対戦履歴をローカルデータベースにインポートする。
    /// 署名の検証と Lamport Clock による競合解決を行う。
    async fn do_import_federated_data(
        &self,
        karmas: Vec<FederatedKarma>,
        rules: Vec<ImmuneRule>,
        matches: Vec<ArenaMatch>,
    ) -> Result<(), AiomeError>;

    /// 指定されたピア（他のノード）との最終同期日時を取得する。
    async fn do_get_peer_sync_time(&self, peer_url: &str) -> Result<Option<String>, AiomeError>;

    /// 指定されたピアとの最終同期日時を更新または新規登録する。
    async fn do_update_peer_sync_time(
        &self,
        peer_url: &str,
        sync_time: &str,
    ) -> Result<(), AiomeError>;

    /// まだフェデレーション（外部への送信）が行われていないローカルのカルマとルールを取得する。
    async fn do_fetch_unfederated_data(
        &self,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>), AiomeError>;

    /// 指定されたカルマ味方ルールを「フェデレーション済み」としてマークする。
    async fn do_mark_as_federated(
        &self,
        karma_ids: Vec<String>,
        rule_ids: Vec<String>,
    ) -> Result<(), AiomeError>;

    /// ノードの統計情報（カルマ、ジョブ、エージェント自身の状態）を含むメトリクスを取得する。
    async fn do_fetch_federated_metrics(
        &self,
    ) -> Result<aiome_core::contracts::FederatedMetrics, AiomeError>;

    /// ノードの最新メトリクスを Samsara Hub にプッシュ送信する。
    async fn do_push_federated_metrics(&self) -> Result<(), AiomeError>;
}

#[async_trait]
impl FederationOps for UniversalJobQueue {
    async fn do_export_federated_data(
        &self,
        since: Option<&str>,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>, Vec<ArenaMatch>), AiomeError> {
        let since_ts = since.unwrap_or("1970-01-01T00:00:00");

        let mut fed_karmas = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query("SELECT id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, node_id, signature, clone_origin_id FROM karma_logs WHERE created_at > ?")
                    .bind(since_ts).fetch_all(p).await.map_err(|e| AiomeError::Infrastructure { reason: format!("Export Karma failed: {}", e) })?;
                for r in rows {
                    fed_karmas.push(FederatedKarma {
                        id: r.get("id"),
                        job_id: try_get_opt(&r, "job_id"),
                        karma_type: r.get("karma_type"),
                        related_skill: r.get("related_skill"),
                        lesson: r.get("lesson"),
                        weight: r.get::<i64, _>("weight") as i32,
                        soul_version_hash: try_get_opt(&r, "soul_version_hash"),
                        created_at: r.get("created_at"),
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.get("node_id"),
                        signature: try_get_opt(&r, "signature"),
                        clone_origin_id: try_get_opt(&r, "clone_origin_id"),
                        generation: r.try_get::<i64, _>("generation").map(|g| g as u32).ok(),
                        somatic_valence: r.try_get::<f64, _>("somatic_valence").ok(),
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query("SELECT id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, node_id, signature, clone_origin_id FROM karma_logs WHERE created_at > $1")
                    .bind(since_ts).fetch_all(p).await.map_err(|e| AiomeError::Infrastructure { reason: format!("Export Karma failed: {}", e) })?;
                for r in rows {
                    fed_karmas.push(FederatedKarma {
                        id: r.get("id"),
                        job_id: try_get_opt(&r, "job_id"),
                        karma_type: r.get("karma_type"),
                        related_skill: r.get("related_skill"),
                        lesson: r.get("lesson"),
                        weight: r.get::<i32, _>("weight"),
                        soul_version_hash: try_get_opt(&r, "soul_version_hash"),
                        created_at: r.get("created_at"),
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.get("node_id"),
                        signature: try_get_opt(&r, "signature"),
                        clone_origin_id: try_get_opt(&r, "clone_origin_id"),
                        generation: r.try_get::<i32, _>("generation").map(|g| g as u32).ok(),
                        somatic_valence: r.try_get::<f64, _>("somatic_valence").ok(),
                    });
                }
            }
        };

        let mut fed_rules = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query("SELECT id, pattern, severity, action, status, created_at, lamport_clock, node_id, signature FROM immune_rules WHERE created_at > ?")
                    .bind(since_ts).fetch_all(p).await.map_err(|e| AiomeError::Infrastructure { reason: format!("Export Rules failed: {}", e) })?;
                for r in rows {
                    fed_rules.push(ImmuneRule {
                        id: r.get("id"),
                        pattern: r.get("pattern"),
                        severity: r.get::<i64, _>("severity") as u8,
                        action: r.get("action"),
                        created_at: r.get("created_at"),
                        approval_status: match r.get::<String, _>("status").as_str() {
                            "Approved" | "Active" => aiome_core::contracts::ApprovalState::Approved,
                            "Rejected" | "Quarantined" => {
                                aiome_core::contracts::ApprovalState::Rejected
                            }
                            _ => aiome_core::contracts::ApprovalState::Pending,
                        },
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.get("node_id"),
                        signature: try_get_opt(&r, "signature"),
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query("SELECT id, pattern, severity, action, status, created_at, lamport_clock, node_id, signature FROM immune_rules WHERE created_at > $1")
                    .bind(since_ts).fetch_all(p).await.map_err(|e| AiomeError::Infrastructure { reason: format!("Export Rules failed: {}", e) })?;
                for r in rows {
                    fed_rules.push(ImmuneRule {
                        id: r.get("id"),
                        pattern: r.get("pattern"),
                        severity: r.get::<i32, _>("severity") as u8,
                        action: r.get("action"),
                        created_at: r.get("created_at"),
                        approval_status: match r.get::<String, _>("status").as_str() {
                            "Approved" | "Active" => aiome_core::contracts::ApprovalState::Approved,
                            "Rejected" | "Quarantined" => {
                                aiome_core::contracts::ApprovalState::Rejected
                            }
                            _ => aiome_core::contracts::ApprovalState::Pending,
                        },
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.get("node_id"),
                        signature: try_get_opt(&r, "signature"),
                    });
                }
            }
        };

        let mut fed_matches = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query("SELECT id, skill_a, skill_b, topic, winner, reasoning, created_at FROM arena_history WHERE created_at > ?")
                    .bind(since_ts).fetch_all(p).await.map_err(|e| AiomeError::Infrastructure { reason: format!("Export Matches failed: {}", e) })?;
                for r in rows {
                    fed_matches.push(ArenaMatch {
                        id: r.get("id"),
                        skill_a: r.get("skill_a"),
                        skill_b: r.get("skill_b"),
                        topic: r.get("topic"),
                        winner: try_get_opt(&r, "winner"),
                        reasoning: r.get("reasoning"),
                        created_at: r.get("created_at"),
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query("SELECT id, skill_a, skill_b, topic, winner, reasoning, created_at FROM arena_history WHERE created_at > $1")
                    .bind(since_ts).fetch_all(p).await.map_err(|e| AiomeError::Infrastructure { reason: format!("Export Matches failed: {}", e) })?;
                for r in rows {
                    fed_matches.push(ArenaMatch {
                        id: r.get("id"),
                        skill_a: r.get("skill_a"),
                        skill_b: r.get("skill_b"),
                        topic: r.get("topic"),
                        winner: try_get_opt(&r, "winner"),
                        reasoning: r.get("reasoning"),
                        created_at: r.get("created_at"),
                    });
                }
            }
        };

        Ok((fed_karmas, fed_rules, fed_matches))
    }

    async fn do_import_federated_data(
        &self,
        karmas: Vec<FederatedKarma>,
        rules: Vec<ImmuneRule>,
        matches: Vec<ArenaMatch>,
    ) -> Result<(), AiomeError> {
        if !karmas.is_empty() || !rules.is_empty() || !matches.is_empty() {
            info!(
                "📥 [Federation] Importing {} karmas, {} rules, {} matches.",
                karmas.len(),
                rules.len(),
                matches.len()
            );
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Import Tx start failed: {}", e),
            })?;

        use base64::{prelude::BASE64_STANDARD, Engine};
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        for k in karmas {
            // Verify Ed25519 Signature
            let mut valid = false;
            if let Some(ref sig_b64) = k.signature {
                let payload = format!("{}:{}:{}", k.id, k.lesson, k.lamport_clock);
                if let (Ok(pubkey_bytes), Ok(sig_bytes)) = (
                    BASE64_STANDARD.decode(&k.node_id),
                    BASE64_STANDARD.decode(sig_b64),
                ) {
                    if let (Ok(pubkey_arr), Ok(sig)) = (
                        pubkey_bytes.try_into() as Result<[u8; 32], _>,
                        Signature::from_slice(&sig_bytes),
                    ) {
                        if let Ok(pubkey) = VerifyingKey::from_bytes(&pubkey_arr) {
                            if pubkey.verify(payload.as_bytes(), &sig).is_ok() {
                                valid = true;
                            } else {
                                warn!(
                                    "🛡️ [Federation] Signature verification failed for Karma {}.",
                                    k.id
                                );
                            }
                        }
                    }
                }
            }

            if !valid {
                warn!(
                    "🛡️ [Federation] Skipping Karma {} due to invalid/missing signature.",
                    k.id
                );
                continue;
            }

            let clean_lesson = if k.lesson.len() > 2000 {
                format!(
                    "{}... [Truncated for Swarm Safety]",
                    k.lesson.chars().take(2000).collect::<String>()
                )
            } else {
                k.lesson.clone()
            };

            let _ = self.sync_local_clock(k.lamport_clock).await;

            let q = format!("INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, is_federated, lamport_clock, node_id, signature, clone_origin_id) VALUES ({0}, NULL, {1}, {2}, {3}, {4}, {5}, {6}, 1, {7}, {8}, {9}, {10}) ON CONFLICT(id) DO UPDATE SET lesson = excluded.lesson, weight = excluded.weight, lamport_clock = excluded.lamport_clock, node_id = excluded.node_id, signature = excluded.signature, is_federated = 1 WHERE excluded.lamport_clock > karma_logs.lamport_clock OR (excluded.lamport_clock = karma_logs.lamport_clock AND excluded.node_id > karma_logs.node_id)",
                self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6), self.pool.ph(7), self.pool.ph(8), self.pool.ph(9), self.pool.ph(10));
            match &mut tx {
                crate::db::DatabaseTransaction::Sqlite(t) => {
                    let _ = sqlx::query(&q)
                        .bind(&k.id)
                        .bind(&k.karma_type)
                        .bind(&k.related_skill)
                        .bind(&clean_lesson)
                        .bind(k.weight as i64)
                        .bind(&k.soul_version_hash)
                        .bind(&k.created_at)
                        .bind(k.lamport_clock as i64)
                        .bind(&k.node_id)
                        .bind(&k.signature)
                        .bind(&k.clone_origin_id)
                        .execute(&mut **t)
                        .await;
                }
                crate::db::DatabaseTransaction::Postgres(t) => {
                    let _ = sqlx::query(&q)
                        .bind(&k.id)
                        .bind(&k.karma_type)
                        .bind(&k.related_skill)
                        .bind(&clean_lesson)
                        .bind(k.weight as i64)
                        .bind(&k.soul_version_hash)
                        .bind(&k.created_at)
                        .bind(k.lamport_clock as i64)
                        .bind(&k.node_id)
                        .bind(&k.signature)
                        .bind(&k.clone_origin_id)
                        .execute(&mut **t)
                        .await;
                }
            }
        }

        for r in rules {
            // Verify Ed25519 Signature
            let mut valid = false;
            if let Some(ref sig_b64) = r.signature {
                let payload = format!("{}:{}:{}", r.id, r.pattern, r.lamport_clock);
                if let (Ok(pubkey_bytes), Ok(sig_bytes)) = (
                    BASE64_STANDARD.decode(&r.node_id),
                    BASE64_STANDARD.decode(sig_b64),
                ) {
                    if let (Ok(pubkey_arr), Ok(sig)) = (
                        pubkey_bytes.try_into() as Result<[u8; 32], _>,
                        Signature::from_slice(&sig_bytes),
                    ) {
                        if let Ok(pubkey) = VerifyingKey::from_bytes(&pubkey_arr) {
                            if pubkey.verify(payload.as_bytes(), &sig).is_ok() {
                                valid = true;
                            }
                        }
                    }
                }
            }

            if !valid {
                warn!(
                    "🛡️ [Federation] Skipping Rule {} due to invalid signature.",
                    r.id
                );
                continue;
            }

            let _ = self.sync_local_clock(r.lamport_clock).await;

            let q = format!("INSERT INTO immune_rules (id, pattern, severity, action, created_at, is_federated, lamport_clock, node_id, signature, status) VALUES ({0}, {1}, {2}, {3}, {4}, 1, {5}, {6}, {7}, 'Quarantined') ON CONFLICT(id) DO UPDATE SET pattern = excluded.pattern, severity = excluded.severity, action = excluded.action, lamport_clock = excluded.lamport_clock, node_id = excluded.node_id, signature = excluded.signature WHERE excluded.lamport_clock > immune_rules.lamport_clock OR (excluded.lamport_clock = immune_rules.lamport_clock AND excluded.node_id > immune_rules.node_id)",
                self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6), self.pool.ph(7));
            match &mut tx {
                crate::db::DatabaseTransaction::Sqlite(t) => {
                    let _ = sqlx::query(&q)
                        .bind(&r.id)
                        .bind(&r.pattern)
                        .bind(r.severity as i64)
                        .bind(&r.action)
                        .bind(&r.created_at)
                        .bind(r.lamport_clock as i64)
                        .bind(&r.node_id)
                        .bind(&r.signature)
                        .execute(&mut **t)
                        .await;
                }
                crate::db::DatabaseTransaction::Postgres(t) => {
                    let _ = sqlx::query(&q)
                        .bind(&r.id)
                        .bind(&r.pattern)
                        .bind(r.severity as i64)
                        .bind(&r.action)
                        .bind(&r.created_at)
                        .bind(r.lamport_clock as i64)
                        .bind(&r.node_id)
                        .bind(&r.signature)
                        .execute(&mut **t)
                        .await;
                }
            }
        }

        for m in matches {
            let q = format!("INSERT INTO arena_history (id, skill_a, skill_b, topic, winner, reasoning, created_at) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}) ON CONFLICT(id) DO NOTHING",
                self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6));
            match &mut tx {
                crate::db::DatabaseTransaction::Sqlite(t) => {
                    let _ = sqlx::query(&q)
                        .bind(&m.id)
                        .bind(&m.skill_a)
                        .bind(&m.skill_b)
                        .bind(&m.topic)
                        .bind(&m.winner)
                        .bind(&m.reasoning)
                        .bind(&m.created_at)
                        .execute(&mut **t)
                        .await;
                }
                crate::db::DatabaseTransaction::Postgres(t) => {
                    let _ = sqlx::query(&q)
                        .bind(&m.id)
                        .bind(&m.skill_a)
                        .bind(&m.skill_b)
                        .bind(&m.topic)
                        .bind(&m.winner)
                        .bind(&m.reasoning)
                        .bind(&m.created_at)
                        .execute(&mut **t)
                        .await;
                }
            }
        }

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Import Tx commit failed: {}", e),
        })?;
        Ok(())
    }

    async fn do_get_peer_sync_time(&self, peer_url: &str) -> Result<Option<String>, AiomeError> {
        let q = format!(
            "SELECT last_sync_at FROM federation_peers WHERE peer_url = {}",
            self.pool.ph(0)
        );
        let opt: Option<String> = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar(&q)
                .bind(peer_url)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar(&q)
                .bind(peer_url)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
        };
        Ok(opt)
    }

    async fn do_update_peer_sync_time(
        &self,
        peer_url: &str,
        sync_time: &str,
    ) -> Result<(), AiomeError> {
        let cols = ["peer_url", "last_sync_at"];
        let q = self
            .pool
            .upsert_query("federation_peers", "peer_url", &cols, 0);
        sql_exec!(
            &self.pool,
            "INSERT OR REPLACE INTO federation_peers (peer_url, last_sync_at) VALUES (?, ?)",
            peer_url,
            sync_time
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;
        Ok(())
    }

    async fn do_fetch_unfederated_data(
        &self,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>), AiomeError> {
        let q_k = "SELECT id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, node_id, signature, clone_origin_id FROM karma_logs WHERE is_federated = 0";
        let q_r = "SELECT id, pattern, severity, action, created_at, lamport_clock, node_id, signature, status FROM immune_rules WHERE is_federated = 0";

        let mut fed_karmas = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(q_k).fetch_all(p).await.map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
                for r in rows {
                    fed_karmas.push(FederatedKarma {
                        id: r.get("id"),
                        job_id: try_get_opt(&r, "job_id"),
                        karma_type: r.get("karma_type"),
                        related_skill: r.get("related_skill"),
                        lesson: r.get("lesson"),
                        weight: r.get::<i64, _>("weight") as i32,
                        soul_version_hash: try_get_opt(&r, "soul_version_hash"),
                        created_at: r.get("created_at"),
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.get("node_id"),
                        signature: try_get_opt(&r, "signature"),
                        clone_origin_id: try_get_opt(&r, "clone_origin_id"),
                        generation: r.try_get::<i64, _>("generation").map(|g| g as u32).ok(),
                        somatic_valence: r.try_get::<f64, _>("somatic_valence").ok(),
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(q_k).fetch_all(p).await.map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
                for r in rows {
                    fed_karmas.push(FederatedKarma {
                        id: r.get("id"),
                        job_id: try_get_opt(&r, "job_id"),
                        karma_type: r.get("karma_type"),
                        related_skill: r.get("related_skill"),
                        lesson: r.get("lesson"),
                        weight: r.get::<i32, _>("weight"),
                        soul_version_hash: try_get_opt(&r, "soul_version_hash"),
                        created_at: r.get("created_at"),
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.get("node_id"),
                        signature: try_get_opt(&r, "signature"),
                        clone_origin_id: try_get_opt(&r, "clone_origin_id"),
                        generation: r.try_get::<i32, _>("generation").map(|g| g as u32).ok(),
                        somatic_valence: r.try_get::<f64, _>("somatic_valence").ok(),
                    });
                }
            }
        }

        let mut fed_rules = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(q_r).fetch_all(p).await.map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
                for r in rows {
                    fed_rules.push(ImmuneRule {
                        id: r.get("id"),
                        pattern: r.get("pattern"),
                        severity: r.get::<i64, _>("severity") as u8,
                        action: r.get("action"),
                        created_at: r.get("created_at"),
                        approval_status: match r.get::<String, _>("status").as_str() {
                            "Approved" | "Active" => aiome_core::contracts::ApprovalState::Approved,
                            "Rejected" | "Quarantined" => {
                                aiome_core::contracts::ApprovalState::Rejected
                            }
                            _ => aiome_core::contracts::ApprovalState::Pending,
                        },
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.get("node_id"),
                        signature: try_get_opt(&r, "signature"),
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(q_r).fetch_all(p).await.map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
                for r in rows {
                    fed_rules.push(ImmuneRule {
                        id: r.get("id"),
                        pattern: r.get("pattern"),
                        severity: r.get::<i32, _>("severity") as u8,
                        action: r.get("action"),
                        created_at: r.get("created_at"),
                        approval_status: match r.get::<String, _>("status").as_str() {
                            "Approved" | "Active" => aiome_core::contracts::ApprovalState::Approved,
                            "Rejected" | "Quarantined" => {
                                aiome_core::contracts::ApprovalState::Rejected
                            }
                            _ => aiome_core::contracts::ApprovalState::Pending,
                        },
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.get("node_id"),
                        signature: try_get_opt(&r, "signature"),
                    });
                }
            }
        }

        Ok((fed_karmas, fed_rules))
    }

    async fn do_mark_as_federated(
        &self,
        karma_ids: Vec<String>,
        rule_ids: Vec<String>,
    ) -> Result<(), AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Mark federated Tx failed: {}", e),
            })?;

        let q_k = format!(
            "UPDATE karma_logs SET is_federated = 1 WHERE id = {}",
            self.pool.ph(0)
        );
        let q_r = format!(
            "UPDATE immune_rules SET is_federated = 1 WHERE id = {}",
            self.pool.ph(0)
        );
        for id in karma_ids {
            match &mut tx {
                crate::db::DatabaseTransaction::Sqlite(t) => {
                    let _ = sqlx::query(&q_k).bind(id).execute(&mut **t).await;
                }
                crate::db::DatabaseTransaction::Postgres(t) => {
                    let _ = sqlx::query(&q_k).bind(id).execute(&mut **t).await;
                }
            }
        }
        for id in rule_ids {
            match &mut tx {
                crate::db::DatabaseTransaction::Sqlite(t) => {
                    let _ = sqlx::query(&q_r).bind(id).execute(&mut **t).await;
                }
                crate::db::DatabaseTransaction::Postgres(t) => {
                    let _ = sqlx::query(&q_r).bind(id).execute(&mut **t).await;
                }
            }
        }

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Mark federated commit failed: {}", e),
        })?;
        Ok(())
    }

    async fn do_fetch_federated_metrics(
        &self,
    ) -> Result<aiome_core::contracts::FederatedMetrics, AiomeError> {
        let stats = <Self as super::evolution::EvolutionOps>::do_get_agent_stats(self).await?;

        // 1. Job Metrics
        let q_jc = "SELECT COUNT(*) FROM jobs WHERE status = 'Completed'";
        let q_jf = "SELECT COUNT(*) FROM jobs WHERE status = 'Failed'";
        let q_jp = "SELECT COUNT(*) FROM jobs WHERE status = 'Pending'";
        let q_kc = "SELECT COUNT(*) FROM karma_logs WHERE is_archived = 0";
        let q_kw = "SELECT COALESCE(SUM(weight), 0) FROM karma_logs WHERE karma_type = 'Technical' AND is_archived = 0";
        let q_kcc =
            "SELECT COUNT(*) FROM karma_logs WHERE karma_type = 'Creative' AND is_archived = 0";

        let total_completed = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar::<_, i64>(q_jc)
                .fetch_one(p)
                .await
                .unwrap_or(0),
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar::<_, i64>(q_jc)
                .fetch_one(p)
                .await
                .unwrap_or(0),
        };
        let total_failed = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar::<_, i64>(q_jf)
                .fetch_one(p)
                .await
                .unwrap_or(0),
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar::<_, i64>(q_jf)
                .fetch_one(p)
                .await
                .unwrap_or(0),
        };
        let pending_count = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar::<_, i64>(q_jp)
                .fetch_one(p)
                .await
                .unwrap_or(0),
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar::<_, i64>(q_jp)
                .fetch_one(p)
                .await
                .unwrap_or(0),
        };
        let total_karma = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar::<_, i64>(q_kc)
                .fetch_one(p)
                .await
                .unwrap_or(0),
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar::<_, i64>(q_kc)
                .fetch_one(p)
                .await
                .unwrap_or(0),
        };
        let technical_weight = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar::<_, i64>(q_kw)
                .fetch_one(p)
                .await
                .unwrap_or(0),
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar::<_, i64>(q_kw)
                .fetch_one(p)
                .await
                .unwrap_or(0),
        };
        let creative_count = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar::<_, i64>(q_kcc)
                .fetch_one(p)
                .await
                .unwrap_or(0),
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar::<_, i64>(q_kcc)
                .fetch_one(p)
                .await
                .unwrap_or(0),
        };

        // Map shared::watchtower::AgentStats to aiome_contracts::AgentStats
        let contract_stats = aiome_contracts::AgentStats {
            level: stats.level,
            exp: stats.exp,
            resonance: stats.resonance,
            creativity: stats.creativity,
            fatigue: stats.fatigue,
        };

        Ok(aiome_core::contracts::FederatedMetrics {
            stats: contract_stats,
            job_metrics: aiome_core::contracts::JobMetrics {
                total_completed,
                total_failed,
                pending_count,
            },
            karma_metrics: aiome_core::contracts::KarmaMetrics {
                total_count: total_karma,
                technical_weight,
                creative_count,
            },
        })
    }

    async fn do_push_federated_metrics(&self) -> Result<(), AiomeError> {
        let metrics = self.do_fetch_federated_metrics().await?;
        let node_id = self.get_node_id().await?;

        let hub_url = std::env::var("SAMSARA_HUB_URL")
            .unwrap_or_else(|_| shared::config::DEFAULT_SAMSARA_HUB_URL.to_string());
        let hub_secret = std::env::var("FEDERATION_SECRET").ok();

        if let Some(secret) = hub_secret {
            let client = aiome_core::http::get_http_client();
            let push_req = aiome_core::contracts::FederationPushRequest {
                node_id: node_id.clone(),
                karmas: vec![],
                rules: vec![],
                arena_matches: vec![],
                automerge_snapshot: None,
                metrics: Some(metrics),
            };

            let res = client
                .post(format!("{}/api/v1/federation/push", hub_url))
                .header("Authorization", format!("Bearer {}", secret))
                .json(&push_req)
                .send()
                .await;

            match res {
                Ok(r) if r.status().is_success() => {
                    info!(
                        "🚀 [Federation] Periodic metrics pushed to Samsara Hub for Node: {}",
                        node_id
                    );
                }
                Ok(r) => {
                    let status = r.status();
                    let text = r.text().await.unwrap_or_default();
                    warn!(
                        "⚠️ [Federation] Hub metrics push failed [{}]: {}",
                        status, text
                    );
                }
                Err(e) => {
                    warn!("⚠️ [Federation] Hub metrics push connection failed: {}", e);
                }
            }
        }
        Ok(())
    }
}
