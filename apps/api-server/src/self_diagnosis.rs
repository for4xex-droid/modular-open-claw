/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use anyhow::Result;
use tracing::{error, info};

/// システム起動前の自己診断を実行する (Phase A-3)
pub async fn run_startup_diagnosis(config: &shared::config::AiomeConfig) -> Result<()> {
    info!("🔍 [Self-Diagnosis] Running startup checks...");

    // 1. App Directory Check
    let rw_test_path = config.resolver.resolve(".diagnosis_test");
    if let Err(e) = tokio::fs::write(&rw_test_path, "diagnosis_ok").await {
        error!(
            "🚨 [Self-Diagnosis] App directory is NOT writable at {}: {}",
            rw_test_path.display(),
            e
        );
        anyhow::bail!("Directory write failed: {}", rw_test_path.display());
    }
    let _ = tokio::fs::remove_file(&rw_test_path).await;
    info!("✅ [Self-Diagnosis] Workspace directory is perfectly writable.");

    // 2. DB Connection Check
    if config.db_path.starts_with("postgres:") || config.db_path.starts_with("postgresql:") {
        match sqlx::PgPool::connect(&config.db_path).await {
            Ok(pool) => {
                pool.close().await;
            }
            Err(e) => {
                error!(
                    "🚨 [Self-Diagnosis] PostgreSQL database is unreachable: {}",
                    e
                );
                anyhow::bail!("Postgres connection failed");
            }
        }
    } else {
        use std::str::FromStr;
        let p = config.db_path.replace("sqlite://", "sqlite:");
        let opts = sqlx::sqlite::SqliteConnectOptions::from_str(&p)
            .unwrap_or_else(|_| sqlx::sqlite::SqliteConnectOptions::new().filename(&p))
            .create_if_missing(true);
        match sqlx::sqlite::SqlitePoolOptions::new()
            .connect_with(opts)
            .await
        {
            Ok(pool) => {
                pool.close().await;
            }
            Err(e) => {
                error!("🚨 [Self-Diagnosis] SQLite database is unreachable: {}", e);
                anyhow::bail!("SQLite connection failed");
            }
        }
    }
    info!("✅ [Self-Diagnosis] Database connection successfully established.");

    // 3. Docker API Ping
    let docker_out = tokio::process::Command::new("docker")
        .arg("info")
        .output()
        .await;

    match docker_out {
        Ok(out) if out.status.success() => {
            info!("✅ [Self-Diagnosis] Docker daemon is perfectly reachable.");
        }
        _ => {
            error!("🚨 [Self-Diagnosis] Docker daemon is NOT reachable. Please ensure Docker is running and accessible.");
            anyhow::bail!("Docker unreachable");
        }
    }

    info!("✨ [Self-Diagnosis] System is perfectly healthy. Ignition sequence starting! 🚀");
    Ok(())
}
