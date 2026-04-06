/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! # CortexFileProjector — Agent-Native Document Discovery (ADR-025)
//!
//! Duke大学の研究に基づき、Cortex Wiki記事をファイルシステム階層として
//! 物理投影する。エージェントの自律探索（grep/cat/ls）による
//! ドキュメント処理精度を最大化する設計。
//!
//! ## ディレクトリ構造
//! ```text
//! workspace/cortex_fs/
//! ├── _index.md              ← 全記事タイトルの一覧
//! ├── rust/
//! │   ├── _concept.md        ← カテゴリの要約
//! │   └── async_await.md     ← 個別記事
//! └── security/
//!     ├── _concept.md
//!     └── abyss_vault.md
//! ```

use crate::db::DatabasePool;
use aiome_core::error::AiomeError;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// ファイルシステム投影レポート
#[derive(Debug, Default)]
pub struct ProjectionReport {
    /// 新規作成された記事ファイル数
    pub files_created: u32,
    /// 差分更新された記事ファイル数
    pub files_updated: u32,
    /// 変更なしでスキップされた数
    pub files_skipped: u32,
    /// カテゴリディレクトリ数
    pub categories_count: u32,
}

/// Cortex Wiki記事をファイルシステム階層として物理投影するモジュール。
///
/// DB上の `cortex_wiki_articles` と `cortex_concept_index` を基に、
/// エージェントが自律的に探索できるディレクトリ構造を生成する。
pub struct CortexFileProjector {
    pool: DatabasePool,
    projection_root: PathBuf,
}

impl CortexFileProjector {
    /// 新しいインスタンスを生成する
    ///
    /// # Arguments
    /// * `pool` - データベース接続プール
    /// * `projection_root` - 投影先のルートディレクトリ（例: `workspace/cortex_fs/`）
    pub fn new(pool: DatabasePool, projection_root: PathBuf) -> Self {
        Self {
            pool,
            projection_root,
        }
    }

    /// 投影済みファイルのルートパスを返す
    pub fn projection_root(&self) -> &Path {
        &self.projection_root
    }

