/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::plugin::AiomePlugin;
use axum::Router;
use std::sync::Arc;
use tracing::{info, warn};

pub struct PluginRegistry {
    plugins: Vec<Arc<dyn AiomePlugin>>,
    /// JWT 外 S2S（InProcess）。`nest_service("/internal", …)` 用。
    s2s_router: Option<Router>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            s2s_router: None,
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

    /// OP-088 P1: JWT 外に載せる S2S ルータを保持する。
    pub fn set_s2s_router(&mut self, router: Router) {
        self.s2s_router = Some(router);
    }

    pub fn take_s2s_router(&mut self) -> Option<Router> {
        self.s2s_router.take()
    }

    /// OP-088 P5-a: nest 用に残しつつ AppState へ渡す clone（二重 `s2s_internal_service` 禁止）。
    pub fn clone_s2s_router(&self) -> Option<Router> {
        self.s2s_router.clone()
    }

    /// `Router<()>` の Plugin ルート（`with_state` 後に JWT 付きで merge する）。
    pub fn plugin_unit_routers(&self) -> Vec<Router> {
        let mut out = Vec::new();
        for plugin in &self.plugins {
            if let Some(opaque_router) = plugin.routes() {
                if let Some(plugin_router) = opaque_router.downcast_ref::<Router>() {
                    info!(
                        "🛣️  Collecting unit-state routes from plugin: {}",
                        plugin.name()
                    );
                    out.push(plugin_router.clone());
                } else {
                    warn!(
                        "⚠️  Plugin {} returned a router that is not an axum::Router<()>",
                        plugin.name()
                    );
                }
            }
        }
        out
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
                        "⚠️  Plugin {} returned a router that is not an axum::Router<{}> (use plugin_unit_routers after with_state)",
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

    pub fn get_agent_hooks(&self) -> Vec<Arc<dyn aiome_core_contracts::security::AgentHook>> {
        self.plugins.iter().flat_map(|p| p.agent_hooks()).collect()
    }

    /// OP-088 P5-c: 登録済み Plugin の CommerceEngine（InProcess Bridge 正本）。
    pub fn commerce_engine(
        &self,
    ) -> Option<Arc<dyn aiome_core_contracts::commerce::CommerceEngine>> {
        self.plugins.iter().find_map(|p| p.commerce_engine())
    }

    /// C1' 後の二重 `create_plugin` 防止。
    pub fn has_s2s_router(&self) -> bool {
        self.s2s_router.is_some()
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
