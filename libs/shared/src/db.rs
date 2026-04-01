/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::error::AiomeError;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Pool, Postgres, Sqlite};
use std::str::FromStr;
use std::time::Duration;

/// Abstracted Database Pool for multi-backend support
#[derive(Debug, Clone)]
pub enum DatabasePool {
    /// SQLite backend pool
    Sqlite(Pool<Sqlite>),
    /// PostgreSQL backend pool
    Postgres(Pool<Postgres>),
}

/// Abstracted Database Transaction for multi-backend support
pub enum DatabaseTransaction<'a> {
    /// SQLite transaction instance
    Sqlite(sqlx::Transaction<'a, sqlx::Sqlite>),
    /// PostgreSQL transaction instance
    Postgres(sqlx::Transaction<'a, sqlx::Postgres>),
}

impl<'a> DatabaseTransaction<'a> {
    /// Commit the active transaction
    pub async fn commit(self) -> Result<(), sqlx::Error> {
        match self {
            Self::Sqlite(tx) => tx.commit().await,
            Self::Postgres(tx) => tx.commit().await,
        }
    }

    /// Rollback the active transaction
    pub async fn rollback(self) -> Result<(), sqlx::Error> {
        match self {
            Self::Sqlite(tx) => tx.rollback().await,
            Self::Postgres(tx) => tx.rollback().await,
        }
    }
}

impl DatabasePool {
    /// Initialize a new SQLite database connection pool
    pub async fn new_sqlite(db_path: &str) -> Result<Self, AiomeError> {
        let options = SqliteConnectOptions::from_str(db_path)
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Invalid sqlite path {}: {}", db_path, e),
            })?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_millis(5000));

        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(options)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to connect to SQLite: {}", e),
            })?;

        Ok(Self::Sqlite(pool))
    }

    /// Initialize a new PostgreSQL database connection pool
    pub async fn new_postgres(url: &str) -> Result<Self, AiomeError> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .acquire_timeout(Duration::from_secs(5))
            .connect(url)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to connect to PostgreSQL: {}", e),
            })?;

        Ok(Self::Postgres(pool))
    }

    /// Returns true if the active database is PostgreSQL
    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    /// Returns true if the active database is SQLite
    pub fn is_sqlite(&self) -> bool {
        matches!(self, Self::Sqlite(_))
    }

    /// Get a reference to the SQLite underlying pool if available
    pub fn get_sqlite_pool(&self) -> Option<&sqlx::Pool<sqlx::Sqlite>> {
        match self {
            Self::Sqlite(p) => Some(p),
            _ => None,
        }
    }

    /// Get a reference to the SQLite underlying pool or return an error if not available
    pub fn get_sqlite_pool_or_err(&self) -> Result<&sqlx::Pool<sqlx::Sqlite>, AiomeError> {
        self.get_sqlite_pool()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "SQLite pool not available (running on Postgres?)".into(),
            })
    }

    /// Get a reference to the PostgreSQL underlying pool if available
    pub fn get_postgres_pool(&self) -> Option<&sqlx::Pool<sqlx::Postgres>> {
        match self {
            Self::Postgres(p) => Some(p),
            _ => None,
        }
    }

    /// Get a reference to the PostgreSQL underlying pool or return an error if not available
    pub fn get_postgres_pool_or_err(&self) -> Result<&sqlx::Pool<sqlx::Postgres>, AiomeError> {
        self.get_postgres_pool()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "PostgreSQL pool not available (running on SQLite?)".into(),
            })
    }

    /// Begin a new generic database transaction
    pub async fn begin(&self) -> Result<DatabaseTransaction<'_>, AiomeError> {
        match self {
            Self::Sqlite(pool) => {
                let tx = pool.begin().await.map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to begin SQLite transaction: {}", e),
                })?;
                Ok(DatabaseTransaction::Sqlite(tx))
            }
            Self::Postgres(pool) => {
                let tx = pool.begin().await.map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to begin PostgreSQL transaction: {}", e),
                })?;
                Ok(DatabaseTransaction::Postgres(tx))
            }
        }
    }

    /// Returns the N-th placeholder for the current database type
    pub fn ph(&self, idx: usize) -> String {
        match self {
            DatabasePool::Sqlite(_) => "?".to_string(),
            DatabasePool::Postgres(_) => format!("${}", idx + 1),
        }
    }

    /// Returns the current timestamp function for the dialect
    pub fn now_fn(&self) -> &str {
        match self {
            Self::Sqlite(_) => "datetime('now')",
            Self::Postgres(_) => "CURRENT_TIMESTAMP",
        }
    }

    /// Returns NOW() + ? days or datetime('now', ? || ' days')
    pub fn now_with_dynamic_days_interval(&self, ph_idx: usize) -> String {
        match self {
            Self::Sqlite(_) => format!("datetime('now', {} || ' days')", self.ph(ph_idx)),
            Self::Postgres(_) => format!("NOW() + ({} * INTERVAL '1 day')", self.ph(ph_idx)),
        }
    }

    /// Returns the literal or expression for interval check
    pub fn interval_minutes_check(&self, col: &str, minutes: i64) -> String {
        match self {
            Self::Sqlite(_) => format!(
                "(julianday('now') - julianday({})) * 24 * 60 > {}",
                col, minutes
            ),
            Self::Postgres(_) => format!("{} < NOW() - INTERVAL '{} minutes'", col, minutes),
        }
    }

    /// Generates UPSERT query
    pub fn upsert_query(
        &self,
        table: &str,
        conflict_col: &str,
        cols: &[&str],
        ph_offset: usize,
    ) -> String {
        let col_names = cols.join(", ");
        let placeholders = cols
            .iter()
            .enumerate()
            .map(|(i, _)| self.ph(i + ph_offset))
            .collect::<Vec<_>>()
            .join(", ");
        match self {
            Self::Sqlite(_) => format!(
                "INSERT OR REPLACE INTO {} ({}) VALUES ({})",
                table, col_names, placeholders
            ),
            Self::Postgres(_) => {
                let updates = cols
                    .iter()
                    .filter(|&&c| c != conflict_col)
                    .map(|c| format!("{} = EXCLUDED.{}", c, c))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO UPDATE SET {}",
                    table, col_names, placeholders, conflict_col, updates
                )
            }
        }
    }

    /// Returns FTS match expression for the dialect
    pub fn fts_match_expr(&self, table: &str, col: &str, ph_idx: usize) -> String {
        match self {
            Self::Sqlite(_) => format!(
                "{} IN (SELECT rowid FROM {} WHERE {} MATCH {})",
                if table == "karma_fts" { "rowid" } else { "id" },
                table,
                col,
                self.ph(ph_idx)
            ),
            Self::Postgres(_) => format!(
                "to_tsvector('japanese', {}) @@ to_tsquery('japanese', {})",
                col,
                self.ph(ph_idx)
            ),
        }
    }

    /// Returns the complex karma weight expression
    pub fn karma_sql_weight_expr(&self, fts_ph_idx: usize) -> String {
        match self {
            Self::Sqlite(_) => format!(
                "(k.weight * 0.1 + (CASE WHEN k.tier = 'HOT' THEN 30.0 WHEN k.tier = 'WARM' THEN 10.0 ELSE 0.0 END) + (CASE WHEN k.rowid IN (SELECT rowid FROM karma_fts WHERE lesson MATCH {}) THEN 50.0 ELSE 0.0 END))",
                self.ph(fts_ph_idx)
            ),
            Self::Postgres(_) => format!(
                "(k.weight * 0.1 + (CASE WHEN k.tier = 'HOT' THEN 30.0 WHEN k.tier = 'WARM' THEN 10.0 ELSE 0.0 END) + (CASE WHEN to_tsvector('japanese', k.lesson) @@ to_tsquery('japanese', {}) THEN 50.0 ELSE 0.0 END))",
                self.ph(fts_ph_idx)
            ),
        }
    }
}

