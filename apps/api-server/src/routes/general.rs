/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::error::AppError;
use crate::AppState;
use aiome_contracts::contracts::QuarantinedAsset;
use aiome_core::traits::*;
use axum::{
    extract::Path, extract::State, http::StatusCode, response::IntoResponse, response::Json,
    routing::get,
};
use shared::health::{HealthMonitor, ResourceStatus};
use std::fs;

#[utoipa::path(
    get,
    path = "/api/wiki",
    responses(
        (status = 200, description = "List wiki markdown files", body = [String])
    ),
    security(("api_key" = []))
)]
pub async fn list_wiki_files(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<Vec<String>>, AppError> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(&state.docs_path) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".md") {
                    files.push(name.to_string());
                }
            }
        }
    }
    files.sort();
    Ok(Json(files))
}

#[utoipa::path(
    get,
    path = "/api/wiki/{filename}",
    params(
        ("filename" = String, Path, description = "Filename with .md extension")
    ),
    responses(
        (status = 200, description = "Wiki markdown content", body = String),
        (status = 404, description = "File not found")
    ),
    security(("api_key" = []))
)]
pub async fn get_wiki_content(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Path(filename): Path<String>,
) -> Result<String, AppError> {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Invalid filename".to_string(),
        }
        .into());
    }

    let path = std::path::PathBuf::from(&state.docs_path).join(filename);
    fs::read_to_string(path)
        .map_err(|e| aiome_core::error::AiomeError::OsError { source: e.into() }.into())
}

#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Get current system and agent health status", body = ResourceStatus),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn get_health_status(
    State(state): State<AppState>,
) -> Result<Json<ResourceStatus>, AppError> {
    let mut monitor = state.health_monitor.lock().await;
    let mut status = monitor.check();

    // Fetch real agent stats
    if let Ok(stats) = state.job_queue.get_agent_stats().await {
        status.level = stats.level;
        status.exp = stats.exp;
        status.resonance = stats.resonance;
        status.creativity = stats.creativity;
        status.fatigue = stats.fatigue;
    }

    // G-1: LLM サーキットブレーカーの状態を取得して追加
    let cb_status = state.circuit_breaker.get_status().await;
    status.llm_circuit_breaker = Some(serde_json::to_value(cb_status).unwrap_or_default());

    // 🔍 Sprint 4: LoRA 学習エンジンの健全性チェック
    let lora_ok = state.lora_engine.health_check().await.unwrap_or(false);
    status.lora_engine = Some(serde_json::json!({
        "mlx_available": lora_ok,
        "status": if lora_ok { "ready" } else { "unavailable" }
    }));

    Ok(Json(status))
}

#[derive(serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct LogEntryResponse {
    pub id: i64,
    pub timestamp: Option<String>,
    pub level: String,
    pub target: String,
    pub message: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/logs",
    params(
        ("limit" = Option<i64>, Query, description = "Maximum number of logs to return")
    ),
    responses(
        (status = 200, description = "Fetch application logs", body = Vec<LogEntryResponse>)
    ),
    security(("api_key" = []))
)]
pub async fn get_logs(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<LogEntryResponse>>, AppError> {
    // G-Log: Only system agent is allowed to access global application logs
    if auth.agent_id != state.system_agent_id {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Access denied: Only the system agent can view global infrastructure logs."
                .to_string(),
        }
        .into());
    }
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(100);

    let pool = state.job_queue.get_pool().get_sqlite_pool_or_err()?;
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
#[derive(serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct AuditLedgerResponse {
    pub id: i64,
    pub table_name: String,
    pub operation: String,
    pub record_id: String,
    pub current_hash: String,
    pub timestamp: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/audit/ledger",
    responses(
        (status = 200, description = "Fetch secure audit ledger", body = [AuditLedgerResponse]),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn get_audit_ledger(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
) -> Result<Json<Vec<AuditLedgerResponse>>, AppError> {
    if auth.agent_id != state.system_agent_id {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Access denied".to_string(),
        }
        .into());
    }
    let pool = state.job_queue.get_pool().get_sqlite_pool_or_err()?;
    let rows = sqlx::query_as::<_, AuditLedgerResponse>(
        "SELECT id, table_name, operation, record_id, current_hash, timestamp FROM audit_ledger_global ORDER BY id DESC LIMIT 100",
    )
    .fetch_all(pool)
    .await;

    let ledger = rows.map_err(|e| aiome_core::error::AiomeError::Infrastructure {
        reason: format!("DB Error: {}", e),
    })?;

    Ok(Json(ledger))
}
#[derive(serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct DiagnosisResponse {
    pub id: i64,
    pub job_id: String,
    pub root_cause: Option<String>,
    pub self_repair_hint: Option<String>,
    pub failure_category: Option<String>,
    pub timestamp: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/audit/diagnostics",
    responses(
        (status = 200, description = "Fetch agent diagnostics history", body = [DiagnosisResponse]),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn get_diagnoses(
    State(state): State<AppState>,
    auth: crate::auth::Authenticated,
) -> Result<Json<Vec<DiagnosisResponse>>, AppError> {
    if auth.agent_id != state.system_agent_id {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Access denied".to_string(),
        }
        .into());
    }
    let pool = state.job_queue.get_pool().get_sqlite_pool_or_err()?;
    let rows = sqlx::query_as::<_, DiagnosisResponse>(
        "SELECT id, job_id, root_cause, self_repair_hint, failure_category, diagnosed_at as timestamp FROM agent_diagnoses ORDER BY id DESC LIMIT 100",
    )
    .fetch_all(pool)
    .await;

    let diagnoses = rows.map_err(|e| {
        eprintln!("DB Error in get_diagnoses: {:?}", e);
        aiome_core::error::AiomeError::Infrastructure {
            reason: format!("DB Error: {}", e),
        }
    })?;

    Ok(Json(diagnoses))
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
    if auth.agent_id != state.system_agent_id {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Access denied: System agent only".to_string(),
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

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct TrendsResponse {
    pub trends: Vec<aiome_core::traits::TrendItem>,
}

#[utoipa::path(
    get,
    path = "/api/v1/trends",
    responses(
        (status = 200, description = "Fetch current AI trends (Skeleton)", body = TrendsResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn get_trends(
    _state: State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<TrendsResponse>, AppError> {
    // Phase 8.6: Skeleton only. Real implementation in Phase 10.
    Ok(Json(TrendsResponse { trends: vec![] }))
}
