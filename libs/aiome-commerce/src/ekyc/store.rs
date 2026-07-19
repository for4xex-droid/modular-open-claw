/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::ekyc::EkycSessionStore;
use aiome_core_contracts::error::AiomeError;
use async_trait::async_trait;
use shared::db::DatabasePool;
use shared::sql_exec;

/// Universal (SQLite/PostgreSQL) implementation for EkycSessionStore
pub struct UniversalEkycSessionStore {
    pool: DatabasePool,
}

impl UniversalEkycSessionStore {
    /// EkycSessionStore の新インスタンスを作成する
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EkycSessionStore for UniversalEkycSessionStore {
    async fn save(&self, user_id: &str, session_id: &str) -> Result<(), AiomeError> {
        let q = format!(
            "INSERT INTO ekyc_sessions (user_id, session_id) VALUES ({0}, {1}) ON CONFLICT(user_id) DO UPDATE SET session_id = excluded.session_id",
            self.pool.ph(0), self.pool.ph(1)
        );

        sql_exec!(&self.pool, &q, user_id, session_id).map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;
        Ok(())
    }

    async fn get_session_id(&self, user_id: &str) -> Result<Option<String>, AiomeError> {
        let q = format!(
            "SELECT session_id FROM ekyc_sessions WHERE user_id = {}",
            self.pool.ph(0)
        );

        let res: Option<String> = match &self.pool {
            DatabasePool::Sqlite(p) => sqlx::query_scalar(&q)
                .bind(user_id)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
            DatabasePool::Postgres(p) => sqlx::query_scalar(&q)
                .bind(user_id)
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
        };

        Ok(res)
    }
}

/// テスト用のモック実装
#[cfg(any(test, debug_assertions))]
pub struct MockEkycSessionStore;

#[cfg(any(test, debug_assertions))]
#[async_trait]
impl EkycSessionStore for MockEkycSessionStore {
    async fn save(&self, _user_id: &str, _session_id: &str) -> Result<(), AiomeError> {
        Ok(())
    }

    async fn get_session_id(&self, _user_id: &str) -> Result<Option<String>, AiomeError> {
        Ok(None)
    }
}
