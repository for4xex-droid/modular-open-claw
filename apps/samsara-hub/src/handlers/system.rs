/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use crate::state::HubState;

pub async fn health_handler() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(serde_json::json!({"status": "healthy", "service": "samsara-hub"})),
    )
}

pub async fn list_agents_handler(
    State(state): State<Arc<HubState>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let reg = state.agent_registry.read().await;
    let mut agents = Vec::new();
    for info in reg.values() {
        agents.push(serde_json::json!({
            "did": info.did,
            "ip": info.ip,
            "port": info.port,
            "last_seen_seconds_ago": info.last_seen.elapsed().as_secs()
        }));
    }
    (StatusCode::OK, Json(serde_json::json!(agents)))
}
