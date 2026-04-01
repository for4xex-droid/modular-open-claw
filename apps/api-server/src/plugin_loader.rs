/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_contracts::plugin::AiomePlugin;
use axum::Router;
use std::sync::Arc;
use tracing::{info, warn};

pub struct PluginRegistry {
    plugins: Vec<Arc<dyn AiomePlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn register(&mut self, plugin: Arc<dyn AiomePlugin>) {
        info!(
            "🔌 Registering plugin: {} v{}",
            plugin.name(),
            plugin.version()
        );
        self.plugins.push(plugin);
    }

    pub fn merge_routes<S>(&self, mut router: Router<S>) -> Router<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        for plugin in &self.plugins {
            if let Some(opaque_router) = plugin.routes() {
                if let Some(plugin_router) = opaque_router.downcast_ref::<Router<S>>() {
                    info!("🛣️  Merging routes from plugin: {}", plugin.name());
                    router = router.merge(plugin_router.clone());
                } else {
                    warn!(
                        "⚠️  Plugin {} returned a router that is not an axum::Router<{}>",
                        plugin.name(),
                        std::any::type_name::<S>()
                    );
                }
            }
        }
        router
    }

    pub fn registered_tools(&self) -> Vec<String> {
        self.plugins
            .iter()
            .flat_map(|p| p.registered_tools())
            .collect()
    }

    pub fn check_env_vars(&self) -> bool {
        let mut missing = false;
        for plugin in &self.plugins {
            for var in plugin.required_env_vars() {
                if std::env::var(&var).is_err() {
                    warn!(
                        "⚠️  Plugin {} requires environment variable {} which is not set",
                        plugin.name(),
                        var
                    );
                    missing = true;
                }
            }
        }
        !missing
    }
}
