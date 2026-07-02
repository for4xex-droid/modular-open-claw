/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use sqlx::Row;

/// DB 行から必須カラムを取得する。欠損時は Infrastructure エラーを返す。
pub fn require_column<'r, T, R>(row: &'r R, column: &'r str) -> Result<T, AiomeError>
where
    R: Row,
    T: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    &'r str: sqlx::ColumnIndex<R>,
{
    row.try_get(column).map_err(|e| AiomeError::Infrastructure {
        reason: format!("Missing or invalid column '{column}': {e}"),
    })
}

/// JSON シリアライズ失敗を Infrastructure エラーとして返す。
pub fn json_string<T: serde::Serialize>(value: &T, field: &str) -> Result<String, AiomeError> {
    serde_json::to_string(value).map_err(|e| AiomeError::Infrastructure {
        reason: format!("Failed to serialize {field}: {e}"),
    })
}

/// JSON デシリアライズ失敗を Infrastructure エラーとして返す。
pub fn json_parse<T: serde::de::DeserializeOwned>(raw: &str, field: &str) -> Result<T, AiomeError> {
    serde_json::from_str(raw).map_err(|e| AiomeError::Infrastructure {
        reason: format!("Failed to deserialize {field}: {e}"),
    })
}
