/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::config::{AppState, WpProxyRequest, WpProxyResponse};
use crate::quota::check_and_increment_quota;
use crate::telemetry::{
    record_caller_on_span, redact_display, redact_url_secrets, sanitize_caller_id, sanitize_for_log,
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use secrecy::ExposeSecret;
use tracing::info;

#[tracing::instrument(
    skip(state, payload),
    fields(caller_id = tracing::field::Empty)
)]
pub(crate) async fn handle_wp_publish(
    State(state): State<AppState>,
    Json(payload): Json<WpProxyRequest>,
) -> impl IntoResponse {
    let safe_caller_id = sanitize_caller_id(&payload.caller_id);
    record_caller_on_span(&safe_caller_id);
    info!("📩 [KeyProxy] WP Publish Request from: {}", safe_caller_id);

    if let Err(status) = check_and_increment_quota(&state, &safe_caller_id).await {
        return status.into_response();
    }

    let url = match &state.wp_api_url {
        Some(u) => format!("{}/wp-json/wp/v2/posts", u.trim_end_matches('/')),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "WP Integration not configured",
            )
                .into_response();
        }
    };

    let token = match &state.wp_api_token {
        Some(t) => t.expose_secret(),
        None => {
            return (StatusCode::SERVICE_UNAVAILABLE, "WP Token not configured").into_response();
        }
    };

    // §SEC: Validate WP status to prevent unauthorized state transitions (e.g. "trash")
    const ALLOWED_WP_STATUSES: &[&str] = &["draft", "publish", "pending", "private", "future"];
    if !ALLOWED_WP_STATUSES.contains(&payload.status.as_str()) {
        tracing::warn!(
            "🚫 [KeyProxy] Rejected invalid WP status: {}",
            sanitize_for_log(&payload.status)
        );
        return (StatusCode::BAD_REQUEST, "Invalid WordPress post status").into_response();
    }

    let body = serde_json::json!({
        "title": payload.title,
        "content": payload.content,
        "status": payload.status,
    });

    let res = state
        .client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await;

    match res {
        Ok(resp) => {
            if resp.status().is_success() {
                if let Ok(wp_res) = resp.json::<serde_json::Value>().await {
                    let link = wp_res
                        .get("link")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    return Json(WpProxyResponse { link }).into_response();
                }
            } else {
                let status = resp.status();
                let err_text = resp.text().await.unwrap_or_default();
                let err_text: String = redact_url_secrets(&err_text).chars().take(200).collect();
                tracing::error!("❌ [KeyProxy] WP Upstream error [{}]: {}", status, err_text);
            }
            (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
        }
        Err(e) => {
            tracing::error!("❌ [KeyProxy] WP Request failed: {}", redact_display(&e));
            (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
        }
    }
}
