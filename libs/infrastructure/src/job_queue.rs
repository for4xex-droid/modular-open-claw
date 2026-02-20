use async_trait::async_trait;
use factory_core::traits::{Job, JobQueue, JobStatus};
use factory_core::error::FactoryError;
use sqlx::{SqlitePool, Row};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::time::Duration;
use uuid::Uuid;
use chrono::Utc;

/// Job Queue that utilizes SQLite in WAL Mode to allow multi-threaded queue operations.
/// Prevents concurrent access database locking via busy_timeout.
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

    async fn init_db(&self) -> Result<(), FactoryError> {
        sqlx::query("DROP TABLE IF EXISTS jobs;")
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to drop old jobs table: {}", e) })?;

        sqlx::query(
            "CREATE TABLE jobs (
                id TEXT PRIMARY KEY,
                topic TEXT NOT NULL,
                style TEXT NOT NULL,
                karma_directives TEXT,
                status TEXT NOT NULL,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to create jobs table: {}", e) })?;

        sqlx::query("DROP TABLE IF EXISTS karma_logs;")
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to drop old karma_logs table: {}", e) })?;

        sqlx::query(
            "CREATE TABLE karma_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id TEXT NOT NULL,
                skill_id TEXT NOT NULL,
                lesson TEXT NOT NULL,
                is_success BOOLEAN NOT NULL,
                human_rating INTEGER,
                weight INTEGER DEFAULT 100,
                created_at TEXT NOT NULL
            );"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to create karma_logs table: {}", e) })?;

        Ok(())
    }
}

#[async_trait]
impl JobQueue for SqliteJobQueue {
    async fn enqueue(&self, topic: &str, style: &str, karma_directives: Option<&str>) -> Result<String, FactoryError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO jobs (id, topic, style, karma_directives, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(topic)
        .bind(style)
        .bind(karma_directives)
        .bind(JobStatus::Pending.to_string())
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to enqueue job: {}", e) })?;

        Ok(id)
    }

    async fn dequeue(&self) -> Result<Option<Job>, FactoryError> {
        // We use a transaction to safely mark a job as Processing
        let mut tx = self.pool.begin().await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to start transaction: {}", e) })?;

        let row = sqlx::query(
            "SELECT id, topic, style, karma_directives, status, error_message FROM jobs WHERE status = ? ORDER BY created_at ASC LIMIT 1"
        )
        .bind(JobStatus::Pending.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch pending job: {}", e) })?;

        if let Some(r) = row {
            let id: String = r.get("id");
            let topic: String = r.get("topic");
            let style: String = r.get("style");
            let karma_directives: Option<String> = try_get_optional_string(&r, "karma_directives");
            let status_str: String = r.get("status");
            let error_message: Option<String> = try_get_optional_string(&r, "error_message");

            let now = Utc::now().to_rfc3339();
            sqlx::query("UPDATE jobs SET status = ?, updated_at = ? WHERE id = ?")
                .bind(JobStatus::Processing.to_string())
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
                error_message, // usually None at this stage
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
        // RAG-Driven Karma Injection: Fetch karma relevant to the specific skill_id OR matching topic keywords.
        // We do a simple LIKE query for local SQLite RAG emulation. weight > 0 ensures we omit "decayed" karma.
        let topic_pattern = format!("%{}%", topic);

        let rows = sqlx::query("SELECT lesson FROM karma_logs WHERE weight > 0 AND (skill_id = ? OR lesson LIKE ?) ORDER BY weight DESC, created_at DESC LIMIT ?")
            .bind(skill_id)
            .bind(&topic_pattern)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch relevant karma: {}", e) })?;

        let mut karma = Vec::new();
        for row in rows {
            let lesson: String = row.get("lesson");
            karma.push(lesson);
        }
        Ok(karma)
    }

    async fn store_karma(&self, job_id: &str, skill_id: &str, lesson: &str, is_success: bool, human_rating: Option<i32>) -> Result<(), FactoryError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("INSERT INTO karma_logs (job_id, skill_id, lesson, is_success, human_rating, created_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(job_id)
            .bind(skill_id)
            .bind(lesson)
            .bind(is_success)
            .bind(human_rating)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to store karma for job {}: {}", job_id, e) })?;
        Ok(())
    }
}

// Helper function because `get` on Option panics if type is unexpected, 
// using try_get is safer if column can be NULL.
fn try_get_optional_string(row: &sqlx::sqlite::SqliteRow, col: &str) -> Option<String> {
    row.try_get(col).ok()
}
