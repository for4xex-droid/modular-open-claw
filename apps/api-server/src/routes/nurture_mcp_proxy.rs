/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use axum::{
    body::{Body, Bytes},
    extract::{Query, State},
    http::{header::AUTHORIZATION, HeaderMap, HeaderValue, StatusCode},
    response::Response,
    routing::{get, post},
    Router,
};
use futures_util::StreamExt;
use std::collections::HashMap;

use crate::app_state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sse", get(proxy_sse))
        .route("/message", post(proxy_message))
}

fn nurture_in_process() -> bool {
    std::env::var("NURTURE_IN_PROCESS")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
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

/// Local sidecar: `{url}/api/v1/mcp`. InProcess plugin: `{self}/mcp`（Aiome `/api/v1/mcp` と衝突回避）。
fn mcp_path_prefix() -> &'static str {
    if nurture_in_process() {
        "/mcp"
    } else {
        "/api/v1/mcp"
    }
}

fn copy_upstream_headers(
    upstream: &reqwest::Response,
    builder: axum::http::response::Builder,
) -> axum::http::response::Builder {
    let mut builder = builder;
    for (key, value) in upstream.headers().iter() {
        // hop-by-hop + Content-Length（ボディ書換・ストリームで不一致になるため除去）
        if key == "connection"
            || key == "transfer-encoding"
            || key == "content-length"
            || key == "content-encoding"
        {
            continue;
        }
        builder = builder.header(key, value);
    }
    builder
}

/// SSE の先頭イベント（`\n\n` まで）だけ endpoint パスを書換。以降は素通し。
fn rewrite_first_sse_endpoint_event(event: &[u8], in_process: bool) -> Vec<u8> {
    let Ok(text) = std::str::from_utf8(event) else {
        return event.to_vec();
    };
    let is_endpoint = text.contains("event: endpoint") || text.contains("event:endpoint");
    if !is_endpoint {
        return event.to_vec();
    }
    let rewritten = if in_process {
        // upstream が古い `/api/v1/mcp/message` を出す場合のみ補正（既に `/mcp/message` なら no-op）
        text.replace("/api/v1/mcp/message", "/mcp/message")
    } else {
        text.replace("/api/v1/mcp/message", "/api/v1/nurture-mcp/message")
    };
    rewritten.into_bytes()
}

/// SSE イベント区切り（`\n\n` または `\r\n\r\n`）の終端インデックス（inclusive）。
fn find_sse_event_end(buf: &[u8]) -> Option<usize> {
    let lf = buf.windows(2).position(|w| w == b"\n\n").map(|i| i + 1);
    let crlf = buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 3);
    match (lf, crlf) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// 先頭イベント未確定のまま巨大バッファを溜めない（DoS / ハング回避）。
const MAX_FIRST_SSE_EVENT_BUF: usize = 64 * 1024;

