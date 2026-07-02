/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core::traits::*;
use axum::{extract::Path, extract::State, response::Json};
use shared::health::ResourceStatus;
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
// auth-exempt: ヘルスチェック
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
    status.llm_circuit_breaker = Some(shared::health::CircuitBreakerStatus {
        name: cb_status.name,
        state: match cb_status.state {
            infrastructure::circuit_breaker::CircuitState::Closed => {
                shared::health::CircuitState::Closed
            }
            infrastructure::circuit_breaker::CircuitState::Open => {
                shared::health::CircuitState::Open
            }
            infrastructure::circuit_breaker::CircuitState::HalfOpen => {
                shared::health::CircuitState::HalfOpen
            }
        },
        failure_count: cb_status.failure_count as u64,
        last_failure_at: cb_status.last_failure_at.map(|t| {
            let datetime: chrono::DateTime<chrono::Utc> = t.into();
            datetime.to_rfc3339()
        }),
        reset_timeout_seconds: cb_status.reset_timeout_seconds,
    });

    // 🔍 Sprint 4: LoRA 学習エンジンの健全性チェック
    let lora_ok = lora_check.unwrap_or(false);
    status.lora_engine = Some(shared::health::LoraStatus {
        mlx_available: lora_ok,
        status: if lora_ok {
            "ready".to_string()
        } else {
            "unavailable".to_string()
        },
    });

    // 🛡️ Phase S-5: サポートインシデント週間統計のロード
    let support_repo = infrastructure::support::incident::SupportIncidentRepository::new(
        state.job_queue.pool.clone(),
    );
    if let Ok(stats) = support_repo.compute_weekly_stats().await {
        status.support_incidents = Some(shared::health::IncidentStats {
            total_incidents_7d: stats.total_incidents_7d,
            distinct_users: stats.distinct_users,
            unresolved: stats.unresolved,
            top_severity: stats.top_severity.unwrap_or_else(|| "None".to_string()),
        });
    }

    Ok(Json(status))
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct PromptStatsResponse {
    pub period: String,
    pub providers: Vec<infrastructure::llm::evaluation_logger::ProviderEvalStat>,
}

#[derive(serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct LogEntryResponse {
    pub id: i64,
    pub timestamp: Option<String>,
    pub level: String,
    pub target: String,
    pub message: String,
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

#[derive(serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct DiagnosisResponse {
    pub id: i64,
    pub job_id: String,
    pub root_cause: Option<String>,
    pub self_repair_hint: Option<String>,
    pub failure_category: Option<String>,
    pub timestamp: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, Debug, PartialEq)]
pub struct DiagnosisSummaryResponse {
    pub total_diagnoses: i64,
    pub categories: Vec<CategoryCount>,
}

#[derive(
    serde::Serialize, serde::Deserialize, sqlx::FromRow, utoipa::ToSchema, Debug, PartialEq,
)]
pub struct CategoryCount {
    pub failure_category: String,
    pub count: i64,
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
        &**_state.job_queue.get_inner(),
        _state.provider.0.clone(),
        vec![std::sync::Arc::new(
            crate::internal_services::x_mcp_trend::XMcpTrendAdapter::new(
                _state.mcp_manager.get_inner().clone(),
            ),
        )],
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

#[utoipa::path(
    get,
    path = "/api/v1/system/spec-export",
    responses(
        (status = 200, description = "Export internal specifications to spec-kit format"),
        (status = 500, description = "Internal server error")
    ),
    security(("api_key" = []))
)]
pub async fn export_spec_kit(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(spec_provider) = state.spec_provider.as_opt() {
        let export_dir = std::path::Path::new(&state.docs_path)
            .join(".specify-export-tmp")
            .to_string_lossy()
            .to_string();
        match spec_provider.export_to_spec_kit(&export_dir).await {
            Ok(_) => Ok(Json(serde_json::json!({
                "status": "success",
                "export_path": ".specify-export-tmp"
            }))),
            Err(e) => {
                tracing::error!("Failed to export spec-kit: {:?}", e);
                Err(AppError::internal(e.to_string()))
            }
        }
    } else {
        Err(AppError::internal(
            "SpecProvider not initialized".to_string(),
        ))
    }
}
