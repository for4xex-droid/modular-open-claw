/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::security::BastionGuard;
use aiome_core::security::PermissionManifest;
use extism::{Manifest, Plugin, Wasm};
use serde_json::Value;
use tracing::warn;

use crate::constraint_checker::ActionHarness;

use std::sync::{Arc, Mutex, RwLock};

use std::collections::HashMap;

/// ハーネスの WASM バイナリをキャッシュし、重複読み込みやメモリ消費を抑える
pub struct HarnessCache {
    cache: RwLock<HashMap<String, Arc<[u8]>>>,
    plugins: RwLock<HashMap<String, Arc<tokio::sync::OnceCell<Arc<Mutex<Plugin>>>>>>,
}

impl Default for HarnessCache {
    fn default() -> Self {
        Self::new()
    }
}

impl HarnessCache {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            plugins: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, id: &str) -> Option<Arc<[u8]>> {
        let cache = self.cache.read().ok()?;
        cache.get(id).cloned()
    }

    pub fn set(&self, id: String, data: Arc<[u8]>) {
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(id, data);
        }
    }

    pub async fn get_or_create_plugin(
        &self,
        id: &str,
        wasm_data: &Arc<[u8]>,
    ) -> Option<Arc<Mutex<Plugin>>> {
        let cell_arc = {
            let plugins = self.plugins.read().ok()?;
            plugins.get(id).cloned()
        };

        let cell_arc = match cell_arc {
            Some(c) => c,
            None => {
                let mut plugins = self.plugins.write().ok()?;
                plugins
                    .entry(id.to_string())
                    .or_insert_with(|| Arc::new(tokio::sync::OnceCell::new()))
                    .clone()
            }
        };

        let wasm_clone = wasm_data.clone();

        let res = cell_arc
            .get_or_try_init(|| async move {
                tokio::task::spawn_blocking(move || {
                    let permissions = PermissionManifest::default();
                    let _guard = BastionGuard::new(permissions);

                    let manifest = Manifest::new([Wasm::data(wasm_clone.to_vec())])
                        .with_timeout(std::time::Duration::from_secs(5));

                    Plugin::new(&manifest, [], true)
                        .map(|p| Arc::new(Mutex::new(p)))
                        .map_err(|e| e.to_string())
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            })
            .await;

        match res {
            Ok(plugin) => Some(plugin.clone()),
            Err(e) => {
                warn!("Failed to create Plugin for harness {}: {}", id, e);
                if let Ok(mut plugins) = self.plugins.write() {
                    plugins.remove(id);
                }
                None
            }
        }
    }

    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
        if let Ok(mut plugins) = self.plugins.write() {
            plugins.clear();
        }
    }
}

/// LLM が生成したコードを WASM 隔離空間で実行するハーネス
pub struct WasmHarness {
    id: String,
    domain: String,
    description: String,
    plugin: Arc<Mutex<Plugin>>,
    severity: u8,
    agent_id: Option<uuid::Uuid>,
}

impl WasmHarness {
    /// 新しい WasmHarness を作成する
    pub fn new(
        id: impl Into<String>,
        domain: impl Into<String>,
        description: impl Into<String>,
        plugin: Arc<Mutex<Plugin>>,
        severity: u8,
        agent_id: Option<uuid::Uuid>,
    ) -> Self {
        Self {
            id: id.into(),
            domain: domain.into(),
            description: description.into(),
            plugin,
            severity,
            agent_id,
        }
    }
}

impl ActionHarness for WasmHarness {
    fn id(&self) -> &str {
        &self.id
    }

    fn agent_id(&self) -> Option<uuid::Uuid> {
        self.agent_id
    }

    fn is_legal_action(&self, action: &str, input: &Value, output: &Value) -> bool {
        let payload = serde_json::json!({
            "action": action,
            "input": input,
            "output": output,
        });

        match serde_json::to_vec(&payload) {
            Ok(input_bytes) => {
                let permissions = PermissionManifest::default();
                let _guard = BastionGuard::new(permissions);

                let mut plugin = match self.plugin.lock() {
                    Ok(p) => p,
                    Err(_) => return false,
                };

                match plugin.call::<&[u8], String>("is_legal_action", &input_bytes) {
                    Ok(res_str) => {
                        let trimmed = res_str.trim();
                        trimmed == "true" || trimmed == "1"
                    }
                    Err(e) => {
                        warn!("WasmHarness ({}): Execution failed - {}", self.domain, e);
                        false
                    }
                }
            }
            Err(_) => false,
        }
    }

    fn describe_constraint(&self) -> String {
        self.description.clone()
    }

    fn domain(&self) -> &str {
        &self.domain
    }

    fn severity(&self) -> u8 {
        self.severity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_wasm_harness_invalid_wasm() {
        let cache = HarnessCache::new();
        let wasm_data: Arc<[u8]> = vec![0, 1, 2, 3].into();
        assert!(cache
            .get_or_create_plugin("test_harness", &wasm_data)
            .await
            .is_none());
    }
}
