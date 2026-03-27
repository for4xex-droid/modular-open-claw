/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 予算上限超過エラー
#[derive(Debug, Error, Serialize, Deserialize, Clone)]
#[error("🚨 [JobBudget] 予算上限超過: limit=${limit:.4}, actual=${actual:.4}")]
pub struct BudgetExhaustedError {
    pub limit: f64,
    pub actual: f64,
}

/// Framework のドメインエラー
#[derive(Debug, Error)]
pub enum AiomeError {
    // === コンテキスト調査 (旧 トレンド調査) ===
    #[error("コンテキスト取得に失敗: {source}")]
    ContextFetch {
        #[source]
        source: anyhow::Error,
    },

    // === 生成エンジン (旧 動画生成) ===
    #[error("外部サービス接続エラー (url: {url}): {source}")]
    RemoteServiceError {
        url: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("外部サービス実行タイムアウト ({timeout_secs}秒)")]
    RemoteServiceTimeout { timeout_secs: u64 },

    #[error("外部サービス実行失敗: {reason}")]
    RemoteServiceExecutionFailed { reason: String },

    // === 外部プロセッサー (旧 メディア編集) ===
    #[error("外部プロセス実行エラー: {reason}")]
    SubprocessFailed { reason: String },

    #[error("アーティファクトが見つからない: {path}")]
    ArtifactNotFound { path: String },

    // === ログ・通知 ===
    #[error("ログ記録エラー: {source}")]
    LogWrite {
        #[source]
        source: anyhow::Error,
    },

    // === LLM ===
    #[error("LLM 応答エラー: {source}")]
    LlmResponse {
        #[source]
        source: anyhow::Error,
    },

    #[error("Guardrails がプロンプトをブロック: {reason}")]
    PromptBlocked { reason: String },

    // === 設定 ===
    #[error("設定ファイル読み込みエラー: {source}")]
    ConfigLoad {
        #[source]
        source: anyhow::Error,
    },

    // === 運用・リソース管理 ===
    #[error("リソース不足: 必要 {required_mb}MB, 利用可能 {available_mb}MB")]
    ResourceShortage { required_mb: u64, available_mb: u64 },

    #[error("ストレージ不足: 使用率が閾値 {threshold}% を超過")]
    StorageFull { threshold: f32 },

    #[error("運用タイムアウト: {reason}")]
    OperationalTimeout { reason: String },

    #[error("OSエラー: {source}")]
    OsError {
        #[source]
        source: anyhow::Error,
    },

    #[error("インフラ構造エラー: {reason}")]
    Infrastructure { reason: String },

    #[error("生成インターフェース失敗: {reason}")]
    GenerativeInterfaceError { reason: String },

    #[error("セキュリティ法規違反: {reason}")]
    SecurityViolation { reason: String },

    #[error("予算上限超過 (Budget Exhausted): {0}")]
    BudgetExhausted(#[from] BudgetExhaustedError),

    #[error("名誉ある撤退 (Honorable Abort): {reason}")]
    HonorableAbort { reason: String },

    // === Phase A-0: Plugin & Content Rights ===
    #[error("プラグインエラー (plugin: {plugin}): {reason}")]
    PluginError { plugin: String, reason: String },

    #[error("コンテンツ権利未検証: item_id={item_id}")]
    ContentNotVerified { item_id: String },

    #[error("リソースビジー（処理限界）: {reason}")]
    ResourceBusy { reason: String },

    #[error("リソースが見つかりません: {reason}")]
    NotFound { reason: String },

    #[error("権限がありません: {reason}")]
    Unauthorized { reason: String },
}

#[cfg(feature = "axum")]
impl axum::response::IntoResponse for AiomeError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;

        let (status, error_message) = match &self {
            AiomeError::PromptBlocked { reason } => (StatusCode::FORBIDDEN, reason.clone()),
            AiomeError::ArtifactNotFound { .. } => {
                (StatusCode::NOT_FOUND, "Artifact not found".to_string())
            }
            AiomeError::SecurityViolation { reason } => (
                StatusCode::FORBIDDEN,
                format!("Security violation: {}", reason),
            ),
            AiomeError::BudgetExhausted(e) => (
                StatusCode::TOO_MANY_REQUESTS,
                format!("Budget exhausted: {}", e),
            ),
            AiomeError::RemoteServiceTimeout { timeout_secs } => (
                StatusCode::GATEWAY_TIMEOUT,
                format!("Remote service timeout after {}s", timeout_secs),
            ),
            AiomeError::StorageFull { threshold } => (
                StatusCode::INSUFFICIENT_STORAGE,
                format!("Storage is full (limit: {}%)", threshold),
            ),
            AiomeError::RemoteServiceError { .. } => {
                (StatusCode::BAD_GATEWAY, "Remote service error".to_string())
            }
            AiomeError::ContentNotVerified { .. } => (
                StatusCode::UNAUTHORIZED,
                "Content rights not verified".to_string(),
            ),
            AiomeError::ContextFetch { .. }
            | AiomeError::LlmResponse { .. }
            | AiomeError::OsError { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
            AiomeError::ConfigLoad { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Configuration error".to_string(),
            ),
            AiomeError::Infrastructure { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Infrastructure error".to_string(),
            ),
            AiomeError::RemoteServiceExecutionFailed { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Execution failed".to_string(),
            ),
            AiomeError::ResourceBusy { reason } => (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("System busy: {}", reason),
            ),
            AiomeError::NotFound { reason } => (StatusCode::NOT_FOUND, reason.clone()),
            AiomeError::Unauthorized { reason } => (StatusCode::UNAUTHORIZED, reason.clone()),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unexpected error occurred".to_string(),
            ),
        };

        // Get variant name as code using Debug output hack (standard pattern for simple enums)
        let code = format!("{:?}", self);
        let code = code
            .split('(')
            .next()
            .unwrap_or("Unknown")
            .split('{')
            .next()
            .unwrap_or("Unknown")
            .trim();

        let body = Json(serde_json::json!({
            "error": error_message,
            "code": code,
        }));

        (status, body).into_response()
    }
}
