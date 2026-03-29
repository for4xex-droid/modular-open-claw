/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::forecast::{AnomalyResult, ForecastConfig, ForecastProvider, ForecastResult};
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use serde_json::json;
use std::time::Duration;

/// RT-1 FIX: Sidecar-specific timeout (10s) independent of global 60s client.
/// This prevents a hanging sidecar from killing the Heartbeat loop.
const SIDECAR_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// RT-2 FIX: Maximum number of series and elements per series.
const MAX_SERIES_COUNT: usize = 10;
const MAX_ELEMENTS_PER_SERIES: usize = 2048;

pub struct TimesFmProvider {
    client: reqwest::Client,
    endpoint: String,
    auth_token: String,
}

impl TimesFmProvider {
    pub fn new(endpoint: String, auth_token: String) -> Self {
        // RT-1 FIX: Build a dedicated client with a short timeout for the sidecar,
        // rather than sharing the global 60s client.
        let client = reqwest::Client::builder()
            .timeout(SIDECAR_REQUEST_TIMEOUT)
            .connect_timeout(Duration::from_secs(3))
            .pool_idle_timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("TimesFM reqwest client build should never fail with these settings");

        Self {
            client,
            endpoint,
            auth_token,
        }
    }
}

#[async_trait]
impl ForecastProvider for TimesFmProvider {
    async fn forecast(
        &self,
        series: Vec<Vec<f64>>,
        horizon: usize,
        config: ForecastConfig,
    ) -> Result<ForecastResult, AiomeError> {
        // RT-2 FIX: Validate input bounds before sending to sidecar
        if series.is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "Forecast series must not be empty.".into(),
            });
        }
        if series.len() > MAX_SERIES_COUNT {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Too many series ({}/{}). Possible resource exhaustion attempt.",
                    series.len(),
                    MAX_SERIES_COUNT
                ),
            });
        }
        for (i, s) in series.iter().enumerate() {
            if s.len() > MAX_ELEMENTS_PER_SERIES {
                return Err(AiomeError::Infrastructure {
                    reason: format!(
                        "Series[{}] has {} elements (max {}). Possible DoS attempt.",
                        i,
                        s.len(),
                        MAX_ELEMENTS_PER_SERIES
                    ),
                });
            }
            // RT-4 FIX: Reject NaN/Infinity in input data
            if s.iter().any(|v| v.is_nan() || v.is_infinite()) {
                return Err(AiomeError::Infrastructure {
                    reason: format!(
                        "Series[{}] contains NaN or Infinity. Data integrity violation.",
                        i
                    ),
                });
            }
        }

        let url = format!("{}/forecast", self.endpoint);
        let payload = json!({
            "series": series,
            "horizon": horizon,
            "context_length": config.context_length,
            "quantiles": config.quantiles
        });

        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.auth_token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!(
                    "TimesFM Sidecar request failed (timeout={}s): {}",
                    SIDECAR_REQUEST_TIMEOUT.as_secs(),
                    e
                ),
            })?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                // RT-6 FIX: Truncate error body to prevent information leakage
                // RT3-1 FIX: Use char boundary to avoid panic on multi-byte UTF-8 truncation
                reason: format!("TimesFM error [{}]: {}", status, truncate_utf8(&body, 256)),
            });
        }

        let result: ForecastResult = res.json().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to parse TimesFM forecast response: {}", e),
        })?;

        // RT-5 FIX: Validate response shape
        if result.point_forecast.is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "TimesFM returned empty point_forecast array.".into(),
            });
        }

        Ok(result)
    }

    async fn detect_anomaly(
        &self,
        historical: Vec<f64>,
        recent: Vec<f64>,
        threshold_sigma: f64,
    ) -> Result<AnomalyResult, AiomeError> {
        let horizon = recent.len();
        if horizon == 0 {
            return Err(AiomeError::Infrastructure {
                reason: "Recent validation window cannot be empty for anomaly detection.".into(),
            });
        }
        // RT-4 FIX: Guard against NaN in historical data
        if historical.is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "Historical data cannot be empty for anomaly detection.".into(),
            });
        }
        if historical.iter().any(|v| v.is_nan() || v.is_infinite()) {
            return Err(AiomeError::Infrastructure {
                reason: "Historical data contains NaN or Infinity.".into(),
            });
        }
        // RT3-4 FIX: Guard against NaN in recent data too
        if recent.iter().any(|v| v.is_nan() || v.is_infinite()) {
            return Err(AiomeError::Infrastructure {
                reason: "Recent data contains NaN or Infinity.".into(),
            });
        }

        let forecast_res = self
            .forecast(
                vec![historical.clone()],
                horizon,
                ForecastConfig {
                    context_length: 512,
                    quantiles: false,
                },
            )
            .await?;

        // RT-5 FIX: Bounds-checked access
        let predicted =
            forecast_res
                .point_forecast
                .first()
                .ok_or_else(|| AiomeError::Infrastructure {
                    reason: "TimesFM returned no forecast series for anomaly detection.".into(),
                })?;

        if predicted.len() < horizon {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "TimesFM returned {} predictions but {} were expected.",
                    predicted.len(),
                    horizon
                ),
            });
        }

        let mean = historical.iter().sum::<f64>() / historical.len() as f64;
        let variance =
            historical.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / historical.len() as f64;
        let std_dev = variance.sqrt().max(0.1);

        let mut max_err_sigma = 0.0_f64;

        for (i, p) in predicted.iter().enumerate() {
            // RT3-5 FIX: bounds-checked access to recent
            let actual = match recent.get(i) {
                Some(v) => *v,
                None => break, // predicted is longer than recent; extra predictions are ignored
            };
            let err = (actual - p).abs();
            let sigma = err / std_dev;
            if sigma > max_err_sigma {
                max_err_sigma = sigma;
            }
        }

        Ok(AnomalyResult {
            is_anomaly: max_err_sigma >= threshold_sigma,
            deviation_sigma: max_err_sigma,
            predicted_values: predicted.clone(),
        })
    }

    fn name(&self) -> &str {
        "TimesFM"
    }
}

/// Safely truncate a UTF-8 string to at most `max_bytes` bytes
/// without splitting a multi-byte character.
fn truncate_utf8(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // Find the last char boundary at or before max_bytes
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
