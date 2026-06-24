/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-04-01
 * Change License: Apache License 2.0
 *
 * Usage of this software is subject to the BSL 1.1 terms.
 * Commercial use requires a separate license agreement.
 */

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use commerce_protocol::error::NurtureError;
use nurture_bridge::db::DatabasePool;
use nurture_bridge::error::AiomeError;
use nurture_bridge::{sql_exec, sql_fetch_optional_map};
use sqlx::Row;

/// 冪等性キーに紐付く保存されたレスポンス
#[derive(Debug, Clone)]
pub struct IdempotencyResponse {
    pub status_code: u16,
    pub body: String,
}

/// 冪等性キーストアのトレイト定義
#[async_trait]
pub trait IdempotencyStore: Send + Sync {
    /// キーの状態を確認し、保存済みのレスポンスがあれば返す
    async fn get_response(
        &self,
        key: &str,
    ) -> Result<Option<Option<IdempotencyResponse>>, NurtureError>;

    /// キーを予約（「処理中」状態にする）
    async fn reserve_key(&self, key: &str, expires_in: Duration) -> Result<(), NurtureError>;

    /// レスポンスを保存して確定させる
    async fn save_response(&self, key: &str, status: u16, body: String)
        -> Result<(), NurtureError>;

    /// 進行中のロック（未完了のキー）を解放・削除する（リトライ可能にするため）
    async fn delete_key(&self, key: &str) -> Result<(), NurtureError>;
}

pub struct SQLiteIdempotencyStore {
    pool: DatabasePool,
}

impl SQLiteIdempotencyStore {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IdempotencyStore for SQLiteIdempotencyStore {
    async fn get_response(
        &self,
        key: &str,
    ) -> Result<Option<Option<IdempotencyResponse>>, NurtureError> {
        let row_opt = sql_fetch_optional_map!(
            &self.pool,
            sqlite: "SELECT response_body, status_code, expires_at FROM nurture_idempotency WHERE key = ?",
            |row| {
                let response_body: Option<String> = row.get("response_body");
                let status_code: Option<i64> = row.get("status_code");
                let expires_at: DateTime<Utc> = row.get("expires_at");
                Ok::<_, AiomeError>((response_body, status_code, expires_at))
            },
            pg: "SELECT response_body, status_code, expires_at FROM nurture_idempotency WHERE key = $1",
            |row| {
                let response_body: Option<String> = row.get("response_body");
                let status_code: Option<i64> = row.get("status_code");
                let expires_at: DateTime<Utc> = row.get("expires_at");
                Ok::<_, AiomeError>((response_body, status_code, expires_at))
            },
            key
        )
        .map_err(|e| NurtureError::Infrastructure(format!("冪等性キー取得失敗: {}", e)))?;

        match row_opt {
            Some((response_body, status_code, expires_at)) => {
                if Utc::now() > expires_at {
                    return Ok(None);
                }

                match (response_body, status_code) {
                    (Some(body), Some(status)) => Ok(Some(Some(IdempotencyResponse {
                        status_code: u16::try_from(status).unwrap_or(500),
                        body,
                    }))),
                    _ => Ok(Some(None)), // 予約はされているが、まだレスポンスがない
                }
            }
            None => Ok(None),
        }
    }

    async fn reserve_key(&self, key: &str, expires_in: Duration) -> Result<(), NurtureError> {
        let expires_at = Utc::now() + expires_in;

        // 期限切れの既存キーは削除してから予約（再利用可能にする）
        sql_exec!(
            &self.pool,
            sqlite: "DELETE FROM nurture_idempotency WHERE key = ? AND expires_at < ?",
            pg: "DELETE FROM nurture_idempotency WHERE key = $1 AND expires_at < $2",
            key,
            Utc::now()
        )
        .map_err(|e| NurtureError::Infrastructure(format!("冪等性キー削除失敗: {}", e)))?;

        let insert_res = sql_exec!(
            &self.pool,
            sqlite: "INSERT INTO nurture_idempotency (key, expires_at) VALUES (?, ?)",
            pg: "INSERT INTO nurture_idempotency (key, expires_at) VALUES ($1, $2)",
            key,
            expires_at
        );
        match insert_res {
            Ok(_) => Ok(()),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("UNIQUE") || err_str.contains("unique") {
                    Err(NurtureError::IdempotencyConflict {
                        key: key.to_string(),
                    })
                } else {
                    Err(NurtureError::Infrastructure(format!(
                        "冪等性キー予約失敗: {}",
                        e
                    )))
                }
            }
        }
    }

    async fn save_response(
        &self,
        key: &str,
        status: u16,
        body: String,
    ) -> Result<(), NurtureError> {
        sql_exec!(
            &self.pool,
            sqlite: "UPDATE nurture_idempotency SET response_body = ?, status_code = ? WHERE key = ?",
            pg: "UPDATE nurture_idempotency SET response_body = $1, status_code = $2 WHERE key = $3",
            body,
            i64::from(status),
            key
        )
        .map(|_| ())
        .map_err(|e| NurtureError::Infrastructure(format!("冪等性レスポンス保存失敗: {}", e)))
    }

    async fn delete_key(&self, key: &str) -> Result<(), NurtureError> {
        sql_exec!(
            &self.pool,
            sqlite: "DELETE FROM nurture_idempotency WHERE key = ?",
            pg: "DELETE FROM nurture_idempotency WHERE key = $1",
            key
        )
        .map(|_| ())
        .map_err(|e| NurtureError::Infrastructure(format!("冪等性キー削除失敗: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use sqlx::SqlitePool;

    #[tokio::test]
    async fn test_status_code_clamping() {
        // [Reflexion Sprint A v3] 回帰テスト: DBのステータスコード異常値フォールバック
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE nurture_idempotency (
                key TEXT PRIMARY KEY,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                response_body TEXT,
                status_code INTEGER,
                expires_at TIMESTAMP NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        let store = SQLiteIdempotencyStore::new(DatabasePool::Sqlite(pool.clone()));
        let key = "test_key_status_clamp";

        // 有効な事前予約を挿入
        sqlx::query(
            "INSERT INTO nurture_idempotency (key, created_at, response_body, status_code, expires_at)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(key)
        .bind(chrono::Utc::now())
        .bind("Success".to_string())
        .bind(70000_i64) // u16::MAX (65535) を超える不正なステータスコード
        .bind(chrono::Utc::now() + Duration::days(1))
        .execute(&pool)
        .await
        .unwrap();

        let result = store.get_response(key).await.unwrap();

        match result {
            Some(Some(response)) => {
                // 異常値がフォールバック先である 500 に変換され、パニックやサイレント切り捨てを防ぐこと
                assert_eq!(response.status_code, 500);
            }
            _ => panic!("Expected completed idempotency record"),
        }
    }
}
