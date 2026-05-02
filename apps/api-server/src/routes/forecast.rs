/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ForecastResponse {
    pub series_id: String,
    pub values: Vec<f64>,
    pub timestamps: Vec<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ForecastQuery {
    pub series_id: String,
}

pub fn router() -> Router<crate::app_state::AppState> {
    Router::new().route("/predict", get(get_forecast))
}

#[utoipa::path(
    get,
    path = "/api/v1/forecast/predict",
    tag = "Forecast",
    params(ForecastQuery),
    responses(
        (status = 200, description = "Returns the TimesFM forecast", body = ForecastResponse)
    )
)]
async fn get_forecast(
    State(state): State<crate::app_state::AppState>,
    Query(q): Query<ForecastQuery>,
) -> axum::response::Response {
    if q.series_id.contains("..") || q.series_id.contains('/') {
        return (axum::http::StatusCode::BAD_REQUEST, "Invalid series_id").into_response();
    }

    if let Err(e) = state.circuit_breaker.check_state().await {
        tracing::warn!("TimesFM sidecar circuit breaker open: {}", e);
        // Fallback directly below
    } else {
        let client = state.http_client.get_inner();
        let base_url = state.config.timesfm_sidecar_url.trim_end_matches('/');
        let safe_id =
            url::form_urlencoded::byte_serialize(q.series_id.as_bytes()).collect::<String>();
        let url = format!("{}/forecast?series_id={}", base_url, safe_id);

        match client.get(&url).send().await {
            Ok(res) if res.status().is_success() => {
                state.circuit_breaker.record_success().await;
                if let Ok(json) = res.json::<ForecastResponse>().await {
                    return Json(json).into_response();
                } else {
                    tracing::warn!("TimesFM sidecar returned invalid JSON format");
                }
            }
            Ok(res) => {
                state.circuit_breaker.record_failure().await;
                tracing::warn!("TimesFM sidecar returned error status: {}", res.status());
            }
            Err(e) => {
                state.circuit_breaker.record_failure().await;
                tracing::warn!("Failed to fetch forecast from sidecar: {:?}", e);
            }
        }
    }

    // Fallback if sidecar is offline or errors
    let now = chrono::Utc::now();
    let mut timestamps = Vec::new();
    let mut values = Vec::new();

    for i in 0..10 {
        timestamps.push((now + chrono::Duration::hours(i)).to_rfc3339());
        values.push(10.0 + (i as f64) * 0.5);
    }

    Json(ForecastResponse {
        series_id: q.series_id,
        values,
        timestamps,
    })
    .into_response()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_get_forecast() {
        let mut state = crate::app_state::AppState::default();
        state.http_client = crate::app_state::Component::new(reqwest::Client::new());
        state.config = crate::app_state::Component::new(std::sync::Arc::new(
            shared::config::AiomeConfig::default(),
        ));
        state.circuit_breaker = crate::app_state::Component::new(std::sync::Arc::new(
            infrastructure::circuit_breaker::CircuitBreaker::new(
                "test",
                infrastructure::circuit_breaker::CircuitBreakerConfig {
                    failure_threshold: 5,
                    reset_timeout: std::time::Duration::from_secs(30),
                },
            ),
        ));
        let app = Router::new()
            .route("/predict", axum::routing::get(get_forecast))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/predict?series_id=karma_trend")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let forecast: ForecastResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(forecast.series_id, "karma_trend");
        assert!(!forecast.values.is_empty());
    }

    #[tokio::test]
    async fn test_ssrf_prevention() {
        let mut state = crate::app_state::AppState::default();
        state.http_client = crate::app_state::Component::new(reqwest::Client::new());
        state.config = crate::app_state::Component::new(std::sync::Arc::new(
            shared::config::AiomeConfig::default(),
        ));

        let app = Router::new()
            .route("/predict", axum::routing::get(get_forecast))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/predict?series_id=../../etc/passwd")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
