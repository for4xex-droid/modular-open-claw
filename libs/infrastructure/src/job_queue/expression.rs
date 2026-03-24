/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::UniversalJobQueue;
use aiome_contracts::expression::ResourceUsageLog;
use aiome_core::error::AiomeError;
use aiome_core::expression::Expression;
use aiome_core::traits::JobQueue;
use async_trait::async_trait;
use sqlx::Row;
use tracing::{error, info, warn};

#[async_trait]
pub trait ExpressionOps {
    async fn store_expression(&self, expression: &Expression) -> Result<(), AiomeError>;
    async fn fetch_expressions(&self, limit: i64) -> Result<Vec<Expression>, AiomeError>;
    async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError>;
    async fn set_auto_expression_enabled(&self, enabled: bool) -> Result<(), AiomeError>;
    /// DP-10: リソース使用量を記録
    async fn record_resource_usage(&self, log: &ResourceUsageLog) -> Result<(), AiomeError>;
}

#[async_trait]
impl ExpressionOps for UniversalJobQueue {
    async fn store_expression(&self, expression: &Expression) -> Result<(), AiomeError> {
        let karma_refs_json =
            serde_json::to_string(&expression.karma_refs).unwrap_or_else(|_| "[]".to_string());
        let avatar_params_json = expression
            .avatar_params
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok());
        let tts_status_str = expression.tts_status.to_string();

        let cols = [
            "id",
            "content",
            "emotion",
            "karma_refs",
            "audio_path",
            "duration_ms",
            "avatar_params",
            "created_at",
            "tts_status",
        ];
        let q = self.pool.upsert_query("expressions", "id", &cols, 0);

        sql_exec!(
            &self.pool,
            &q,
            &expression.id,
            &expression.content,
            &expression.emotion,
            &karma_refs_json,
            &expression.audio_path,
            &expression.duration_ms,
            &avatar_params_json,
            &expression.created_at,
            &tts_status_str
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to store expression: {}", e),
        })?;
        Ok(())
    }

    async fn fetch_expressions(&self, limit: i64) -> Result<Vec<Expression>, AiomeError> {
        let q = format!("SELECT id, content, emotion, karma_refs, audio_path, duration_ms, avatar_params, created_at, tts_status FROM expressions ORDER BY created_at DESC LIMIT {}", self.pool.ph(0));
        let mut results = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(&q)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for row in rows {
                    let karma_refs_str: String = row.get("karma_refs");
                    results.push(Expression {
                        id: row.get("id"),
                        content: row.get("content"),
                        emotion: row.get("emotion"),
                        karma_refs: serde_json::from_str(&karma_refs_str).unwrap_or_default(),
                        audio_path: row.get("audio_path"),
                        duration_ms: row.get("duration_ms"),
                        avatar_params: row
                            .get::<Option<String>, _>("avatar_params")
                            .and_then(|s| serde_json::from_str(&s).ok()),
                        created_at: row.get("created_at"),
                        tts_status: aiome_contracts::expression::TtsStatus::from_string(
                            &row.get::<String, _>("tts_status"),
                        ),
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(&q)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for row in rows {
                    let karma_refs_str: String = row.get("karma_refs");
                    results.push(Expression {
                        id: row.get("id"),
                        content: row.get("content"),
                        emotion: row.get("emotion"),
                        karma_refs: serde_json::from_str(&karma_refs_str).unwrap_or_default(),
                        audio_path: row.get("audio_path"),
                        duration_ms: row.get("duration_ms"),
                        avatar_params: row
                            .get::<Option<serde_json::Value>, _>("avatar_params")
                            .and_then(|v| serde_json::from_value(v).ok()),
                        created_at: row.get("created_at"),
                        tts_status: aiome_contracts::expression::TtsStatus::from_string(
                            &row.get::<String, _>("tts_status"),
                        ),
                    });
                }
            }
        }
        Ok(results)
    }

    async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
        let q = format!(
            "SELECT value FROM system_settings WHERE key = {}",
            self.pool.ph(0)
        );
        let opt: Option<String> = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar(&q)
                .bind("auto_expression_enabled")
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar(&q)
                .bind("auto_expression_enabled")
                .fetch_optional(p)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?,
        };
        Ok(opt.map(|v| v == "true").unwrap_or(false))
    }

    async fn set_auto_expression_enabled(&self, enabled: bool) -> Result<(), AiomeError> {
        let val = if enabled { "true" } else { "false" };
        let cols = ["key", "value", "category", "is_secret"];
        let q = self.pool.upsert_query("system_settings", "key", &cols, 0);
        sql_exec!(
            &self.pool,
            &q,
            "auto_expression_enabled",
            val,
            "expression",
            0_i32
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;
        Ok(())
    }

    async fn record_resource_usage(&self, log: &ResourceUsageLog) -> Result<(), AiomeError> {
        let q = format!("INSERT INTO resource_usage_logs (job_id, provider_name, model_name, usage_type, amount, estimated_cost_usd, created_at) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6));
        sql_exec!(
            &self.pool,
            &q,
            &log.job_id,
            &log.provider_name,
            &log.model_name,
            &log.usage_type,
            &log.amount,
            &log.estimated_cost_usd,
            &log.created_at
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;
        Ok(())
    }
}
