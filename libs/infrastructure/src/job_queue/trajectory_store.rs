/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::SqliteJobQueue;
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
impl TrajectoryStore for SqliteJobQueue {
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
impl TrajectoryOps for SqliteJobQueue {
    async fn do_record_step(&self, job_id: &str, step: TrajectoryStep) -> Result<(), AiomeError> {
        let input_json = serde_json::to_string(&step.input).unwrap_or_else(|_| "{}".to_string());
        let output_json = serde_json::to_string(&step.output).unwrap_or_else(|_| "{}".to_string());
        let violations_json =
            serde_json::to_string(&step.constraint_violations).unwrap_or_else(|_| "[]".to_string());
        let failure_cat = step.failure_category.map(|c| c.to_string());
        let is_critical = if step.is_critical_failure { 1 } else { 0 };

        sqlx::query(
            "INSERT INTO trajectory_steps (job_id, step_id, action, tool_name, input_json, output_json, timestamp, constraint_violations, is_critical_failure, failure_category)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(job_id)
        .bind(step.step_id as i64)
        .bind(step.action)
        .bind(step.tool_name)
        .bind(input_json)
        .bind(output_json)
        .bind(step.timestamp)
        .bind(violations_json)
        .bind(is_critical)
        .bind(failure_cat)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to record trajectory step: {}", e) })?;

        Ok(())
    }

    async fn do_fetch_trajectory(&self, job_id: &str) -> Result<Vec<TrajectoryStep>, AiomeError> {
        let rows = sqlx::query("SELECT step_id, action, tool_name, input_json, output_json, timestamp, constraint_violations, is_critical_failure, failure_category FROM trajectory_steps WHERE job_id = ? ORDER BY step_id ASC")
            .bind(job_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch trajectory: {}", e) })?;

        let mut steps = Vec::new();
        for row in rows {
            let input_json: String = row.get(2);
            let output_json: String = row.get(3); // Wait, index might be wrong if I use get by index. Let's use name.

            // Re-fetch using names to be safe
            let step_id: i64 = row.get("step_id");
            let action: String = row.get("action");
            let tool_name: Option<String> = row.get("tool_name");
            let input_str: String = row.get("input_json");
            let output_str: String = row.get("output_json");
            let timestamp: String = row.get("timestamp");
            let violations_str: String = row.get("constraint_violations");
            let is_critical: i64 = row.get("is_critical_failure");
            let failure_cat_str: Option<String> = row.get("failure_category");

            let input = serde_json::from_str(&input_str).unwrap_or(serde_json::Value::Null);
            let output = serde_json::from_str(&output_str).unwrap_or(serde_json::Value::Null);
            let constraint_violations = serde_json::from_str(&violations_str).unwrap_or_default();
            let failure_category = failure_cat_str.and_then(|s| s.parse().ok());

            steps.push(TrajectoryStep {
                step_id: step_id as u32,
                action,
                tool_name,
                input,
                output,
                timestamp,
                constraint_violations,
                is_critical_failure: is_critical != 0,
                failure_category,
            });
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

        sqlx::query(
            "INSERT OR REPLACE INTO agent_diagnoses (job_id, critical_failure_step, failure_category, root_cause, evidence, self_repair_hint, diagnosed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(job_id)
        .bind(diagnosis.critical_failure_step as i64)
        .bind(category_str)
        .bind(diagnosis.root_cause)
        .bind(evidence_json)
        .bind(diagnosis.self_repair_hint)
        .bind(diagnosis.diagnosed_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to store agent diagnosis: {}", e) })?;

        Ok(())
    }

    async fn do_fetch_diagnosis(&self, job_id: &str) -> Result<Option<AgentDiagnosis>, AiomeError> {
        let row = sqlx::query("SELECT critical_failure_step, failure_category, root_cause, evidence, self_repair_hint, diagnosed_at FROM agent_diagnoses WHERE job_id = ?")
            .bind(job_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch agent diagnosis: {}", e) })?;

        if let Some(row) = row {
            let step_id: i64 = row.get("critical_failure_step");
            let cat_str: String = row.get("failure_category");
            let root_cause: String = row.get("root_cause");
            let evidence_str: String = row.get("evidence");
            let hint: String = row.get("self_repair_hint");
            let diagn_at: String = row.get("diagnosed_at");

            let category = cat_str.parse().unwrap_or(FailureCategory::SystemFailure);
            let evidence = serde_json::from_str(&evidence_str).unwrap_or_default();

            Ok(Some(AgentDiagnosis {
                critical_failure_step: step_id as u32,
                category,
                root_cause,
                evidence,
                self_repair_hint: hint,
                diagnosed_at: diagn_at,
            }))
        } else {
            Ok(None)
        }
    }
}
