/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::config::AppState;
use crate::telemetry::sanitize_caller_id;
use axum::http::StatusCode;
use chrono::{Datelike, Utc};
use tracing::{error, info, warn};

#[tracing::instrument(
    skip(state, caller_id),
    fields(caller_id = tracing::field::Empty)
)]
pub(crate) async fn check_and_increment_quota(
    state: &AppState,
    caller_id: &str,
) -> Result<(u64, u32), StatusCode> {
    // Defense in depth: handlers sanitize, but quota is the shared log/metrics choke point.
    // Skip raw arg in instrument — record only after sanitize (log-injection guard).
    let caller_id = sanitize_caller_id(caller_id);
    tracing::Span::current().record("caller_id", tracing::field::display(&caller_id));

    if !state.caller_quotas.contains_key(caller_id.as_str()) {
        warn!("🚫 [KeyProxy] Unknown caller: {}", caller_id);
        return Err(StatusCode::FORBIDDEN);
    }

    let mut q = state.state.write().await;
    let today = Utc::now().ordinal();
    if q.last_reset_day != today {
        info!("🗓️ [KeyProxy] New day detected. Resetting global quota.");
        q.total_calls = 0;
        q.per_caller_calls.clear();
        q.last_reset_day = today;
    }

    q.total_calls += 1;
    let total = q.total_calls;

    let caller_total = {
        let count = q.per_caller_calls.entry(caller_id.clone()).or_insert(0);
        *count += 1;
        *count
    };

    if let Some(&limit) = state.caller_quotas.get(caller_id.as_str()) {
        tracing::info!(
            target: "key_proxy::metrics",
            caller_id = %caller_id,
            caller_calls = caller_total,
            caller_limit = limit,
            usage_ratio = (caller_total as f64 / limit as f64),
            "📈 [KeyProxy] Rate limit usage statistics"
        );

        if caller_total > limit {
            warn!(
                "🛑 [KeyProxy] Caller {} exceeded quota ({})",
                caller_id, limit
            );
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
    }

    if total > 150000 {
        error!(
            "🛑 [KeyProxy] Global quota exceeded! (Day: {})",
            q.last_reset_day
        );
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    if total % 10 == 0 {
        let path = state.persistence_path.clone();
        let state_clone = q.clone();
        tokio::spawn(async move {
            if let Ok(data) = serde_json::to_string(&state_clone) {
                let _ = tokio::fs::write(path, data).await;
            }
        });
    }

    Ok((total, q.last_reset_day))
}
