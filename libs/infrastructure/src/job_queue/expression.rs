/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use super::SqliteJobQueue;
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
impl ExpressionOps for SqliteJobQueue {
    async fn store_expression(&self, expression: &Expression) -> Result<(), AiomeError> {
        let karma_refs_json =
            serde_json::to_string(&expression.karma_refs).unwrap_or_else(|_| "[]".to_string());

        let avatar_params_json = expression
            .avatar_params
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok());

        sqlx::query(
            "INSERT INTO expressions (id, content, emotion, karma_refs, audio_path, duration_ms, avatar_params, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&expression.id)
        .bind(&expression.content)
        .bind(&expression.emotion)
        .bind(&karma_refs_json)
        .bind(&expression.audio_path)
        .bind(&expression.duration_ms)
        .bind(&avatar_params_json)
        .bind(&expression.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to store expression: {}", e) })?;

        Ok(())
    }

    async fn fetch_expressions(&self, limit: i64) -> Result<Vec<Expression>, AiomeError> {
        let rows = sqlx::query(
            "SELECT id, content, emotion, karma_refs, audio_path, duration_ms, avatar_params, created_at FROM expressions ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch expressions: {}", e) })?;

        let mut results = Vec::new();
        for row in rows {
            let karma_refs_str: String = row.get("karma_refs");
            let karma_refs: Vec<String> = serde_json::from_str(&karma_refs_str).unwrap_or_default();

            results.push(Expression {
                id: row.get("id"),
                content: row.get("content"),
                emotion: row.get("emotion"),
                karma_refs,
                audio_path: row.get("audio_path"),
                duration_ms: row.get("duration_ms"),
                avatar_params: row
                    .get::<Option<String>, _>("avatar_params")
                    .and_then(|s| serde_json::from_str(&s).ok()),
                created_at: row.get("created_at"),
            });
        }

        Ok(results)
    }

    async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
        let row =
            sqlx::query("SELECT value FROM system_settings WHERE key = 'auto_expression_enabled'")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to fetch setting: {}", e),
                })?;

        if let Some(r) = row {
            let val: String = r.get("value");
            Ok(val == "true")
        } else {
            Ok(false)
        }
    }

    async fn set_auto_expression_enabled(&self, enabled: bool) -> Result<(), AiomeError> {
        sqlx::query(
            "INSERT OR REPLACE INTO system_settings (key, value, category, is_secret) VALUES ('auto_expression_enabled', ?, 'expression', 0)"
        )
        .bind(if enabled { "true" } else { "false" })
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to set setting: {}", e) })?;

        Ok(())
    }

    async fn record_resource_usage(&self, log: &ResourceUsageLog) -> Result<(), AiomeError> {
        sqlx::query(
            "INSERT INTO resource_usage_logs (job_id, provider_name, model_name, usage_type, amount, estimated_cost_usd, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&log.job_id)
        .bind(&log.provider_name)
        .bind(&log.model_name)
        .bind(&log.usage_type)
        .bind(&log.amount)
        .bind(&log.estimated_cost_usd)
        .bind(&log.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to record resource usage: {}", e) })?;

        Ok(())
    }
}
