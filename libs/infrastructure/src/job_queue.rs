use async_trait::async_trait;
use factory_core::traits::{Job, JobQueue, JobStatus};
use factory_core::error::FactoryError;
use sqlx::{SqlitePool, Row};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::time::Duration;
use uuid::Uuid;
use chrono::Utc;

/// Job Queue that utilizes SQLite in WAL Mode to allow multi-threaded queue operations.
/// Implements **The Immortal Samsara Schema** — crash-resistant, self-healing, and eternal.
#[derive(Clone)]
pub struct SqliteJobQueue {
    pool: SqlitePool,
}

impl SqliteJobQueue {
    /// Connects to the SQLite database and initializes the WAL mode and schema.
    pub async fn new(db_path: &str) -> Result<Self, FactoryError> {
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_millis(5000));

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to connect to SQLite: {}", e) })?;

        let queue = Self { pool };
        queue.init_db().await?;
        Ok(queue)
    }

    /// The Immortal Samsara Schema (完全不可侵DDL)
    /// 
    /// Guardrails implemented at the DB level:
    /// - `CHECK(json_valid(karma_directives))`: Native JSON validation (罠3 防衛)
    /// - `started_at`: Zombie Process detection (The Zombie Hunter)
    /// - `ON DELETE SET NULL`: Eternal Karma — jobs die, lessons live (The Memory Wipe Trap 防衛)
    /// - `CHECK(weight BETWEEN 0 AND 100)`: Bounded Confidence (The Karma Singularity 防衛)
    /// - `last_applied_at`: Usage tracking for TTL decay (The Static Decay Trap 防衛)
    async fn init_db(&self) -> Result<(), FactoryError> {
        // Use CREATE TABLE IF NOT EXISTS to prevent data loss on restart.
        // The old DROP TABLE approach is replaced for production safety.
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY, 
                topic TEXT NOT NULL,
                style_name TEXT NOT NULL, 
                karma_directives TEXT NOT NULL CHECK(json_valid(karma_directives)), 
                status TEXT NOT NULL CHECK(status IN ('Pending', 'Processing', 'Completed', 'Failed')),
                started_at TEXT, 
                tech_karma_extracted INTEGER NOT NULL DEFAULT 0, 
                creative_rating INTEGER CHECK(creative_rating IN (-1, 0, 1)), 
                error_message TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to create jobs table: {}", e) })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS karma_logs (
                id TEXT PRIMARY KEY,
                job_id TEXT, 
                karma_type TEXT NOT NULL CHECK(karma_type IN ('Technical', 'Creative', 'Synthesized')),
                related_skill TEXT NOT NULL, 
                lesson TEXT NOT NULL,        
                weight INTEGER NOT NULL DEFAULT 100 CHECK(weight BETWEEN 0 AND 100), 
                last_applied_at TEXT DEFAULT (datetime('now')),
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY(job_id) REFERENCES jobs(id) ON DELETE SET NULL
            );"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to create karma_logs table: {}", e) })?;

        // Indices for optimal performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_jobs_status_started ON jobs(status, started_at);")
            .execute(&self.pool).await.ok();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_karma_logs_skill_weight ON karma_logs(related_skill, weight DESC);")
            .execute(&self.pool).await.ok();

        Ok(())
    }
}

#[async_trait]
impl JobQueue for SqliteJobQueue {
    async fn enqueue(&self, topic: &str, style: &str, karma_directives: Option<&str>) -> Result<String, FactoryError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        // Default to empty JSON object if None, satisfying CHECK(json_valid(...))
        let directives = karma_directives.unwrap_or("{}");

        sqlx::query(
            "INSERT INTO jobs (id, topic, style_name, karma_directives, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(topic)
        .bind(style)
        .bind(directives)
        .bind(JobStatus::Pending.to_string())
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to enqueue job: {}", e) })?;

        Ok(id)
    }

    async fn dequeue(&self) -> Result<Option<Job>, FactoryError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to start transaction: {}", e) })?;

