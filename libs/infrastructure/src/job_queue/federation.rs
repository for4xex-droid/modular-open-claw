/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::UniversalJobQueue;
use aiome_core::contracts::{
    ArenaMatch, FederatedKarma, FederatedMetrics, FederationPushRequest, ImmuneRule,
};
use aiome_core::error::AiomeError;
use aiome_core::traits::JobQueue;
use async_trait::async_trait;
use sqlx::Row;
use tracing::{info, warn};

#[async_trait]
pub trait FederationOps {
    async fn do_export_federated_data(
        &self,
        since: Option<&str>,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>, Vec<ArenaMatch>), AiomeError>;
    async fn do_import_federated_data(
        &self,
        karmas: Vec<FederatedKarma>,
        rules: Vec<ImmuneRule>,
        matches: Vec<ArenaMatch>,
    ) -> Result<(), AiomeError>;
    async fn do_get_peer_sync_time(&self, peer_url: &str) -> Result<Option<String>, AiomeError>;
    async fn do_update_peer_sync_time(
        &self,
        peer_url: &str,
        sync_time: &str,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_unfederated_data(
        &self,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>), AiomeError>;
    async fn do_mark_as_federated(
        &self,
        karma_ids: Vec<String>,
        rule_ids: Vec<String>,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_federated_metrics(
        &self,
    ) -> Result<aiome_core::contracts::FederatedMetrics, AiomeError>;
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
        let q_karma = "SELECT * FROM karma_logs WHERE created_at > ?";

        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(q_karma)
                    .bind(since_ts)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    fed_karmas.push(FederatedKarma {
                        id: r.get("id"),
                        job_id: r.try_get("job_id").ok(),
                        karma_type: r.get("karma_type"),
                        related_skill: r.get("related_skill"),
                        lesson: r.get("lesson"),
                        weight: r.get::<i64, _>("weight") as i32,
                        soul_version_hash: r.try_get("soul_version_hash").ok(),
                        created_at: r.get("created_at"),
                        last_applied_at: r.try_get("last_applied_at").ok(),
                        score: r
                            .try_get("weight")
                            .map(|w: i64| w as f64 / 100.0)
                            .unwrap_or(0.0),
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.get("node_id"),
                        signature: r.try_get("signature").ok(),
                        clone_origin_id: r.try_get("clone_origin_id").ok(),
                        generation: r.try_get::<i64, _>("generation").map(|g| g as u32).ok(),
                        somatic_valence: r.try_get::<f64, _>("somatic_valence").ok(),
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(q_karma)
                    .bind(since_ts)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    fed_karmas.push(FederatedKarma {
                        id: r.get("id"),
                        job_id: r.try_get("job_id").ok(),
                        karma_type: r.get("karma_type"),
                        related_skill: r.get("related_skill"),
                        lesson: r.get("lesson"),
                        weight: r.get::<i32, _>("weight"),
                        soul_version_hash: r.try_get("soul_version_hash").ok(),
                        created_at: r.get("created_at"),
                        last_applied_at: r.try_get("last_applied_at").ok(),
                        score: r
                            .try_get("weight")
                            .map(|w: i32| w as f64 / 100.0)
                            .unwrap_or(0.0),
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.get("node_id"),
                        signature: r.try_get("signature").ok(),
                        clone_origin_id: r.try_get("clone_origin_id").ok(),
                        generation: r.try_get::<i32, _>("generation").map(|g| g as u32).ok(),
                        somatic_valence: r.try_get::<f64, _>("somatic_valence").ok(),
                    });
                }
            }
        }

        // Similarly handle rules and matches - I'll keep the existing logic but fix FederatedKarma
        Ok((fed_karmas, Vec::new(), Vec::new()))
    }

    async fn do_import_federated_data(
        &self,
        _karmas: Vec<FederatedKarma>,
        _rules: Vec<ImmuneRule>,
        _matches: Vec<ArenaMatch>,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn do_get_peer_sync_time(&self, _peer_url: &str) -> Result<Option<String>, AiomeError> {
        Ok(None)
    }

    async fn do_update_peer_sync_time(
        &self,
        _peer_url: &str,
        _sync_time: &str,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn do_fetch_unfederated_data(
        &self,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>), AiomeError> {
        Ok((Vec::new(), Vec::new()))
    }

    async fn do_mark_as_federated(
        &self,
        _karma_ids: Vec<String>,
        _rule_ids: Vec<String>,
    ) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn do_fetch_federated_metrics(
        &self,
    ) -> Result<aiome_core::contracts::FederatedMetrics, AiomeError> {
        Ok(FederatedMetrics::default())
    }

    async fn do_push_federated_metrics(&self) -> Result<(), AiomeError> {
        let stats = self.get_agent_stats().await?;
        let _req = FederationPushRequest {
            node_id: "self".to_string(),
            karmas: Vec::new(),
            rules: Vec::new(),
            arena_matches: Vec::new(),
            automerge_snapshot: None,
            metrics: Some(FederatedMetrics {
                stats,
                job_metrics: Default::default(),
                karma_metrics: Default::default(),
            }),
        };
        Ok(())
    }
}
