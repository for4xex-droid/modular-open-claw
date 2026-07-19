/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::config::AppState;
use crate::telemetry::{redact_display, sanitize_for_log};
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

/// Drop client-supplied `key=` so the real secret is never placed in the upstream URL.
pub(crate) fn strip_api_key_query(query: &str) -> String {
    query
        .split('&')
        .filter(|part| {
            if part.is_empty() {
                return false;
            }
            let key = part.split('=').next().unwrap_or(part);
            !key.eq_ignore_ascii_case("key")
        })
        .collect::<Vec<_>>()
        .join("&")
}

/// Reject control characters in path segments (request-line / log injection).
pub(crate) fn is_safe_upstream_path(path: &str) -> bool {
    !path.is_empty() && !path.chars().any(|c| c.is_control())
}

#[tracing::instrument(
    skip(state, req),
    fields(path = tracing::field::Empty)
)]
pub(crate) async fn handle_gemini_passthrough(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<String>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    if !is_safe_upstream_path(&path) {
        return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }

    let safe_path = sanitize_for_log(&path);
    tracing::Span::current().record("path", tracing::field::display(&safe_path));

    let query_string = req.uri().query().unwrap_or("").to_string();
    let method = req.method().clone();

    // Convert Request body to bytes
    let (parts, body) = req.into_parts();
    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(e) => {
            error!(
                "❌ [KeyProxy] Failed to read passthrough body: {}",
                redact_display(&e)
            );
            return (StatusCode::BAD_REQUEST, "Invalid body").into_response();
        }
    };

    let cleaned_query = strip_api_key_query(&query_string);
    let target_url = build_gemini_passthrough_url(
        &safe_path,
        if cleaned_query.is_empty() {
            None
        } else {
            Some(cleaned_query.as_str())
        },
    );

    info!("🌐 [KeyProxy] Passthrough to Gemini API: {}", safe_path);

    let mut request_builder = state
        .client
        .request(method, &target_url)
        // Prefer header auth (same as LLM handlers) — never put GEMINI_API_KEY in the URL.
        .header("x-goog-api-key", state.gemini_key.expose_secret());

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
            error!(
                "❌ [KeyProxy] Gemini upstream error: {}",
                redact_display(&e)
            );
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
            error!(
                "❌ [KeyProxy] Failed to read Gemini response: {}",
                redact_display(&e)
            );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_api_key_query_removes_key_param() {
        assert_eq!(strip_api_key_query("key=SECRET&alt=sse"), "alt=sse");
        assert_eq!(strip_api_key_query("KEY=SECRET"), "");
        assert_eq!(strip_api_key_query("alt=sse"), "alt=sse");
    }

    #[test]
    fn passthrough_url_never_requires_key_in_query() {
        let cleaned = strip_api_key_query("key=TEST_DUMMY_KEY&foo=1");
        let url = build_gemini_passthrough_url(
            "v1beta/models/gemini-2.0-flash:generateContent",
            Some(cleaned.as_str()),
        );
        assert!(
            !url.contains("key="),
            "upstream URL must not contain key=: {url}"
        );
        assert!(url.contains("foo=1"));
    }

    #[test]
    fn unsafe_upstream_path_rejected() {
        assert!(is_safe_upstream_path("v1beta/models/x:generateContent"));
        assert!(!is_safe_upstream_path("v1beta\nHost: evil"));
        assert!(!is_safe_upstream_path(""));
    }
}