        let row = sqlx::query(
            "SELECT id, topic, style_name, karma_directives, status, started_at, tech_karma_extracted, creative_rating, error_message FROM jobs WHERE status = ? ORDER BY created_at ASC LIMIT 1"
        )
        .bind(JobStatus::Pending.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch pending job: {}", e) })?;

        if let Some(r) = row {
            let id: String = r.get("id");
            let topic: String = r.get("topic");
            let style: String = r.get("style_name");
            let karma_directives: Option<String> = try_get_optional_string(&r, "karma_directives");
            let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
            let creative_rating: Option<i32> = r.try_get("creative_rating").ok();
            let error_message: Option<String> = try_get_optional_string(&r, "error_message");

            let now = Utc::now().to_rfc3339();
            // Set status to Processing AND record the started_at timestamp for Zombie Hunter
            sqlx::query("UPDATE jobs SET status = ?, started_at = ?, updated_at = ? WHERE id = ?")
                .bind(JobStatus::Processing.to_string())
                .bind(&now)
                .bind(&now)
                .bind(&id)
                .execute(&mut *tx)
                .await
                .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to update job status: {}", e) })?;

            tx.commit().await
                .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to commit transaction: {}", e) })?;

            Ok(Some(Job {
                id,
                topic,
                style,
                karma_directives,
                status: JobStatus::Processing,
                started_at: Some(now),
                tech_karma_extracted: tech_karma_extracted != 0,
                creative_rating,
                error_message,
            }))
        } else {
            Ok(None)
        }
    }

    async fn complete_job(&self, job_id: &str) -> Result<(), FactoryError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET status = ?, updated_at = ? WHERE id = ?")
            .bind(JobStatus::Completed.to_string())
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to complete job {}: {}", job_id, e) })?;
        Ok(())
    }

    async fn fail_job(&self, job_id: &str, reason: &str) -> Result<(), FactoryError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET status = ?, error_message = ?, updated_at = ? WHERE id = ?")
            .bind(JobStatus::Failed.to_string())
            .bind(reason)
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fail job {}: {}", job_id, e) })?;
        Ok(())
    }

    async fn fetch_relevant_karma(&self, topic: &str, skill_id: &str, limit: i64) -> Result<Vec<String>, FactoryError> {
        // RAG-Driven Karma Injection:
        // - Fetch karma relevant to the specific skill_id OR matching topic keywords OR 'global' scope.
        // - weight > 0 ensures we omit fully "decayed" karma (The Karma Singularity protection).
        let topic_pattern = format!("%{}%", topic);

        let rows = sqlx::query(
            "SELECT id, lesson FROM karma_logs 
             WHERE weight > 0 AND (related_skill = ? OR related_skill = 'global' OR lesson LIKE ?) 
             ORDER BY weight DESC, created_at DESC LIMIT ?"
        )
        .bind(skill_id)
        .bind(&topic_pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch relevant karma: {}", e) })?;

        let mut karma = Vec::new();
        for row in &rows {
            let lesson: String = row.get("lesson");
            karma.push(lesson);
        }

        // Update last_applied_at for applied karma entries (Usage Tracking for TTL Decay)
        let now = Utc::now().to_rfc3339();
        for row in &rows {
            let karma_id: String = row.get("id");
            let _ = sqlx::query("UPDATE karma_logs SET last_applied_at = ? WHERE id = ?")
                .bind(&now)
                .bind(&karma_id)
                .execute(&self.pool)
                .await;
        }

        Ok(karma)
    }

    async fn store_karma(&self, job_id: &str, skill_id: &str, lesson: &str, karma_type: &str) -> Result<(), FactoryError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, created_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(job_id)
        .bind(karma_type)
        .bind(skill_id)
        .bind(lesson)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to store karma for job {}: {}", job_id, e) })?;
        Ok(())
    }

    /// The Zombie Hunter: Reclaims jobs stuck in 'Processing' for longer than `timeout_hours`.
    /// This prevents "eternal ghost jobs" after a crash, power outage, or OS restart.
    async fn reclaim_zombie_jobs(&self, timeout_hours: i64) -> Result<u64, FactoryError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE jobs SET status = 'Failed', error_message = 'Zombie reclaimed: exceeded processing timeout', updated_at = ? 
             WHERE status = 'Processing' AND started_at IS NOT NULL 
             AND (julianday('now') - julianday(started_at)) * 24 > ?"
        )
        .bind(&now)
        .bind(timeout_hours)
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to reclaim zombie jobs: {}", e) })?;

        let count = result.rows_affected();
        if count > 0 {
            tracing::warn!("🧟 Zombie Hunter: Reclaimed {} ghost job(s)", count);
        }
        Ok(count)
    }

    /// Sets the creative rating for a completed job (Human-in-the-Loop, Asynchronous Karma).
    /// This is decoupled from the Samsara cycle to prevent deadlock (罠1 防衛).
    async fn set_creative_rating(&self, job_id: &str, rating: i32) -> Result<(), FactoryError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET creative_rating = ?, updated_at = ? WHERE id = ?")
            .bind(rating)
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to set creative rating for job {}: {}", job_id, e) })?;
        Ok(())
    }
}

// Helper function because `get` on Option panics if type is unexpected, 
// using try_get is safer if column can be NULL.
fn try_get_optional_string(row: &sqlx::sqlite::SqliteRow, col: &str) -> Option<String> {
    row.try_get(col).ok()
}
