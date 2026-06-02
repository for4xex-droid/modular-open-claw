/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::db::DatabasePool;
use aiome_core::error::AiomeError;
use aiome_core_contracts::traits::KarmaRegistry;
use sqlx::Row;
use std::sync::Arc;

#[derive(Clone)]
pub struct SupportFeedbackCollector {
    pool: DatabasePool,
    karma_registry: Arc<dyn KarmaRegistry>,
}

impl SupportFeedbackCollector {
    pub fn new(pool: DatabasePool, karma_registry: Arc<dyn KarmaRegistry>) -> Self {
        Self {
            pool,
            karma_registry,
        }
    }

    pub async fn handle_feedback(
        &self,
        incident_id: &str,
        resolved: bool,
    ) -> Result<(), AiomeError> {
        // 1. Update support incident status and resolved_at timestamp
        let status = if resolved { "Resolved" } else { "Escalated" };
        let query_update_sqlite = if resolved {
            "UPDATE support_incidents SET status = $1, resolved_at = datetime('now'), updated_at = CURRENT_TIMESTAMP WHERE id = $2"
        } else {
            "UPDATE support_incidents SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2"
        };
        let query_update_pg = if resolved {
            "UPDATE support_incidents SET status = $1, resolved_at = TO_CHAR(NOW(), 'YYYY-MM-DD HH24:MI:SS'), updated_at = CURRENT_TIMESTAMP WHERE id = $2"
        } else {
            "UPDATE support_incidents SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2"
        };

        match &self.pool {
            DatabasePool::Sqlite(p) => {
                sqlx::query(query_update_sqlite)
                    .bind(status)
                    .bind(incident_id)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to update incident feedback: {}", e),
                    })?;
            }
            DatabasePool::Postgres(p) => {
                sqlx::query(query_update_pg)
                    .bind(status)
                    .bind(incident_id)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to update incident feedback: {}", e),
                    })?;
            }
        }

        // 2. Resolve karma log ID through agent_diagnoses
        let query_find_karma = r#"
            SELECT kl.id as karma_id
            FROM support_incidents si
            JOIN agent_diagnoses ad ON si.related_diagnosis_id = ad.id
            JOIN karma_logs kl ON ad.job_id = kl.job_id
            WHERE si.id = $1
            LIMIT 1
        "#;

        let karma_id_opt = match &self.pool {
            DatabasePool::Sqlite(p) => sqlx::query(query_find_karma)
                .bind(incident_id)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to query associated karma: {}", e),
                })?
                .map(|row| row.get::<String, _>("karma_id")),
            DatabasePool::Postgres(p) => sqlx::query(query_find_karma)
                .bind(incident_id)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to query associated karma: {}", e),
                })?
                .map(|row| row.get::<String, _>("karma_id")),
        };

        // 3. Adjust Karma weight if associated karma is found
        if let Some(karma_id) = karma_id_opt {
            let delta = if resolved { 10 } else { -15 };
            self.karma_registry
                .adjust_karma_weight(&karma_id, delta)
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DatabasePool;
    use aiome_core_contracts::traits::KarmaRegistry as _;
    use uuid::Uuid;

    async fn setup_db_and_data() -> (DatabasePool, String, String) {
        let pool = DatabasePool::new_sqlite("sqlite::memory:").await.unwrap();

        // UniversalJobQueueの初期化（マイグレーション実行）
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let _jq = crate::job_queue::UniversalJobQueue::new(pool.clone(), None, ts.clone())
            .await
            .expect("Failed to create in-memory job queue");

        let job_id = Uuid::new_v4().to_string();
        let karma_id = Uuid::new_v4().to_string();

        // 1. Insert dummy job
        let q_job = "INSERT INTO jobs (id, category, topic, style_name, karma_directives, status) VALUES ($1, 'cat', 'topic', 'style', '{}', 'Failed')";
        sqlx::query(q_job)
            .bind(&job_id)
            .execute(pool.get_sqlite_pool_or_err().unwrap())
            .await
            .unwrap();

        // 2. Insert associated karma log with initial weight = 80 to satisfy weight CHECK constraint [0, 100] when boosted
        let q_karma = "INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, weight) VALUES ($1, $2, 'Technical', 'test_skill', 'Always test before deploy', 80)";
        sqlx::query(q_karma)
            .bind(&karma_id)
            .bind(&job_id)
            .execute(pool.get_sqlite_pool_or_err().unwrap())
            .await
            .unwrap();

        // 3. Insert agent diagnosis
        let q_diag = "INSERT INTO agent_diagnoses (id, job_id, critical_failure_step, failure_category, root_cause, evidence, self_repair_hint, diagnosed_at) VALUES (42, $1, 1, 'Logic', 'typo', 'log', 'fix typo', 'now')";
        sqlx::query(q_diag)
            .bind(&job_id)
            .execute(pool.get_sqlite_pool_or_err().unwrap())
            .await
            .unwrap();

        // 4. Insert support incident associated with diagnosis ID 42
        let incident_id = Uuid::new_v4().to_string();
        let q_incident = "INSERT INTO support_incidents (id, title, description, user_hash, related_diagnosis_id, status) VALUES ($1, 'Panic in api', 'Server returns 500', 'u1', 42, 'Open')";
        sqlx::query(q_incident)
            .bind(&incident_id)
            .execute(pool.get_sqlite_pool_or_err().unwrap())
            .await
            .unwrap();

        (pool, incident_id, karma_id)
    }

    #[tokio::test]
    async fn test_feedback_resolved_boosts_karma() -> Result<(), AiomeError> {
        let (pool, incident_id, karma_id) = setup_db_and_data().await;

        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = std::sync::Arc::new(
            crate::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(),
        );

        let collector = SupportFeedbackCollector::new(pool.clone(), jq.clone());
        collector.handle_feedback(&incident_id, true).await?;

        // Verify incident status updated to Resolved
        let q_verify_inc = "SELECT status, resolved_at FROM support_incidents WHERE id = $1";
        let row_inc = sqlx::query(q_verify_inc)
            .bind(&incident_id)
            .fetch_one(pool.get_sqlite_pool_or_err().unwrap())
            .await
            .unwrap();
        let status: String = row_inc.get("status");
        let resolved_at: Option<String> = row_inc.get("resolved_at");
        assert_eq!(status, "Resolved");
        assert!(resolved_at.is_some());

        // Verify karma weight was boosted (Initial 80 + 10 = 90)
        let q_verify_karma = "SELECT weight FROM karma_logs WHERE id = $1";
        let row_karma = sqlx::query(q_verify_karma)
            .bind(&karma_id)
            .fetch_one(pool.get_sqlite_pool_or_err().unwrap())
            .await
            .unwrap();
        let weight: i32 = row_karma.get("weight");
        assert_eq!(weight, 90);

        Ok(())
    }

    #[tokio::test]
    async fn test_feedback_unresolved_penalizes_karma() -> Result<(), AiomeError> {
        let (pool, incident_id, karma_id) = setup_db_and_data().await;

        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = std::sync::Arc::new(
            crate::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap(),
        );

        let collector = SupportFeedbackCollector::new(pool.clone(), jq.clone());
        collector.handle_feedback(&incident_id, false).await?;

        // Verify incident status updated to Escalated
        let q_verify_inc = "SELECT status, resolved_at FROM support_incidents WHERE id = $1";
        let row_inc = sqlx::query(q_verify_inc)
            .bind(&incident_id)
            .fetch_one(pool.get_sqlite_pool_or_err().unwrap())
            .await
            .unwrap();
        let status: String = row_inc.get("status");
        assert_eq!(status, "Escalated");

        // Verify karma weight was penalized (Initial 80 - 15 = 65)
        let q_verify_karma = "SELECT weight FROM karma_logs WHERE id = $1";
        let row_karma = sqlx::query(q_verify_karma)
            .bind(&karma_id)
            .fetch_one(pool.get_sqlite_pool_or_err().unwrap())
            .await
            .unwrap();
        let weight: i32 = row_karma.get("weight");
        assert_eq!(weight, 65);

        Ok(())
    }
}
