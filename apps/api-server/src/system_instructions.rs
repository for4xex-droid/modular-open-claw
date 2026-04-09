/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::AppState;
use aiome_core::commerce::EconomicContext;
use infrastructure::soul_store::SoulSnapshot;

use tokio::fs;
use tracing::warn;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn build_system_instructions(
    state: &AppState,
    karma_str: &str,
    summary: Option<&str>,
    ai_name: Option<String>,
    _knowledge_str: Option<&str>,
    _economic_context: Option<EconomicContext>,
    soul_snapshot: Option<SoulSnapshot>,
    self_repair_hint: Option<String>,
) -> String {
    let mut skill_list = state
        .wasm_skill_manager
        .list_skills_with_metadata()
        .iter()
        .map(|m| {
            format!(
                "- {}: {}",
                m.name,
                m.description.split('.').next().unwrap_or(&m.description)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mcp_servers = state
        .registry
        .list_assets_by_type(infrastructure::registry::AssetType::McpServer, None, "all")
        .await
        .unwrap_or_default();

    for mcp in mcp_servers {
        if !skill_list.is_empty() {
            skill_list.push('\n');
        }
        skill_list.push_str(&format!(
            "- {}: {}",
            mcp.name,
            mcp.description
                .split('.')
                .next()
                .unwrap_or(&mcp.description)
        ));
    }

    let resolver = &state.config.get_inner().resolver;
    let soul_md = safe_truncate(&read_app_data_file(resolver, "SOUL.md").await, 20000);
    let evolving_soul_md = safe_truncate(
        &read_app_data_file(resolver, "EVOLVING_SOUL.md").await,
        20000,
    );
    // let forge_prompt = safe_truncate(&read_app_data_file(resolver, "config/SKILL_FORGE_PROMPT.md").await, 20000);

    let soul_dynamic = if let Some(sn) = soul_snapshot {
        let narrative = if sn.narrative_self.is_empty() {
            "安定したアイデンティティを維持しています。"
        } else {
            &sn.narrative_self
        };
        format!(
            "\n[Anamnesis (内省的な自己認識)]\n{}\n[愛着スタイル: {}]\n",
            narrative, sn.attachment_style
        )
    } else {
        "".to_string()
    };

    let user_md = safe_truncate(&read_app_data_file(resolver, "USER.md").await, 20000);
    let agents_md = safe_truncate(&read_app_data_file(resolver, "AGENTS.md").await, 20000);
    let project_rules = resolve_project_rules(state).await;

    let name_prompt = if let Some(name) = ai_name {
        format!("あなたの名前は「{}」です。\n", name)
    } else {
        "".to_string()
    };

    let repair_prompt = if let Some(hint) = self_repair_hint {
        format!("\n[Watchtower Self-Repair Insight]\n過去の推論で失敗が検出されました。以下の修復ヒントを必ず考慮して実行してください:\n{}\n", hint)
    } else {
        "".to_string()
    };

    format!(
        "# IDENTITY: \n{}{}{}{}{}\n\
        [ユーザー情報]\n{}\n[利用可能なスキル]\n{}\n[システム]\n{}\n教訓: {}\n要約: {}\n{}",
        name_prompt,
        soul_md,
        evolving_soul_md,
        soul_dynamic,
        repair_prompt,
        user_md,
        skill_list,
        project_rules,
        karma_str,
        summary.unwrap_or("なし"),
        agents_md
    )
}

pub(crate) async fn resolve_project_rules(state: &crate::AppState) -> String {
    if let Ok(cwd) = std::env::current_dir() {
        resolve_project_rules_from_path(state, cwd).await
    } else {
        String::new()
    }
}

pub(crate) async fn resolve_project_rules_from_path(
    state: &crate::AppState,
    start_dir: std::path::PathBuf,
) -> String {
    if let Some(cached) = state.project_rules_cache.get(&start_dir).await {
        return cached;
    }

    let mut current_dir = start_dir.clone();
    loop {
        for filename in &[".aiome.md", "AIOME.md", ".cursorrules"] {
            let p = current_dir.join(filename);
            let is_file = tokio::fs::metadata(&p)
                .await
                .map(|m| m.is_file())
                .unwrap_or(false);

            if is_file {
                let content = tokio::fs::read_to_string(&p).await.unwrap_or_default();
                let budget = state.config.get_inner().max_project_rules_chars;
                let truncated = safe_truncate(&content, budget);
                let final_str = format!("[Project Rules ({})]\n{}\n", filename, truncated);
                state
                    .project_rules_cache
                    .insert(start_dir.clone(), final_str.clone())
                    .await;
                return final_str;
            }
        }
        if !current_dir.pop() {
            break;
        }
    }

    state
        .project_rules_cache
        .insert(start_dir.clone(), String::new())
        .await;
    String::new()
}

pub(crate) fn safe_truncate(s: &str, max_chars: usize) -> String {
    shared::strings::truncate_chars_safely(s, max_chars, true).into_owned()
}

pub(crate) async fn read_app_data_file(
    resolver: &shared::app_data::AppDataResolver,
    filename: &str,
) -> String {
    let path = resolver.resolve(filename);
    match fs::read_to_string(&path).await {
        Ok(content) => content,
        Err(e) => {
            tracing::warn!("Failed to read context file at {:?}: {}", path, e);
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::Component;
    use infrastructure::job_queue::UniversalJobQueue;
    use infrastructure::registry::{AssetManifest, AssetType, RegistryManager};
    use infrastructure::skills::WasmSkillManager;
    use std::sync::Arc;

    async fn setup_test_state() -> (crate::AppState, tempfile::TempDir) {
        let tmp_dir = tempfile::TempDir::new().unwrap(); // allow-anti-pattern
        let db_path = tmp_dir.path().join("test_agent.db");
        let pool_url = format!("sqlite://{}", db_path.to_str().unwrap()); // allow-anti-pattern

        let ts = std::sync::Arc::new(
            infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(
                infrastructure::db::DatabasePool::new_sqlite(&pool_url)
                    .await
                    .unwrap(), // allow-anti-pattern
            ),
        );
        let jq = Arc::new(UniversalJobQueue::new(&pool_url, None, ts).await.unwrap()); // allow-anti-pattern
        let registry = Arc::new(RegistryManager::new(jq.get_pool().clone()));

        let skills_dir = tmp_dir.path().join("skills");
        let sandbox_dir = tmp_dir.path().join("sandbox");
        std::fs::create_dir_all(&skills_dir).unwrap(); // allow-anti-pattern
        std::fs::create_dir_all(&sandbox_dir).unwrap(); // allow-anti-pattern

        let wsm = Arc::new(
            WasmSkillManager::new(skills_dir.to_str().unwrap(), sandbox_dir.to_str().unwrap()) // allow-anti-pattern
                .unwrap(), // allow-anti-pattern
        );

        let mut config = shared::config::AiomeConfig::default();
        config.resolver = shared::app_data::AppDataResolver::new();

        let state = crate::AppState {
            registry: Component::new(registry),
            wasm_skill_manager: Component::new(wsm),
            job_queue: Component::new(jq),
            config: Component::new(Arc::new(config)),
            project_rules_cache: Component::new(Arc::new(
                moka::future::Cache::builder()
                    .time_to_live(std::time::Duration::from_secs(30))
                    .build(),
            )),
            ..Default::default()
        };

        (state, tmp_dir)
    }

    #[tokio::test]
    async fn test_build_system_instructions_mcp_inclusion() {
        let (state, _tmp) = setup_test_state().await;

        let mcp_manifest = AssetManifest {
            id: uuid::Uuid::new_v4(),
            creator_id: uuid::Uuid::new_v4(),
            asset_type: AssetType::McpServer,
            name: "mcp-test".to_string(),
            description: "Test description".to_string(),
            price_coins: 0,
            safety_level: aiome_core_contracts::contracts::ToolSafetyLevel::Safe,
            metadata: Some(serde_json::json!({"command": "echo"})),
        };
        state.registry.register_asset(mcp_manifest).await.unwrap(); // allow-anti-pattern

        let instructions = build_system_instructions(
            &state,
            "karma",
            None,
            Some("Aiome".to_string()),
            None,
            None,
            None,
            None,
        )
        .await;

        assert!(instructions.contains("mcp-test"));
    }

    #[tokio::test]
    async fn test_build_system_instructions_user_md_inclusion() {
        let (state, _tmp) = setup_test_state().await;

        let resolver = &state.config.get_inner().resolver;
        let user_md_path = resolver.resolve("USER.md");
        if let Some(parent) = user_md_path.parent() {
            std::fs::create_dir_all(parent).unwrap(); // allow-anti-pattern
        }
        std::fs::write(&user_md_path, "Special User Instruction 123").unwrap(); // allow-anti-pattern

        let instructions = build_system_instructions(
            &state,
            "karma",
            None,
            Some("Aiome".to_string()),
            None,
            None,
            None,
            None,
        )
        .await;

        assert!(
            instructions.contains("Special User Instruction 123"),
            "System instructions should contain USER.md contents"
        );
    }

    #[tokio::test]
    async fn test_resolve_project_rules_priority() {
        let (state, tmp_dir) = setup_test_state().await;

        let sub_dir = tmp_dir.path().join("sub");
        std::fs::create_dir_all(&sub_dir).unwrap(); // allow-anti-pattern

        std::fs::write(sub_dir.join(".aiome.md"), "aiome md content").unwrap(); // allow-anti-pattern
        std::fs::write(sub_dir.join(".cursorrules"), "cursorrules content").unwrap(); // allow-anti-pattern

        let rules = resolve_project_rules_from_path(&state, sub_dir.clone()).await;
        assert_eq!(rules, "[Project Rules (.aiome.md)]\naiome md content\n");

        // Verify it was cached
        let cached = state.project_rules_cache.get(&sub_dir).await;
        assert_eq!(
            cached,
            Some("[Project Rules (.aiome.md)]\naiome md content\n".to_string())
        );

        // Remove .aiome.md and verify cache still returns the same
        std::fs::remove_file(sub_dir.join(".aiome.md")).unwrap(); // allow-anti-pattern
        let cached_rules = resolve_project_rules_from_path(&state, sub_dir.clone()).await;
        assert_eq!(
            cached_rules,
            "[Project Rules (.aiome.md)]\naiome md content\n"
        );
    }

    #[tokio::test]
    async fn test_resolve_project_rules_not_found_traversal() {
        let (state, tmp_dir) = setup_test_state().await;

        // Deep nested directory with NO rule files.
        let sub_dir = tmp_dir.path().join("sub1").join("sub2").join("sub3");
        std::fs::create_dir_all(&sub_dir).unwrap(); // allow-anti-pattern

        let rules = resolve_project_rules_from_path(&state, sub_dir.clone()).await;

        // Should traverse safely to root of the provided path without infinite loop
        // and return empty string if no rule files found.
        assert_eq!(rules, "");

        // It should cache the empty string
        let cached = state.project_rules_cache.get(&sub_dir).await;
        assert_eq!(cached, Some("".to_string()));
    }
}
