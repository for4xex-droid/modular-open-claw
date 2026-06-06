/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use async_trait::async_trait;
use commerce_protocol::error::NurtureError;
use sqlx::SqlitePool;
use uuid::Uuid;

#[async_trait]
pub trait NcmecReporter: Send + Sync {
    async fn report_csam(
        &self,
        item_id: &Uuid,
        reason: &str,
        metadata: &serde_json::Value,
    ) -> Result<(), NurtureError>;
}

pub struct SQLiteNcmecReporter {
    pool: SqlitePool,
}

impl SQLiteNcmecReporter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NcmecReporter for SQLiteNcmecReporter {
    async fn report_csam(
        &self,
        item_id: &Uuid,
        reason: &str,
        metadata: &serde_json::Value,
    ) -> Result<(), NurtureError> {
        // 18 U.S.C. § 2258A に基づく証拠保全と通報キュー
        // 実稼働環境では、このレコードが NCMEC CyberTipline に送信されるバッチに回される
        let metadata_str = serde_json::to_string(metadata).unwrap_or_else(|_| "{}".to_string());

        sqlx::query(
            "INSERT INTO nurture_ncmec_reports (id, item_id, reason, evidence_metadata, reported_at, status)
             VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, 'queued')"
        )
        .bind(Uuid::new_v4().to_string())
        .bind(item_id.to_string())
        .bind(reason)
        .bind(metadata_str)
        .execute(&self.pool)
        .await
        .map_err(|e| NurtureError::Infrastructure(format!("NCMECキュー保存失敗: {}", e)))?;

        tracing::error!(
            "🚨 [COMPLIANCE-P0] CSAM Detected & Evidence Preserved. Queued for NCMEC CyberTipline. Item ID: {}, Reason: {}",
            item_id,
            reason
        );

        Ok(())
    }
}

pub struct MockNcmecReporter;

#[async_trait]
impl NcmecReporter for MockNcmecReporter {
    async fn report_csam(
        &self,
        item_id: &Uuid,
        reason: &str,
        _metadata: &serde_json::Value,
    ) -> Result<(), NurtureError> {
        tracing::error!(
            "🚨 [MOCK-NCMEC] CSAM Detected! Item ID: {}, Reason: {}",
            item_id,
            reason
        );
        Ok(())
    }
}
