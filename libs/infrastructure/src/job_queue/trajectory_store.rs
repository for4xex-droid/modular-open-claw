/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::UniversalJobQueue;
use aiome_core::error::AiomeError;
use aiome_core::trajectory::{AgentDiagnosis, FailureCategory, TrajectoryStep, TrajectoryStore};
use async_trait::async_trait;
use sqlx::Row;

#[async_trait]
pub trait TrajectoryOps {
    async fn do_record_step(&self, job_id: &str, step: TrajectoryStep) -> Result<(), AiomeError>;
    async fn do_fetch_trajectory(&self, job_id: &str) -> Result<Vec<TrajectoryStep>, AiomeError>;
    async fn do_store_diagnosis(
        &self,
        job_id: &str,
        diagnosis: AgentDiagnosis,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_diagnosis(&self, job_id: &str) -> Result<Option<AgentDiagnosis>, AiomeError>;
}

#[async_trait]
impl TrajectoryStore for UniversalJobQueue {
    async fn record_step(&self, job_id: &str, step: TrajectoryStep) -> Result<(), AiomeError> {
        self.do_record_step(job_id, step).await
    }

    async fn fetch_trajectory(&self, job_id: &str) -> Result<Vec<TrajectoryStep>, AiomeError> {
        self.do_fetch_trajectory(job_id).await
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
impl TrajectoryOps for UniversalJobQueue {
    async fn do_record_step(&self, job_id: &str, step: TrajectoryStep) -> Result<(), AiomeError> {
        let input_json = serde_json::to_string(&step.input).unwrap_or_else(|_| "{}".to_string());
        let output_json = serde_json::to_string(&step.output).unwrap_or_else(|_| "{}".to_string());
        let violations_json =
            serde_json::to_string(&step.constraint_violations).unwrap_or_else(|_| "[]".to_string());
        let failure_cat = step.failure_category.map(|c| c.to_string());
        let is_critical = if step.is_critical_failure { 1 } else { 0 };

        let step_cat = serde_json::to_string(&step.step_category)
            .unwrap_or_else(|_| "\"General\"".to_string());

        let q = format!("INSERT INTO trajectory_steps (job_id, step_id, action, tool_name, input_json, output_json, timestamp, constraint_violations, is_critical_failure, failure_category, reasoning, parent_step_id, step_category) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, {10}, {11}, {12})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6), self.pool.ph(7), self.pool.ph(8), self.pool.ph(9), self.pool.ph(10), self.pool.ph(11), self.pool.ph(12));
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
            &step.parent_step_id,
            &step_cat
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to record trajectory step: {}", e),
        })?;
        Ok(())
    }

    async fn do_fetch_trajectory(&self, job_id: &str) -> Result<Vec<TrajectoryStep>, AiomeError> {
        let q = format!("SELECT step_id, action, tool_name, input_json, output_json, timestamp, constraint_violations, is_critical_failure, failure_category, reasoning, parent_step_id, step_category FROM trajectory_steps WHERE job_id = {} ORDER BY step_id ASC", self.pool.ph(0));
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
                        job_id: Some(job_id.to_string()),
                        action: row.get("action"),
                        tool_name: row.get("tool_name"),
                        input: serde_json::from_str(&input_str).unwrap_or(serde_json::Value::Null),
                        output: serde_json::from_str(&output_str)
                            .unwrap_or(serde_json::Value::Null),
                        timestamp: row.get("timestamp"),
                        constraint_violations: serde_json::from_str(&violations_str)
                            .unwrap_or_default(),
                        is_critical_failure: row.get::<i64, _>("is_critical_failure") != 0,
                        failure_category: failure_cat_str.and_then(|s| s.parse().ok()),
                        reasoning: row.get("reasoning"),
                        parent_step_id: row.get("parent_step_id"),
                        step_category: serde_json::from_str(&row.get::<String, _>("step_category"))
                            .unwrap_or_default(),
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
                        job_id: Some(job_id.to_string()),
                        action: row.get("action"),
                        tool_name: row.get("tool_name"),
                        input: serde_json::from_str(&input_str).unwrap_or(serde_json::Value::Null),
                        output: serde_json::from_str(&output_str)
                            .unwrap_or(serde_json::Value::Null),
                        timestamp: row.get("timestamp"),
                        constraint_violations: serde_json::from_str(&violations_str)
                            .unwrap_or_default(),
                        is_critical_failure: row.get::<i32, _>("is_critical_failure") != 0,
                        failure_category: failure_cat_str.and_then(|s| s.parse().ok()),
                        reasoning: row.get("reasoning"),
                        parent_step_id: row.get("parent_step_id"),
                        step_category: serde_json::from_str(&row.get::<String, _>("step_category"))
                            .unwrap_or_default(),
                    });
                }
            }
        }
        Ok(steps)
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