/// Helper macro to execute a query against either SQLite or PostgreSQL
#[macro_export]
macro_rules! sql_exec {
    ($pool:expr, $sql:expr $(, $arg:expr)*) => {{
        match $pool {
            $crate::db::DatabasePool::Sqlite(p) => {
                let res = sqlx::query($sql)
                    $(.bind($arg))*
                    .execute(p)
                    .await;
                res.map(|r| r.rows_affected())
                   .map_err(|e: sqlx::Error| $crate::reexport::AiomeError::Infrastructure { reason: e.to_string() })
            }
            $crate::db::DatabasePool::Postgres(p) => {
                let res = sqlx::query($sql)
                    $(.bind($arg))*
                    .execute(p)
                    .await;
                res.map(|r| r.rows_affected())
                   .map_err(|e: sqlx::Error| $crate::reexport::AiomeError::Infrastructure { reason: e.to_string() })
            }
        }
    }};
}

/// Helper macro to fetch multiple rows
#[macro_export]
macro_rules! sql_fetch_all {
    ($pool:expr, $output_type:ty, $sql:expr $(, $arg:expr)*) => {{
        match $pool {
            $crate::db::DatabasePool::Sqlite(p) => {
                let res: Result<Vec<$output_type>, sqlx::Error> = sqlx::query_as($sql)
                    $(.bind($arg))*
                    .fetch_all(p)
                    .await;
                res.map_err(|e: sqlx::Error| $crate::reexport::AiomeError::Infrastructure { reason: e.to_string() })
            }
            $crate::db::DatabasePool::Postgres(p) => {
                let res: Result<Vec<$output_type>, sqlx::Error> = sqlx::query_as($sql)
                    $(.bind($arg))*
                    .fetch_all(p)
                    .await;
                res.map_err(|e: sqlx::Error| $crate::reexport::AiomeError::Infrastructure { reason: e.to_string() })
            }
        }
    }};
}

