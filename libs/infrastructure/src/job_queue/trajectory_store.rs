/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::{sql_exec, sql_fetch_all, sql_fetch_one, sql_fetch_optional};
use aiome_core::error::AiomeError;
use aiome_core::trajectory::{AgentDiagnosis, FailureCategory, TrajectoryStep, TrajectoryStore};
use async_trait::async_trait;
use sqlx::Row;

// --- Trajectory Operations ---

#[async_trait]
pub trait TrajectoryOps {
    async fn do_record_step(&self, job_id: &str, step: TrajectoryStep) -> Result<(), AiomeError>;
    async fn do_fetch_trajectory(&self, job_id: &str) -> Result<Vec<TrajectoryStep>, AiomeError>;
    async fn do_clear_trajectory_steps(&self, job_id: &str) -> Result<(), AiomeError>;
    async fn do_store_diagnosis(
        &self,
        job_id: &str,
        diagnosis: AgentDiagnosis,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_diagnosis(&self, job_id: &str) -> Result<Option<AgentDiagnosis>, AiomeError>;
}

/// SQL-based implementation of TrajectoryStore
#[derive(Clone)]
pub struct SqliteTrajectoryStore {
    pub pool: crate::db::DatabasePool,
}

impl SqliteTrajectoryStore {
    /// 既存の `DatabasePool` から構築する
    pub fn new(pool: crate::db::DatabasePool) -> Self {
        Self { pool }
    }

    /// DB パスから自動的にプールを構築するファクトリ関数
    ///
    /// 外部クレート（napi-bridge 等）が sqlx に直接依存せずに
    /// TrajectoryStore を構築できるようにする便利メソッド。
    pub async fn from_db_path(db_path: &str) -> Result<Self, AiomeError> {
        let pool = if db_path.starts_with("postgres://") || db_path.starts_with("postgresql://") {
            let pg =
                sqlx::PgPool::connect(db_path)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to connect TS pool: {}", e),
                    })?;
            crate::db::DatabasePool::Postgres(pg)
        } else {
            use std::str::FromStr;
            let clean = db_path
                .strip_prefix("sqlite://")
                .or_else(|| db_path.strip_prefix("sqlite:"))
                .unwrap_or(db_path);
            let opts = sqlx::sqlite::SqliteConnectOptions::from_str(clean)
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Invalid DB path for TS: {}", e),
                })?
                .create_if_missing(true);
            let sq = sqlx::sqlite::SqlitePoolOptions::new()
                .connect_with(opts)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to connect TS pool: {}", e),
                })?;
            crate::db::DatabasePool::Sqlite(sq)
        };
        Ok(Self { pool })
    }
}

#[async_trait]
impl TrajectoryStore for SqliteTrajectoryStore {
    async fn record_step(&self, job_id: &str, step: TrajectoryStep) -> Result<(), AiomeError> {
        self.do_record_step(job_id, step).await
    }

    async fn fetch_trajectory(&self, job_id: &str) -> Result<Vec<TrajectoryStep>, AiomeError> {
        self.do_fetch_trajectory(job_id).await
    }

    async fn clear_trajectory_steps(&self, job_id: &str) -> Result<(), AiomeError> {
        self.do_clear_trajectory_steps(job_id).await
    }

    async fn store_diagnosis(
        &self,
        job_id: &str,
        diagnosis: AgentDiagnosis,
    ) -> Result<(), AiomeError> {
        self.do_store_diagnosis(job_id, diagnosis).await
    }

    async fn fetch_diagnosis(&self, job_id: &str) -> Result<Option<AgentDiagnosis>, AiomeError> {
        self.do_fetch_diagnosis(job_id).await
    }
}

