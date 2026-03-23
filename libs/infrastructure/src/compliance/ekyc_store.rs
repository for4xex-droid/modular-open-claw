/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use async_trait::async_trait;
use sqlx::SqlitePool;

/// eKYCセッションの永続化インターフェース
#[async_trait]
pub trait EkycSessionStore: Send + Sync {
    /// セッションIDを保存する (1ユーザー1セッション)
    async fn save(&self, user_id: &str, session_id: &str) -> anyhow::Result<()>;
    /// 保存されているセッションIDを取得する
    async fn get_session_id(&self, user_id: &str) -> anyhow::Result<Option<String>>;
}

/// SQLiteを使用した EkycSessionStore 実装
pub struct SqliteEkycSessionStore {
    pool: SqlitePool,
}

impl SqliteEkycSessionStore {
    /// EkycSessionStore の新インスタンスを作成する
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EkycSessionStore for SqliteEkycSessionStore {
    async fn save(&self, user_id: &str, session_id: &str) -> anyhow::Result<()> {
        sqlx::query("INSERT OR REPLACE INTO ekyc_sessions (user_id, session_id) VALUES (?, ?)")
            .bind(user_id)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_session_id(&self, user_id: &str) -> anyhow::Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT session_id FROM ekyc_sessions WHERE user_id = ?")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(|r| r.0))
    }
}

/// テスト用のモック実装
#[cfg(any(test, debug_assertions))]
pub struct MockEkycSessionStore;

#[cfg(any(test, debug_assertions))]
#[async_trait]
impl EkycSessionStore for MockEkycSessionStore {
    async fn save(&self, _user_id: &str, _session_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn get_session_id(&self, _user_id: &str) -> anyhow::Result<Option<String>> {
        Ok(None)
    }
}
