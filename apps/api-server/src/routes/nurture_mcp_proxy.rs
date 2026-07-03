/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use axum::{
    body::Body,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::Response,
    routing::{get, post},
    Router,
};
use std::collections::HashMap;

use crate::app_state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sse", get(proxy_sse))
        .route("/message", post(proxy_message))
}

fn nurture_upstream(state: &AppState) -> Result<(String, String), StatusCode> {
    let url = state
        .nurture_url
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let secret = state
        .nurture_internal_secret
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    Ok((url.trim_end_matches('/').to_string(), secret.clone()))
}

async fn build_proxy_response(upstream: reqwest::Response) -> Result<Response, StatusCode> {
    let status =
        StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut builder = Response::builder().status(status);
    for (key, value) in upstream.headers().iter() {
        if key == "connection" || key == "transfer-encoding" {
            continue;
        }
        builder = builder.header(key, value);
    }
    let body = upstream
        .bytes()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    builder
        .body(Body::from(body))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn proxy_sse(State(state): State<AppState>) -> Result<Response, StatusCode> {
    let (base_url, secret) = nurture_upstream(&state)?;
    let target = format!("{base_url}/api/v1/mcp/sse");
    let upstream = state
        .http_client
        .get_inner()
        .get(&target)
        .header("Authorization", format!("Bearer {secret}"))
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    build_proxy_response(upstream).await
}

async fn proxy_message(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Response, StatusCode> {
    let (base_url, secret) = nurture_upstream(&state)?;
    let mut req_builder = state
        .http_client
        .get_inner()
        .post(format!("{base_url}/api/v1/mcp/message"));
    for (k, v) in &params {
        req_builder = req_builder.query(&[(k.as_str(), v.as_str())]);
    }

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    let mut req = req_builder
        .header("Authorization", format!("Bearer {secret}"))
        .header(
            "Content-Type",
            HeaderValue::from_str(content_type)
                .unwrap_or_else(|_| HeaderValue::from_static("application/json")),
        )
        .body(body.to_vec());

    if let Some(accept) = headers.get("accept").and_then(|v| v.to_str().ok()) {
        req = req.header("Accept", accept);
    }

    let upstream = req.send().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
    build_proxy_response(upstream).await
}
