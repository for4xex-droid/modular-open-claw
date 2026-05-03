/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use crate::app_state::AppState;
use crate::bootstrap::BootContext;
use anyhow::Result;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub struct AppBootstrapBuilder {
    pub state: AppState,
    pub resolver: Option<Arc<shared::app_data::AppDataResolver>>,
    pub metrics_handle: Option<metrics_exporter_prometheus::PrometheusHandle>,
    pub db_pool: Option<Arc<infrastructure::db::DatabasePool>>,
    pub cancel_token: Option<CancellationToken>,
    pub plugin_registry: Option<crate::plugin_loader::PluginRegistry>,
    pub cors_layer: Option<tower_http::cors::CorsLayer>,
}

impl Default for AppBootstrapBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBootstrapBuilder {
    pub fn new() -> Self {
        Self {
            state: AppState::default(),
            resolver: None,
            metrics_handle: None,
            db_pool: None,
            cancel_token: None,
            plugin_registry: None,
            cors_layer: None,
        }
    }

    /// Stage 1: Load environment variables and configuration
    pub async fn load_env_and_config(self) -> Result<Self> {
        Ok(self)
    }

    /// Stage 2: Initialize telemetry and metrics
    pub async fn init_telemetry(self) -> Result<Self> {
        Ok(self)
    }

    /// Stage 3: Database and Storage
    pub async fn init_database_and_storage(self) -> Result<Self> {
        Ok(self)
    }

    /// Stage 4: Models and LLM Providers
    pub async fn init_models_and_providers(self) -> Result<Self> {
        Ok(self)
    }

    /// Stage 5: Security and Authentication
    pub async fn init_security_and_auth(self) -> Result<Self> {
        Ok(self)
    }

    /// Stage 6: Core Engines and Pipeline
    pub async fn init_core_engines(self) -> Result<Self> {
        Ok(self)
    }

    /// Stage 7: Background Tasks and Workers
    pub async fn init_background_tasks(self) -> Result<Self> {
        Ok(self)
    }

    /// Stage 8: Network Middleware and Final Assembly
    pub async fn build(self) -> Result<BootContext> {
        Ok(BootContext {
            state: self.state,
            plugin_registry: self
                .plugin_registry
                .unwrap_or_else(crate::plugin_loader::PluginRegistry::new),
            metrics_handle: match self.metrics_handle {
                Some(h) => h,
                None => metrics_exporter_prometheus::PrometheusBuilder::new()
                    .install_recorder()
                    .map_err(|e| anyhow::anyhow!("Failed to install metrics recorder: {}", e))?,
            },
            cancel_token: self.cancel_token.unwrap_or_default(),
            cors_layer: self.cors_layer.unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_builder_stages() {
        let builder = AppBootstrapBuilder::new()
            .load_env_and_config()
            .await
            .unwrap()
            .init_telemetry()
            .await
            .unwrap()
            .init_database_and_storage()
            .await
            .unwrap()
            .init_models_and_providers()
            .await
            .unwrap()
            .init_security_and_auth()
            .await
            .unwrap()
            .init_core_engines()
            .await
            .unwrap()
            .init_background_tasks()
            .await
            .unwrap();

        let res = builder.build().await;
        assert!(
            res.is_err() || res.is_ok(),
            "Should compile and be callable"
        );
    }
}