    /// DB上のWiki記事を ファイルシステムに投影する。
    ///
    /// 1. 全カテゴリ（concept）を取得し、ディレクトリを作成
    /// 2. 各記事を `content_hash` で差分検出し、変更のあったもののみ書き出し
    /// 3. `_index.md` にカテゴリ横断の記事一覧を生成
    pub async fn project_to_filesystem(&self) -> Result<ProjectionReport, AiomeError> {
        let pool = self.pool.get_sqlite_pool_or_err()?;
        let mut report = ProjectionReport::default();

        // ルートディレクトリの確保
        tokio::fs::create_dir_all(&self.projection_root)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!(
                    "Failed to create projection root {}: {}",
                    self.projection_root.display(),
                    e
                ),
            })?;

        // 1. 全コンセプトとそれに紐づく article_ids を取得
        let concept_rows = sqlx::query(
            "SELECT concept, article_ids, summary FROM cortex_concept_index ORDER BY concept",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to fetch concept index: {}", e),
        })?;

        let mut index_entries: Vec<String> = Vec::new();
        let mut valid_paths = std::collections::HashSet::new();

        for row in &concept_rows {
            use sqlx::Row;
            let concept: String = row.try_get("concept").unwrap_or_default();
            let article_ids_json: String = row
                .try_get("article_ids")
                .unwrap_or_else(|_| "[]".to_string());
            let summary: Option<String> = row.try_get("summary").ok();

            if concept.is_empty() {
                continue;
            }

            // 安全なパース。無効なJSONの場合は単に空として扱い全体を止めない
            let article_ids: Vec<String> =
                serde_json::from_str(&article_ids_json).unwrap_or_default();

            // カテゴリディレクトリの作成
            let mut category_slug = slugify(&concept);
            if category_slug.is_empty() {
                // 非ASCII文字のみのカテゴリ名（例: 日本語）に対する Deterministic なスラグフォールバック
                category_slug = format!(
                    "cat_{}",
                    concept
                        .bytes()
                        .map(|b| format!("{:02x}", b))
                        .collect::<String>()
                );
            }
            let category_dir = self.projection_root.join(&category_slug);
            tokio::fs::create_dir_all(&category_dir)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to create category dir {}: {}", category_slug, e),
                })?;
            report.categories_count += 1;

            let concept_file = category_dir.join("_concept.md");
            valid_paths.insert(concept_file.clone());

            let mut concept_article_links = Vec::new();

            // 各記事の投影
            for article_id in &article_ids {
                // N+1について: aiomeはローカルのSQLiteを前提としているため、1ミリ秒で数千クエリを処理可能。
                // したがって、異常なJSONパーサーエラーを避けるこのループ方式が最も堅牢（Golden Rule準拠）。
                let article_opt = sqlx::query(
                    "SELECT title, content_md, content_hash FROM cortex_wiki_articles WHERE id = ?",
                )
                .bind(article_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to fetch article {}: {}", article_id, e),
                })?;

                if let Some(article_row) = article_opt {
                    use sqlx::Row;
                    let title: String = article_row.try_get("title").unwrap_or_default();
                    let content_md: String = article_row.try_get("content_md").unwrap_or_default();
                    let content_hash: String =
                        article_row.try_get("content_hash").unwrap_or_default();
                    let mut article_slug = slugify(&title);
                    if article_slug.is_empty() {
                        article_slug = slugify(article_id); // 非ASCII文字のみのタイトルの場合
                        if article_slug.is_empty() {
                            article_slug = "untitled".to_string();
                        }
                    }

                    let article_file = category_dir.join(format!("{}.md", article_slug));
                    valid_paths.insert(article_file.clone());

                    // 差分検出: ハッシュファイルが存在し、中身が一致していればスキップ
                    let hash_file = category_dir.join(format!(".{}.hash", article_slug));
                    valid_paths.insert(hash_file.clone());

                    let needs_update = match tokio::fs::read_to_string(&hash_file).await {
                        Ok(existing_hash) => existing_hash.trim() != content_hash,
                        Err(_) => true, // ファイルが存在しない = 新規
                    };

                    if needs_update {
                        // 記事本体の書き出し
                        let full_content = format!(
                            "# {}\n\n<!-- source_id: {} -->\n<!-- content_hash: {} -->\n\n{}\n",
                            title, article_id, content_hash, content_md
                        );
                        tokio::fs::write(&article_file, &full_content)
                            .await
                            .map_err(|e| AiomeError::Infrastructure {
                                reason: format!("Failed to write article {}: {}", title, e),
                            })?;

                        // ハッシュファイルの更新
                        tokio::fs::write(&hash_file, &content_hash)
                            .await
                            .map_err(|e| AiomeError::Infrastructure {
                                reason: format!("Failed to write hash file for {}: {}", title, e),
                            })?;

                        if article_file.exists() {
                            report.files_updated += 1;
                        } else {
                            report.files_created += 1;
                        }
                    } else {
                        report.files_skipped += 1;
                    }

                    concept_article_links.push(format!("- [{}]({}.md)", title, article_slug));
                }
            }

            // _concept.md (カテゴリ要約) の生成 (全記事を処理した後に正しいファイルパスで書き出す)
            let concept_md = format!(
                "# {}\n\n{}\n\n## Articles in this category\n\n{}\n",
                concept,
                summary.as_deref().unwrap_or("(No summary available)"),
                if concept_article_links.is_empty() {
                    String::from("*No articles in this category.*")
                } else {
                    concept_article_links.join("\n")
                }
            );
            tokio::fs::write(&concept_file, &concept_md)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to write _concept.md: {}", e),
                })?;

            // トップレベルインデックスにはカテゴリ情報だけを記述してトークンサイズ爆発を防止する
            index_entries.push(format!(
                "- **{}**: [Explore articles]({}/_concept.md)",
                concept, category_slug
            ));
        }

        // _index.md の生成
        let index_content = format!(
            "# Cortex Knowledge Base — File System Index\n\n> Auto-projected from Cortex Wiki. {} articles across {} categories.\n> Navigate categories by reading their `_concept.md` files.\n\n{}\n",
            report.files_created + report.files_updated + report.files_skipped,
            report.categories_count,
            index_entries.join("\n")
        );
        let index_file = self.projection_root.join("_index.md");
        valid_paths.insert(index_file.clone());
        tokio::fs::write(&index_file, &index_content)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to write _index.md: {}", e),
            })?;

        // 孤立ファイルのクリーンアップ（削除されたWiki記事や古いハッシュの完全消去）
        if let Ok(mut entries) = tokio::fs::read_dir(&self.projection_root).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(mut sub_entries) = tokio::fs::read_dir(&path).await {
                        while let Ok(Some(sub_entry)) = sub_entries.next_entry().await {
                            let sub_path = sub_entry.path();
                            if !valid_paths.contains(&sub_path) {
                                let _ = tokio::fs::remove_file(&sub_path).await;
                            }
                        }
                    }
                    let _ = tokio::fs::remove_dir(&path).await; // 空になったら成功する
                } else if !valid_paths.contains(&path) {
                    let _ = tokio::fs::remove_file(&path).await;
                }
            }
        }

        info!(
            "📂 [CortexFileProjector] Projection complete: {} created, {} updated, {} skipped across {} categories",
            report.files_created, report.files_updated, report.files_skipped, report.categories_count
        );

        Ok(report)
    }
}

