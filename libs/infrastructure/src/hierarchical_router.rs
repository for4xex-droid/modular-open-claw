/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use crate::knowledge_indexer::TreeNode;
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    pub content: String,
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    result: RouteResult,
    doc_hash: String,
    expires_at: DateTime<Utc>,
}

pub struct HierarchicalRouter {
    provider: Arc<dyn LlmProvider>,
    pool: SqlitePool,
    hkr_semaphore: Arc<Semaphore>,
}

impl HierarchicalRouter {
    pub fn new(provider: Arc<dyn LlmProvider>, pool: SqlitePool) -> Self {
        Self {
            provider,
            pool,
            hkr_semaphore: Arc::new(Semaphore::new(2)), // CHAIN-6
        }
    }

    /// LLM の出力から選択肢（数値）を安全にパースする
    pub fn parse_llm_selection(response: &str, max_choices: usize) -> Option<usize> {
        // [CHAIN-3] Robust parsing: extract digits and check bounds
        let digits: String = response.chars().filter(|c| c.is_ascii_digit()).collect();

        if let Ok(choice) = digits.parse::<usize>() {
            if choice > 0 && choice <= max_choices {
                return Some(choice);
            }
        }

        // Fallback: search for "choice X" or "X" in word boundary
        for word in response.split_whitespace() {
            let cleaned = word.trim_matches(|c: char| !c.is_ascii_digit());
            if let Ok(choice) = cleaned.parse::<usize>() {
                if choice > 0 && choice <= max_choices {
                    return Some(choice);
                }
            }
        }
        None
    }

    /// 階層的探索を実行する
    pub async fn route(
        &self,
        query: &str,
        tree_id: &str,
    ) -> Result<Option<RouteResult>, AiomeError> {
        // [CHAIN-6] 리ソース制御: Semaphore acquire
        let _permit =
            self.hkr_semaphore
                .acquire()
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Semaphore error: {}", e),
                })?;

        // 1. Load Tree and current Hash from system_state
        let tree_key = format!("knowledge_tree_{}", tree_id);
        let hash_key = format!("knowledge_hash_{}", tree_id);

        let (tree_json, current_hash): (Option<String>, Option<String>) = sqlx::query_as("SELECT (SELECT value FROM system_state WHERE key = ?), (SELECT value FROM system_state WHERE key = ?)")
            .bind(&tree_key)
            .bind(&hash_key)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        let current_hash = match current_hash {
            Some(h) => h,
            None => return Ok(None), // Index not found
        };

        let root: TreeNode = match tree_json {
            Some(json) => serde_json::from_str(&json).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to parse tree JSON: {}", e),
            })?,
            None => return Ok(None),
        };

        // 2. Check RouteCache (CHAIN-4: TTL & Hash validation)
        let cache_key = format!("hkr_cache:{}:{}", tree_id, query);
        let cached_json: Option<String> =
            sqlx::query_scalar("SELECT value FROM system_state WHERE key = ?")
                .bind(&cache_key)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;

        if let Some(json) = cached_json {
            if let Ok(entry) = serde_json::from_str::<CacheEntry>(&json) {
                // Validate TTL and Hash
                if entry.expires_at > Utc::now() && entry.doc_hash == current_hash {
                    return Ok(Some(entry.result));
                }
            }
        }

        // 3. Traversal
        let mut current_node = root;
        while !current_node.children.is_empty() {
            let choices = &current_node.children;
            let mut prompt_context = String::new();
            for (i, child) in choices.iter().enumerate() {
                prompt_context.push_str(&format!(
                    "{}. {}: {}\n",
                    i + 1,
                    child.title,
                    child.summary
                ));
            }

            let prompt = format!(
                "You are an expert router. Given the query, select the most relevant section number (1-{})\n\nQuery: {}\n\nSections:\n{}\n\nReturn ONLY the number.",
                choices.len(),
                query,
                prompt_context
            );

            let res = self.provider.complete(&prompt, None).await?;
            if let Some(idx) = Self::parse_llm_selection(&res.content, choices.len()) {
                current_node = choices[idx - 1].clone();
            } else {
                // [CHAIN-3] Fallback: top-1 if parsing fails
                current_node = choices[0].clone();
            }
        }

        if let Some(content) = current_node.content {
            let result = RouteResult {
                content,
                source_path: tree_id.to_string(),
            };

            // 4. Update Cache (Max 1 hour TTL: CHAIN-4)
            let cache_entry = CacheEntry {
                result: result.clone(),
                doc_hash: current_hash,
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };
            let cache_json =
                serde_json::to_string(&cache_entry).map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Serialization failed: {}", e),
                })?;

            sqlx::query("INSERT INTO system_state (key, value, updated_at) VALUES (?, ?, datetime('now')) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at")
                .bind(&cache_key)
                .bind(cache_json)
                .execute(&self.pool)
                .await
                .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}
