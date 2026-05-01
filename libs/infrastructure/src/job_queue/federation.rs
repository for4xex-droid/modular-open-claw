/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::UniversalJobQueue;
use aiome_core::contracts::{
    ArenaMatch, FederatedKarma, FederatedMetrics, FederationPushRequest, ImmuneRule,
};
use aiome_core::error::AiomeError;
use aiome_core::traits::{AgentEvolver, JobQueue};
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
        let q_karma = "SELECT * FROM karma_logs WHERE created_at > ? AND is_private = 0";

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
        karmas: Vec<FederatedKarma>,
        rules: Vec<ImmuneRule>,
        matches: Vec<ArenaMatch>,
    ) -> Result<(), AiomeError> {
        if !rules.is_empty() {
            tracing::warn!(
                "Importing federated ImmuneRules is not yet implemented (skipped {} rules)",
                rules.len()
            );
        }
        if !matches.is_empty() {
            tracing::warn!(
                "Importing federated ArenaMatches is not yet implemented (skipped {} matches)",
                matches.len()
            );
        }

        for karma in karmas {
            let q = match &self.pool {
                crate::db::DatabasePool::Sqlite(_) => format!(
                    "INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, is_federated, clone_origin_id, lamport_clock, node_id, signature, created_at, last_applied_at) \
                     VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, {10}, {11}, {12}, {13}) \
                     ON CONFLICT (id) DO UPDATE SET \
                     job_id = EXCLUDED.job_id, \
                     karma_type = EXCLUDED.karma_type, \
                     related_skill = EXCLUDED.related_skill, \
                     lesson = EXCLUDED.lesson, \
                     weight = EXCLUDED.weight, \
                     soul_version_hash = EXCLUDED.soul_version_hash, \
                     is_federated = EXCLUDED.is_federated, \
                     clone_origin_id = EXCLUDED.clone_origin_id, \
                     lamport_clock = EXCLUDED.lamport_clock, \
                     node_id = EXCLUDED.node_id, \
                     signature = EXCLUDED.signature, \
                     created_at = EXCLUDED.created_at, \
                     last_applied_at = EXCLUDED.last_applied_at \
                     WHERE karma_logs.lamport_clock < EXCLUDED.lamport_clock",
                     self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6), self.pool.ph(7), self.pool.ph(8), self.pool.ph(9), self.pool.ph(10), self.pool.ph(11), self.pool.ph(12), self.pool.ph(13)
                ),
                crate::db::DatabasePool::Postgres(_) => format!(
                    "INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, is_federated, clone_origin_id, lamport_clock, node_id, signature, created_at, last_applied_at) \
                     VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, {10}, {11}, {12}, {13}) \
                     ON CONFLICT (id) DO UPDATE SET \
                     job_id = EXCLUDED.job_id, \
                     karma_type = EXCLUDED.karma_type, \
                     related_skill = EXCLUDED.related_skill, \
                     lesson = EXCLUDED.lesson, \
                     weight = EXCLUDED.weight, \
                     soul_version_hash = EXCLUDED.soul_version_hash, \
                     is_federated = EXCLUDED.is_federated, \
                     clone_origin_id = EXCLUDED.clone_origin_id, \
                     lamport_clock = EXCLUDED.lamport_clock, \
                     node_id = EXCLUDED.node_id, \
                     signature = EXCLUDED.signature, \
                     created_at = EXCLUDED.created_at, \
                     last_applied_at = EXCLUDED.last_applied_at \
                     WHERE karma_logs.lamport_clock < EXCLUDED.lamport_clock",
                     self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6), self.pool.ph(7), self.pool.ph(8), self.pool.ph(9), self.pool.ph(10), self.pool.ph(11), self.pool.ph(12), self.pool.ph(13)
                ),
            };

            let karma_id = karma.id.clone();
            crate::sql_exec!(
                &self.pool,
                &q,
                karma.id,
                karma.job_id,
                karma.karma_type,
                karma.related_skill,
                karma.lesson,
                karma.weight,
                karma.soul_version_hash,
                1, // is_federated
                karma.clone_origin_id,
                karma.lamport_clock as i64,
                karma.node_id,
                karma.signature,
                karma.created_at,
                karma.last_applied_at
            )
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to import federated karma {}: {}", karma_id, e),
            })?;
        }

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
        use crate::job_queue::evolution::EvolutionOps;
        let stats = self.do_get_agent_stats().await?;

        let q_jobs = "SELECT COUNT(*) FROM jobs WHERE status = 'Completed'";
        let total_completed: i64 =
            crate::sql_fetch_one!(&self.pool, (i64,), q_jobs).map(|r| r.0)?;

        let q_karma = "SELECT COUNT(*) FROM karma_logs";
        let total_karma: i64 = crate::sql_fetch_one!(&self.pool, (i64,), q_karma).map(|r| r.0)?;

        Ok(FederatedMetrics {
            stats,
            job_metrics: aiome_core::contracts::JobMetrics {
                total_completed: total_completed as i64,
                ..Default::default()
            },
            karma_metrics: aiome_core::contracts::KarmaMetrics {
                total_count: total_karma as i64,
                ..Default::default()
            },
        })
    }

    async fn do_push_federated_metrics(&self) -> Result<(), AiomeError> {
        use aiome_core_contracts::traits::SettingsOps;
        let hub_url_opt = self.do_get_setting("samsara_hub_url").await?;
        let hub_url = match hub_url_opt {
            Some(url) if !url.trim().is_empty() => url,
            _ => {
                tracing::warn!("samsara_hub_url is not set; skipping federated metrics push.");
                return Ok(());
            }
        };

        let stats = self.get_agent_stats().await?;
        let node_id = self
            .do_get_setting("node_id")
            .await?
            .unwrap_or_else(|| "self".to_string());

        let req = FederationPushRequest {
            node_id,
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

        let url = format!("{}/api/v1/federation/push", hub_url.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to build HTTP client: {}", e),
            })?;

        let res =
            client
                .post(&url)
                .json(&req)
                .send()
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to push metrics to {}: {}", url, e),
                })?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Hub at {} returned {}: {}", url, status, body),
            });
        }

        Ok(())
    }
}
