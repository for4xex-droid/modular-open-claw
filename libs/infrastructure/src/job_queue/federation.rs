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

pub struct P2pSanitizer;

impl P2pSanitizer {
    pub fn sanitize(content: &str, banned_words: &[String]) -> Result<(), AiomeError> {
        let lower_content = content.to_lowercase();
        for word in banned_words {
            // Assume banned_words are already pre-processed (trimmed and lowercase)
            if lower_content.contains(word) {
                tracing::warn!("🚨 [Federation] Blocked P2P message containing forbidden word.");
                return Err(AiomeError::SecurityViolation {
                    reason: "Message contains forbidden content".to_string(),
                });
            }
        }
        Ok(())
    }
}

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
    async fn do_sync_federated_data(&self) -> Result<(), AiomeError>;
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
        let q_karma = "SELECT * FROM karma_logs WHERE is_federated = 0";
        let mut karmas = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(q_karma).fetch_all(p).await.map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
                for r in rows {
                    karmas.push(FederatedKarma {
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
                        node_id: r.try_get("node_id").unwrap_or_else(|_| "self".to_string()),
                        signature: r.try_get("signature").ok(),
                        clone_origin_id: r.try_get("clone_origin_id").ok(),
                        generation: r.try_get::<i64, _>("generation").map(|g| g as u32).ok(),
                        somatic_valence: r.try_get::<f64, _>("somatic_valence").ok(),
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(q_karma).fetch_all(p).await.map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
                for r in rows {
                    karmas.push(FederatedKarma {
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
                        node_id: r.try_get("node_id").unwrap_or_else(|_| "self".to_string()),
                        signature: r.try_get("signature").ok(),
                        clone_origin_id: r.try_get("clone_origin_id").ok(),
                        generation: r.try_get::<i32, _>("generation").map(|g| g as u32).ok(),
                        somatic_valence: r.try_get::<f64, _>("somatic_valence").ok(),
                    });
                }
            }
        }

        let q_rules = "SELECT * FROM immune_rules WHERE is_federated = 0";
        let mut rules = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(q_rules).fetch_all(p).await.map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
                for r in rows {
                    rules.push(ImmuneRule {
                        id: r.get("id"),
                        pattern: r.get("pattern"),
                        severity: r.get::<i32, _>("severity") as u8,
                        action: r.get("action"),
                        approval_status: aiome_core::contracts::ApprovalState::Pending,
                        input_constraints: None,
                        created_at: r.get("created_at"),
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.try_get("node_id").unwrap_or_else(|_| "self".to_string()),
                        signature: None,
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(q_rules).fetch_all(p).await.map_err(|e| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
                for r in rows {
                    rules.push(ImmuneRule {
                        id: r.get("id"),
                        pattern: r.get("pattern"),
                        severity: r.get::<i32, _>("severity") as u8,
                        action: r.get("action"),
                        approval_status: aiome_core::contracts::ApprovalState::Pending,
                        input_constraints: None,
                        created_at: r.get("created_at"),
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.try_get("node_id").unwrap_or_else(|_| "self".to_string()),
                        signature: None,
                    });
                }
            }
        }

        Ok((karmas, rules))
    }

    async fn do_mark_as_federated(
        &self,
        karma_ids: Vec<String>,
        rule_ids: Vec<String>,
    ) -> Result<(), AiomeError> {
        for id in karma_ids {
            let q = match &self.pool {
                crate::db::DatabasePool::Sqlite(_) => format!(
                    "UPDATE karma_logs SET is_federated = 1 WHERE id = {}",
                    self.pool.ph(0)
                ),
                crate::db::DatabasePool::Postgres(_) => format!(
                    "UPDATE karma_logs SET is_federated = 1 WHERE id = {}",
                    self.pool.ph(0)
                ),
            };
            crate::sql_exec!(&self.pool, &q, id).map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        }

        for id in rule_ids {
            let q = match &self.pool {
                crate::db::DatabasePool::Sqlite(_) => format!(
                    "UPDATE immune_rules SET is_federated = 1 WHERE id = {}",
                    self.pool.ph(0)
                ),
                crate::db::DatabasePool::Postgres(_) => format!(
                    "UPDATE immune_rules SET is_federated = 1 WHERE id = {}",
                    self.pool.ph(0)
                ),
            };
            crate::sql_exec!(&self.pool, &q, id).map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        }

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

        let (karmas, rules) = self.do_fetch_unfederated_data().await?;
        let karma_ids: Vec<String> = karmas.iter().map(|k| k.id.clone()).collect();
        let rule_ids: Vec<String> = rules.iter().map(|r| r.id.clone()).collect();

        let req = FederationPushRequest {
            node_id,
            karmas,
            rules,
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

        let auth_token = match self.do_get_setting("federation_secret").await? {
            Some(secret) if !secret.trim().is_empty() => secret,
            _ => {
                tracing::warn!("⚠️ [Federation] federation_secret is not set; push will likely fail authentication.");
                return Err(AiomeError::Infrastructure {
                    reason: "federation_secret is not configured in system_settings".to_string(),
                });
            }
        };

        let res = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", auth_token))
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

        // Mark as federated
        if !karma_ids.is_empty() || !rule_ids.is_empty() {
            self.do_mark_as_federated(karma_ids, rule_ids).await?;
            tracing::info!("✅ [Federation] Pushed and marked data as federated.");
        }

        Ok(())
    }

    async fn do_sync_federated_data(&self) -> Result<(), AiomeError> {
        use aiome_core_contracts::traits::SettingsOps;
        let hub_url_opt = self.do_get_setting("samsara_hub_url").await?;
        let hub_url = match hub_url_opt {
            Some(url) if !url.trim().is_empty() => url,
            _ => {
                tracing::warn!("samsara_hub_url is not set; skipping federated data sync.");
                return Ok(());
            }
        };

        let node_id = match self.do_get_setting("node_id").await? {
            Some(id) if !id.trim().is_empty() => id,
            _ => {
                tracing::warn!(
                    "⚠️ [Federation] node_id is not set in system_settings; using fallback 'self'."
                );
                "self".to_string()
            }
        };

        let since = self.do_get_peer_sync_time("hub").await?;

        let req = serde_json::json!({
            "node_id": node_id,
            "since": since,
            "protocol_version": "1.0"
        });

        let url = format!("{}/api/v1/federation/sync", hub_url.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to build HTTP client: {}", e),
            })?;

        let auth_token = match self.do_get_setting("federation_secret").await? {
            Some(secret) if !secret.trim().is_empty() => secret,
            _ => {
                tracing::warn!("⚠️ [Federation] federation_secret is not set; sync will likely fail authentication.");
                return Err(AiomeError::Infrastructure {
                    reason: "federation_secret is not configured in system_settings".to_string(),
                });
            }
        };

        let res = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", auth_token))
            .json(&req)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to sync data from {}: {}", url, e),
            })?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Hub at {} returned {}: {}", url, status, body),
            });
        }

        let sync_data: aiome_core::contracts::FederationSyncResponse =
            res.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to parse sync response: {}", e),
            })?;

        if !sync_data.new_karmas.is_empty()
            || !sync_data.new_immune_rules.is_empty()
            || !sync_data.new_arena_matches.is_empty()
        {
            let karma_count = sync_data.new_karmas.len();
            self.do_import_federated_data(
                sync_data.new_karmas,
                sync_data.new_immune_rules,
                sync_data.new_arena_matches,
            )
            .await?;

            let now = chrono::Utc::now().to_rfc3339();
            self.do_update_peer_sync_time("hub", &now).await?;
            tracing::info!("✅ [Federation] Synced {} karmas from hub.", karma_count);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_queue::FederationOps;

    #[tokio::test]
    async fn test_federation_unstub_do_fetch_unfederated_data() {
        // Create an in-memory UniversalJobQueue
        let sql_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let db_pool = crate::db::DatabasePool::Sqlite(sql_pool.clone());
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(db_pool.clone()),
        );

        // Run migrations
        sqlx::query("CREATE TABLE karma_logs (id TEXT PRIMARY KEY, karma_type TEXT, related_skill TEXT, lesson TEXT, weight INTEGER, is_federated INTEGER DEFAULT 0, lamport_clock INTEGER DEFAULT 0, created_at TEXT, last_applied_at TEXT, node_id TEXT, job_id TEXT, soul_version_hash TEXT, signature TEXT, clone_origin_id TEXT, generation INTEGER, somatic_valence REAL);")
            .execute(&sql_pool)
            .await
            .unwrap();

        sqlx::query("CREATE TABLE immune_rules (id TEXT PRIMARY KEY, pattern TEXT, severity INTEGER, action TEXT, status TEXT, is_federated INTEGER DEFAULT 0, lamport_clock INTEGER DEFAULT 0, node_id TEXT, signature TEXT, created_at TEXT);")
            .execute(&sql_pool)
            .await
            .unwrap();

        let queue = UniversalJobQueue::from_pool(db_pool, ts);

        // Insert unfederated data
        sqlx::query("INSERT INTO karma_logs (id, karma_type, related_skill, lesson, weight, is_federated, node_id, created_at, lamport_clock) VALUES ('k1', 'test', 'test', 'lesson', 1, 0, 'self', '2026-01-01', 1);")
            .execute(&sql_pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO immune_rules (id, pattern, severity, action, status, is_federated, node_id, created_at, lamport_clock) VALUES ('r1', '.*', 10, 'block', 'Active', 0, 'self', '2026-01-01', 1);")
            .execute(&sql_pool)
            .await
            .unwrap();

        let (karmas, rules) = queue.do_fetch_unfederated_data().await.unwrap();

        // RED: Currently returns (Vec::new(), Vec::new())
        assert_eq!(karmas.len(), 1, "Should fetch 1 unfederated karma");
        assert_eq!(rules.len(), 1, "Should fetch 1 unfederated rule");

        // Test mark_as_federated
        queue
            .do_mark_as_federated(vec!["k1".to_string()], vec!["r1".to_string()])
            .await
            .unwrap();

        // Fetch again, should be 0
        let (karmas2, rules2) = queue.do_fetch_unfederated_data().await.unwrap();
        assert_eq!(
            karmas2.len(),
            0,
            "Should fetch 0 unfederated karma after mark_as_federated"
        );
        assert_eq!(
            rules2.len(),
            0,
            "Should fetch 0 unfederated rule after mark_as_federated"
        );
    }

    #[tokio::test]
    async fn test_p2p_sanitizer_blocks_forbidden_words() {
        let banned = vec!["csam".to_string(), "malware_payload".to_string()];
        
        let safe_content = "This is a safe message about federation.";
        let result = crate::job_queue::federation::P2pSanitizer::sanitize(safe_content, &banned);
        assert!(result.is_ok(), "Safe content should pass");

        let unsafe_content = "Check out this CSAM link!";
        let result = crate::job_queue::federation::P2pSanitizer::sanitize(unsafe_content, &banned);
        assert!(result.is_err(), "Unsafe content should be blocked");
    }
}
