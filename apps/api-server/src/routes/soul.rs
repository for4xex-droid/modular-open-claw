/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::traits::*; 
use axum::{
    extract::State,
    response::{IntoResponse, Json},
};
use serde::Serialize;
use tracing::error;

use crate::{auth::Authenticated, AppState};

#[derive(Serialize)]
pub struct SoulStatusResponse {
    pub active: bool,
    pub generation: u32,
    pub attachment_style: String,
    pub active_defenses_count: usize,
    pub somatic_markers_count: usize,
    pub soul_resonance_avg: f64,
    pub karma_resonance: i32,
    pub lora_adapter_path: Option<String>,
    pub lora_base_model: Option<String>,
}

pub async fn get_soul_status(
    State(state): State<AppState>,
    _auth: Authenticated,
) -> impl IntoResponse {
    let mut response = SoulStatusResponse {
        active: false,
        generation: 0,
        attachment_style: "Secure".to_string(),
        active_defenses_count: 0,
        somatic_markers_count: 0,
        soul_resonance_avg: 0.0,
        karma_resonance: 0,
        lora_adapter_path: None,
        lora_base_model: None,
    };

    // RS-6: Distinguish karma resonance from soul resonance
    if let Ok(stats) = state.job_queue.get_agent_stats().await {
        response.karma_resonance = stats.resonance;
    }

    match state.soul_store.load_soul("system-soul").await {
        Ok(Some(soul)) => {
            response.active = true;
            response.generation = soul.generation;
            response.attachment_style = format!("{:?}", soul.attachment.style);
            response.active_defenses_count = soul.defenses.len();
            response.somatic_markers_count = soul.somatic_markers.len();
            response.lora_adapter_path = soul.lora_adapter_path.clone();
            response.lora_base_model = soul.lora_base_model.clone();

            if !soul.somatic_markers.is_empty() {
                let sum: f64 = soul
                    .somatic_markers
                    .iter()
                    .map(|m| m.intensity * m.arousal)
                    .sum();
                response.soul_resonance_avg = sum / soul.somatic_markers.len() as f64;
            }
        }
        Ok(None) => {
            // Genesis state or soul hasn't been initialized yet
        }
        Err(e) => {
            error!("Failed to load soul for status API: {:?}", e);
        }
    }

    Json(response)
}
