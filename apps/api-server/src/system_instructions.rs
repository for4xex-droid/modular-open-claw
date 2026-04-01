use crate::AppState;
use aiome_core::commerce::EconomicContext;
use infrastructure::soul_store::SoulSnapshot;

use tokio::fs;
use tracing::warn;

pub(crate) async fn build_system_instructions(
    state: &AppState,
    karma_str: &str,
    summary: Option<&str>,
    ai_name: Option<String>,
    _knowledge_str: Option<&str>,
    _economic_context: Option<EconomicContext>,
    soul_snapshot: Option<SoulSnapshot>,
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

    let name_prompt = if let Some(name) = ai_name {
        format!("あなたの名前は「{}」です。\n", name)
    } else {
        "".to_string()
    };

    format!(
        "# IDENTITY: \n{}{}{}{}\n\
        [利用可能なスキル]\n{}\n[システム]\n教訓: {}\n要約: {}\n{}",
        name_prompt,
        soul_md,
        evolving_soul_md,
        soul_dynamic,
        skill_list,
        karma_str,
        summary.unwrap_or("なし"),
        agents_md
    )
}

pub(crate) fn safe_truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() > max_chars {
        let mut truncated: String = s.chars().take(max_chars).collect();
        truncated.push_str("... (truncated)");
        truncated
    } else {
        s.to_string()
    }
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
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test_agent.db");
        let pool_url = format!("sqlite://{}", db_path.to_str().unwrap());

        let ts = std::sync::Arc::new(
            infrastructure::job_queue::trajectory_store::SqliteTrajectoryStore::new(
                infrastructure::db::DatabasePool::new_sqlite(&pool_url)
                    .await
                    .unwrap(),
            ),
        );
        let jq = Arc::new(UniversalJobQueue::new(&pool_url, None, ts).await.unwrap());
        let registry = Arc::new(RegistryManager::new(jq.get_pool().clone()));

        let skills_dir = tmp_dir.path().join("skills");
        let sandbox_dir = tmp_dir.path().join("sandbox");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::create_dir_all(&sandbox_dir).unwrap();

        let wsm = Arc::new(
            WasmSkillManager::new(skills_dir.to_str().unwrap(), sandbox_dir.to_str().unwrap())
                .unwrap(),
        );

        let mut config = shared::config::AiomeConfig::default();
        config.resolver = shared::app_data::AppDataResolver::new();

        let state = crate::AppState {
            registry: Component::new(registry),
            wasm_skill_manager: Component::new(wsm),
            job_queue: Component::new(jq),
            config: Component::new(Arc::new(config)),
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
        )
        .await;

        assert!(instructions.contains("mcp-test"));
    }
}
