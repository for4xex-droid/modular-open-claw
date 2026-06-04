/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::routes::general::{
    AuditLedgerResponse, CategoryCount, DiagnosisResponse, DiagnosisSummaryResponse,
    LogEntryResponse, PromptStatsResponse,
};
use crate::AppState;
use aiome_core_contracts::contracts::QuarantinedAsset;
use axum::{extract::Path, extract::State, http::StatusCode, response::Json};
use shared::auth::Role;

#[utoipa::path(
    get,
    path = "/api/v1/audit/prompt-stats",
    params(
        ("period" = String, Query, description = "Statistics period (e.g., 7d)")
    ),
    responses(
        (status = 200, description = "Fetch prompt evaluation stats", body = PromptStatsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("api_key" = []))
)]
pub async fn get_audit_prompt_stats(
    State(state): State<AppState>,
    axum::extract::Query(_params): axum::extract::Query<std::collections::HashMap<String, String>>,
    auth: crate::auth::Authenticated,
) -> Result<Json<PromptStatsResponse>, AppError> {
    // Phase 1 Hardening: Strict Admin/System Role check
    if !auth
        .roles
        .iter()
        .any(|r| matches!(r, Role::Admin | Role::System))
    {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Access denied: Admin or System role required".to_string(),
        }
        .into());
    }

    let raw_period = _params.get("period").map(|s| s.as_str()).unwrap_or("7d");
    let actual_period = if raw_period.ends_with('d') {
        if let Ok(days) = raw_period.trim_end_matches('d').parse::<u32>() {
            let clamped = days.clamp(1, 3650);
            format!("{}d", clamped)
        } else {
            "7d".to_string()
        }
    } else {
        "7d".to_string()
    };
    let days = actual_period
        .trim_end_matches('d')
        .parse::<u32>()
        .unwrap_or(7);

    let logger = state.eval_logger.get_inner().clone();
    let stats = logger.get_all_provider_stats(days).await?;

    Ok(Json(PromptStatsResponse {
        period: actual_period,
        providers: stats,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/audit/ledger",
    responses(
        (status = 200, description = "Fetch secure audit ledger", body = [AuditLedgerResponse]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("api_key" = []))
)]
pub async fn get_audit_ledger(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
) -> Result<Json<Vec<AuditLedgerResponse>>, AppError> {
    if !auth
        .roles
        .iter()
        .any(|r| matches!(r, Role::Admin | Role::System))
    {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Access denied: Admin or System role required".to_string(),
        }
        .into());
    }
    let pool = state.db_pool.get_inner().get_sqlite_pool_or_err()?;
    let rows = sqlx::query_as::<_, AuditLedgerResponse>(
        "SELECT id, table_name, operation, record_id, current_hash, timestamp FROM audit_ledger_global ORDER BY id DESC LIMIT 100",
    )
    .fetch_all(pool)
    .await;

    let ledger = rows.map_err(|e| {
        tracing::error!("Failed to fetch audit ledger: {}", e);
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Failed to retrieve ledger".to_string(),
        }
    })?;

    Ok(Json(ledger))
}

#[utoipa::path(
    get,
    path = "/api/v1/audit/diagnostics",
    responses(
        (status = 200, description = "Fetch agent diagnostics history", body = [DiagnosisResponse]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("api_key" = []))
)]
pub async fn get_diagnoses(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
) -> Result<Json<Vec<DiagnosisResponse>>, AppError> {
    if !auth
        .roles
        .iter()
        .any(|r| matches!(r, Role::Admin | Role::System))
    {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Access denied: Admin or System role required".to_string(),
        }
        .into());
    }
    let pool = state.db_pool.get_inner().get_sqlite_pool_or_err()?;
    let rows = sqlx::query_as::<_, DiagnosisResponse>(
        "SELECT id, job_id, root_cause, self_repair_hint, failure_category, diagnosed_at as timestamp FROM agent_diagnoses ORDER BY id DESC LIMIT 100",
    )
    .fetch_all(pool)
    .await;

    let diagnoses = rows.map_err(|e| aiome_core::error::AiomeError::Infrastructure {
        reason: format!("DB Error: {}", e),
    })?;

    Ok(Json(diagnoses))
}

