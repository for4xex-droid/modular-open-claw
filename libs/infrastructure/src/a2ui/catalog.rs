/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{error, warn};

#[derive(Debug, Clone)]
pub struct AiomeCatalog {
    pub name: String,
    components: HashMap<String, Value>,
}

impl Default for AiomeCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl AiomeCatalog {
    pub fn new() -> Self {
        Self {
            name: "AiomeCore".to_string(),
            components: HashMap::new(),
        }
    }

    pub fn register_component(&mut self, name: &str, schema: Value) {
        if self.components.contains_key(name) {
            warn!(
                "⚠️ [AiomeCatalog] Overwriting existing A2UI component: {}",
                name
            );
        }
        self.components.insert(name.to_string(), schema);
    }

    /// 登録済みコンポーネント数を返す
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// 指定名のコンポーネントが登録済みか確認
    pub fn has_component(&self, name: &str) -> bool {
        self.components.contains_key(name)
    }

    pub fn to_prompt_schema(&self) -> String {
        if self.components.is_empty() {
            return String::new();
        }

        let schema_json = json!({
            "catalog": self.name,
            "components": self.components
        });

        let stringified = match serde_json::to_string_pretty(&schema_json) {
            Ok(s) => s,
            Err(e) => {
                error!("🚨 [AiomeCatalog] Failed to serialize A2UI catalog schema: {e}");
                return String::new();
            }
        };

        // UTF-8 バイト安全な truncate（マルチバイト文字境界パニック防止）
        let truncated = if stringified.len() > 8000 {
            let safe_boundary = stringified
                .char_indices()
                .map(|(i, _)| i)
                .take_while(|&i| i < 8000)
                .last()
                .unwrap_or(0);
            format!("{}\n... (truncated)", &stringified[..safe_boundary])
        } else {
            stringified
        };

        format!("You can generate responsive UI components. Here is the JSON Schema for valid components you can output:\n{}", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_schema_generation() {
        let mut catalog = AiomeCatalog::new();
        catalog.register_component("taskApproval", json!({ "description": "string" }));

        let schema = catalog.to_prompt_schema();
        assert!(
            schema.contains("taskApproval"),
            "Catalog schema should contain registered components"
        );
    }

    #[test]
    fn test_empty_catalog_returns_empty_string() {
        let catalog = AiomeCatalog::new();
        let schema = catalog.to_prompt_schema();
        assert!(
            schema.is_empty(),
            "Empty catalog should return empty string"
        );
    }

    #[test]
    fn test_default_catalog_has_correct_name() {
        let catalog = AiomeCatalog::default();
        assert_eq!(
            catalog.name, "AiomeCore",
            "Default catalog name should be AiomeCore, not empty string"
        );
    }

    #[test]
    fn test_large_catalog_is_safely_truncated() {
        let mut catalog = AiomeCatalog::new();
        // 大量のコンポーネントを登録して 8000 バイト超を強制する
        for i in 0..200 {
            let long_name = format!("component_{i}_abcdefghijklmnopqrstuvwxyz");
            catalog.register_component(
                &long_name,
                json!({ "prop_a": "string",  "prop_b": "number" }),
            );
        }
        let schema = catalog.to_prompt_schema();
        // Panicking truncate でなく安全に完了することを確認
        assert!(
            schema.contains("... (truncated)"),
            "Large catalog must be truncated with indicator"
        );
        // 有効な UTF-8 文字列であることを確認（panic しないことが目的）
        assert!(
            std::str::from_utf8(schema.as_bytes()).is_ok(),
            "Truncated schema must be valid UTF-8"
        );
    }

    #[test]
    fn test_component_count_and_has_component() {
        let mut catalog = AiomeCatalog::new();
        assert_eq!(catalog.component_count(), 0);
        assert!(!catalog.has_component("taskApproval"));

        catalog.register_component("taskApproval", json!({"description": "string"}));
        assert_eq!(catalog.component_count(), 1);
        assert!(catalog.has_component("taskApproval"));
        assert!(!catalog.has_component("nonExistent"));
    }
}
