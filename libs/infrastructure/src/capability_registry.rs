/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::traits::CapabilityProvider;
use async_trait::async_trait;
use std::sync::Arc;

/// 能力（Capability）を統合管理するレジストリ
pub struct CapabilityRegistry {
    providers: Vec<Arc<dyn CapabilityProvider>>,
}

impl CapabilityRegistry {
    /// 新しい CapabilityRegistry を作成
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// プロバイダーを登録
    pub fn register(&mut self, provider: Arc<dyn CapabilityProvider>) {
        self.providers.push(provider);
    }

    /// 全コンポーネントの能力要約を取得 (Progressive Disclosure)
    pub fn get_capabilities_summary(&self) -> Vec<serde_json::Value> {
        self.providers
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.capability_name(),
                    "description": p.capability_description(),
                })
            })
            .collect()
    }

    /// 指定したコンポーネントの詳細な能力仕様を取得
    pub fn get_capability_detail(&self, name: &str) -> Option<serde_json::Value> {
        self.providers
            .iter()
            .find(|p| p.capability_name() == name)
            .map(|p| {
                serde_json::json!({
                    "name": p.capability_name(),
                    "description": p.capability_description(),
                    "schema": p.capability_schema(),
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct MockProvider;
    #[async_trait]
    impl CapabilityProvider for MockProvider {
        fn capability_name(&self) -> &str {
            "MockEngine"
        }
        fn capability_description(&self) -> &str {
            "A mock engine for testing"
        }
    }

    #[test]
    fn test_capability_registration_and_summary() {
        let mut registry = CapabilityRegistry::new();
        registry.register(Arc::new(MockProvider));

        let summary = registry.get_capabilities_summary();
        assert_eq!(summary.len(), 1, "Should have 1 registered provider");
        assert_eq!(summary[0]["name"], "MockEngine");
        assert_eq!(summary[0]["description"], "A mock engine for testing");
    }

    #[test]
    fn test_capability_detail_lookup() {
        let mut registry = CapabilityRegistry::new();
        registry.register(Arc::new(MockProvider));

        let detail = registry.get_capability_detail("MockEngine").unwrap();
        assert_eq!(detail["name"], "MockEngine");
        assert_eq!(detail["schema"], json!({}));

        // Non-existent
        assert!(registry.get_capability_detail("NoSuchEngine").is_none());
    }
}
