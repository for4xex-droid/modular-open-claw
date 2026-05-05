/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::skills::WasmSkillManager;
use aiome_core::llm_provider::LlmProvider;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::traits::ToolDiscoveryEngine;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// デフォルトのツール発見エンジン
pub struct DefaultToolDiscoveryEngine {
    skill_manager: Arc<WasmSkillManager>,
    llm: Arc<dyn LlmProvider>,
}

impl DefaultToolDiscoveryEngine {
    /// 新しいインスタンスを生成する
    pub fn new(skill_manager: Arc<WasmSkillManager>, llm: Arc<dyn LlmProvider>) -> Self {
        Self { skill_manager, llm }
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
        // Phase 15: LLM based semantic discovery
        let tools = self.discover_tools().await?;
        if tools.is_empty() {
            return Ok(vec![]);
        }

        let tools_json = serde_json::to_string(&tools).unwrap_or_default();
        let prompt = format!(
            "Available Tools (JSON): {}\n\nUser Instruction: \"{}\"\n\n上記のツールリストの中から、ユーザーの指示を満たすために最適なツールを最大3つ選択し、ツール名のみをカンマ区切りで出力してください。該当するものがない場合は 'None' と出力してください。",
            tools_json, instruction
        );

        let response = self
            .llm
            .complete(
                &prompt,
                Some("You are a Tool Selector. Output only the comma-separated tool names."),
            )
            .await?;
        let content = response.content.trim();

        if content.to_lowercase() == "none" {
            return Ok(vec![]);
        }

        let suggestions: Vec<String> = content
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            // 実際に存在するツール名のみに絞り込む（幻覚防止）
            .filter(|s| tools.iter().any(|t| t["name"].as_str() == Some(s)))
            .collect();

        if !suggestions.is_empty() {
            tracing::info!("🧠 [ToolDiscovery] LLM suggested tools: {:?}", suggestions);
            return Ok(suggestions);
        }

        // フォールバック: キーワードマッチ
        let mut fallback_suggestions = Vec::new();
        let instruction_lc = instruction.to_lowercase();
        for tool in tools {
            let name = tool["name"].as_str().unwrap_or_default();
            let desc = tool["description"].as_str().unwrap_or_default();

            if instruction_lc.contains(&name.to_lowercase())
                || instruction_lc.contains(&desc.to_lowercase())
            {
                fallback_suggestions.push(name.to_string());
            }
        }
        Ok(fallback_suggestions)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::WasmSkillManager;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[derive(Debug)]
    struct MockLlm {
        response: String,
    }

    #[async_trait]
    impl aiome_core::llm_provider::LlmProvider for MockLlm {
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<aiome_core_contracts::llm::LlmResponse, AiomeError> {
            Ok(aiome_core_contracts::llm::LlmResponse {
                content: self.response.clone(),
                stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "mock"
        }
        async fn complete_with_cache(
            &self,
            _req: aiome_core_contracts::llm::LlmRequest,
        ) -> Result<aiome_core_contracts::llm::LlmResponse, AiomeError> {
            Err(AiomeError::Infrastructure {
                reason: "Not yet implemented".into(),
            })
        }
    }

    #[tokio::test]
    async fn test_suggest_tools_semantic_green() {
        let temp = tempdir().unwrap();
        let skills_dir = temp.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();

        // 1. fs_reader のメタデータを登録
        let meta = json!({
            "name": "fs_reader",
            "description": "Read content from local files",
            "capabilities": ["read"],
            "inputs": ["path"],
            "outputs": ["content"],
            "permissions": { "allow_filesystem_write": false, "allow_network": false, "allow_shell_execution": false, "allowed_domains": [] }
        });
        std::fs::write(skills_dir.join("fs_reader.meta.json"), meta.to_string()).unwrap();
        std::fs::write(skills_dir.join("fs_reader.wasm"), b"wasm").unwrap();

        let manager =
            Arc::new(WasmSkillManager::new(skills_dir, temp.path().to_path_buf()).unwrap());
        let mock_llm = Arc::new(MockLlm {
            response: "fs_reader".to_string(),
        });
        let engine = DefaultToolDiscoveryEngine::new(manager, mock_llm);

        // 2. セマンティックな要求
        let suggestions = engine
            .suggest_tools("I want to see what is inside a document")
            .await
            .unwrap();

        // 3. LLM ベースの推薦により成功するはず (GREEN)
        assert!(
            suggestions.contains(&"fs_reader".to_string()),
            "Should suggest fs_reader for semantic instruction, but got {:?}",
            suggestions
        );
    }
}
