/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core_contracts::traits::SoulStore;
use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct MonologueQuery {
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct MonologueEntry {
    pub id: String,
    pub timestamp: String,
    pub content: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct MonologueResponse {
    pub entries: Vec<MonologueEntry>,
    pub next_cursor: Option<String>,
}

/// 過去の Whisper モノローグ（自己省察履歴）を取得する
#[utoipa::path(
    get,
    path = "/api/v1/whisper/monologue",
    params(MonologueQuery),
    responses(
        (status = 200, description = "モノローグ履歴", body = MonologueResponse),
        (status = 500, description = "内部システムエラー")
    )
)]
pub async fn get_monologue_history(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Query(query): Query<MonologueQuery>,
) -> Result<Json<MonologueResponse>, AppError> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);

    let soul_value_opt = state
        .soul_store
        .load_soul("system-soul")
        .await
        .map_err(|e| {
            error!("🚨 [Whisper] Failed to load system-soul: {}", e);
            AppError::internal("Failed to access soul storage")
        })?;

    let agent_soul = match soul_value_opt {
        Some(v) => v,
        None => {
            return Ok(Json(MonologueResponse {
                entries: vec![],
                next_cursor: None,
            }))
        }
    };

    // timestamp 降順でフィルタリング（新しい記憶が最初）
    // NOTE: Whisper は content に "\nWhisper: " が含まれる
    let mut whispers: Vec<&soul::model::Experience> = agent_soul
        .experience_buffer
        .iter()
        .filter(|e| e.content.contains("Whisper:"))
        .collect();

    whispers.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // カーソル処理（指定された timestamp または id より古いものを取得）
    // ここでは cursor を timestamp と仮定
    let start_idx = if let Some(cursor_val) = &query.cursor {
        whispers
            .iter()
            .position(|e| &e.timestamp < cursor_val)
            .unwrap_or(whispers.len())
    } else {
        0
    };

    let paged_whispers = whispers
        .into_iter()
        .skip(start_idx)
        .take(limit)
        .collect::<Vec<_>>();

    let next_cursor = paged_whispers.last().map(|e| e.timestamp.clone());

    let entries = paged_whispers
        .into_iter()
        .map(|e| {
            // Whisper: ... の部分だけを抽出する（可能であれば）
            let content = extract_whisper_content(&e.content);
            MonologueEntry {
                id: e.id.clone(),
                timestamp: e.timestamp.clone(),
                content,
            }
        })
        .collect();

    Ok(Json(MonologueResponse {
        entries,
        next_cursor,
    }))
}

/// "Whisper:" 以降の文字列をキレイに抽出するヘルパー関数
fn extract_whisper_content(raw: &str) -> String {
    if let Some(idx) = raw.find("Whisper:") {
        raw[idx + "Whisper:".len()..].trim().to_string()
    } else {
        raw.to_string()
    }
}
