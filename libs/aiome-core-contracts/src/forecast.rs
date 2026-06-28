/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastConfig {
    pub context_length: usize,
    pub quantiles: bool,
}

impl Default for ForecastConfig {
    fn default() -> Self {
        Self {
            context_length: 512,
            quantiles: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastResult {
    /// Point forecasts for each series. Dimension: [num_series, horizon]
    pub point_forecast: Vec<Vec<f64>>,
    /// Quantile forecasts for each series. Dimension: [num_series, horizon, num_quantiles]
    pub quantile_forecast: Option<Vec<Vec<Vec<f64>>>>,
    pub model_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyResult {
    pub is_anomaly: bool,
    pub deviation_sigma: f64,
    pub predicted_values: Vec<f64>,
}

#[async_trait]
pub trait ForecastProvider: Send + Sync {
    /// Forecast future values from historical time series data.
    async fn forecast(
        &self,
        series: Vec<Vec<f64>>,
        horizon: usize,
        config: ForecastConfig,
    ) -> Result<ForecastResult, crate::error::AiomeError>;

    /// Detect anomalies by comparing recent values against forecasted expectation based on history.
    async fn detect_anomaly(
        &self,
        historical: Vec<f64>,
        recent: Vec<f64>,
        threshold_sigma: f64,
    ) -> Result<AnomalyResult, crate::error::AiomeError>;

    fn name(&self) -> &str;
}