#[async_trait]
impl TrajectoryOps for SqliteTrajectoryStore {
    async fn do_record_step(&self, job_id: &str, step: TrajectoryStep) -> Result<(), AiomeError> {
        let input_json = serde_json::to_string(&step.input).unwrap_or_else(|_| "{}".to_string());
        let output_json = serde_json::to_string(&step.output).unwrap_or_else(|_| "{}".to_string());
        let violations_json =
            serde_json::to_string(&step.constraint_violations).unwrap_or_else(|_| "[]".to_string());
        let failure_cat = step.failure_category.as_ref().map(|c| c.to_string());
        let is_critical = if step.is_critical_failure { 1 } else { 0 };

        let step_cat = serde_json::to_string(&step.step_category)
            .unwrap_or_else(|_| "\"General\"".to_string());

        let q = format!("INSERT INTO trajectory_steps (job_id, step_id, action, tool_name, input_json, output_json, timestamp, constraint_violations, is_critical_failure, failure_category, reasoning, parent_step_id, step_category, completion_criteria, interaction_id) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, {10}, {11}, {12}, {13}, {14})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6), self.pool.ph(7), self.pool.ph(8), self.pool.ph(9), self.pool.ph(10), self.pool.ph(11), self.pool.ph(12), self.pool.ph(13), self.pool.ph(14));

        let parent_id_cast = step.parent_step_id.map(|id| id as i64);

        sql_exec!(
            &self.pool,
            &q,
            job_id,
            step.step_id as i64,
            &step.action,
            &step.tool_name,
            &input_json,
            &output_json,
            &step.timestamp,
            &violations_json,
            is_critical,
            &failure_cat,
            &step.reasoning,
            parent_id_cast,
            &step_cat,
            &step.completion_criteria,
            &step.interaction_id
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to record trajectory step: {}", e),
        })?;
        Ok(())
    }

