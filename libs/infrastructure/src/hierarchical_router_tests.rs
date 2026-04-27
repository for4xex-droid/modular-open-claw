/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

#[cfg(test)]
mod tests {
    use crate::hierarchical_router::HierarchicalRouter;
    use crate::test_utils::job_queue_mock::GlobalMockLlm;
    use std::sync::Arc;

    #[test]
    fn test_parse_llm_selection_safe() {
        // CHAIN-3: Robust parsing
        assert_eq!(
            HierarchicalRouter::parse_llm_selection("Choice is 2", 5),
            Some(2)
        );
        assert_eq!(
            HierarchicalRouter::parse_llm_selection("Total 10, pick 3.", 5),
            Some(3)
        );
        assert_eq!(
            HierarchicalRouter::parse_llm_selection("I don't know", 5),
            None
        );
        assert_eq!(
            HierarchicalRouter::parse_llm_selection("999", 5),
            None,
            "Should respect max_choices"
        );
        assert_eq!(
            HierarchicalRouter::parse_llm_selection("0", 5),
            None,
            "0 is usually invalid in 1-based lists"
        );
    }

    #[tokio::test]
    async fn test_hierarchical_route_success() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE system_state (key TEXT PRIMARY KEY, value TEXT, updated_at DATETIME)",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Setup a mock tree
        use crate::knowledge_indexer::TreeNode;
        let leaf = TreeNode {
            id: "root-leaf".to_string(),
            title: "Leaf".to_string(),
            level: 1,
            summary: "Leaf summary".to_string(),
            children: vec![],
            content: Some("Target content found!".to_string()),
        };
        let root = TreeNode {
            id: "root".to_string(),
            title: "Root".to_string(),
            level: 0,
            summary: "".to_string(),
            children: vec![leaf],
            content: None,
        };

        let tree_json = serde_json::to_string(&root).unwrap();
        sqlx::query(
            "INSERT INTO system_state (key, value, updated_at) VALUES (?, ?, datetime('now'))",
        )
        .bind("knowledge_tree_test_doc")
        .bind(tree_json)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO system_state (key, value, updated_at) VALUES (?, ?, datetime('now'))",
        )
        .bind("knowledge_hash_test_doc")
        .bind("hash_v1")
        .execute(&pool)
        .await
        .unwrap();

        #[derive(Debug)]
        struct SelectionMockLlm;
        #[async_trait::async_trait]
        impl aiome_core::llm_provider::LlmProvider for SelectionMockLlm {
            fn name(&self) -> &str {
                "SelectionMockLlm"
            }
            async fn test_connection(&self) -> Result<(), aiome_core::error::AiomeError> {
                Ok(())
            }
            async fn complete(
                &self,
                _prompt: &str,
                _system: Option<&str>,
            ) -> Result<aiome_core::llm_provider::LlmResponse, aiome_core::error::AiomeError>
            {
                Ok(aiome_core::llm_provider::LlmResponse {
                    content: "I choose option 1 because it fits best.".into(),
                    stop_reason: aiome_core_contracts::StopReason::EndTurn,
                    reasoning: None,
                    metadata: None,
                })
            }
        }

        let mock_llm = Arc::new(SelectionMockLlm);
        let router = HierarchicalRouter::new(mock_llm, pool.clone());

        let res = router.route("Find me leaf", "test_doc").await.unwrap();
        assert!(res.is_some());
        assert_eq!(res.unwrap().content, "Target content found!");

        // Test Cache inclusion
        let cached: Option<String> =
            sqlx::query_scalar("SELECT value FROM system_state WHERE key LIKE 'hkr_cache:%'")
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert!(cached.is_some());
    }
}
