/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use aiome_core::traits::{ArtifactCategory, ArtifactStore, CreateArtifactRequest};
use bastion::fs_guard::Jail;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

/// ProjectKnowledgeIndexer scans local documentation and indexes it for RAG.
/// NOTE: Secured by infrastructure layer. Standard std::fs is used here because
/// it only targets internal project files (docs/, ARCHITECTURE.md)
/// and not unknown user-uploaded content.
pub struct ProjectKnowledgeIndexer {
    artifact_store: Arc<dyn ArtifactStore>,
    pool: SqlitePool,
    workspace_root: PathBuf,
}

impl ProjectKnowledgeIndexer {
    /// 新しいインスタンスを生成する
    pub fn new(
        artifact_store: Arc<dyn ArtifactStore>,
        pool: SqlitePool,
        workspace_root: PathBuf,
    ) -> Self {
        Self {
            artifact_store,
            pool,
            workspace_root,
        }
    }

    /// `run_indexing` を実行する
    pub async fn run_indexing(&self) -> Result<(), AiomeError> {
        info!("📚 [KnowledgeIndexer] Starting project knowledge indexing...");

        const MAX_INDEX_DEPTH: usize = 5; // DS-8: Depth limit to prevent scan runaway
                                          // Scan docs directory
        let docs_dir = self.workspace_root.join("docs");
        let arch_file = self.workspace_root.join("ARCHITECTURE.md");

        let mut files_to_index = Vec::new();
        if arch_file.exists() {
            files_to_index.push(arch_file);
        }

        let skills_dir = self.workspace_root.join(".agents").join("skills");

        let mut stack = Vec::new();
        if docs_dir.exists() {
            stack.push((docs_dir, 0)); // (path, current_depth)
        }
        if skills_dir.exists() {
            stack.push((skills_dir, 0));
        }

        while let Some((dir, depth)) = stack.pop() {
            if depth > MAX_INDEX_DEPTH {
                warn!("⚠️ [KnowledgeIndexer] Skip deep directory: {:?}", dir);
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    // [ATK-6] Security: Block symlinks to prevent path traversal
                    if path.is_symlink() {
                        warn!("🛡️ [KnowledgeIndexer] Blocked symlink: {:?}", path);
                        continue;
                    }
                    if path.is_dir() {
                        stack.push((path, depth + 1));
                    } else if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false)
                    {
                        files_to_index.push(path);
                    }
                }
            }
        }

        let jail = Jail::new(&self.workspace_root).map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        for file_path in files_to_index {
            let relative_path = file_path
                .strip_prefix(&self.workspace_root)
                .unwrap_or(&file_path)
                .to_string_lossy()
                .to_string();

            if let Err(e) = self.index_file(&file_path, &relative_path, &jail).await {
                warn!(
                    "⚠️ [KnowledgeIndexer] Failed to index {}: {:?}",
                    relative_path, e
                );
            }
        }

        info!("📚 [KnowledgeIndexer] Indexing cycle complete.");
        Ok(())
    }

    async fn index_file(&self, path: &Path, rel_path: &str, jail: &Jail) -> Result<(), AiomeError> {
        let content = std::fs::read_to_string(path).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to read {}: {}", rel_path, e),
        })?;

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        let hash_key = format!("knowledge_hash_{}", rel_path);
        let tree_key = format!("knowledge_tree_{}", rel_path);

        // Check if already indexed
        let existing_hash: Option<String> =
            sqlx::query_scalar("SELECT value FROM system_state WHERE key = ?")
                .bind(&hash_key)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;

        if existing_hash.as_deref() == Some(&hash) {
            return Ok(());
        }

        info!(
            "📚 [KnowledgeIndexer] File changed: {}. Re-indexing with Tree...",
            rel_path
        );

        // 1. Delete old chunks
        let escaped_tag = rel_path.replace('%', "\\%").replace('_', "\\_");
        let source_tag = format!("source:{}", rel_path);
        let like_pattern = format!("%source:{}%", escaped_tag);
        let old_ids: Vec<String> =
            sqlx::query_scalar("SELECT id FROM ai_artifacts WHERE tags LIKE ? ESCAPE '\\'")
                .bind(like_pattern)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;

        for id in old_ids {
            let _ = self.artifact_store.delete_artifact(&id, jail).await;
        }

        // 2. Build Hierarchical Tree and Save as Metadata
        let tree = Self::build_tree(&content, rel_path);
        let tree_json = serde_json::to_string(&tree).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to serialize tree: {}", e),
        })?;

        // 3. Flat Chunking (Legacy support for Hybrid RAG)
        let chunks: Vec<(String, String)> = Self::chunk_markdown(&content);
        for (i, (title, chunk_content)) in chunks.into_iter().enumerate() {
            let artifact_title = if title.is_empty() {
                format!("Knowledge: {} (Part {})", rel_path, i + 1)
            } else {
                format!("Knowledge: {} ({})", title, rel_path)
            };

            let req = CreateArtifactRequest {
                title: artifact_title,
                category: ArtifactCategory::Knowledge,
                tags: vec![source_tag.clone(), "rag".to_string()],
                created_by: "KnowledgeIndexer".to_string(),
                files: vec![(
                    format!("chunk_{}.md", i),
                    chunk_content.clone().into_bytes(),
                    "text/markdown".to_string(),
                )],
                karma_refs: vec![],
                text_content: Some(chunk_content),
                job_ref: None,
                parent_refs: vec![],
                is_protected: false,
            };

            self.artifact_store.save_artifact(req, jail).await?;
        }

        // 4. Update hash AND tree in system_state
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        sqlx::query("INSERT INTO system_state (key, value, updated_at) VALUES (?, ?, datetime('now')) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at")
            .bind(&hash_key)
            .bind(&hash)
            .execute(&mut *tx)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        sqlx::query("INSERT INTO system_state (key, value, updated_at) VALUES (?, ?, datetime('now')) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at")
            .bind(&tree_key)
            .bind(&tree_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
/// `TreeNode` 階層的ナレッジインデックスのノード
pub struct TreeNode {
    /// 一意識別子 (パス等)
    pub id: String,
    /// 見出しタイトル
    pub title: String,
    /// 見出しレベル (1=#, 2=##, 3=###)
    pub level: u8,
    /// セクションの要約 (サニタイズ済み)
    pub summary: String,
    /// 子ノード
    pub children: Vec<TreeNode>,
    /// 本文コンテンツ (リーフノードのみ)
    pub content: Option<String>,
}

impl ProjectKnowledgeIndexer {
    /// プロンプトインジェクション等を防ぐためにサニタイズする
    pub fn sanitize_summary(raw: &str) -> String {
        let blocklist = [
            "ignore all",
            "disregard",
            "forget previous",
            "select chapter",
            "always choose",
            "override",
        ];
        let s_lower = raw.to_lowercase();
        let mut found_injection = false;
        for pattern in blocklist {
            if s_lower.contains(pattern) {
                found_injection = true;
                break;
            }
        }

        if found_injection {
            let mut s = s_lower;
            for pattern in blocklist {
                s = s.replace(pattern, "[REDACTED]");
            }
            s
        } else {
            raw.to_string()
        }
    }

    /// YAML Frontmatter があれば除去する
    pub fn strip_yaml_frontmatter(content: &str) -> &str {
        if content.starts_with("---\n") || content.starts_with("---\r\n") {
            if let Some(end_idx) = content[3..].find("\n---") {
                let remainder = &content[3 + end_idx + 4..];
                return remainder.trim_start();
            }
        }
        content
    }

    /// Markdown テキストから階層ツリーを構築する
    pub fn build_tree(content: &str, base_id: &str) -> TreeNode {
        let clean_content = Self::strip_yaml_frontmatter(content);
        let mut lines = clean_content.lines().peekable();
        let mut root = TreeNode {
            id: base_id.to_string(),
            title: "Root".to_string(),
            level: 0,
            summary: String::new(),
            children: Vec::new(),
            content: None,
        };

        const MAX_TREE_DEPTH: u8 = 4;
        const MAX_NODES_PER_TREE: usize = 500;
        let mut node_count = 1;

        root.children = Self::build_level_recursive(
            &mut lines,
            0,
            base_id,
            MAX_TREE_DEPTH,
            &mut node_count,
            MAX_NODES_PER_TREE,
        );

        root
    }

    fn build_level_recursive(
        lines: &mut std::iter::Peekable<std::str::Lines>,
        parent_level: u8,
        base_id: &str,
        max_depth: u8,
        node_count: &mut usize,
        max_nodes: usize,
    ) -> Vec<TreeNode> {
        let mut nodes = Vec::new();

        while let Some(&line) = lines.peek() {
            let level = if line.starts_with('#') {
                line.chars().take_while(|&c| c == '#').count() as u8
            } else {
                0
            };

            if level > 0 {
                if level <= parent_level {
                    break;
                }

                let header_line = lines.next().unwrap_or_default();
                let title = header_line.trim_start_matches('#').trim();

                if level > max_depth || *node_count >= max_nodes {
                    continue;
                }

                *node_count += 1;
                let mut node = TreeNode {
                    id: format!("{}-{}", base_id, title.replace(' ', "-").to_lowercase()),
                    title: title.to_string(),
                    level,
                    summary: String::new(),
                    children: Vec::new(),
                    content: None,
                };

                // Capture text content immediately following this header
                let mut content_lines = Vec::new();
                while let Some(&next_line) = lines.peek() {
                    if next_line.starts_with('#') {
                        break;
                    }
                    let val = lines.next().unwrap_or_default();
                    content_lines.push(val);
                }

                if !content_lines.is_empty() {
                    let text = content_lines.join("\n").trim().to_string();
                    node.summary = Self::sanitize_summary(&text);
                    if node.summary.chars().count() > 200 {
                        node.summary = node.summary.chars().take(200).collect();
                    }
                    node.content = Some(text);
                }

                // Recursive call for sub-headers
                node.children = Self::build_level_recursive(
                    lines, level, base_id, max_depth, node_count, max_nodes,
                );

                nodes.push(node);
            } else {
                // Skip orphan lines before any header or consume them?
                // For Root, these could be added to root content, but build_tree handles root separately.
                let _ = lines.next();
            }
        }

        nodes
    }

    /// Markdown をセクションごとに分割する (OSS インデクサー等で再利用)
    pub fn chunk_markdown(content: &str) -> Vec<(String, String)> {
        let mut chunks = Vec::new();
        let mut current_title = String::new();
        let mut current_chunk = Vec::new();

        for line in content.lines() {
            if line.starts_with("## ") {
                if !current_chunk.is_empty() {
                    chunks.push((current_title.clone(), current_chunk.join("\n")));
                    current_chunk.clear();
                }
                current_title = line.trim_start_matches("## ").to_string();
            } else if line.starts_with("# ") && current_title.is_empty() {
                // Main title if no ## yet
                current_title = line.trim_start_matches("# ").to_string();
            }
            current_chunk.push(line);
        }

        if !current_chunk.is_empty() {
            chunks.push((current_title, current_chunk.join("\n")));
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_tree_valid_markdown() {
        let md = "\
# Root Title
Some root content
## Section 1
Content 1
### Subsection 1.1
Content 1.1
## Section 2
Content 2
";
        let tree = ProjectKnowledgeIndexer::build_tree(md, "test-doc");
        assert_eq!(tree.title, "Root");
        assert_eq!(tree.children.len(), 1); // "Root Title" is level 1
        
        let root_title = &tree.children[0];
        assert_eq!(root_title.title, "Root Title");
        assert_eq!(root_title.children.len(), 2);
        
        assert_eq!(root_title.children[0].title, "Section 1");
        assert_eq!(root_title.children[0].content.as_deref(), Some("Content 1"));
        assert_eq!(root_title.children[0].children[0].title, "Subsection 1.1");
        
        assert_eq!(root_title.children[1].title, "Section 2");
    }

    #[test]
    fn test_build_tree_edge_case_empty_headers() {
        let md = "\
#
##
###
";
        let tree = ProjectKnowledgeIndexer::build_tree(md, "test-doc");
        assert_eq!(tree.title, "Root");
        // Should not panic, should just parse empty titles
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].title, "");
    }
    
    #[test]
    fn test_build_tree_edge_case_abrupt_eof() {
        let md = "# ";
        let tree = ProjectKnowledgeIndexer::build_tree(md, "test-doc");
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].title, "");
    }
}
