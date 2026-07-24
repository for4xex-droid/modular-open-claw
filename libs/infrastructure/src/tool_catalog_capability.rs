/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! Tool catalog → [`CapabilityProvider`] adapter (OP-092 / ADR-020).
//!
//! Catalog listing **reuses** [`WasmSkillManager::list_skills`] /
//! [`WasmSkillManager::list_skills_with_metadata`]. No independent scan.

use crate::skills::WasmSkillManager;
use aiome_core_contracts::traits::CapabilityProvider;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Max tools embedded in `capability_schema` (remainder reported via `truncated`).
const MAX_TOOLS_IN_SCHEMA: usize = 50;

/// Progressive Disclosure provider for the local WASM/tool catalog.
pub struct ToolCatalogCapabilityProvider {
    skill_manager: Arc<WasmSkillManager>,
}

impl ToolCatalogCapabilityProvider {
    /// Bind to the same [`WasmSkillManager`] instance used by bootstrap / discovery.
    pub fn new(skill_manager: Arc<WasmSkillManager>) -> Self {
        Self { skill_manager }
    }
}

#[async_trait]
impl CapabilityProvider for ToolCatalogCapabilityProvider {
    fn capability_name(&self) -> &str {
        "tool_catalog"
    }

    fn capability_description(&self) -> &str {
        "Discoverable local WASM tool catalog (list_skills / list_skills_with_metadata)"
    }

    fn capability_schema(&self) -> serde_json::Value {
        let all = self.skill_manager.list_skills_with_metadata();
        // Bare names cover skills without .meta.json (list_skills reuse).
        let names = self.skill_manager.list_skills();
        let tool_count = all.len().max(names.len());
        let truncated = tool_count > MAX_TOOLS_IN_SCHEMA;
        let tools: Vec<serde_json::Value> = all
            .into_iter()
            .take(MAX_TOOLS_IN_SCHEMA)
            .map(|m| {
                json!({
                    "name": m.name,
                    "description": m.description,
                    "capabilities": m.capabilities,
                })
            })
            .collect();
        let skill_names: Vec<String> = names.into_iter().take(MAX_TOOLS_IN_SCHEMA).collect();
        json!({
            "tool_count": tool_count,
            "truncated": truncated,
            "skill_names": skill_names,
            "tools": tools,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability_registry::CapabilityRegistry;
    use std::fs;
    use tempfile::TempDir;

    fn manager_with_fake_skill(dir: &TempDir, name: &str) -> Arc<WasmSkillManager> {
        let skills = dir.path().join("skills");
        fs::create_dir_all(&skills).unwrap();
        fs::write(skills.join(format!("{name}.wasm")), b"\0asm").unwrap();
        let root = dir.path().to_path_buf();
        Arc::new(WasmSkillManager::new(skills, root).expect("WasmSkillManager"))
    }

    #[test]
    fn positive_summary_and_detail_from_list_skills() {
        let tmp = TempDir::new().unwrap();
        let mgr = manager_with_fake_skill(&tmp, "demo_tool");
        let provider = Arc::new(ToolCatalogCapabilityProvider::new(mgr));
        let mut registry = CapabilityRegistry::new();
        registry.register(provider);

        assert_eq!(registry.provider_count(), 1);
        let summary = registry.get_capabilities_summary();
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0]["name"], "tool_catalog");

        let detail = registry.get_capability_detail("tool_catalog").unwrap();
        assert_eq!(detail["name"], "tool_catalog");
        let schema = &detail["schema"];
        assert!(schema["tool_count"].as_u64().unwrap() >= 1);
        let names = schema["skill_names"].as_array().unwrap();
        assert!(names.iter().any(|n| n == "demo_tool"));
    }

    #[test]
    fn negative_unknown_detail_and_empty_catalog() {
        let tmp = TempDir::new().unwrap();
        let skills = tmp.path().join("skills");
        fs::create_dir_all(&skills).unwrap();
        let root = tmp.path().to_path_buf();
        let mgr = Arc::new(WasmSkillManager::new(skills, root).expect("WasmSkillManager"));
        let mut registry = CapabilityRegistry::new();
        registry.register(Arc::new(ToolCatalogCapabilityProvider::new(mgr)));

        assert!(registry
            .get_capability_detail("no_such_capability")
            .is_none());
        let detail = registry.get_capability_detail("tool_catalog").unwrap();
        assert_eq!(detail["schema"]["tool_count"], 0);
        // Empty catalog must not panic (schema still valid JSON object).
        assert!(detail["schema"]["tools"].as_array().unwrap().is_empty());
    }
}
