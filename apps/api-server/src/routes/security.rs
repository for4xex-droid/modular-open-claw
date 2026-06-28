/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use axum::{routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OxiLeanPowerResponse {
    pub power: u32,
    pub status: String,
}

pub fn router() -> Router<crate::app_state::AppState> {
    Router::new().route("/oxilean/power", get(get_oxilean_power))
}

#[utoipa::path(
    get,
    path = "/api/v1/security/oxilean/power",
    tag = "Security",
    responses(
        (status = 200, description = "Returns the current OxiLean formal verification proof power", body = OxiLeanPowerResponse)
    )
)]
async fn get_oxilean_power(
    axum::extract::State(state): axum::extract::State<crate::app_state::AppState>,
) -> Json<OxiLeanPowerResponse> {
    let power = state
        .oxilean_power
        .load(std::sync::atomic::Ordering::Relaxed);
    Json(OxiLeanPowerResponse {
        power,
        status: "Verified".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::AppState;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::sync::{atomic::AtomicU32, Arc};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_get_oxilean_power() {
        // Arrange
        let state = AppState {
            oxilean_power: Arc::new(AtomicU32::new(999)),
            ..Default::default()
        };
        let app = router().with_state(state);

        // Act
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/oxilean/power")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Assert
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let power_res: OxiLeanPowerResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(
            power_res.power, 999,
            "Power should dynamically match the state value"
        );
        assert_eq!(power_res.status, "Verified");
    }
}
