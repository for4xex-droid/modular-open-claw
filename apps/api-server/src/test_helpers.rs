/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

#![cfg(test)]

use crate::app_state::{AppState, Component};
use async_trait::async_trait;
use infrastructure::job_queue::UniversalJobQueue;
use infrastructure::registry::RegistryManager;
use infrastructure::skills::WasmSkillManager;
use std::sync::Arc;

/// 軽量ユニットテスト用 AppState。HTTP サーバは起動しない。
/// HTTP が必要なら `api_integration_tests::create_test_server` を使う。
pub async fn create_test_app_state() -> (AppState, tempfile::TempDir) {
    let tmp_dir = tempfile::TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test_agent.db");
    let pool_url = format!("sqlite://{}", db_path.to_str().unwrap());

    let pool = infrastructure::db::DatabasePool::new_sqlite(&pool_url)
        .await
        .unwrap();
    let ts = Arc::new(
        infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
    );
    let jq = Arc::new(
        UniversalJobQueue::new(pool.clone(), None, ts)
            .await
            .unwrap(),
    );
    let registry = Arc::new(RegistryManager::new(pool.clone()));

    let skills_dir = tmp_dir.path().join("skills");
    let sandbox_dir = tmp_dir.path().join("sandbox");
    std::fs::create_dir_all(&skills_dir).unwrap();
    std::fs::create_dir_all(&sandbox_dir).unwrap();

    let wsm = Arc::new(
        WasmSkillManager::new(skills_dir.to_str().unwrap(), sandbox_dir.to_str().unwrap()).unwrap(),
    );

    let mut config = shared::config::AiomeConfig::default();
    config.resolver = shared::app_data::AppDataResolver::new().unwrap();

    #[derive(Debug)]
    struct MockLlm;
    #[async_trait]
    impl aiome_core::llm_provider::LlmProvider for MockLlm {
        async fn complete(
            &self,
            _prompt: &str,
            _sys: Option<&str>,
        ) -> Result<aiome_core_contracts::LlmResponse, aiome_core::error::AiomeError> {
            Ok(aiome_core_contracts::LlmResponse {
                content: "Mocked Execution Result".into(),
                metadata: Some(std::collections::HashMap::new()),
                stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
                ..Default::default()
            })
        }
        async fn test_connection(&self) -> Result<(), aiome_core::error::AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "MockLlm"
        }
    }

    let state = AppState {
        registry: Component::new(registry),
        wasm_skill_manager: Component::new(wsm),
        job_queue: Component::new(jq),
        config: Component::new(Arc::new(config)),
        provider: Component::new(Arc::new(MockLlm)),
        hook_chain: Component::new(Arc::new(infrastructure::skills::hooks::HookChain::new())),
        skill_arena: Component::new(Arc::new(
            infrastructure::skills::skill_arena::SkillArena::new(),
        )),
        project_rules_cache: Component::new(Arc::new(
            moka::future::Cache::builder()
                .time_to_live(std::time::Duration::from_secs(30))
                .build(),
        )),
        prompt_registry: Component::new(Arc::new(
            infrastructure::prompt_registry::MockPromptRegistry,
        )
            as Arc<dyn infrastructure::prompt_registry::PromptRegistry>),
        ..Default::default()
    };

    (state, tmp_dir)
}
