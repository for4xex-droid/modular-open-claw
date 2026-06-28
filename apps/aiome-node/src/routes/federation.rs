/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FederationHandshake {
    pub node_id: String,
    pub timestamp: String,
    pub protocol_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HandshakeResponse {
    pub ack: bool,
    pub server_time: String,
}

use axum::{extract::State, http::HeaderMap, routing::post, Json, Router};
use std::sync::Arc;

pub fn router() -> Router<Arc<shared::config::AiomeConfig>> {
    Router::new()
        .route("/handshake", post(handle_handshake))
        .route("/sync", post(handle_sync))
}

async fn handle_handshake(Json(_payload): Json<FederationHandshake>) -> Json<HandshakeResponse> {
    Json(HandshakeResponse {
        ack: true,
        server_time: "2026-04-23T00:00:00Z".to_string(),
    })
}

/// Proxy the CRDT sync payload to the core samsara-hub (Smart Edge Pattern).
async fn handle_sync(
    State(config): State<Arc<shared::config::AiomeConfig>>,
    headers: HeaderMap,
    Json(payload): Json<aiome_core_contracts::contracts::FederationSyncRequest>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let target_url = format!("{}/api/v1/federation/sync", config.samsara_hub_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    let mut req_builder = client.post(&target_url).json(&payload);

    if let Some(auth) = headers.get(axum::http::header::AUTHORIZATION) {
        req_builder = req_builder.header(reqwest::header::AUTHORIZATION, auth);
    }

    match req_builder.send().await {
        Ok(res) => {
            let status = res.status();
            match res.bytes().await {
                Ok(body) => (
                    StatusCode::from_u16(status.as_u16())
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                    body,
                )
                    .into_response(),
                Err(e) => {
                    tracing::error!("Proxy error reading body: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Proxy Error").into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Proxy request failed: {}", e);
            (StatusCode::BAD_GATEWAY, "Hub Unreachable").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_handle_handshake() {
        let config = Arc::new(shared::config::AiomeConfig::default());
        let app = router().with_state(config);

        let payload = FederationHandshake {
            node_id: "node-123".to_string(),
            timestamp: "2026-04-23T00:00:00Z".to_string(),
            protocol_version: "1.0".to_string(),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/handshake")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let res: HandshakeResponse = serde_json::from_slice(&body).unwrap();

        assert!(res.ack);
    }

    #[tokio::test]
    async fn test_handle_sync_proxy_failure() {
        let config = Arc::new(shared::config::AiomeConfig {
            samsara_hub_url: "http://localhost:1".to_string(), // Unreachable
            ..Default::default()
        });

        let app = router().with_state(config);

        let payload = aiome_core_contracts::contracts::FederationSyncRequest {
            node_id: "node-123".to_string(),
            since: None,
            protocol_version: "1.0".to_string(),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/sync")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Expect BAD_GATEWAY because http://localhost:1 is unreachable
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }
}
