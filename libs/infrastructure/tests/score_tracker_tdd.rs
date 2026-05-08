use aiome_core::error::AiomeError;
use aiome_core_contracts::forecast::{
    AnomalyResult, ForecastConfig, ForecastProvider, ForecastResult,
};
use async_trait::async_trait;
use infrastructure::db::DatabasePool;
use infrastructure::score_tracker::ScoreTracker;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;

struct TestForecastProvider {
    predictions: Vec<f64>,
}

#[async_trait]
impl ForecastProvider for TestForecastProvider {
    async fn forecast(
        &self,
        series: Vec<Vec<f64>>,
        _horizon: usize,
        _config: ForecastConfig,
    ) -> Result<ForecastResult, AiomeError> {
        let point_forecast: Vec<Vec<f64>> =
            series.iter().map(|_| self.predictions.clone()).collect();
        Ok(ForecastResult {
            point_forecast,
            quantile_forecast: None,
            model_version: "test".to_string(),
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
            deviation_sigma: 0.0,
            predicted_values: vec![],
        })
    }

    fn name(&self) -> &str {
        "TestForecast"
    }
}

async fn setup_db() -> DatabasePool {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory sqlite pool");

    let sql = r#"
    CREATE TABLE IF NOT EXISTS score_snapshots (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        snapshot_date TEXT NOT NULL,
        metric_name TEXT NOT NULL,
        metric_value REAL NOT NULL,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE UNIQUE INDEX IF NOT EXISTS idx_score_snapshot_date_metric 
        ON score_snapshots(snapshot_date, metric_name);
    "#;

    sqlx::query(sql)
        .execute(&pool)
        .await
        .expect("Failed to create table");

    DatabasePool::Sqlite(pool)
}

async fn insert_history(pool: &DatabasePool, metric: &str, values: &[f64]) {
    for (i, val) in values.iter().enumerate() {
        let date = format!("2026-05-{:02}", i + 1); // Mock sequential dates
        let q = format!(
            "INSERT INTO score_snapshots (snapshot_date, metric_name, metric_value) VALUES ({}, {}, {})",
            pool.ph(0), pool.ph(1), pool.ph(2)
        );
        infrastructure::sql_exec!(pool, &q, date, metric.to_string(), *val)
            .expect("Failed to insert mock data");
    }
}

#[tokio::test]
async fn test_detect_plateau_insufficient_data() {
    let pool = setup_db().await;
    let provider = Arc::new(TestForecastProvider {
        predictions: vec![1.0, 1.0],
    });
    let tracker = ScoreTracker::new(Some(provider), pool.clone());

    // Insert only 2 data points, but require 5
    insert_history(&pool, "exp", &[10.0, 20.0]).await;

    let result = tracker
        .detect_plateau("exp", 5)
        .await
        .expect("detect_plateau failed");
    assert!(
        result.is_none(),
        "Should return None due to insufficient data"
    );
}

#[tokio::test]
async fn test_detect_plateau_stagnating() {
    let pool = setup_db().await;
    // Current growth will be calculated from last 7 values
    let history = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0];
    insert_history(&pool, "exp", &history).await;

    // Current growth is (70 - 10) / 7 = ~8.57
    // We predict flat growth (stagnation) -> 70.0, 70.0, 70.0
    let provider = Arc::new(TestForecastProvider {
        predictions: vec![70.0, 70.0, 70.0],
    });
    let tracker = ScoreTracker::new(Some(provider), pool.clone());

    let report = tracker
        .detect_plateau("exp", 5)
        .await
        .expect("detect_plateau failed")
        .expect("Should return report");

    assert!(
        report.is_stagnating,
        "Should be detected as stagnating due to flat predictions"
    );
    assert_eq!(report.current_growth_rate, (70.0 - 10.0) / 7.0);
    assert_eq!(report.predicted_growth_rate, 0.0);
}

#[tokio::test]
async fn test_detect_plateau_healthy() {
    let pool = setup_db().await;
    let history = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0];
    insert_history(&pool, "exp", &history).await;

    // Predict healthy growth -> 80.0, 90.0, 100.0
    // Predicted growth = (100 - 80) / 3 = 6.66
    let provider = Arc::new(TestForecastProvider {
        predictions: vec![80.0, 90.0, 100.0],
    });
    let tracker = ScoreTracker::new(Some(provider), pool.clone());

    let report = tracker
        .detect_plateau("exp", 5)
        .await
        .expect("detect_plateau failed")
        .expect("Should return report");

    assert!(!report.is_stagnating, "Should be healthy");
    assert_eq!(report.predicted_growth_rate, (100.0 - 80.0) / 3.0);
}

#[tokio::test]
async fn test_fetch_metric_history_filters_nan() {
    let pool = setup_db().await;

    // Insert valid data
    insert_history(&pool, "exp", &[10.0, 20.0]).await;

    // Insert Infinity directly via SQL to bypass sqlx parameter binding limits for SQLite
    let q = "INSERT INTO score_snapshots (snapshot_date, metric_name, metric_value) VALUES ('2026-05-03', 'exp', 9e999)";
    sqlx::query(q)
        .execute(match &pool {
            DatabasePool::Sqlite(p) => p,
            _ => panic!("Expected sqlite"),
        })
        .await
        .expect("Failed to insert Infinity");

    // Insert valid data
    let q2 = "INSERT INTO score_snapshots (snapshot_date, metric_name, metric_value) VALUES ('2026-05-04', 'exp', 30.0)";
    sqlx::query(q2)
        .execute(match &pool {
            DatabasePool::Sqlite(p) => p,
            _ => panic!("Expected sqlite"),
        })
        .await
        .expect("Failed to insert valid data");

    let tracker = ScoreTracker::new(None, pool.clone());
    let history = tracker
        .fetch_metric_history("exp")
        .await
        .expect("fetch_metric_history failed");

    // We expect 10.0, 20.0, 30.0 (Infinity is filtered)
    assert_eq!(history.len(), 3);
    assert_eq!(history, vec![10.0, 20.0, 30.0]);
}
