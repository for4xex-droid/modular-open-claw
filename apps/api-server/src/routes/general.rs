/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core::traits::*;
use aiome_core_contracts::contracts::QuarantinedAsset;
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
    let mut status = {
        let mut monitor = state.health_monitor.lock().await;
        monitor.check()
    };

    let (stats_res, cb_status, lora_check) = tokio::join!(
        state.job_queue.get_agent_stats(),
        state.circuit_breaker.get_status(),
        state.lora_engine.health_check()
    );

    // Fetch real agent stats
    if let Ok(stats) = stats_res {
        status.level = stats.level;
        status.exp = stats.exp;
        status.resonance = stats.resonance;
        status.creativity = stats.creativity;
        status.fatigue = stats.fatigue;
    }

    // G-1: LLM サーキットブレーカーの状態を取得して追加
    status.llm_circuit_breaker = Some(serde_json::to_value(cb_status).unwrap_or_default());

    // 🔍 Sprint 4: LoRA 学習エンジンの健全性チェック
    let lora_ok = lora_check.unwrap_or(false);
    status.lora_engine = Some(serde_json::json!({
        "mlx_available": lora_ok,
        "status": if lora_ok { "ready" } else { "unavailable" }
    }));

    Ok(Json(status))
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct PromptStatsResponse {
    pub period: String,
    pub providers: Vec<infrastructure::llm::evaluation_logger::ProviderEvalStat>,
}

#[utoipa::path(
    get,
    path = "/api/v1/audit/prompt-stats",
    params(
        ("period" = String, Query, description = "Statistics period (e.g., 7d)")
    ),
    responses(
        (status = 200, description = "Fetch prompt evaluation stats", body = PromptStatsResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn get_audit_prompt_stats(
    State(state): State<AppState>,
    axum::extract::Query(_params): axum::extract::Query<std::collections::HashMap<String, String>>,
    auth: crate::auth::Authenticated,
) -> Result<Json<PromptStatsResponse>, AppError> {
    if auth.agent_id != state.system_agent_id {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Access denied".to_string(),
        }
        .into());
    }

    let raw_period = _params.get("period").map(|s| s.as_str()).unwrap_or("7d");
    let actual_period = if raw_period.ends_with('d') {
        if let Ok(days) = raw_period.trim_end_matches('d').parse::<u32>() {
            // Clamp: min 1 day, max 10 years (prevent SQLite datetime NULL boundary faults)
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

    let providers = stats;

    Ok(Json(PromptStatsResponse {
        period: actual_period,
        providers,
    }))
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

    let ledger = rows.map_err(|e| {
        tracing::error!("Failed to fetch audit ledger: {}", e);
        aiome_core::error::AiomeError::Infrastructure {
            reason: "Failed to retrieve ledger".to_string(),
        }
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
    if auth.agent_id != state.system_agent_id {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Access denied: System admin role required".to_string(),
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

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct TrendsResponse {
    pub trends: Vec<aiome_core::traits::TrendItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

#[utoipa::path(
    get,
    path = "/api/v1/trends",
    responses(
        (status = 200, description = "Fetch current trends from configured adapters (X, WebSearch, SERP)", body = TrendsResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn get_trends(
    _state: State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<TrendsResponse>, AppError> {
    let mut warnings = Vec::new();
    let trend_sonar = infrastructure::trend_sonar::build_active_trend_sonar(
        &_state.job_queue,
        _state.provider.0.clone(),
    )
    .await;

    // Use a default broader category for now
    let trends = match trend_sonar.get_trends("general").await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("Failed to fetch trends: {}", e);
            warnings.push(format!("Failed to fetch trends: {}", e));
            vec![]
        }
    };

    Ok(Json(TrendsResponse {
        trends,
        warnings: if warnings.is_empty() {
            None
        } else {
            Some(warnings)
        },
    }))
}
