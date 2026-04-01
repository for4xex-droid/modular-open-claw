/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::db::DatabasePool;
use aiome_contracts::forecast::{ForecastConfig, ForecastProvider};
use aiome_contracts::traits::AgentEvolver;
use aiome_core::error::AiomeError;
use chrono::{Datelike, Utc};
use sqlx::Row;
use std::sync::Arc;
use tracing::{info, warn};

pub struct PlateauReport {
    pub is_stagnating: bool,
    pub metric_name: String,
    pub current_growth_rate: f64,
    pub predicted_growth_rate: f64,
}

/// ScoreTracker
/// Phase 3D: Monitors Karma/EXP growth to detect Score Plateau (stagnation).
pub struct ScoreTracker {
    forecast_provider: Option<Arc<dyn ForecastProvider>>,
    pool: DatabasePool,
}

impl ScoreTracker {
    pub fn new(forecast_provider: Option<Arc<dyn ForecastProvider>>, pool: DatabasePool) -> Self {
        Self {
            forecast_provider,
            pool,
        }
    }

    /// Record a daily snapshot of the metrics for forecasting
    pub async fn record_daily_snapshot(
        &self,
        agent_evolver: &Arc<dyn AgentEvolver>,
    ) -> Result<(), AiomeError> {
        let stats = agent_evolver.get_agent_stats().await?;
        let today = Utc::now().format("%Y-%m-%d").to_string();

        let metrics = vec![
            ("exp", stats.exp as f64),
            ("resonance", stats.resonance as f64),
            ("creativity", stats.creativity as f64),
        ];

        for (name, val) in metrics {
            // RT-4 FIX: Never store NaN or Infinity in the snapshot table.
            // This prevents poisoned data from cascading into TimesFM predictions.
            if val.is_nan() || val.is_infinite() {
                warn!(
                    "⚠️ [ScoreTracker] Skipping NaN/Inf value for metric '{}' (val={}).",
                    name, val
                );
                continue;
            }

            let q = format!(
                "INSERT INTO score_snapshots (snapshot_date, metric_name, metric_value) 
                 VALUES ({0}, {1}, {2}) 
                 ON CONFLICT(snapshot_date, metric_name) DO UPDATE SET metric_value = excluded.metric_value",
                self.pool.ph(0), self.pool.ph(1), self.pool.ph(2)
            );
            crate::sql_exec!(&self.pool, &q, today.clone(), name.to_string(), val)?;
        }

        info!("ScoreTracker: Daily snapshot recorded for {}.", today);
        Ok(())
    }

    /// Check if the metric growth is plateauing
    pub async fn detect_plateau(
        &self,
        metric: &str,
        min_days: i64,
    ) -> Result<Option<PlateauReport>, AiomeError> {
        let provider = match &self.forecast_provider {
            Some(p) => p,
            None => return Ok(None),
        };

        // 1. Fetch history order by date ascending
        let q = format!(
            "SELECT metric_value FROM score_snapshots WHERE metric_name = {} ORDER BY snapshot_date ASC",
            self.pool.ph(0)
        );

        let mut history: Vec<f64> = Vec::new();

        match &self.pool {
            DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(&q)
                    .bind(metric)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for row in rows {
                    history.push(row.get::<f64, _>("metric_value"));
                }
            }
            DatabasePool::Postgres(p) => {
                let rows = sqlx::query(&q)
                    .bind(metric)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for row in rows {
                    history.push(row.get::<f64, _>("metric_value"));
                }
            }
        }

        if history.len() < min_days as usize {
            info!(
                "ScoreTracker: Not enough data points ({}/{}) to detect plateau for {}",
                history.len(),
                min_days,
                metric
            );
            return Ok(None);
        }

        // RT-4 FIX: Filter out any NaN/Inf that may have existed before the guard was added
        history.retain(|v| v.is_finite());
        if history.len() < min_days as usize {
            warn!(
                "ScoreTracker: After NaN/Inf filtering, insufficient data points for {}",
                metric
            );
            return Ok(None);
        }

        // 2. Calculate current moving average growth (slope of last 7 days)
        let n = history.len();
        let last_7 = if n > 7 {
            &history[n - 7..]
        } else {
            &history[..]
        };
        let current_growth = if last_7.len() > 1 {
            let first = last_7.first().copied().unwrap_or(0.0);
            let last = last_7.last().copied().unwrap_or(0.0);
            (last - first) / (last_7.len() as f64)
        } else {
            0.0
        };

        // 3. Forecast next 7 days using TimesFM
        let horizon = 7;
        let p_res = provider
            .forecast(
                vec![history],
                horizon,
                ForecastConfig {
                    context_length: 512,
                    quantiles: false,
                },
            )
            .await?;

        // RT-5 FIX: Bounds-checked access to point_forecast
        let predicted = match p_res.point_forecast.first() {
            Some(p) if !p.is_empty() => p,
            _ => {
                warn!("⚠️ [ScoreTracker] TimesFM returned empty predictions for {}. Skipping plateau check.", metric);
                return Ok(None);
            }
        };

        let predicted_growth = if predicted.len() > 1 {
            let first = predicted.first().copied().unwrap_or(0.0);
            let last = predicted.last().copied().unwrap_or(0.0);
            (last - first) / (predicted.len() as f64)
        } else {
            0.0
        };

        // 4. Stagnation Criteria
        // If predicted growth is near zero OR significantly lower than current growth
        let is_stagnating = predicted_growth < 0.1 || (predicted_growth < (current_growth * 0.2));

        if is_stagnating {
            warn!("📉 [ScoreTracker] Plateau Detected for {}: Current Growth = {:.2}/day, Predicted = {:.2}/day", metric, current_growth, predicted_growth);
        } else {
            info!("📈 [ScoreTracker] Growth healthy for {}: Current Growth = {:.2}/day, Predicted = {:.2}/day", metric, current_growth, predicted_growth);
        }

        Ok(Some(PlateauReport {
            is_stagnating,
            metric_name: metric.to_string(),
            current_growth_rate: current_growth,
            predicted_growth_rate: predicted_growth,
        }))
    }
}