    async fn do_fetch_trajectory(&self, job_id: &str) -> Result<Vec<TrajectoryStep>, AiomeError> {
        let q = format!("SELECT step_id, action, tool_name, input_json, output_json, timestamp, constraint_violations, is_critical_failure, failure_category, reasoning, parent_step_id, step_category, completion_criteria, interaction_id FROM trajectory_steps WHERE job_id = {} ORDER BY step_id ASC", self.pool.ph(0));
        let mut steps = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(&q)
                    .bind(job_id)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for row in rows {
                    let input_str: String = row.get("input_json");
                    let output_str: String = row.get("output_json");
                    let violations_str: String = row.get("constraint_violations");
                    let failure_cat_str: Option<String> = row.get("failure_category");

                    steps.push(TrajectoryStep {
                        step_id: row.get::<i64, _>("step_id") as u32,
                        action: row.get("action"),
                        tool_name: row.get("tool_name"),
                        input: serde_json::from_str(&input_str).unwrap_or(serde_json::Value::Null),
                        output: serde_json::from_str(&output_str)
                            .unwrap_or(serde_json::Value::Null),
                        timestamp: row.get("timestamp"),
                        constraint_violations: serde_json::from_str(&violations_str)
                            .unwrap_or_default(),
                        is_critical_failure: row.get::<i32, _>("is_critical_failure") != 0,
                        failure_category: failure_cat_str.and_then(|s| {
                            s.parse()
                                .inspect_err(|e| {
                                    tracing::warn!("Failed to parse failure_category: {:?}", e);
                                })
                                .ok()
                        }),
                        reasoning: row.get("reasoning"),
                        parent_step_id: row.get::<Option<String>, _>("parent_step_id").and_then(
                            |v| {
                                v.parse()
                                    .map_err(|e| {
                                        tracing::warn!("Failed to parse parent_step_id: {:?}", e);
                                        e
                                    })
                                    .ok()
                            },
                        ),
                        step_category: serde_json::from_str(&row.get::<String, _>("step_category"))
                            .unwrap_or_default(),
                        completion_criteria: row.get("completion_criteria"),
                        interaction_id: row.get("interaction_id"),
                        verified_invariants: vec![],
                        verification_time_us: None,
                        state_hash: None,
                        parent_state_hash: None,
                        ..Default::default()
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(&q)
                    .bind(job_id)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for row in rows {
                    let input_str: String = row.get("input_json");
                    let output_str: String = row.get("output_json");
                    let violations_str: String = row.get("constraint_violations");
                    let failure_cat_str: Option<String> = row.get("failure_category");

                    steps.push(TrajectoryStep {
                        step_id: row.get::<i32, _>("step_id") as u32,
                        action: row.get("action"),
                        tool_name: row.get("tool_name"),
                        input: serde_json::from_str(&input_str).unwrap_or(serde_json::Value::Null),
                        output: serde_json::from_str(&output_str)
                            .unwrap_or(serde_json::Value::Null),
                        timestamp: row.get("timestamp"),
                        constraint_violations: serde_json::from_str(&violations_str)
                            .unwrap_or_default(),
                        is_critical_failure: row.get::<i32, _>("is_critical_failure") != 0,
                        failure_category: failure_cat_str.and_then(|s| {
                            s.parse()
                                .inspect_err(|e| {
                                    tracing::warn!("Failed to parse failure_category: {:?}", e);
                                })
                                .ok()
                        }),
                        reasoning: row.get("reasoning"),
                        parent_step_id: row.get::<Option<String>, _>("parent_step_id").and_then(
                            |v| {
                                v.parse()
                                    .map_err(|e| {
                                        tracing::warn!("Failed to parse parent_step_id: {:?}", e);
                                        e
                                    })
                                    .ok()
                            },
                        ),
                        step_category: serde_json::from_str(&row.get::<String, _>("step_category"))
                            .unwrap_or_default(),
                        completion_criteria: row.get("completion_criteria"),
                        interaction_id: row.get("interaction_id"),
                        verified_invariants: vec![],
                        verification_time_us: None,
                        state_hash: None,
                        parent_state_hash: None,
                        ..Default::default()
                    });
                }
            }
        }
        Ok(steps)
    }

    async fn do_clear_trajectory_steps(&self, job_id: &str) -> Result<(), AiomeError> {
        let q = format!(
            "DELETE FROM trajectory_steps WHERE job_id = {}",
            self.pool.ph(0)
        );
        crate::sql_exec!(&self.pool, &q, job_id).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to clear trajectory steps: {}", e),
        })?;
        Ok(())
    }

    async fn do_store_diagnosis(
        &self,
        job_id: &str,
        diagnosis: AgentDiagnosis,
    ) -> Result<(), AiomeError> {
        let evidence_json =
            serde_json::to_string(&diagnosis.evidence).unwrap_or_else(|_| "[]".to_string());
        let category_str = diagnosis.category.to_string();
        let cols = [
            "job_id",
            "critical_failure_step",
            "failure_category",
            "root_cause",
            "evidence",
            "self_repair_hint",
            "diagnosed_at",
        ];
        let q = self
            .pool
            .upsert_query("agent_diagnoses", "job_id", &cols, 0);
        sql_exec!(
            &self.pool,
            &q,
            job_id,
            diagnosis.critical_failure_step as i64,
            &category_str,
            &diagnosis.root_cause,
            &evidence_json,
            &diagnosis.self_repair_hint,
            &diagnosis.diagnosed_at
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to store agent diagnosis: {}", e),
        })?;
        Ok(())
    }

    async fn do_fetch_diagnosis(&self, job_id: &str) -> Result<Option<AgentDiagnosis>, AiomeError> {
        let q = format!("SELECT critical_failure_step, failure_category, root_cause, evidence, self_repair_hint, diagnosed_at FROM agent_diagnoses WHERE job_id = {}", self.pool.ph(0));
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let opt = sqlx::query(&q)
                    .bind(job_id)
                    .fetch_optional(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                if let Some(row) = opt {
                    let cat_str: String = row.get("failure_category");
                    let evidence_str: String = row.get("evidence");
                    Ok(Some(AgentDiagnosis {
                        critical_failure_step: row.get::<i64, _>("critical_failure_step") as u32,
                        category: cat_str.parse().unwrap_or(FailureCategory::SystemFailure),
                        root_cause: row.get("root_cause"),
                        evidence: serde_json::from_str(&evidence_str).unwrap_or_default(),
                        self_repair_hint: row.get("self_repair_hint"),
                        diagnosed_at: row.get("diagnosed_at"),
                    }))
                } else {
                    Ok(None)
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let opt = sqlx::query(&q)
                    .bind(job_id)
                    .fetch_optional(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                if let Some(row) = opt {
                    let cat_str: String = row.get("failure_category");
                    let evidence_str: String = row.get("evidence");
                    Ok(Some(AgentDiagnosis {
                        critical_failure_step: row.get::<i32, _>("critical_failure_step") as u32,
                        category: cat_str.parse().unwrap_or(FailureCategory::SystemFailure),
                        root_cause: row.get("root_cause"),
                        evidence: serde_json::from_str(&evidence_str).unwrap_or_default(),
                        self_repair_hint: row.get("self_repair_hint"),
                        diagnosed_at: row.get("diagnosed_at"),
                    }))
                } else {
                    Ok(None)
                }
            }
        }
    }
}
