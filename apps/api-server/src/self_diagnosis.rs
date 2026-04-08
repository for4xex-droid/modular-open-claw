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
            tracing::warn!("⚠️ [Self-Diagnosis] Docker daemon is NOT reachable. Some features (like Docker-based Actions) may not work.");
            // We no longer bail here because Docker is not strictly required for users using Cloud LLMs or native actions.
        }
    }

    // 4. Ollama Availability Check (Best Effort)
    let client = aiome_core::http::get_http_client().clone();
    let ollama_host = config.ollama_host.trim_end_matches('/');
    let ollama_url = format!("{}/api/version", ollama_host);
    match client
        .get(&ollama_url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
    {
        Ok(res) if res.status().is_success() => {
            info!(
                "✅ [Self-Diagnosis] Ollama daemon discovered at {}. Local LLM ready.",
                ollama_host
            );

            // Check if model exists
            let tags_url = format!("{}/api/tags", ollama_host);
            if let Ok(res) = client
                .get(&tags_url)
                .timeout(std::time::Duration::from_secs(2))
                .send()
                .await
            {
                if let Ok(json) = res.json::<serde_json::Value>().await {
                    let has_model = json
                        .get("models")
                        .and_then(|m| m.as_array())
                        .map(|arr| {
                            arr.iter().any(|v| {
                                v.get("name").and_then(|n| n.as_str()) == Some(&config.ollama_model)
                            })
                        })
                        .unwrap_or(false);

                    if !has_model {
                        tracing::warn!("⚠️ [Self-Diagnosis] Configured model '{}' is not installed in Ollama. It will be downloaded during the Smart Onboarding phase.", config.ollama_model);
                    } else {
                        info!(
                            "✅ [Self-Diagnosis] Configured model '{}' is ready.",
                            config.ollama_model
                        );
                    }
                }
            }
        }
        _ => {
            tracing::warn!("⚠️ [Self-Diagnosis] Ollama daemon is NOT reachable at {}. If you don't use Cloud LLMs, please install Ollama.", ollama_host);
        }
    }

    info!("✨ [Self-Diagnosis] System is perfectly healthy. Ignition sequence starting! 🚀");
    Ok(())
}
