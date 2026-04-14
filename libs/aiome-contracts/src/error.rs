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

    #[error("ネットワークエラー: {reason}")]
    NetworkError { reason: String },

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

    /// P-4: バリデーションエラー（400 Bad Request を返す）
    #[error("入力値エラー: {reason}")]
    Validation { reason: String },

    #[error("JSON 処理エラー: {0}")]
    JsonSerialization(#[from] serde_json::Error),
}

#[cfg(feature = "axum")]
impl axum::response::IntoResponse for AiomeError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;

        let (status, mut error_message) = match &self {
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
            AiomeError::RemoteServiceError { url, source } => (
                StatusCode::BAD_GATEWAY,
                format!("Remote service error ({}): {}", url, source),
            ),
            AiomeError::ContentNotVerified { item_id } => (
                StatusCode::UNAUTHORIZED,
                format!("Content rights not verified: {}", item_id),
            ),
            AiomeError::ContextFetch { source } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Context fetch error: {}", source),
            ),
            AiomeError::LlmResponse { source } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("LLM response error: {}", source),
            ),
            AiomeError::OsError { source } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("OS error: {}", source),
            ),
            AiomeError::ConfigLoad { source } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Configuration error: {}", source),
            ),
            AiomeError::Infrastructure { reason } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Infrastructure error: {}", reason),
            ),
            AiomeError::RemoteServiceExecutionFailed { reason } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Remote service execution failed: {}", reason),
            ),
            AiomeError::ResourceBusy { reason } => (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("Resource busy: {}", reason),
            ),
            AiomeError::NotFound { reason } => (StatusCode::NOT_FOUND, reason.clone()),
            AiomeError::Unauthorized { reason } => (StatusCode::UNAUTHORIZED, reason.clone()),
            AiomeError::Validation { reason } => (
                StatusCode::BAD_REQUEST,
                format!("Validation error: {}", reason),
            ),
            AiomeError::JsonSerialization(e) => (
                StatusCode::BAD_REQUEST,
                format!("JSON processing error: {}", e),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unexpected error occurred".to_string(),
            ),
        };

        // CWE-209: Prevent Information Leakage in production
        #[cfg(not(debug_assertions))]
        if status.is_server_error() {
            let error_id = uuid::Uuid::new_v4().to_string();
            // Log the actual detailed error in server logs
            tracing::error!(
                "Internal Server Error [Error ID: {}]: {}",
                error_id,
                error_message
            );
            // Replace user-facing message with safe generic response
            error_message = format!(
                "An internal service error occurred. Please contact support with Error ID: {}",
                error_id
            );
        }

        // Get variant name as code using Debug output hack (standard pattern for simple enums)
        let code = {
            let raw = format!("{:?}", self);
            let variant = raw
                .split('(')
                .next()
                .unwrap_or("Unknown")
                .split('{')
                .next()
                .unwrap_or("Unknown")
                .trim()
                .to_string();

            // CWE-209 (P-2): リリースビルドでは内部バリアント名を隠蔽し、
            // サーバーエラー(5xx)の場合は "InternalError" に統一する
            #[cfg(not(debug_assertions))]
            if status.is_server_error() {
                "InternalError".to_string()
            } else {
                variant
            }
            #[cfg(debug_assertions)]
            variant
        };

        let body = Json(serde_json::json!({
            "error": error_message,
            "code": code,
        }));

        (status, body).into_response()
    }
}
