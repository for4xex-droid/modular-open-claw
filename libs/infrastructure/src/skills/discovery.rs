/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::skills::WasmSkillManager;
use aiome_contracts::error::AiomeError;
use aiome_contracts::traits::ToolDiscoveryEngine;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// デフォルトのツール発見エンジン
pub struct DefaultToolDiscoveryEngine {
    skill_manager: Arc<WasmSkillManager>,
}

impl DefaultToolDiscoveryEngine {
    /// 新しいインスタンスを生成する
    pub fn new(skill_manager: Arc<WasmSkillManager>) -> Self {
        Self { skill_manager }
    }
}

#[async_trait]
impl ToolDiscoveryEngine for DefaultToolDiscoveryEngine {
    async fn discover_tools(&self) -> Result<Vec<serde_json::Value>, AiomeError> {
        let skills = self.skill_manager.list_skills_with_metadata();
        let mut tools = Vec::new();

        for meta in skills {
            tools.push(json!({
                "name": meta.name,
                "description": meta.description,
                "capabilities": meta.capabilities,
                "inputs": meta.inputs,
                "outputs": meta.outputs,
            }));
        }

        Ok(tools)
    }

    async fn suggest_tools(&self, instruction: &str) -> Result<Vec<String>, AiomeError> {
        // Phase 13-B: LLM based suggestion (currently simple keyword match)
        // In full implementation, this should call an LLM to match instruction with metadata
        let tools = self.discover_tools().await?;
        let mut suggestions = Vec::new();

        let instruction_lc = instruction.to_lowercase();
        for tool in tools {
            let name = tool["name"].as_str().unwrap_or_default();
            let desc = tool["description"].as_str().unwrap_or_default();

            if instruction_lc.contains(&name.to_lowercase())
                || instruction_lc.contains(&desc.to_lowercase())
            {
                suggestions.push(name.to_string());
            }
        }

        // Karma (教訓) ベースの探索も補完として利用可能 (G-3 への布石)
        Ok(suggestions)
    }
}
