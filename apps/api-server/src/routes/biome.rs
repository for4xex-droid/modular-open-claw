/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use infrastructure::db::DatabasePool;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct BiomeRunPayload {
    pub id: String,
    pub agent_id: String,
    pub generation: i32,
    pub score: f64,
    pub max_generation: i32,
    pub cell_count: i32,
    pub is_dendou: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BiomeSpecimenPayload {
    pub id: String,
    pub run_id: String,
    pub specimen_name: String,
    pub genome_data: String,
    pub rarity: String,
}

pub async fn save_run(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(payload): Json<BiomeRunPayload>,
) -> Result<Response, AppError> {
    match &**state.db_pool.get_inner() {
        DatabasePool::Sqlite(pool) => {
            sqlx::query(
                "INSERT INTO biome_runs (id, agent_id, generation, score, max_generation, cell_count, is_dendou) 
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&payload.id)
            .bind(&payload.agent_id)
            .bind(payload.generation)
            .bind(payload.score)
            .bind(payload.max_generation)
            .bind(payload.cell_count)
            .bind(payload.is_dendou)
            .execute(pool)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        }
        DatabasePool::Postgres(pool) => {
            sqlx::query(
                "INSERT INTO biome_runs (id, agent_id, generation, score, max_generation, cell_count, is_dendou) 
                 VALUES ($1, $2, $3, $4, $5, $6, $7)"
            )
            .bind(&payload.id)
            .bind(&payload.agent_id)
            .bind(payload.generation)
            .bind(payload.score)
            .bind(payload.max_generation)
            .bind(payload.cell_count)
            .bind(payload.is_dendou)
            .execute(pool)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        }
    }
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"status": "success"})),
    )
        .into_response())
}

pub async fn list_runs(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Response, AppError> {
    let runs = match &**state.db_pool.get_inner() {
        DatabasePool::Sqlite(pool) => {
            sqlx::query_as::<_, (String, String, i32, f64, i32, i32, i32, String)>(
                "SELECT id, agent_id, generation, score, max_generation, cell_count, is_dendou, strftime('%Y-%m-%dT%H:%M:%SZ', created_at) FROM biome_runs"
            )
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .into_iter()
            .map(|r| serde_json::json!({
                "id": r.0,
                "agent_id": r.1,
                "generation": r.2,
                "score": r.3,
                "max_generation": r.4,
                "cell_count": r.5,
                "is_dendou": r.6,
                "created_at": r.7
            }))
            .collect::<Vec<_>>()
        }
        DatabasePool::Postgres(pool) => {
            sqlx::query_as::<_, (String, String, i32, f64, i32, i32, i32, DateTime<Utc>)>(
                "SELECT id, agent_id, generation, score, max_generation, cell_count, is_dendou, created_at FROM biome_runs"
            )
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .into_iter()
            .map(|r| serde_json::json!({
                "id": r.0,
                "agent_id": r.1,
                "generation": r.2,
                "score": r.3,
                "max_generation": r.4,
                "cell_count": r.5,
                "is_dendou": r.6,
                "created_at": r.7.to_rfc3339()
            }))
            .collect::<Vec<_>>()
        }
    };
    Ok((StatusCode::OK, Json(runs)).into_response())
}

pub async fn save_specimen(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(payload): Json<BiomeSpecimenPayload>,
) -> Result<Response, AppError> {
    match &**state.db_pool.get_inner() {
        DatabasePool::Sqlite(pool) => {
            sqlx::query(
                "INSERT INTO biome_specimens (id, run_id, specimen_name, genome_data, rarity) 
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&payload.id)
            .bind(&payload.run_id)
            .bind(&payload.specimen_name)
            .bind(&payload.genome_data)
            .bind(&payload.rarity)
            .execute(pool)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        }
        DatabasePool::Postgres(pool) => {
            sqlx::query(
                "INSERT INTO biome_specimens (id, run_id, specimen_name, genome_data, rarity) 
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(&payload.id)
            .bind(&payload.run_id)
            .bind(&payload.specimen_name)
            .bind(&payload.genome_data)
            .bind(&payload.rarity)
            .execute(pool)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        }
    }
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"status": "success"})),
    )
        .into_response())
}

pub async fn list_specimens(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Response, AppError> {
    let specimens = match &**state.db_pool.get_inner() {
        DatabasePool::Sqlite(pool) => {
            sqlx::query_as::<_, (String, String, String, String, String, String)>(
                "SELECT id, run_id, specimen_name, genome_data, rarity, strftime('%Y-%m-%dT%H:%M:%SZ', created_at) FROM biome_specimens"
            )
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .into_iter()
            .map(|r| serde_json::json!({
                "id": r.0,
                "run_id": r.1,
                "specimen_name": r.2,
                "genome_data": r.3,
                "rarity": r.4,
                "created_at": r.5
            }))
            .collect::<Vec<_>>()
        }
        DatabasePool::Postgres(pool) => {
            sqlx::query_as::<_, (String, String, String, String, String, DateTime<Utc>)>(
                "SELECT id, run_id, specimen_name, genome_data, rarity, created_at FROM biome_specimens"
            )
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .into_iter()
            .map(|r| serde_json::json!({
                "id": r.0,
                "run_id": r.1,
                "specimen_name": r.2,
                "genome_data": r.3,
                "rarity": r.4,
                "created_at": r.5.to_rfc3339()
            }))
            .collect::<Vec<_>>()
        }
    };
    Ok((StatusCode::OK, Json(specimens)).into_response())
}

pub async fn get_analytics(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    _auth: crate::auth::Authenticated,
) -> Result<Response, AppError> {
    let analytics = match &**state.db_pool.get_inner() {
        DatabasePool::Sqlite(pool) => {
            sqlx::query_as::<_, (String, String, i32, i32, f64, String)>(
                "SELECT id, run_id, active_cells, frozen_cells, element_imbalance, strftime('%Y-%m-%dT%H:%M:%SZ', created_at) FROM biome_analytics WHERE run_id = ?"
            )
            .bind(&run_id)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .into_iter()
            .map(|r| serde_json::json!({
                "id": r.0,
                "run_id": r.1,
                "active_cells": r.2,
                "frozen_cells": r.3,
                "element_imbalance": r.4,
                "created_at": r.5
            }))
            .collect::<Vec<_>>()
        }
        DatabasePool::Postgres(pool) => {
            sqlx::query_as::<_, (String, String, i32, i32, f64, DateTime<Utc>)>(
                "SELECT id, run_id, active_cells, frozen_cells, element_imbalance, created_at FROM biome_analytics WHERE run_id = $1"
            )
            .bind(&run_id)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .into_iter()
            .map(|r| serde_json::json!({
                "id": r.0,
                "run_id": r.1,
                "active_cells": r.2,
                "frozen_cells": r.3,
                "element_imbalance": r.4,
                "created_at": r.5.to_rfc3339()
            }))
            .collect::<Vec<_>>()
        }
    };
    Ok((StatusCode::OK, Json(analytics)).into_response())
}
