/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use crate::state::SharedState;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use commerce_protocol::error::NurtureError;
use tracing::error;

/// 冪等性キーの予約結果
pub enum IdempotencyGate {
    /// 新規処理を実行してよい
    Proceed { key: String },
    /// 保存済みレスポンスを返す
    Cached(StatusCode, String),
    /// 同一キーで処理中
    InProgress,
}

/// 既存 `IdempotencyStore` を用いた reserve 前チェック。
pub async fn begin_idempotent(
    state: &SharedState,
    key: Option<String>,
    operation: &str,
    ttl: chrono::Duration,
) -> Result<IdempotencyGate, Response> {
    let key = match key.filter(|k| !k.is_empty()) {
        Some(k) => k,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("{operation} requires idempotency_key")
                })),
            )
                .into_response());
        }
    };

    let store = state.idempotency.clone();
    match store.get_response(&key).await {
        Ok(Some(Some(cached))) => {
            let status = StatusCode::from_u16(cached.status_code).unwrap_or(StatusCode::OK);
            return Ok(IdempotencyGate::Cached(status, cached.body));
        }
        Ok(Some(None)) => return Ok(IdempotencyGate::InProgress),
        Ok(None) => {}
        Err(e) => {
            error!("❌ [Internal/{operation}] Idempotency check failed: {e}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Idempotency error"})),
            )
                .into_response());
        }
    }

    match store.reserve_key(&key, ttl).await {
        Ok(()) => Ok(IdempotencyGate::Proceed { key }),
        Err(NurtureError::IdempotencyConflict { .. }) => Ok(IdempotencyGate::InProgress),
        Err(e) => {
            error!("❌ [Internal/{operation}] reserve_key failed: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Idempotency error"})),
            )
                .into_response())
        }
    }
}

/// 成功レスポンスを冪等性ストアへ保存する。
pub async fn save_idempotent_success(
    state: &SharedState,
    key: &str,
    status: StatusCode,
    body: String,
) {
    if let Err(e) = state
        .idempotency
        .save_response(key, status.as_u16(), body)
        .await
    {
        error!("⚠️ [Internal] save_response failed (non-fatal): {e}");
    }
}

/// 処理失敗時に予約済みキーを解放する。
///
/// 解放しないと、一時的な障害の後に同一キーでの正当なリトライが
/// TTL（最大24時間）の間 409 Conflict でブロックされてしまう
/// （webhook ハンドラのエラーパスと同じ扱いに揃える）。
pub async fn release_idempotent_key(state: &SharedState, key: &str) {
    if let Err(e) = state.idempotency.delete_key(key).await {
        error!(
            "⚠️ [Internal] delete_key failed; retries with key '{key}' may be blocked until TTL: {e}"
        );
    }
}
