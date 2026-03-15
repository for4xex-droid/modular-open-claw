use std::sync::Arc;
use axum::Router;
use aiome_interface::plugin::AiomePlugin;
use tracing::{info, warn};

pub struct PluginRegistry {
    plugins: Vec<Arc<dyn AiomePlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    pub fn register(&mut self, plugin: Arc<dyn AiomePlugin>) {
        info!("🔌 Registering plugin: {} v{}", plugin.name(), plugin.version());
        self.plugins.push(plugin);
    }

    pub fn merge_routes(&self, mut router: Router) -> Router {
        for plugin in &self.plugins {
            if let Some(plugin_router) = plugin.routes() {
                info!("🛣️  Merging routes from plugin: {}", plugin.name());
                router = router.merge(plugin_router);
            }
        }
        router
    }

    pub fn registered_tools(&self) -> Vec<String> {
        self.plugins.iter().flat_map(|p| p.registered_tools()).collect()
    }

    pub fn check_env_vars(&self) -> bool {
        let mut missing = false;
        for plugin in &self.plugins {
            for var in plugin.required_env_vars() {
                if std::env::var(&var).is_err() {
                    warn!("⚠️  Plugin {} requires environment variable {} which is not set", plugin.name(), var);
                    missing = true;
                }
            }
        }
        !missing
    }
}
