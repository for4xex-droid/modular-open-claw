/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::config::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use secrecy::ExposeSecret;
use tracing::{error, info};

pub(crate) fn build_gemini_passthrough_url(path: &str, query: Option<&str>) -> String {
    let base = format!(
        "https://generativelanguage.googleapis.com/{}",
        path.trim_start_matches('/')
    );
    if let Some(q) = query {
        if !q.is_empty() {
            return format!("{}?{}", base, q);
        }
    }
    base
}

#[tracing::instrument(skip(state))]
pub(crate) async fn handle_gemini_passthrough(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<String>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    let query_string = req.uri().query().unwrap_or("").to_string();
    let method = req.method().clone();

    // Convert Request body to bytes
    let (parts, body) = req.into_parts();
    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(e) => {
            error!("❌ [KeyProxy] Failed to read passthrough body: {}", e);
            return (StatusCode::BAD_REQUEST, "Invalid body").into_response();
        }
    };

    let mut target_url = build_gemini_passthrough_url(&path, Some(&query_string));

    // Inject API key if not present
    if !target_url.contains("key=") {
        let separator = if target_url.contains('?') { "&" } else { "?" };
        target_url = format!(
            "{}{}key={}",
            target_url,
            separator,
            state.gemini_key.expose_secret()
        );
    } else {
        // Replace fake key with real key if it was passed
        let fake_key_start = match target_url.find("key=") {
            Some(idx) => idx + 4,
            None => {
                error!("❌ [KeyProxy] Logically unreachable: key= not found in else branch");
                return (StatusCode::INTERNAL_SERVER_ERROR, "Proxy internal error").into_response();
            }
        };
        let fake_key_end = target_url[fake_key_start..]
            .find('&')
            .map(|i| i + fake_key_start)
            .unwrap_or(target_url.len());
        target_url.replace_range(
            fake_key_start..fake_key_end,
            state.gemini_key.expose_secret(),
        );
    }

    info!("🌐 [KeyProxy] Passthrough to Gemini API: {}", path);

    let mut request_builder = state.client.request(method, &target_url);
    for (name, value) in parts.headers.iter() {
        let name_lower = name.as_str().to_lowercase();
        // Drop dangerous or overwritten headers
        if name_lower == "host" || name_lower == "authorization" || name_lower == "x-goog-api-key" {
            continue;
        }
        request_builder = request_builder.header(name, value);
    }

    // Ensure Content-Type is present
    if !parts.headers.contains_key("Content-Type") {
        request_builder = request_builder.header("Content-Type", "application/json");
    }

    let request_builder = request_builder.body(body_bytes);

    let res = match request_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            error!("❌ [KeyProxy] Gemini upstream error: {}", e);
            return (StatusCode::BAD_GATEWAY, "Proxy error").into_response();
        }
    };

    let status = res.status();
    let headers = res.headers().clone();
    let content_type = headers
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    let res_bytes = match res.bytes().await {
        Ok(b) => b,
        Err(e) => {
            error!("❌ [KeyProxy] Failed to read Gemini response: {}", e);
            return (StatusCode::BAD_GATEWAY, "Proxy read error").into_response();
        }
    };

    (
        status,
        [(axum::http::header::CONTENT_TYPE, content_type)],
        res_bytes,
    )
        .into_response()
}
