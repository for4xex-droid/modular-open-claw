/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use aiome_core::traits::ArtifactStore;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info};

/// 外部リポジトリの知識セッション（RAIIによる自動クリーンアップ）
#[derive(Debug)]
pub struct OssKnowledgeSession {
    pub temp_dir: PathBuf,
}

impl Drop for OssKnowledgeSession {
    fn drop(&mut self) {
        if self.temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&self.temp_dir);
            info!(
                "🧹 [OssKnowledgeSession] Cleaned up temporary repository: {:?}",
                self.temp_dir
            );
        }
    }
}

/// 外部リポジトリを自動クローンし、RAG 用にインデックス化する
pub struct OssRepositoryIndexer {
    artifact_store: Arc<dyn ArtifactStore>,
    pool: SqlitePool,
}

impl OssRepositoryIndexer {
    pub fn new(artifact_store: Arc<dyn ArtifactStore>, pool: SqlitePool) -> Self {
        Self {
            artifact_store,
            pool,
        }
    }

    fn validate_url(&self, url: &str) -> Result<(), AiomeError> {
        let url = url.trim();
        let allowed = if cfg!(test) {
            url.starts_with("https://") || url.starts_with("git://") || url.starts_with("file://")
        } else {
            url.starts_with("https://") || url.starts_with("git://")
        };

        if !allowed {
            return Err(AiomeError::Infrastructure {
                reason: "Invalid URL protocol. Only https:// and git:// are allowed.".to_string(),
            });
        }

        let dangerous_chars = [';', '&', '|', '$', '>', '<', '`', ' ', '\n', '\r'];
        for ch in dangerous_chars {
            if url.contains(ch) {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Security Violation: Prohibited character '{}' in URL.", ch),
                });
            }
        }

        Ok(())
    }

    /// 指定された GitHub URL からリポジトリをクローンし、インデックス化を実行する
    pub async fn clone_and_index(
        &self,
        github_url: &str,
        _focus_paths: &[&str],
    ) -> Result<OssKnowledgeSession, AiomeError> {
        self.validate_url(github_url)?;
        let session_id = uuid::Uuid::new_v4().to_string();
        let temp_parent = std::env::temp_dir().join("aiome_oss_cache");
        if !temp_parent.exists() {
            let _ = std::fs::create_dir_all(&temp_parent);
        }
        let temp_dir = temp_parent.join(format!("{}_{}", session_id, "repo"));

        info!(
            "🌐 [OssRepositoryIndexer] Cloning repository: {} to {:?}",
            github_url, temp_dir
        );

        // 1. git clone --depth 1
        let output = crate::security::SafeCommandBuilder::new("git")
            .arg("clone")
            .arg("--depth")
            .arg("1")
            .arg(github_url)
            .arg(temp_dir.to_string_lossy().to_string())
            .build_internal()
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to build git command: {}", e),
            })?
            .output()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to execute git clone: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("❌ [OssRepositoryIndexer] Git clone failed: {}", stderr);
            return Err(AiomeError::Infrastructure {
                reason: format!("Git clone failed: {}", stderr),
            });
        }

        let session = OssKnowledgeSession {
            temp_dir: temp_dir.clone(),
        };

        // 2. Scan and Index
        let mut files_to_scan = Vec::new();
        let target_exts = ["md", "rs"];

        let mut stack = vec![temp_dir.clone()];
        while let Some(dir) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if name != "node_modules" && name != ".git" && name != "target" {
                            stack.push(path);
                        }
                    } else if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                            if target_exts.contains(&ext) {
                                files_to_scan.push(path);
                            }
                        }
                    }
                }
            }
        }

        info!(
            "📚 [OssRepositoryIndexer] Found {} files to index in {}",
            files_to_scan.len(),
            github_url
        );

        let jail =
            bastion::fs_guard::Jail::new(&temp_dir).map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        for file_path in files_to_scan {
            let rel_path = file_path
                .strip_prefix(&temp_dir)
                .unwrap_or(&file_path)
                .to_string_lossy()
                .to_string();
            let content =
                std::fs::read_to_string(&file_path).map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to read {}: {}", rel_path, e),
                })?;

            let source_tag = format!("oss_source:{}", github_url);
            let file_tag = format!("file:{}", rel_path);

            if file_path.extension().map(|e| e == "md").unwrap_or(false) {
                let chunks =
                    crate::knowledge_indexer::ProjectKnowledgeIndexer::chunk_markdown(&content);
                for (i, (title, chunk_content)) in chunks.into_iter().enumerate() {
                    let req = aiome_core::traits::CreateArtifactRequest {
                        title: format!("OSS Knowledge: {} [{}] ({})", title, github_url, rel_path),
                        category: aiome_core::traits::ArtifactCategory::Knowledge,
                        tags: vec![
                            source_tag.clone(),
                            file_tag.clone(),
                            "rag".to_string(),
                            "oss".to_string(),
                        ],
                        created_by: "OssRepositoryIndexer".to_string(),
                        files: vec![(
                            format!("{}_chunk_{}.md", rel_path.replace('/', "_"), i),
                            chunk_content.clone().into_bytes(),
                            "text/markdown".to_string(),
                        )],
                        karma_refs: vec![],
                        text_content: Some(chunk_content),
                        job_ref: None,
                        parent_refs: vec![],
                        is_protected: false,
                    };
                    if let Err(e) = self.artifact_store.save_artifact(req, &jail).await {
                        error!(
                            "❌ [OssRepositoryIndexer] Failed to save markdown chunk {}: {:?}",
                            i, e
                        );
                    }
                }
            } else if file_path.extension().map(|e| e == "rs").unwrap_or(false) {
                let req = aiome_core::traits::CreateArtifactRequest {
                    title: format!("OSS Source: {} [{}]", rel_path, github_url),
                    category: aiome_core::traits::ArtifactCategory::Knowledge,
                    tags: vec![
                        source_tag.clone(),
                        file_tag.clone(),
                        "source".to_string(),
                        "oss".to_string(),
                    ],
                    created_by: "OssRepositoryIndexer".to_string(),
                    files: vec![(
                        rel_path.replace('/', "_"),
                        content.clone().into_bytes(),
                        "text/x-rust".to_string(),
                    )],
                    karma_refs: vec![],
                    text_content: Some(content),
                    job_ref: None,
                    parent_refs: vec![],
                    is_protected: false,
                };
                if let Err(e) = self.artifact_store.save_artifact(req, &jail).await {
                    error!(
                        "❌ [OssRepositoryIndexer] Failed to save rust source: {:?}",
                        e
                    );
                }
            }
        }

        Ok(session)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::artifact_store::UniversalArtifactStore;
    use crate::db::DatabasePool;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_clone_and_index_success() {
        let temp_dir = tempdir().unwrap(); // allow-anti-pattern
        let db_path = temp_dir.path().join("test_oss.db");
        let pool = sqlx::SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.display()))
            .await
            .unwrap(); // allow-anti-pattern

        // Setup minimalist tables - matching UniversalArtifactStore exactly
        sqlx::query(
            "CREATE TABLE ai_artifacts (
            id TEXT PRIMARY KEY, 
            title TEXT, 
            category TEXT, 
            tags TEXT, 
            created_by TEXT, 
            dir_path TEXT,
            file_manifest TEXT,
            karma_refs TEXT,
            job_ref TEXT,
            signature TEXT,
            embedding BLOB,
            text_content TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        )
        .execute(&pool)
        .await
        .unwrap(); // allow-anti-pattern
        sqlx::query(
            "CREATE TABLE system_state (key TEXT PRIMARY KEY, value TEXT, updated_at TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap(); // allow-anti-pattern

        let db_pool = DatabasePool::Sqlite(pool.clone());
        let store = Arc::new(UniversalArtifactStore::new(
            db_pool,
            temp_dir.path().join("artifacts"),
        ));
        let indexer = OssRepositoryIndexer::new(store, pool.clone());

        // 1. Create a "remote" local git repo
        let remote_dir = temp_dir.path().join("remote_repo");
        std::fs::create_dir_all(&remote_dir).unwrap(); // allow-anti-pattern
        std::fs::write(
            remote_dir.join("README.md"),
            "# Test Repo\n## Section 1\nContent 1",
        )
        .unwrap(); // allow-anti-pattern
        std::fs::write(
            remote_dir.join("main.rs"),
            "fn main() { println!(\"hello\"); }",
        )
        .unwrap(); // allow-anti-pattern

        // Init git repo
        let mut cmd1 = std::process::Command::new("git");
        cmd1.arg("init").current_dir(&remote_dir);
        shared::security::harden_command(&mut cmd1);
        let _ = cmd1.output().unwrap(); // allow-anti-pattern

        let mut cmd2 = std::process::Command::new("git");
        cmd2.arg("add").arg(".").current_dir(&remote_dir);
        shared::security::harden_command(&mut cmd2);
        let _ = cmd2.output().unwrap(); // allow-anti-pattern

        let mut cmd3 = std::process::Command::new("git");
        cmd3.arg("commit")
            .arg("-m")
            .arg("initial commit")
            .current_dir(&remote_dir);
        shared::security::harden_command(&mut cmd3);
        let _ = cmd3.output().unwrap(); // allow-anti-pattern

        // 2. Clone and Index
        let remote_url = format!("file://{}", remote_dir.display());
        let result = indexer.clone_and_index(&remote_url, &["/"]).await;

        assert!(result.is_ok(), "Clone and index failed: {:?}", result.err());
        let session = result.unwrap(); // allow-anti-pattern
        assert!(session.temp_dir.exists());

        // 3. Verify Artifacts in DB
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ai_artifacts")
            .fetch_one(&pool)
            .await
            .unwrap(); // allow-anti-pattern
                       // Expecting 2 artifacts (1 for README chunk, 1 for main.rs)
        assert!(count >= 2, "Expected at least 2 artifacts, found {}", count);
    }

    #[tokio::test]
    async fn test_url_validation() {
        let temp_dir = tempfile::tempdir().unwrap(); // allow-anti-pattern
        let db_path = temp_dir.path().join("test_sec.db");
        let pool = sqlx::SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.display()))
            .await
            .unwrap(); // allow-anti-pattern
        let store = Arc::new(crate::artifact_store::UniversalArtifactStore::new(
            crate::db::DatabasePool::Sqlite(pool.clone()),
            temp_dir.path().join("artifacts"),
        ));
        let indexer = OssRepositoryIndexer::new(store, pool);

        assert!(indexer.validate_url("https://github.com/foo/bar").is_ok());
        assert!(indexer.validate_url("git://github.com/foo/bar.git").is_ok());

        // Protocol violation
        assert!(indexer.validate_url("http://github.com/foo/bar").is_err());
        assert!(indexer.validate_url("ftp://server.com/repo").is_err());

        // Injection violation
        assert!(indexer
            .validate_url("https://github.com/foo/bar;rm -rf /")
            .is_err());
        assert!(indexer
            .validate_url("https://github.com/foo/bar&whoami")
            .is_err());
        assert!(indexer
            .validate_url("https://github.com/foo/bar $(id)")
            .is_err());
    }
}
