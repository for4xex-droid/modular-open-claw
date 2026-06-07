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
        let mut fed_rules = Vec::new();
        let mut fed_matches = Vec::new();

        let q_karma = format!(
            "SELECT * FROM karma_logs WHERE created_at > {} AND is_private = 0 AND is_archived = 0",
            self.pool.ph(0)
        );
        let q_rules = format!(
            "SELECT * FROM immune_rules WHERE created_at > {}",
            self.pool.ph(0)
        );
        let q_matches = format!(
            "SELECT * FROM arena_history WHERE created_at > {}",
            self.pool.ph(0)
        );

        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = crate::sql_fetch_raw!(p, &q_karma, since_ts)?;
                for r in rows {
                    fed_karmas.push(map_sqlite_row_to_karma(&r, "unknown"));
                }

                let rows_rules = crate::sql_fetch_raw!(p, &q_rules, since_ts)?;
                for r in rows_rules {
                    fed_rules.push(ImmuneRule {
                        id: r.try_get("id").unwrap_or_default(),
                        pattern: r.try_get("pattern").unwrap_or_default(),
                        severity: r.try_get::<i64, _>("severity").unwrap_or(0) as u8,
                        action: r.try_get("action").unwrap_or_default(),
                        approval_status: match r
                            .try_get::<String, _>("status")
                            .unwrap_or_default()
                            .as_str()
                        {
                            "Approved" => aiome_core::contracts::ApprovalState::Approved,
                            "Rejected" => aiome_core::contracts::ApprovalState::Rejected,
                            _ => aiome_core::contracts::ApprovalState::Pending,
                        },
                        input_constraints: None,
                        created_at: r.try_get("created_at").unwrap_or_default(),
                        lamport_clock: r.try_get::<i64, _>("lamport_clock").unwrap_or(0) as u64,
                        node_id: r
                            .try_get("node_id")
                            .unwrap_or_else(|_| "unknown".to_string()),
                        signature: r.try_get("signature").ok(),
                    });
                }

                let rows_matches = crate::sql_fetch_raw!(p, &q_matches, since_ts)?;
                for r in rows_matches {
                    fed_matches.push(ArenaMatch {
                        id: r.try_get("id").unwrap_or_default(),
                        skill_a: r.try_get("skill_a").unwrap_or_default(),
                        skill_b: r.try_get("skill_b").unwrap_or_default(),
                        topic: r.try_get("topic").unwrap_or_default(),
                        output_a: r.try_get("output_a").ok(),
                        output_b: r.try_get("output_b").ok(),
                        winner: r.try_get("winner").ok(),
                        reasoning: r.try_get("reasoning").unwrap_or_default(),
                        created_at: r.try_get("created_at").unwrap_or_default(),
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = crate::sql_fetch_raw!(p, &q_karma, since_ts)?;
                for r in rows {
                    fed_karmas.push(map_postgres_row_to_karma(&r, "unknown"));
                }

                let rows_rules = crate::sql_fetch_raw!(p, &q_rules, since_ts)?;
                for r in rows_rules {
                    fed_rules.push(ImmuneRule {
                        id: r.try_get("id").unwrap_or_default(),
                        pattern: r.try_get("pattern").unwrap_or_default(),
                        severity: r.try_get::<i32, _>("severity").unwrap_or(0) as u8,
                        action: r.try_get("action").unwrap_or_default(),
                        approval_status: match r
                            .try_get::<String, _>("status")
                            .unwrap_or_default()
                            .as_str()
                        {
                            "Approved" => aiome_core::contracts::ApprovalState::Approved,
                            "Rejected" => aiome_core::contracts::ApprovalState::Rejected,
                            _ => aiome_core::contracts::ApprovalState::Pending,
                        },
                        input_constraints: None,
                        created_at: r.try_get("created_at").unwrap_or_default(),
                        lamport_clock: r.try_get::<i64, _>("lamport_clock").unwrap_or(0) as u64,
                        node_id: r
                            .try_get("node_id")
                            .unwrap_or_else(|_| "unknown".to_string()),
                        signature: r.try_get("signature").ok(),
                    });
                }

                let rows_matches = crate::sql_fetch_raw!(p, &q_matches, since_ts)?;
                for r in rows_matches {
                    fed_matches.push(ArenaMatch {
                        id: r.try_get("id").unwrap_or_default(),
                        skill_a: r.try_get("skill_a").unwrap_or_default(),
                        skill_b: r.try_get("skill_b").unwrap_or_default(),
                        topic: r.try_get("topic").unwrap_or_default(),
                        output_a: r.try_get("output_a").ok(),
                        output_b: r.try_get("output_b").ok(),
                        winner: r.try_get("winner").ok(),
                        reasoning: r.try_get("reasoning").unwrap_or_default(),
                        created_at: r.try_get("created_at").unwrap_or_default(),
                    });
                }
            }
        }

        Ok((fed_karmas, fed_rules, fed_matches))
    }

    async fn do_import_federated_data(
        &self,
        karmas: Vec<FederatedKarma>,
        rules: Vec<ImmuneRule>,
        matches: Vec<ArenaMatch>,
    ) -> Result<(), AiomeError> {
        for karma in karmas {
            let karma_id = karma.id.clone();
            crate::sql_exec!(
                &self.pool,
                sqlite: "INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, is_federated, clone_origin_id, lamport_clock, node_id, signature, created_at, last_applied_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT (id) DO UPDATE SET job_id = EXCLUDED.job_id, karma_type = EXCLUDED.karma_type, related_skill = EXCLUDED.related_skill, lesson = EXCLUDED.lesson, weight = EXCLUDED.weight, soul_version_hash = EXCLUDED.soul_version_hash, is_federated = EXCLUDED.is_federated, clone_origin_id = EXCLUDED.clone_origin_id, lamport_clock = EXCLUDED.lamport_clock, node_id = EXCLUDED.node_id, signature = EXCLUDED.signature, created_at = EXCLUDED.created_at, last_applied_at = EXCLUDED.last_applied_at WHERE karma_logs.lamport_clock < EXCLUDED.lamport_clock",
                pg: "INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, is_federated, clone_origin_id, lamport_clock, node_id, signature, created_at, last_applied_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) ON CONFLICT (id) DO UPDATE SET job_id = EXCLUDED.job_id, karma_type = EXCLUDED.karma_type, related_skill = EXCLUDED.related_skill, lesson = EXCLUDED.lesson, weight = EXCLUDED.weight, soul_version_hash = EXCLUDED.soul_version_hash, is_federated = EXCLUDED.is_federated, clone_origin_id = EXCLUDED.clone_origin_id, lamport_clock = EXCLUDED.lamport_clock, node_id = EXCLUDED.node_id, signature = EXCLUDED.signature, created_at = EXCLUDED.created_at, last_applied_at = EXCLUDED.last_applied_at WHERE karma_logs.lamport_clock < EXCLUDED.lamport_clock",
                karma.id,
                karma.job_id,
                karma.karma_type,
                karma.related_skill,
                karma.lesson,
                karma.weight,
                karma.soul_version_hash,
                1,
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

        for rule in rules {
            let status_str = match rule.approval_status {
                aiome_core::contracts::ApprovalState::Approved => "Approved",
                aiome_core::contracts::ApprovalState::Rejected => "Rejected",
                _ => "Pending",
            };

            let rule_id = rule.id.clone();
            crate::sql_exec!(
                &self.pool,
                sqlite: "INSERT INTO immune_rules (id, pattern, severity, action, status, is_federated, lamport_clock, node_id, signature, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT (id) DO UPDATE SET pattern = EXCLUDED.pattern, severity = EXCLUDED.severity, action = EXCLUDED.action, status = EXCLUDED.status, is_federated = EXCLUDED.is_federated, lamport_clock = EXCLUDED.lamport_clock, node_id = EXCLUDED.node_id, signature = EXCLUDED.signature, created_at = EXCLUDED.created_at WHERE immune_rules.lamport_clock < EXCLUDED.lamport_clock",
                pg: "INSERT INTO immune_rules (id, pattern, severity, action, status, is_federated, lamport_clock, node_id, signature, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) ON CONFLICT (id) DO UPDATE SET pattern = EXCLUDED.pattern, severity = EXCLUDED.severity, action = EXCLUDED.action, status = EXCLUDED.status, is_federated = EXCLUDED.is_federated, lamport_clock = EXCLUDED.lamport_clock, node_id = EXCLUDED.node_id, signature = EXCLUDED.signature, created_at = EXCLUDED.created_at WHERE immune_rules.lamport_clock < EXCLUDED.lamport_clock",
                rule.id,
                rule.pattern,
                rule.severity as i32,
                rule.action,
                status_str,
                1,
                rule.lamport_clock as i64,
                rule.node_id,
                rule.signature,
                rule.created_at
            )
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to import federated rule {}: {}", rule_id, e),
            })?;
        }

        for mat in matches {
            let match_id = mat.id.clone();
            crate::sql_exec!(
                &self.pool,
                sqlite: "INSERT INTO arena_history (id, skill_a, skill_b, topic, output_a, output_b, winner, reasoning, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT (id) DO NOTHING",
                pg: "INSERT INTO arena_history (id, skill_a, skill_b, topic, output_a, output_b, winner, reasoning, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) ON CONFLICT (id) DO NOTHING",
                mat.id,
                mat.skill_a,
                mat.skill_b,
                mat.topic,
                mat.output_a,
                mat.output_b,
                mat.winner,
                mat.reasoning,
                mat.created_at
            )
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to import federated match {}: {}", match_id, e),
            })?;
        }

        Ok(())
    }

    async fn do_get_peer_sync_time(&self, peer_url: &str) -> Result<Option<String>, AiomeError> {
        let opt: Option<(String,)> = crate::sql_fetch_optional!(
            &self.pool,
            (String,),
            sqlite: "SELECT last_sync_at FROM peer_sync_times WHERE peer_url = ?",
            pg: "SELECT last_sync_at FROM peer_sync_times WHERE peer_url = $1",
            peer_url
        )?;
        Ok(opt.map(|r| r.0))
    }

    async fn do_update_peer_sync_time(
        &self,
        peer_url: &str,
        sync_time: &str,
    ) -> Result<(), AiomeError> {
        crate::sql_exec!(
            &self.pool,
            sqlite: "INSERT OR REPLACE INTO peer_sync_times (peer_url, last_sync_at) VALUES (?, ?)",
            pg: "INSERT INTO peer_sync_times (peer_url, last_sync_at) VALUES ($1, $2) ON CONFLICT(peer_url) DO UPDATE SET last_sync_at = EXCLUDED.last_sync_at",
            peer_url,
            sync_time
        ).map(|_| ())
    }

    async fn do_fetch_unfederated_data(
        &self,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>), AiomeError> {
        let q_karma = "SELECT * FROM karma_logs WHERE is_federated = 0";
        let mut karmas = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = crate::sql_fetch_raw!(p, q_karma)?;
                for r in rows {
                    karmas.push(map_sqlite_row_to_karma(&r, "self"));
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = crate::sql_fetch_raw!(p, q_karma)?;
                for r in rows {
                    karmas.push(map_postgres_row_to_karma(&r, "self"));
                }
            }
        }

        let q_rules = "SELECT * FROM immune_rules WHERE is_federated = 0";
        let mut rules = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = crate::sql_fetch_raw!(p, q_rules)?;
                for r in rows {
                    rules.push(ImmuneRule {
                        id: r.try_get("id").unwrap_or_default(),
                        pattern: r.try_get("pattern").unwrap_or_default(),
                        severity: r.try_get::<i32, _>("severity").unwrap_or(0) as u8,
                        action: r.try_get("action").unwrap_or_default(),
                        approval_status: aiome_core::contracts::ApprovalState::Pending,
                        input_constraints: None,
                        created_at: r.try_get("created_at").unwrap_or_default(),
                        lamport_clock: r.try_get::<i64, _>("lamport_clock").unwrap_or(0) as u64,
                        node_id: r.try_get("node_id").unwrap_or_else(|_| "self".to_string()),
                        signature: None,
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = crate::sql_fetch_raw!(p, q_rules)?;
                for r in rows {
                    rules.push(ImmuneRule {
                        id: r.try_get("id").unwrap_or_default(),
                        pattern: r.try_get("pattern").unwrap_or_default(),
                        severity: r.try_get::<i32, _>("severity").unwrap_or(0) as u8,
                        action: r.try_get("action").unwrap_or_default(),
                        approval_status: aiome_core::contracts::ApprovalState::Pending,
                        input_constraints: None,
                        created_at: r.try_get("created_at").unwrap_or_default(),
                        lamport_clock: r.try_get::<i64, _>("lamport_clock").unwrap_or(0) as u64,
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
            crate::sql_exec!(
                &self.pool,
                sqlite: "UPDATE karma_logs SET is_federated = 1 WHERE id = ?",
                pg: "UPDATE karma_logs SET is_federated = 1 WHERE id = $1",
                &id
            )
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        }

        for id in rule_ids {
            crate::sql_exec!(
                &self.pool,
                sqlite: "UPDATE immune_rules SET is_federated = 1 WHERE id = ?",
                pg: "UPDATE immune_rules SET is_federated = 1 WHERE id = $1",
                &id
            )
            .map_err(|e| AiomeError::Infrastructure {
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
            .redirect(reqwest::redirect::Policy::none())
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
            .redirect(reqwest::redirect::Policy::none())
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

fn map_sqlite_row_to_karma(r: &sqlx::sqlite::SqliteRow, default_node_id: &str) -> FederatedKarma {
    use sqlx::Row;
    FederatedKarma {
        id: r.try_get("id").unwrap_or_default(),
        job_id: r.try_get("job_id").ok(),
        karma_type: r.try_get("karma_type").unwrap_or_default(),
        related_skill: r.try_get("related_skill").unwrap_or_default(),
        lesson: r.try_get("lesson").unwrap_or_default(),
        weight: r.try_get::<i64, _>("weight").unwrap_or(0) as i32,
        soul_version_hash: r.try_get("soul_version_hash").ok(),
        created_at: r.try_get("created_at").unwrap_or_default(),
        last_applied_at: r.try_get("last_applied_at").ok(),
        score: r
            .try_get("weight")
            .map(|w: i64| w as f64 / 100.0)
            .unwrap_or(0.0),
        lamport_clock: r.try_get::<i64, _>("lamport_clock").unwrap_or(0) as u64,
        node_id: r
            .try_get("node_id")
            .unwrap_or_else(|_| default_node_id.to_string()),
        signature: r.try_get("signature").ok(),
        clone_origin_id: r.try_get("clone_origin_id").ok(),
        generation: r.try_get::<i64, _>("generation").map(|g| g as u32).ok(),
        somatic_valence: r.try_get::<f64, _>("somatic_valence").ok(),
    }
}

fn map_postgres_row_to_karma(r: &sqlx::postgres::PgRow, default_node_id: &str) -> FederatedKarma {
    use sqlx::Row;
    FederatedKarma {
        id: r.try_get("id").unwrap_or_default(),
        job_id: r.try_get("job_id").ok(),
        karma_type: r.try_get("karma_type").unwrap_or_default(),
        related_skill: r.try_get("related_skill").unwrap_or_default(),
        lesson: r.try_get("lesson").unwrap_or_default(),
        weight: r.try_get::<i32, _>("weight").unwrap_or(0),
        soul_version_hash: r.try_get("soul_version_hash").ok(),
        created_at: r.try_get("created_at").unwrap_or_default(),
        last_applied_at: r.try_get("last_applied_at").ok(),
        score: r
            .try_get("weight")
            .map(|w: i32| w as f64 / 100.0)
            .unwrap_or(0.0),
        lamport_clock: r.try_get::<i64, _>("lamport_clock").unwrap_or(0) as u64,
        node_id: r
            .try_get("node_id")
            .unwrap_or_else(|_| default_node_id.to_string()),
        signature: r.try_get("signature").ok(),
        clone_origin_id: r.try_get("clone_origin_id").ok(),
        generation: r.try_get::<i32, _>("generation").map(|g| g as u32).ok(),
        somatic_valence: r.try_get::<f64, _>("somatic_valence").ok(),
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
        sqlx::query("CREATE TABLE karma_logs (id TEXT PRIMARY KEY, karma_type TEXT, related_skill TEXT, lesson TEXT, weight INTEGER, is_federated INTEGER DEFAULT 0, lamport_clock INTEGER DEFAULT 0, created_at TEXT, last_applied_at TEXT, node_id TEXT, job_id TEXT, soul_version_hash TEXT, signature TEXT, clone_origin_id TEXT, generation INTEGER, somatic_valence REAL, is_private INTEGER DEFAULT 0, is_archived INTEGER DEFAULT 0);")
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

    #[tokio::test]
    async fn test_federation_ssrf_prevention() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let target_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&target_server)
            .await;

        let redirect_url = format!("{}/target", target_server.uri());
        Mock::given(method("POST"))
            .and(path("/api/v1/federation/push"))
            .respond_with(
                ResponseTemplate::new(307).insert_header("Location", redirect_url.as_str()),
            )
            .mount(&server)
            .await;

        let sql_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let db_pool = crate::db::DatabasePool::Sqlite(sql_pool.clone());
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(db_pool.clone()),
        );

        sqlx::query("CREATE TABLE system_settings (key TEXT PRIMARY KEY, value TEXT);")
            .execute(&sql_pool)
            .await
            .unwrap();

        sqlx::query(&format!(
            "INSERT INTO system_settings (key, value) VALUES ('samsara_hub_url', '{}');",
            server.uri()
        ))
        .execute(&sql_pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO system_settings (key, value) VALUES ('federation_secret', 'secret');",
        )
        .execute(&sql_pool)
        .await
        .unwrap();

        sqlx::query("CREATE TABLE karma_logs (id TEXT PRIMARY KEY, karma_type TEXT, related_skill TEXT, lesson TEXT, weight INTEGER, is_federated INTEGER DEFAULT 0, lamport_clock INTEGER DEFAULT 0, created_at TEXT, last_applied_at TEXT, node_id TEXT, job_id TEXT, soul_version_hash TEXT, signature TEXT, clone_origin_id TEXT, generation INTEGER, somatic_valence REAL, is_private INTEGER DEFAULT 0, is_archived INTEGER DEFAULT 0);")
            .execute(&sql_pool)
            .await
            .unwrap();

        let queue = UniversalJobQueue::from_pool(db_pool, ts);

        let result = queue.do_push_federated_metrics().await;
        if let Err(ref e) = result {
            println!("Test debug result: {:?}", e);
        }
        assert!(result.is_err());

        let received_requests = target_server.received_requests().await;
        let post_requests = received_requests
            .unwrap_or_default()
            .iter()
            .filter(|r| r.method.as_str() == "POST")
            .count();
        assert_eq!(
            post_requests, 0,
            "SSRF vulnerability: client followed the redirect to the target server!"
        );
    }

    #[tokio::test]
    async fn test_federation_import_export_rules_matches() {
        let sql_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let db_pool = crate::db::DatabasePool::Sqlite(sql_pool.clone());
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(db_pool.clone()),
        );

        sqlx::query("CREATE TABLE karma_logs (id TEXT PRIMARY KEY, karma_type TEXT, related_skill TEXT, lesson TEXT, weight INTEGER, is_federated INTEGER DEFAULT 0, lamport_clock INTEGER DEFAULT 0, created_at TEXT, last_applied_at TEXT, node_id TEXT, job_id TEXT, soul_version_hash TEXT, signature TEXT, clone_origin_id TEXT, generation INTEGER, somatic_valence REAL, is_private INTEGER DEFAULT 0, is_archived INTEGER DEFAULT 0);")
            .execute(&sql_pool)
            .await
            .unwrap();

        sqlx::query("CREATE TABLE immune_rules (id TEXT PRIMARY KEY, pattern TEXT, severity INTEGER, action TEXT, status TEXT, is_federated INTEGER DEFAULT 0, lamport_clock INTEGER DEFAULT 0, node_id TEXT, signature TEXT, created_at TEXT);")
            .execute(&sql_pool)
            .await
            .unwrap();

        sqlx::query("CREATE TABLE arena_history (id TEXT PRIMARY KEY, skill_a TEXT NOT NULL, skill_b TEXT NOT NULL, topic TEXT NOT NULL, output_a TEXT, output_b TEXT, winner TEXT, reasoning TEXT, created_at TEXT);")
            .execute(&sql_pool)
            .await
            .unwrap();

        let queue = UniversalJobQueue::from_pool(db_pool, ts);

        // Insert initial test data
        sqlx::query("INSERT INTO immune_rules (id, pattern, severity, action, status, is_federated, lamport_clock, node_id, created_at) VALUES ('rule-1', 'pattern-1', 50, 'Block', 'Active', 0, 1, 'node-1', '2026-06-07T00:00:00Z');")
            .execute(&sql_pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO arena_history (id, skill_a, skill_b, topic, reasoning, created_at) VALUES ('match-1', 'skill-a', 'skill-b', 'topic-1', 'reasoning-1', '2026-06-07T00:00:00Z');")
            .execute(&sql_pool)
            .await
            .unwrap();

        // 1. Export test
        let (karmas, rules, matches) = queue.do_export_federated_data(None).await.unwrap();
        assert_eq!(rules.len(), 1, "Should export 1 rule");
        assert_eq!(matches.len(), 1, "Should export 1 match");
        assert_eq!(rules[0].id, "rule-1");
        assert_eq!(matches[0].id, "match-1");

        // 2. Import test
        let new_rule = aiome_core_contracts::contracts::ImmuneRule {
            id: "rule-2".to_string(),
            pattern: "pattern-2".to_string(),
            severity: 80,
            action: "Warn".to_string(),
            created_at: "2026-06-07T00:00:00Z".to_string(),
            approval_status: aiome_core_contracts::contracts::ApprovalState::Approved,
            input_constraints: None,
            lamport_clock: 2,
            node_id: "node-2".to_string(),
            signature: None,
        };

        let new_match = aiome_core_contracts::contracts::ArenaMatch {
            id: "match-2".to_string(),
            skill_a: "skill-c".to_string(),
            skill_b: "skill-d".to_string(),
            topic: "topic-2".to_string(),
            output_a: None,
            output_b: None,
            winner: Some("skill-c".to_string()),
            reasoning: "reasoning-2".to_string(),
            created_at: "2026-06-07T00:00:00Z".to_string(),
        };

        queue
            .do_import_federated_data(vec![], vec![new_rule], vec![new_match])
            .await
            .unwrap();

        // Export again to verify import
        let (_, rules2, matches2) = queue.do_export_federated_data(None).await.unwrap();
        assert_eq!(rules2.len(), 2, "Should have 2 rules after import");
        assert_eq!(matches2.len(), 2, "Should have 2 matches after import");
    }
}
