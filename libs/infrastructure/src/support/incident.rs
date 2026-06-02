/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::db::DatabasePool;
use aiome_core::error::AiomeError;
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SupportIncidentRecord {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub user_hash: String,
    pub channel_id: Option<String>,
    pub system_context: Option<String>,
    pub suggested_fix: Option<String>,
    pub related_diagnosis_id: Option<i32>,
    pub status: String,
    pub resolved_at: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct SupportWeeklyStats {
    pub total_incidents_7d: i64,
    pub distinct_users: i64,
    pub unresolved: i64,
    pub top_severity: Option<String>,
}

#[derive(Clone)]
pub struct SupportIncidentRepository {
    pool: DatabasePool,
}

impl SupportIncidentRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    pub async fn insert_incident(
        &self,
        title: &str,
        description: &str,
        severity: &str,
        user_hash: &str,
        channel_id: Option<&str>,
        system_context: Option<&str>,
        suggested_fix: Option<&str>,
        related_diagnosis_id: Option<i32>,
    ) -> Result<String, AiomeError> {
        let id = Uuid::new_v4().to_string();
        let query = "INSERT INTO support_incidents (id, title, description, severity, user_hash, channel_id, system_context, suggested_fix, related_diagnosis_id, status) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'Open')";

        match &self.pool {
            DatabasePool::Sqlite(p) => {
                sqlx::query(query)
                    .bind(&id)
                    .bind(title)
                    .bind(description)
                    .bind(severity)
                    .bind(user_hash)
                    .bind(channel_id)
                    .bind(system_context)
                    .bind(suggested_fix)
                    .bind(related_diagnosis_id)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to insert support incident: {}", e),
                    })?;
            }
            DatabasePool::Postgres(p) => {
                sqlx::query(query)
                    .bind(&id)
                    .bind(title)
                    .bind(description)
                    .bind(severity)
                    .bind(user_hash)
                    .bind(channel_id)
                    .bind(system_context)
                    .bind(suggested_fix)
                    .bind(related_diagnosis_id)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to insert support incident: {}", e),
                    })?;
            }
        }

        Ok(id)
    }

    pub async fn fetch_incident(
        &self,
        id: &str,
    ) -> Result<Option<SupportIncidentRecord>, AiomeError> {
        let query = "SELECT * FROM support_incidents WHERE id = $1";
        let row_opt = match &self.pool {
            DatabasePool::Sqlite(p) => sqlx::query(query)
                .bind(id)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?
                .map(|r| self.map_row_sqlite(r)),
            DatabasePool::Postgres(p) => sqlx::query(query)
                .bind(id)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?
                .map(|r| self.map_row_postgres(r)),
        };

        match row_opt {
            Some(res) => Ok(Some(res?)),
            None => Ok(None),
        }
    }

    pub async fn fetch_open_incidents(
        &self,
        limit: i64,
    ) -> Result<Vec<SupportIncidentRecord>, AiomeError> {
        let query =
            "SELECT * FROM support_incidents WHERE status = 'Open' ORDER BY created_at ASC LIMIT $1";
        let mut incidents = Vec::new();
        match &self.pool {
            DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(query)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    incidents.push(self.map_row_sqlite(r)?);
                }
            }
            DatabasePool::Postgres(p) => {
                let rows = sqlx::query(query)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    incidents.push(self.map_row_postgres(r)?);
                }
            }
        }
        Ok(incidents)
    }

    pub async fn update_status(&self, id: &str, status: &str) -> Result<(), AiomeError> {
        let query = if status == "Resolved" {
            "UPDATE support_incidents SET status = $1, resolved_at = datetime('now'), updated_at = CURRENT_TIMESTAMP WHERE id = $2"
        } else {
            "UPDATE support_incidents SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2"
        };
        let query_pg = if status == "Resolved" {
            "UPDATE support_incidents SET status = $1, resolved_at = TO_CHAR(NOW(), 'YYYY-MM-DD HH24:MI:SS'), updated_at = CURRENT_TIMESTAMP WHERE id = $2"
        } else {
            "UPDATE support_incidents SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2"
        };

        match &self.pool {
            DatabasePool::Sqlite(p) => {
                sqlx::query(query)
                    .bind(status)
                    .bind(id)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
            DatabasePool::Postgres(p) => {
                sqlx::query(query_pg)
                    .bind(status)
                    .bind(id)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
        }
        Ok(())
    }

    pub async fn update_suggested_fix(
        &self,
        id: &str,
        suggested_fix: &str,
    ) -> Result<(), AiomeError> {
        let query = "UPDATE support_incidents SET suggested_fix = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2";
        match &self.pool {
            DatabasePool::Sqlite(p) => {
                sqlx::query(query)
                    .bind(suggested_fix)
                    .bind(id)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
            DatabasePool::Postgres(p) => {
                sqlx::query(query)
                    .bind(suggested_fix)
                    .bind(id)
                    .execute(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
            }
        }
        Ok(())
    }

    pub async fn compute_weekly_stats(&self) -> Result<SupportWeeklyStats, AiomeError> {
        let (total, distinct, unresolved, top_severity) = match &self.pool {
            DatabasePool::Sqlite(p) => {
                let row = sqlx::query(
                    r#"
                    SELECT
                        (SELECT COUNT(*) FROM support_incidents WHERE created_at >= datetime('now', '-7 days')) as total,
                        (SELECT COUNT(DISTINCT user_hash) FROM support_incidents WHERE created_at >= datetime('now', '-7 days')) as distinct_users,
                        (SELECT COUNT(*) FROM support_incidents WHERE status IN ('Open', 'InProgress', 'Escalated')) as unresolved,
                        (SELECT severity FROM support_incidents WHERE created_at >= datetime('now', '-7 days') GROUP BY severity ORDER BY COUNT(*) DESC LIMIT 1) as top_severity
                    "#
                )
                .fetch_one(p)
                .await
                .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

                (
                    row.try_get::<i64, _>("total").unwrap_or(0),
                    row.try_get::<i64, _>("distinct_users").unwrap_or(0),
                    row.try_get::<i64, _>("unresolved").unwrap_or(0),
                    row.try_get::<Option<String>, _>("top_severity")
                        .unwrap_or(None),
                )
            }
            DatabasePool::Postgres(p) => {
                let row = sqlx::query(
                    r#"
                    SELECT
                        (SELECT COUNT(*) FROM support_incidents WHERE created_at >= NOW() - INTERVAL '7 days') as total,
                        (SELECT COUNT(DISTINCT user_hash) FROM support_incidents WHERE created_at >= NOW() - INTERVAL '7 days') as distinct_users,
                        (SELECT COUNT(*) FROM support_incidents WHERE status IN ('Open', 'InProgress', 'Escalated')) as unresolved,
                        (SELECT severity FROM support_incidents WHERE created_at >= NOW() - INTERVAL '7 days' GROUP BY severity ORDER BY COUNT(*) DESC LIMIT 1) as top_severity
                    "#
                )
                .fetch_one(p)
                .await
                .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

                (
                    row.try_get::<i64, _>("total").unwrap_or(0),
                    row.try_get::<i64, _>("distinct_users").unwrap_or(0),
                    row.try_get::<i64, _>("unresolved").unwrap_or(0),
                    row.try_get::<Option<String>, _>("top_severity")
                        .unwrap_or(None),
                )
            }
        };

        Ok(SupportWeeklyStats {
            total_incidents_7d: total,
            distinct_users: distinct,
            unresolved,
            top_severity,
        })
    }

    fn map_row_sqlite(
        &self,
        row: sqlx::sqlite::SqliteRow,
    ) -> Result<SupportIncidentRecord, AiomeError> {
        Ok(SupportIncidentRecord {
            id: row.try_get("id").unwrap_or_default(),
            title: row.try_get("title").unwrap_or_default(),
            description: row.try_get("description").unwrap_or_default(),
            severity: row.try_get("severity").unwrap_or_default(),
            user_hash: row.try_get("user_hash").unwrap_or_default(),
            channel_id: row.try_get("channel_id").ok(),
            system_context: row.try_get("system_context").ok(),
            suggested_fix: row.try_get("suggested_fix").ok(),
            related_diagnosis_id: row.try_get("related_diagnosis_id").ok(),
            status: row.try_get("status").unwrap_or_else(|_| "Open".to_string()),
            resolved_at: row.try_get("resolved_at").ok(),
            created_at: row
                .try_get("created_at")
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: row
                .try_get("updated_at")
                .unwrap_or_else(|_| chrono::Utc::now()),
        })
    }

    fn map_row_postgres(
        &self,
        row: sqlx::postgres::PgRow,
    ) -> Result<SupportIncidentRecord, AiomeError> {
        Ok(SupportIncidentRecord {
            id: row.try_get("id").unwrap_or_default(),
            title: row.try_get("title").unwrap_or_default(),
            description: row.try_get("description").unwrap_or_default(),
            severity: row.try_get("severity").unwrap_or_default(),
            user_hash: row.try_get("user_hash").unwrap_or_default(),
            channel_id: row.try_get("channel_id").ok(),
            system_context: row.try_get("system_context").ok(),
            suggested_fix: row.try_get("suggested_fix").ok(),
            related_diagnosis_id: row.try_get("related_diagnosis_id").ok(),
            status: row.try_get("status").unwrap_or_else(|_| "Open".to_string()),
            resolved_at: row.try_get("resolved_at").ok(),
            created_at: row
                .try_get("created_at")
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: row
                .try_get("updated_at")
                .unwrap_or_else(|_| chrono::Utc::now()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> DatabasePool {
        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();

        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let _jq = crate::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
            .await
            .expect("Failed to create in-memory job queue");

        pool
    }

    #[tokio::test]
    async fn test_support_insert_and_fetch() -> Result<(), AiomeError> {
        let pool = setup_db().await;
        let repo = SupportIncidentRepository::new(pool);

        let id = repo
            .insert_incident(
                "API Failures",
                "Database deadlock on user creations",
                "High",
                "user_hash_123",
                Some("discord_channel_abc"),
                Some("context_data"),
                Some("Check pg_locks table"),
                None,
            )
            .await?;

        assert!(!id.is_empty());

        let fetched = repo.fetch_incident(&id).await?.unwrap();
        assert_eq!(fetched.title, "API Failures");
        assert_eq!(fetched.severity, "High");
        assert_eq!(fetched.user_hash, "user_hash_123");
        assert_eq!(fetched.status, "Open");

        Ok(())
    }

    #[tokio::test]
    async fn test_support_weekly_stats() -> Result<(), AiomeError> {
        let pool = setup_db().await;
        let repo = SupportIncidentRepository::new(pool);

        let _id1 = repo
            .insert_incident("A", "desc A", "Low", "u1", None, None, None, None)
            .await?;
        let _id2 = repo
            .insert_incident("B", "desc B", "Critical", "u1", None, None, None, None)
            .await?;
        let _id3 = repo
            .insert_incident("C", "desc C", "High", "u2", None, None, None, None)
            .await?;

        let stats = repo.compute_weekly_stats().await?;
        assert_eq!(stats.total_incidents_7d, 3);
        assert_eq!(stats.distinct_users, 2);
        assert_eq!(stats.unresolved, 3);

        Ok(())
    }

    #[tokio::test]
    async fn test_support_update_status_and_fix() -> Result<(), AiomeError> {
        let pool = setup_db().await;
        let repo = SupportIncidentRepository::new(pool);

        let id = repo
            .insert_incident("A", "desc A", "Medium", "u1", None, None, None, None)
            .await?;

        repo.update_suggested_fix(&id, "Restart the pod").await?;
        repo.update_status(&id, "Resolved").await?;

        let fetched = repo.fetch_incident(&id).await?.unwrap();
        assert_eq!(fetched.suggested_fix.as_deref(), Some("Restart the pod"));
        assert_eq!(fetched.status, "Resolved");
        assert!(fetched.resolved_at.is_some());

        Ok(())
    }
}
