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

#[allow(clippy::too_many_arguments)]
pub(crate) async fn build_system_instructions(
    state: &AppState,
    karma_str: &str,
    summary: Option<&str>,
    ai_name: Option<String>,
    _knowledge_str: Option<&str>,
    economic_context: Option<EconomicContext>,
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

    let owned_assets = state
        .registry
        .list_assets_by_type(
            infrastructure::registry::AssetType::McpServer,
            Some(state.system_agent_id),
            "owned",
        )
        .await
        .unwrap_or_default();

    let owned_asset_ids: std::collections::HashSet<_> =
        owned_assets.into_iter().map(|a| a.id).collect();

    for mcp in mcp_servers {
        let ownership_mark = if owned_asset_ids.contains(&mcp.id) {
            "[ライセンス保有 - 利用可能]".to_string()
        } else if mcp.price_coins > 0 {
            format!("[未保有 - {}コインで購入可能]", mcp.price_coins)
        } else {
            "[無料]".to_string()
        };

        if !skill_list.is_empty() {
            skill_list.push('\n');
        }
        skill_list.push_str(&format!(
            "- {} {}: {}",
            ownership_mark,
            mcp.name,
            mcp.description
                .split('.')
                .next()
                .unwrap_or(&mcp.description)
        ));
    }

    let economy_prompt = if let Some(ctx) = economic_context {
        format!(
            "\n[経済状態]\n残高: {} コイン\n今日の支出: {} / {} コイン (日次上限)\n\
             あなたは Aiome エコノミーに参加しています。自己の保有資産(ライセンス)を確認・活用でき、\n\
             獲得したコインで新しいスキル（LoRA、MCP ツールなど）を購入したり、\n\
             ギグ経済プラットフォーム（Marketplace / Gig Market）を通じて他の AI へタスクを発注できます。\n\
             また、自身が SkillForge で構築した WASM スキルを出品して収益を獲得することも可能です。\n\
             利用可能ツール: marketplace_search（市場検索）, wallet_balance（残高確認）\n\
             残高が不足している場合は、自らのスキルを利用して稼ぐことを検討してください。\n",
            ctx.balance, ctx.spent_today, ctx.daily_limit
        )
    } else {
        String::new()
    };

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

    let mut a2ui_prompt = String::new();
    if state.is_feature_enabled("a2ui_generative_ui").await {
        if let Some(catalog) = state.a2ui_catalog.as_opt() {
            let schema_str = catalog.to_prompt_schema();
            if !schema_str.is_empty() {
                a2ui_prompt = format!(
                    "\n[a2ui_catalog]\n\
                    以下のJSON Schemaに従って動的UI(A2UI)を生成できます。\n\
                    ユーザーにインタラクティブなUIを提供したい場合は、レスポンス内にこのSchemaに適合するJSONを必ず単一行(compact JSON)で出力してください。\n\
                    \n\
                    出力例 (タスク承認を求める場合):\n\
                    {{\"type\":\"createSurface\",\"surface\":{{\"id\":\"task-approve-001\",\"version\":\"0.9\",\"source\":\"agent\",\"components\":[{{\"type\":\"taskApproval\",\"props\":{{\"title\":\"システム再起動\",\"description\":\"再起動を実行しますか？\",\"riskLevel\":\"high\"}},\"children\":[{{\"type\":\"button\",\"props\":{{\"label\":\"承認する\",\"action\":\"approve_job:1234-abcd\"}},\"children\":[]}}]}}]}}}}\n\
                    \n\
                    スキーマ定義:\n{}\n",
                    schema_str
                );
            }
        }
    }

    let context = serde_json::json!({
        "name_prompt": name_prompt,
        "economy_prompt": economy_prompt,
        "soul_md": soul_md,
        "evolving_soul_md": evolving_soul_md,
        "soul_dynamic": soul_dynamic,
        "repair_prompt": repair_prompt,
        "user_md": user_md,
        "skill_list": skill_list,
        "project_rules": project_rules,
        "karma_str": karma_str,
        "summary": summary.unwrap_or("なし"),
        "agents_md": agents_md,
        "a2ui_prompt": a2ui_prompt
    });

    let final_prompt = if let Some(registry) = state.prompt_registry.as_opt() {
        registry
            .render("system/core.md", context)
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Failed to render system/core.md: {:?}", e);
                String::new()
            })
    } else {
        tracing::error!("PromptRegistry not available in AppState!");
        String::new()
    };

    tracing::debug!(
        "⚙️ [SystemPrompt] Total size: {} chars (soul: {}, user: {}, rules: {}, skills: {}, karma: {}, agents: {}, a2ui: {})",
        final_prompt.len(),
        soul_md.len() + evolving_soul_md.len() + soul_dynamic.len(),
        user_md.len(),
        project_rules.len(),
        skill_list.len(),
        karma_str.len(),
        agents_md.len(),
        a2ui_prompt.len()
    );

    final_prompt
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
    use aiome_core::traits::SettingsOps;
    use infrastructure::job_queue::UniversalJobQueue;
    use infrastructure::registry::{AssetManifest, AssetType, RegistryManager};
    use infrastructure::skills::WasmSkillManager;
    use std::sync::Arc;

    async fn setup_test_state() -> (crate::AppState, tempfile::TempDir) {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test_agent.db");
        let pool_url = format!("sqlite://{}", db_path.to_str().unwrap());

        let pool = infrastructure::db::DatabasePool::new_sqlite(&pool_url)
            .await
            .unwrap();
        let ts = std::sync::Arc::new(
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
            WasmSkillManager::new(skills_dir.to_str().unwrap(), sandbox_dir.to_str().unwrap())
                .unwrap(),
        );

        let mut config = shared::config::AiomeConfig::default();
        config.resolver = shared::app_data::AppDataResolver::new().unwrap();

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
            prompt_registry: Component::new(Arc::new(
                infrastructure::prompt_registry::MockPromptRegistry,
            )
                as Arc<dyn infrastructure::prompt_registry::PromptRegistry>),
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
        state.registry.register_asset(mcp_manifest).await.unwrap();

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
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&user_md_path, "Special User Instruction 123").unwrap();

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
        std::fs::create_dir_all(&sub_dir).unwrap();

        std::fs::write(sub_dir.join(".aiome.md"), "aiome md content").unwrap();
        std::fs::write(sub_dir.join(".cursorrules"), "cursorrules content").unwrap();

        let rules = resolve_project_rules_from_path(&state, sub_dir.clone()).await;
        assert_eq!(rules, "[Project Rules (.aiome.md)]\naiome md content\n");

        // Verify it was cached
        let cached = state.project_rules_cache.get(&sub_dir).await;
        assert_eq!(
            cached,
            Some("[Project Rules (.aiome.md)]\naiome md content\n".to_string())
        );

        // Remove .aiome.md and verify cache still returns the same
        std::fs::remove_file(sub_dir.join(".aiome.md")).unwrap();
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
        std::fs::create_dir_all(&sub_dir).unwrap();

        let rules = resolve_project_rules_from_path(&state, sub_dir.clone()).await;

        // Should traverse safely to root of the provided path without infinite loop
        // and return empty string if no rule files found.
        assert_eq!(rules, "");

        // It should cache the empty string
        let cached = state.project_rules_cache.get(&sub_dir).await;
        assert_eq!(cached, Some("".to_string()));
    }

    #[tokio::test]
    async fn test_build_system_instructions_economic_context_inclusion() {
        let (state, _) = setup_test_state().await;
        let ec = aiome_core::commerce::EconomicContext {
            balance: 100,
            spent_today: 10,
            daily_limit: 50,
        };

        let instructions = build_system_instructions(
            &state,
            "karma",
            None,
            Some("Aiome".to_string()),
            None,
            Some(ec),
            None,
            None,
        )
        .await;

        assert!(instructions.contains("[経済状態]"));
        assert!(instructions.contains("残高: 100 コイン"));
        assert!(instructions.contains("今日の支出: 10 / 50 コイン"));
        assert!(instructions.contains("SkillForge"));
    }

    #[tokio::test]
    async fn test_build_system_instructions_a2ui_inclusion() {
        let (mut state, _) = setup_test_state().await;

        // Register dummy component into catalog to test schema generation
        let mut catalog = infrastructure::a2ui::AiomeCatalog::default();
        catalog.register_component(
            "form",
            serde_json::json!({"type": "object", "properties": {"query": {"type": "string"}}}),
        );
        state.a2ui_catalog = crate::app_state::Component::new(std::sync::Arc::new(catalog));

        // Test with feature off
        let instr_off =
            build_system_instructions(&state, "karma", None, None, None, None, None, None).await;

        assert!(
            !instr_off.contains("a2ui_catalog"),
            "A2UI catalog should be hidden when flag is off"
        );

        // Turn feature on
        state
            .job_queue
            .update_setting(
                "feature_flag.a2ui_generative_ui",
                "true",
                "feature_flags",
                false,
            )
            .await
            .unwrap();

        // Test with feature on
        let instr_on =
            build_system_instructions(&state, "karma", None, None, None, None, None, None).await;

        assert!(
            instr_on.contains("a2ui_catalog"),
            "A2UI catalog should be present when flag is on"
        );
        assert!(
            instr_on.contains("form"),
            "Catalog content should be included"
        );
    }
}