/// Helper macro to fetch exactly one row
#[macro_export]
macro_rules! sql_fetch_one {
    ($pool:expr, $output_type:ty, $sql:expr $(, $arg:expr)*) => {{
        match $pool {
            $crate::db::DatabasePool::Sqlite(p) => {
                let res: Result<$output_type, sqlx::Error> = sqlx::query_as($sql)
                    $(.bind($arg))*
                    .fetch_one(p)
                    .await;
                res.map_err(|e: sqlx::Error| $crate::reexport::AiomeError::Infrastructure { reason: e.to_string() })
            }
            $crate::db::DatabasePool::Postgres(p) => {
                let res: Result<$output_type, sqlx::Error> = sqlx::query_as($sql)
                    $(.bind($arg))*
                    .fetch_one(p)
                    .await;
                res.map_err(|e: sqlx::Error| $crate::reexport::AiomeError::Infrastructure { reason: e.to_string() })
            }
        }
    }};
}

/// Helper macro to fetch an optional row
#[macro_export]
macro_rules! sql_fetch_optional {
    ($pool:expr, $output_type:ty, $sql:expr $(, $arg:expr)*) => {{
        match $pool {
            $crate::db::DatabasePool::Sqlite(p) => {
                let res: Result<Option<$output_type>, sqlx::Error> = sqlx::query_as($sql)
                    $(.bind($arg))*
                    .fetch_optional(p)
                    .await;
                res.map_err(|e: sqlx::Error| $crate::reexport::AiomeError::Infrastructure { reason: e.to_string() })
            }
            $crate::db::DatabasePool::Postgres(p) => {
                let res: Result<Option<$output_type>, sqlx::Error> = sqlx::query_as($sql)
                    $(.bind($arg))*
                    .fetch_optional(p)
                    .await;
                res.map_err(|e: sqlx::Error| $crate::reexport::AiomeError::Infrastructure { reason: e.to_string() })
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_pool_exists_in_shared() {
        let _pool = DatabasePool::new_sqlite(":memory:").await.unwrap();
    }

    #[tokio::test]
    async fn test_sql_exec_macro_exists_in_shared() {
        let pool = DatabasePool::new_sqlite(":memory:").await.unwrap();
        let _ = sql_exec!(&pool, "CREATE TABLE test_table (id INTEGER PRIMARY KEY)");
    }
}
