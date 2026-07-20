/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! OP-088 P5-a: Nurture `/internal` への POST（InProcess = oneshot、それ以外 = HTTP）。
//!
//! oneshot の path は nest 前（`/forget/...`, `/coin-charge`, …）。
//! HTTP 時のみ `{nurture_url}/internal{path}` を組み立てる。

use axum::{
    body::Body,
    http::{header::AUTHORIZATION, Request, StatusCode},
    Router,
};
use std::time::Duration;
use tower::ServiceExt;

const OXP_HEADER: &str = "X-OxiLean-Proof-Certificate";

/// Bearer + OXP ヘッダ文字列を生成する（actor / power は呼び出し側が指定）。
pub fn attach_s2s_headers(
    secret: &str,
    actor: &str,
    power: u32,
) -> Result<(String, String), String> {
    let Some(cert) = aiome_core_contracts::oxilean::OxiLeanProofCertificate::generate_header(
        actor, power, secret,
    ) else {
        return Err(format!(
            "oxp_header_generation_failed for actor={actor}; request denied (fail-closed)"
        ));
    };
    Ok((format!("Bearer {secret}"), cert))
}

/// Nurture S2S POST。`s2s` があれば TCP 無し oneshot、なければ HTTP。
#[allow(clippy::too_many_arguments)] // S2S 経路の引数は呼び出し側で明示（actor/power 定数化禁止）
pub async fn post_internal(
    s2s: Option<&Router>,
    nurture_url: Option<&str>,
    secret: &str,
    actor: &str,
    power: u32,
    path: &str,
    body: Option<&serde_json::Value>,
    timeout: Duration,
) -> Result<(), String> {
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let (auth, oxp) = attach_s2s_headers(secret, actor, power)?;

    if let Some(router) = s2s {
        return post_oneshot(router, &path, &auth, &oxp, body).await;
    }

    let base = nurture_url
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .ok_or_else(|| "NURTURE_API_URL is not configured".to_string())?;
    let req_url = format!("{}{}{}", base.trim_end_matches('/'), "/internal", path);
    post_http(&req_url, &auth, &oxp, body, timeout).await
}

async fn post_oneshot(
    router: &Router,
    path: &str,
    auth: &str,
    oxp: &str,
    body: Option<&serde_json::Value>,
) -> Result<(), String> {
    let (body, json) = match body {
        Some(v) => (
            Body::from(serde_json::to_vec(v).map_err(|e| format!("json_encode: {e}"))?),
            true,
        ),
        None => (Body::empty(), false),
    };
    let mut builder = Request::builder()
        .method("POST")
        .uri(path)
        .header(AUTHORIZATION, auth)
        .header(OXP_HEADER, oxp);
    if json {
        builder = builder.header("content-type", "application/json");
    }
    let req = builder
        .body(body)
        .map_err(|e| format!("request_build: {e}"))?;
    let res = router
        .clone()
        .oneshot(req)
        .await
        .map_err(|e| format!("oneshot: {e}"))?;
    status_result(res.status())
}

async fn post_http(
    req_url: &str,
    auth: &str,
    oxp: &str,
    body: Option<&serde_json::Value>,
    timeout: Duration,
) -> Result<(), String> {
    let client = aiome_core::http::get_http_client();
    let mut req = client
        .post(req_url)
        .header(AUTHORIZATION.as_str(), auth)
        .header(OXP_HEADER, oxp)
        .timeout(timeout);
    if let Some(v) = body {
        req = req.json(v);
    }
    let res = req.send().await.map_err(|e| format!("network: {e}"))?;
    status_result(res.status())
}

fn status_result(status: StatusCode) -> Result<(), String> {
    if status.is_success() {
        Ok(())
    } else {
        Err(format!("http {status}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::Request as AxumRequest,
        middleware::{from_fn, Next},
        response::Response,
        routing::post,
    };
    use serde_json::json;

    #[tokio::test]
    async fn post_internal_prefers_oneshot_over_http_url() {
        async fn ok() -> StatusCode {
            StatusCode::OK
        }
        let router = Router::new().route("/coin-charge", post(ok));
        let result = post_internal(
            Some(&router),
            Some("http://127.0.0.1:9"),
            "mock_secret",
            "aiome-edge-node",
            100,
            "/coin-charge",
            Some(&json!({"amount": 1})),
            Duration::from_secs(2),
        )
        .await;
        assert!(result.is_ok(), "expected oneshot success, got {result:?}");
    }

    #[tokio::test]
    async fn post_internal_oneshot_surfaces_http_status() {
        async fn unauthorized() -> StatusCode {
            StatusCode::UNAUTHORIZED
        }
        let router = Router::new().route("/forget/x", post(unauthorized));
        let Err(err) = post_internal(
            Some(&router),
            None,
            "mock_secret",
            "aiome_system",
            1000,
            "/forget/x",
            None,
            Duration::from_secs(2),
        )
        .await
        else {
            panic!("expected 401");
        };
        assert!(err.contains("401"), "got {err}");
    }

    /// a5: 坏 Bearer は oneshot 経路でも 401 として呼び出し側に伝搬する。
    #[tokio::test]
    async fn post_internal_oneshot_bad_secret_gets_401() {
        async fn require_bearer(req: AxumRequest, next: Next) -> Result<Response, StatusCode> {
            let ok = req
                .headers()
                .get(AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                == Some("Bearer good_secret");
            if ok {
                Ok(next.run(req).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        async fn ok() -> StatusCode {
            StatusCode::OK
        }
        let router = Router::new()
            .route("/coin-charge", post(ok))
            .layer(from_fn(require_bearer));

        let Err(err) = post_internal(
            Some(&router),
            None,
            "bad_secret",
            "aiome-edge-node",
            100,
            "/coin-charge",
            Some(&json!({"amount": 1})),
            Duration::from_secs(2),
        )
        .await
        else {
            panic!("expected 401 for bad secret");
        };
        assert!(err.contains("401"), "got {err}");

        let good = post_internal(
            Some(&router),
            None,
            "good_secret",
            "aiome-edge-node",
            100,
            "/coin-charge",
            Some(&json!({"amount": 1})),
            Duration::from_secs(2),
        )
        .await;
        assert!(good.is_ok(), "expected good secret ok, got {good:?}");
    }

    #[tokio::test]
    async fn attach_s2s_headers_roundtrip_verify() {
        let Ok((auth, oxp)) = attach_s2s_headers("mock_secret", "aiome-edge-node", 950) else {
            panic!("headers");
        };
        assert!(auth.starts_with("Bearer "));
        use base64::Engine;
        let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&oxp) else {
            panic!("oxp base64");
        };
        let Ok(cert) = serde_json::from_slice::<
            aiome_core_contracts::oxilean::OxiLeanProofCertificate,
        >(&decoded) else {
            panic!("oxp json");
        };
        assert_eq!(cert.subject_id, "aiome-edge-node");
        assert_eq!(cert.oxp_score, 950);
        assert!(cert.verify("mock_secret"));
    }

    #[tokio::test]
    async fn post_internal_http_json_ok() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/internal/economy-policy/monthly-limit"))
            .and(header("authorization", "Bearer mock_secret"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock)
            .await;

        let result = post_internal(
            None,
            Some(&mock.uri()),
            "mock_secret",
            "aiome-edge-node",
            10,
            "/economy-policy/monthly-limit",
            Some(&json!({"monthly_spend_limit": 42})),
            Duration::from_secs(5),
        )
        .await;
        assert!(result.is_ok(), "{result:?}");
    }
}