async fn proxy_sse(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let (base_url, secret) = nurture_upstream(&state)?;
    let target = format!("{base_url}{}/sse", mcp_path_prefix());
    let mut req = state.http_client.get_inner().get(&target);
    // InProcess: Plugin `/mcp` は JWT 下。クライアント Authorization を転送。
    // Local: sidecar は Bearer secret。
    if nurture_in_process() {
        let auth = headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        req = req.header(AUTHORIZATION, auth);
    } else {
        req = req.header(AUTHORIZATION, format!("Bearer {secret}"));
    }
    let upstream = req.send().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
    let status =
        StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let builder = copy_upstream_headers(&upstream, Response::builder().status(status));

    let in_process = nurture_in_process();
    let mut byte_stream = upstream.bytes_stream();
    let stream = async_stream::stream! {
        let mut pending: Vec<u8> = Vec::new();
        let mut first_event_done = false;
        while let Some(item) = byte_stream.next().await {
            match item {
                Ok(chunk) => {
                    if !first_event_done {
                        pending.extend_from_slice(&chunk);
                        if let Some(end) = find_sse_event_end(&pending) {
                            let event: Vec<u8> = pending.drain(..=end).collect();
                            let rewritten = rewrite_first_sse_endpoint_event(&event, in_process);
                            first_event_done = true;
                            yield Ok::<Bytes, std::io::Error>(Bytes::from(rewritten));
                            if !pending.is_empty() {
                                yield Ok(Bytes::from(std::mem::take(&mut pending)));
                            }
                        } else if pending.len() > MAX_FIRST_SSE_EVENT_BUF {
                            // 区切り無し巨大先行 → 書換放棄して素通し（接続維持優先）
                            first_event_done = true;
                            yield Ok(Bytes::from(std::mem::take(&mut pending)));
                        }
                    } else {
                        yield Ok(chunk);
                    }
                }
                Err(e) => {
                    yield Err(std::io::Error::other(e));
                    break;
                }
            }
        }
        if !first_event_done && !pending.is_empty() {
            let rewritten = rewrite_first_sse_endpoint_event(&pending, in_process);
            yield Ok(Bytes::from(rewritten));
        }
    };

    builder
        .body(Body::from_stream(stream))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
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
        .post(format!("{base_url}{}/message", mcp_path_prefix()));
    for (k, v) in &params {
        req_builder = req_builder.query(&[(k.as_str(), v.as_str())]);
    }

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    let auth_value = if nurture_in_process() {
        headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?
            .to_string()
    } else {
        format!("Bearer {secret}")
    };

    let mut req = req_builder
        .header(AUTHORIZATION, auth_value)
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
    // message は有限 JSON — パス書換なし（blind replace しない）
    let status =
        StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let builder = copy_upstream_headers(&upstream, Response::builder().status(status));
    let body = upstream
        .bytes()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    builder
        .body(Body::from(body))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_mcp_path_prefix_local_vs_in_process() {
        std::env::remove_var("NURTURE_IN_PROCESS");
        assert_eq!(mcp_path_prefix(), "/api/v1/mcp");
        std::env::set_var("NURTURE_IN_PROCESS", "true");
        assert_eq!(mcp_path_prefix(), "/mcp");
        std::env::remove_var("NURTURE_IN_PROCESS");
    }

    #[test]
    fn test_rewrite_first_sse_endpoint_event_local() {
        let event = b"event: endpoint\ndata: /api/v1/mcp/message?sessionId=abc\n\n";
        let out = rewrite_first_sse_endpoint_event(event, false);
        assert_eq!(
            std::str::from_utf8(&out).unwrap(),
            "event: endpoint\ndata: /api/v1/nurture-mcp/message?sessionId=abc\n\n"
        );
    }

    #[test]
    fn test_rewrite_first_sse_endpoint_event_in_process() {
        let event = b"event: endpoint\ndata: /api/v1/mcp/message?sessionId=abc\n\n";
        let out = rewrite_first_sse_endpoint_event(event, true);
        assert_eq!(
            std::str::from_utf8(&out).unwrap(),
            "event: endpoint\ndata: /mcp/message?sessionId=abc\n\n"
        );
    }

    #[test]
    fn test_rewrite_skips_non_endpoint_events() {
        let event = b"event: message\ndata: {\"path\":\"/api/v1/mcp/message\"}\n\n";
        let out = rewrite_first_sse_endpoint_event(event, false);
        assert_eq!(out, event, "must not rewrite non-endpoint SSE events");
    }

    #[test]
    fn test_find_sse_event_end() {
        assert_eq!(
            find_sse_event_end(b"event: endpoint\ndata: x\n\nmore"),
            Some(24)
        );
        assert_eq!(
            find_sse_event_end(b"event: endpoint\r\ndata: x\r\n\r\nmore"),
            Some(27)
        );
        assert_eq!(find_sse_event_end(b"incomplete"), None);
    }
}
