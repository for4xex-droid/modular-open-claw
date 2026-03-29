/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::forecast::{AnomalyResult, ForecastConfig, ForecastProvider, ForecastResult};
use aiome_core::error::AiomeError;
use async_trait::async_trait;

pub struct MockForecastProvider {}

#[async_trait]
impl ForecastProvider for MockForecastProvider {
    async fn forecast(
        &self,
        series: Vec<Vec<f64>>,
        horizon: usize,
        _config: ForecastConfig,
    ) -> Result<ForecastResult, AiomeError> {
        // RT3-3 FIX: Generate one forecast per input series, each of length `horizon`.
        // Previous implementation used `vec![vec![1.0; series.len()]]` which produced
        // wrong shape: 1 series with len=num_input_series instead of num_series with len=horizon.
        let point_forecast: Vec<Vec<f64>> = series.iter().map(|_| vec![1.0; horizon]).collect();
        Ok(ForecastResult {
            point_forecast,
            quantile_forecast: None,
            model_version: "mock".to_string(),
        })
    }

    async fn detect_anomaly(
        &self,
        _historical: Vec<f64>,
        _recent: Vec<f64>,
        _threshold_sigma: f64,
    ) -> Result<AnomalyResult, AiomeError> {
        Ok(AnomalyResult {
            is_anomaly: false,
            deviation_sigma: 0.1,
            predicted_values: vec![1.0, 1.0],
        })
    }

    fn name(&self) -> &str {
        "MockForecast"
    }
}
