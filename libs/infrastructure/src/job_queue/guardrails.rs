/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::swarm::SwarmOps;
use crate::{sql_exec, sql_fetch_all, sql_fetch_one, sql_fetch_optional};

use super::UniversalJobQueue;
use aiome_core::contracts::{ArenaMatch, ImmuneRule};
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use sqlx::Row;

#[async_trait]
pub trait GuardrailOps {
    async fn do_store_immune_rule(&self, rule: &ImmuneRule) -> Result<(), AiomeError>;
    async fn do_delete_immune_rule(&self, rule_id: &str) -> Result<(), AiomeError>;
    async fn do_fetch_active_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError>;
    async fn do_get_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError>;
    async fn do_record_arena_match(&self, match_data: &ArenaMatch) -> Result<(), AiomeError>;
}

#[async_trait]
impl GuardrailOps for UniversalJobQueue {
    async fn do_store_immune_rule(&self, rule: &ImmuneRule) -> Result<(), AiomeError> {
        // SEC: Use do_* methods directly instead of trait methods (get_node_id, tick_local_clock, sign_swarm_payload)
        // to avoid pulling in the massive `impl JobQueue for UniversalJobQueue` future (60+ async methods)
        // which causes stack overflow.
        let node_id = self.do_get_node_id().await.unwrap_or_default();
        let clock = self.do_tick_local_clock().await.unwrap_or(0);
        let sign_target = format!("{}:{}:{}", rule.id, rule.pattern, clock);
        let signature = match self.do_sign_swarm_payload(&sign_target).await {
            Ok(s) => Some(s),
            Err(e) => {
                tracing::warn!("Failed to sign immune rule payload: {}", e);
                None
            }
        };

        let status_str = match rule.approval_status {
            aiome_core::contracts::ApprovalState::Approved => "Approved",
            aiome_core::contracts::ApprovalState::Pending => "Pending",
            aiome_core::contracts::ApprovalState::Rejected => "Rejected",
        };

        let cols = [
            "id",
            "pattern",
            "severity",
            "action",
            "created_at",
            "node_id",
            "lamport_clock",
            "signature",
            "status",
        ];
        let q = self.pool.upsert_query("immune_rules", "id", &cols, 0);

        sql_exec!(
            &self.pool,
            &q,
            &rule.id,
            &rule.pattern,
            rule.severity as i64,
            &rule.action,
            &rule.created_at,
            &node_id,
            clock as i64,
            signature,
            status_str
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to store immune rule: {}", e),
        })?;
        Ok(())
    }

    async fn do_delete_immune_rule(&self, rule_id: &str) -> Result<(), AiomeError> {
        let q = format!("DELETE FROM immune_rules WHERE id = {}", self.pool.ph(0));
        sql_exec!(&self.pool, &q, rule_id).map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;
        Ok(())
    }

    async fn do_fetch_active_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        let q = "SELECT id, pattern, severity, action, created_at, lamport_clock, node_id, signature, status FROM immune_rules WHERE status = 'Approved' OR status = 'Active' ORDER BY created_at DESC";
        let mut rules = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows =
                    sqlx::query(q)
                        .fetch_all(p)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                for r in rows {
                    let status_str: String = r.get("status");
                    let approval_status = match status_str.as_str() {
                        "Approved" | "Active" => aiome_core::contracts::ApprovalState::Approved,
                        "Rejected" | "Quarantined" => {
                            aiome_core::contracts::ApprovalState::Rejected
                        }
                        _ => aiome_core::contracts::ApprovalState::Pending,
                    };
                    rules.push(ImmuneRule {
                        id: r.get("id"),
                        pattern: r.get("pattern"),
                        severity: r.get::<i64, _>("severity") as u8,
                        action: r.get("action"),
                        created_at: r.get("created_at"),
                        approval_status,
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.get("node_id"),
                        signature: r.try_get("signature").ok(),
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows =
                    sqlx::query(q)
                        .fetch_all(p)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                for r in rows {
                    let status_str: String = r.get("status");
                    let approval_status = match status_str.as_str() {
                        "Approved" | "Active" => aiome_core::contracts::ApprovalState::Approved,
                        "Rejected" | "Quarantined" => {
                            aiome_core::contracts::ApprovalState::Rejected
                        }
                        _ => aiome_core::contracts::ApprovalState::Pending,
                    };
                    rules.push(ImmuneRule {
                        id: r.get("id"),
                        pattern: r.get("pattern"),
                        severity: r.get::<i32, _>("severity") as u8,
                        action: r.get("action"),
                        created_at: r.get("created_at"),
                        approval_status,
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.get("node_id"),
                        signature: r.try_get("signature").ok(),
                    });
                }
            }
        }
        Ok(rules)
    }

    async fn do_get_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        let q = "SELECT id, pattern, severity, action, created_at, lamport_clock, node_id, signature, status FROM immune_rules ORDER BY created_at DESC";
        let mut rules = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows =
                    sqlx::query(q)
                        .fetch_all(p)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                for r in rows {
                    let status_str: String = r.get("status");
                    let approval_status = match status_str.as_str() {
                        "Approved" | "Active" => aiome_core::contracts::ApprovalState::Approved,
                        "Rejected" | "Quarantined" => {
                            aiome_core::contracts::ApprovalState::Rejected
                        }
                        _ => aiome_core::contracts::ApprovalState::Pending,
                    };
                    rules.push(ImmuneRule {
                        id: r.get("id"),
                        pattern: r.get("pattern"),
                        severity: r.get::<i64, _>("severity") as u8,
                        action: r.get("action"),
                        created_at: r.get("created_at"),
                        approval_status,
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.get("node_id"),
                        signature: r.try_get("signature").ok(),
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows =
                    sqlx::query(q)
                        .fetch_all(p)
                        .await
                        .map_err(|e| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                for r in rows {
                    let status_str: String = r.get("status");
                    let approval_status = match status_str.as_str() {
                        "Approved" | "Active" => aiome_core::contracts::ApprovalState::Approved,
                        "Rejected" | "Quarantined" => {
                            aiome_core::contracts::ApprovalState::Rejected
                        }
                        _ => aiome_core::contracts::ApprovalState::Pending,
                    };
                    rules.push(ImmuneRule {
                        id: r.get("id"),
                        pattern: r.get("pattern"),
                        severity: r.get::<i32, _>("severity") as u8,
                        action: r.get("action"),
                        created_at: r.get("created_at"),
                        approval_status,
                        lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                        node_id: r.get("node_id"),
                        signature: r.try_get("signature").ok(),
                    });
                }
            }
        }
        Ok(rules)
    }

    async fn do_record_arena_match(&self, match_data: &ArenaMatch) -> Result<(), AiomeError> {
        let q = format!("INSERT INTO arena_history (id, skill_a, skill_b, topic, winner, reasoning, created_at) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}) ON CONFLICT(id) DO NOTHING",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6));
        sql_exec!(
            &self.pool,
            &q,
            &match_data.id,
            &match_data.skill_a,
            &match_data.skill_b,
            &match_data.topic,
            &match_data.winner,
            &match_data.reasoning,
            &match_data.created_at
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;
        Ok(())
    }
}