#[utoipa::path(
    get,
    path = "/api/v1/audit/diagnostics/summary",
    responses(
        (status = 200, description = "Aggregated diagnostics summary", body = DiagnosisSummaryResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("api_key" = []))
)]
pub async fn get_diagnostics_summary(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
) -> Result<Json<DiagnosisSummaryResponse>, AppError> {
    // Defense-in-Depth: Admin/System ロール検証（ミドルウェアと二重チェック）
    if !auth
        .roles
        .iter()
        .any(|r| matches!(r, Role::Admin | Role::System))
    {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Access denied: Admin or System role required".to_string(),
        }
        .into());
    }

    let pool = state.db_pool.get_inner().get_sqlite_pool_or_err()?;

    let total_row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agent_diagnoses")
        .fetch_one(pool)
        .await
        .map_err(|e| aiome_core::error::AiomeError::Infrastructure {
            reason: format!("DB Error: {}", e),
        })?;

    let categories: Vec<CategoryCount> = sqlx::query_as(
        "SELECT failure_category, COUNT(*) as count FROM agent_diagnoses GROUP BY failure_category",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| aiome_core::error::AiomeError::Infrastructure {
        reason: format!("DB Error: {}", e),
    })?;

    Ok(Json(DiagnosisSummaryResponse {
        total_diagnoses: total_row.0,
        categories,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/audit/quarantine",
    responses(
        (status = 200, description = "Fetch quarantined assets store", body = [QuarantinedAsset]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("api_key" = []))
)]
pub async fn get_quarantined_assets(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
) -> Result<Json<Vec<QuarantinedAsset>>, AppError> {
    if !auth
        .roles
        .iter()
        .any(|r| matches!(r, Role::Admin | Role::System))
    {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Access denied: Admin or System role required".to_string(),
        }
        .into());
    }

    let assets = state.quarantine_store.list_assets().await.map_err(|e| {
        aiome_core::error::AiomeError::Infrastructure {
            reason: format!("Quarantine Store Error: {}", e),
        }
    })?;

    Ok(Json(assets))
}

#[utoipa::path(
    post,
    path = "/api/v1/audit/quarantine/{id}/release",
    params(
        ("id" = String, Path, description = "Asset ID to release")
    ),
    responses(
        (status = 200, description = "Asset successfully released from quarantine"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("api_key" = []))
)]
pub async fn release_quarantined_asset(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    if !auth
        .roles
        .iter()
        .any(|r| matches!(r, Role::Admin | Role::System))
    {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Access denied: Admin role required".to_string(),
        }
        .into());
    }

    state
        .quarantine_store
        .release_asset(&id)
        .await
        .map_err(|e| aiome_core::error::AiomeError::Infrastructure {
            reason: format!("Quarantine Store Error: {}", e),
        })?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/api/v1/logs",
    params(
        ("limit" = Option<i64>, Query, description = "Maximum number of logs to return")
    ),
    responses(
        (status = 200, description = "Fetch application logs", body = Vec<LogEntryResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("api_key" = []))
)]
pub async fn get_logs(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<LogEntryResponse>>, AppError> {
    if !auth
        .roles
        .iter()
        .any(|r| matches!(r, Role::Admin | Role::System))
    {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Access denied: Admin or System role required".to_string(),
        }
        .into());
    }
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(100)
        .clamp(1, 1000);

    let pool = state.db_pool.get_inner().get_sqlite_pool_or_err()?;
    let rows = sqlx::query_as::<_, LogEntryResponse>(
        "SELECT id, timestamp, level, target, message FROM app_logs ORDER BY id DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await;

    let logs = rows.map_err(|e| aiome_core::error::AiomeError::Infrastructure {
        reason: format!("DB Error: {}", e),
    })?;

    Ok(Json(logs))
}