/// 文字列をファイルシステムセーフなスラグに変換し、長さを64バイトに制限する
fn slugify(input: &str) -> String {
    let mut slug = input
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' | '-' | '_' => c,
            ' ' | '/' | '\\' | ':' => '_',
            _ => '_',
        })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_");

    // 255文字(OS制限)を超えないよう、安全余裕を見て64文字で切り詰め（Edge Case対応）
    if slug.len() > 64 {
        let mut end = 64;
        while !slug.is_char_boundary(end) {
            end -= 1;
        }
        slug.truncate(end);
        // 末尾が `_` の場合は除外
        slug = slug.trim_end_matches('_').to_string();
    }
    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("Rust Async"), "rust_async");
        assert_eq!(slugify("Abyss Vault"), "abyss_vault");
        assert_eq!(slugify("Hello / World"), "hello_world");
    }

    #[test]
    fn test_slugify_special_chars() {
        assert_eq!(slugify("C++ Programming"), "c_programming");
        assert_eq!(slugify("日本語テスト"), "");
        assert_eq!(slugify("Hello---World"), "hello---world");
    }

    #[test]
    fn test_slugify_empty_input() {
        assert_eq!(slugify(""), "");
    }

    #[tokio::test]
    async fn test_projector_creates_root_dir() {
        let tmp_dir = std::env::temp_dir().join(format!("cortex_test_{}", uuid::Uuid::new_v4()));

        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .expect("Should create in-memory DB"); // allow-anti-pattern

        let sqlite_pool = pool
            .get_sqlite_pool_or_err()
            .expect("Should get sqlite pool"); // allow-anti-pattern

        // Create required tables
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS cortex_concept_index (
                concept TEXT PRIMARY KEY,
                article_ids TEXT DEFAULT '[]',
                document_ids TEXT DEFAULT '[]',
                summary TEXT,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(sqlite_pool)
        .await
        .expect("Should create concept_index table"); // allow-anti-pattern

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS cortex_wiki_articles (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content_md TEXT NOT NULL,
                concepts TEXT DEFAULT '[]',
                backlinks TEXT DEFAULT '[]',
                source_refs TEXT DEFAULT '[]',
                content_hash TEXT NOT NULL,
                version INTEGER DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(sqlite_pool)
        .await
        .expect("Should create wiki_articles table"); // allow-anti-pattern

        // Seed test data
        sqlx::query(
            "INSERT INTO cortex_wiki_articles (id, title, content_md, content_hash)
             VALUES ('art-1', 'Rust Async', 'Rust uses async/await.', 'hash_abc')",
        )
        .execute(sqlite_pool)
        .await
        .expect("Should insert article"); // allow-anti-pattern

        sqlx::query(
            "INSERT INTO cortex_concept_index (concept, article_ids, summary)
             VALUES ('rust', '[\"art-1\"]', 'The Rust programming language')",
        )
        .execute(sqlite_pool)
        .await
        .expect("Should insert concept"); // allow-anti-pattern

        let projector = CortexFileProjector::new(pool, tmp_dir.clone());
        let report = projector
            .project_to_filesystem()
            .await
            .expect("Projection should succeed"); // allow-anti-pattern

        assert_eq!(report.categories_count, 1);
        assert!(report.files_created + report.files_updated > 0);

        // Verify file exists
        let article_path = tmp_dir.join("rust").join("rust_async.md");
        assert!(
            article_path.exists(),
            "Article file should exist at {:?}",
            article_path
        );

        // Verify index exists
        let index_path = tmp_dir.join("_index.md");
        assert!(index_path.exists(), "Index file should exist");

        // Cleanup
        let _ = tokio::fs::remove_dir_all(&tmp_dir).await;
    }

    #[tokio::test]
    async fn test_projector_skips_unchanged_articles() {
        let tmp_dir = std::env::temp_dir().join(format!("cortex_skip_{}", uuid::Uuid::new_v4()));

        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .expect("Should create in-memory DB"); // allow-anti-pattern
        let sqlite_pool = pool
            .get_sqlite_pool_or_err()
            .expect("Should get sqlite pool"); // allow-anti-pattern

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS cortex_concept_index (
                concept TEXT PRIMARY KEY,
                article_ids TEXT DEFAULT '[]',
                document_ids TEXT DEFAULT '[]',
                summary TEXT,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(sqlite_pool)
        .await
        .expect("Should create concept_index table"); // allow-anti-pattern

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS cortex_wiki_articles (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content_md TEXT NOT NULL,
                concepts TEXT DEFAULT '[]',
                backlinks TEXT DEFAULT '[]',
                source_refs TEXT DEFAULT '[]',
                content_hash TEXT NOT NULL,
                version INTEGER DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(sqlite_pool)
        .await
        .expect("Should create wiki_articles table"); // allow-anti-pattern

        sqlx::query(
            "INSERT INTO cortex_wiki_articles (id, title, content_md, content_hash)
             VALUES ('art-2', 'Security Design', 'Defense in depth.', 'hash_xyz')",
        )
        .execute(sqlite_pool)
        .await
        .expect("Should insert article"); // allow-anti-pattern

        sqlx::query(
            "INSERT INTO cortex_concept_index (concept, article_ids)
             VALUES ('security', '[\"art-2\"]')",
        )
        .execute(sqlite_pool)
        .await
        .expect("Should insert concept"); // allow-anti-pattern

        let projector = CortexFileProjector::new(pool, tmp_dir.clone());

        // First projection
        let report1 = projector
            .project_to_filesystem()
            .await
            .expect("First projection should succeed"); // allow-anti-pattern
        assert!(report1.files_created + report1.files_updated > 0);

        // Second projection (no changes)
        let report2 = projector
            .project_to_filesystem()
            .await
            .expect("Second projection should succeed"); // allow-anti-pattern
        assert_eq!(
            report2.files_skipped, 1,
            "Unchanged article should be skipped"
        );

        // Cleanup
        let _ = tokio::fs::remove_dir_all(&tmp_dir).await;
    }
}
