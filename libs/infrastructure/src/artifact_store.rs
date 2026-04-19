/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::EmbeddingProvider;
use aiome_core_contracts::traits::{
    ArtifactCategory, ArtifactEdge, ArtifactFile, ArtifactMeta, ArtifactStore,
    CreateArtifactRequest,
};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use shared::sandbox::PathSandbox;
use sqlx::Row;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::DatabasePool;
use crate::polar_quant::PolarQuantEncoder;
use crate::sql_exec;
use crate::vector_ops::cosine_similarity;

use crate::vector_ops::{StandardVectorOps, VectorOps};

// Local cosine_similarity replaced by crate::vector_ops::cosine_similarity

/// Artifacts 永続化ストア (Universal: SQLite/PostgreSQL 対応)
pub struct UniversalArtifactStore {
    pool: DatabasePool,
    base_dir: PathBuf,           // Controlled via AppDataResolver (e.g. artifacts)
    vault_path: Option<PathBuf>, // Phase 3: DRM 隔離領域
    embed_provider: Option<Arc<dyn EmbeddingProvider>>,
    audit_logger: Option<Arc<dyn aiome_core_contracts::audit::AuditLogger>>,
    job_queue: Option<Arc<dyn aiome_core_contracts::traits::TaskRegistry>>,
}

const MAX_ARTIFACT_FILE_SIZE: usize = 10 * 1024 * 1024; // 10MB
const MAX_ARTIFACT_FILES: usize = 20;

impl UniversalArtifactStore {
    /// 新しいインスタンスを生成する
    pub fn new(pool: DatabasePool, base_dir: PathBuf) -> Self {
        Self {
            pool,
            base_dir,
            vault_path: None,
            embed_provider: None,
            audit_logger: None,
            job_queue: None,
        }
    }

    /// 隔離領域を設定する
    pub fn with_vault(mut self, path: PathBuf) -> Self {
        self.vault_path = Some(path);
        self
    }

    /// 埋め込みプロバイダを設定する
    pub fn with_embeddings(mut self, provider: Arc<dyn EmbeddingProvider>) -> Self {
        self.embed_provider = Some(provider);
        self
    }

    /// 監査ロガーを設定する
    pub fn with_audit_logger(
        mut self,
        logger: Arc<dyn aiome_core_contracts::audit::AuditLogger>,
    ) -> Self {
        self.audit_logger = Some(logger);
        self
    }

    /// ジョブキューを設定する
    pub fn with_job_queue(
        mut self,
        job_queue: Arc<dyn aiome_core_contracts::traits::TaskRegistry>,
    ) -> Self {
        self.job_queue = Some(job_queue);
        self
    }

    fn calculate_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    // Helper to map Sqlite row to ArtifactMeta
    fn map_sqlite_row(row: sqlx::sqlite::SqliteRow) -> Result<ArtifactMeta, AiomeError> {
        let cat_str: String = row.try_get("category").unwrap_or_default();
        let tags_json: String = row.try_get("tags").unwrap_or_default();
        let manifest_json: String = row.try_get("file_manifest").unwrap_or_default();
        let karma_json: String = row.try_get("karma_refs").unwrap_or_default();

        Ok(ArtifactMeta {
            id: row.try_get("id").unwrap_or_default(),
            title: row.try_get("title").unwrap_or_default(),
            category: serde_json::from_str(&format!("\"{}\"", cat_str))
                .unwrap_or(ArtifactCategory::Report),
            tags: serde_json::from_str(&tags_json).unwrap_or_default(),
            created_by: row.try_get("created_by").unwrap_or_default(),
            dir_path: row.try_get("dir_path").unwrap_or_default(),
            files: serde_json::from_str(&manifest_json).unwrap_or_default(),
            karma_refs: serde_json::from_str(&karma_json).unwrap_or_default(),
            job_ref: row.try_get("job_ref").unwrap_or_default(),
            soul_version_hash: row.try_get("soul_version_hash").unwrap_or_default(),
            signature: row.try_get("signature").unwrap_or_default(),
            text_content: row.try_get("text_content").unwrap_or_default(),
            edges: Vec::new(),
            created_at: row.try_get("created_at").unwrap_or_default(),
        })
    }

    // Helper to map Postgres row to ArtifactMeta
    fn map_postgres_row(row: sqlx::postgres::PgRow) -> Result<ArtifactMeta, AiomeError> {
        let cat_str: String = row.try_get("category").unwrap_or_default();
        let tags_json: String = row.try_get("tags").unwrap_or_default();
        let manifest_json: String = row.try_get("file_manifest").unwrap_or_default();
        let karma_json: String = row.try_get("karma_refs").unwrap_or_default();

        Ok(ArtifactMeta {
            id: row.try_get("id").unwrap_or_default(),
            title: row.try_get("title").unwrap_or_default(),
            category: serde_json::from_str(&format!("\"{}\"", cat_str))
                .unwrap_or(ArtifactCategory::Report),
            tags: serde_json::from_str(&tags_json).unwrap_or_default(),
            created_by: row.try_get("created_by").unwrap_or_default(),
            dir_path: row.try_get("dir_path").unwrap_or_default(),
            files: serde_json::from_str(&manifest_json).unwrap_or_default(),
            karma_refs: serde_json::from_str(&karma_json).unwrap_or_default(),
            job_ref: row.try_get("job_ref").unwrap_or_default(),
            soul_version_hash: row.try_get("soul_version_hash").unwrap_or_default(),
            signature: row.try_get("signature").unwrap_or_default(),
            text_content: row.try_get("text_content").unwrap_or_default(),
            edges: Vec::new(),
            created_at: row.try_get("created_at").unwrap_or_default(),
        })
    }
}

#[async_trait]
impl ArtifactStore for UniversalArtifactStore {
    async fn save_artifact(
        &self,
        req: CreateArtifactRequest,
        jail: &bastion::fs_guard::Jail,
    ) -> Result<String, AiomeError> {
        let id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now();
        let dir_name = format!("{}_{}", timestamp.format("%Y-%m-%d"), &id[..8]);

        // Phase 3: Protected Area 判定
        let current_base_dir = if req.is_protected {
            self.vault_path
                .as_ref()
                .cloned()
                .unwrap_or_else(|| self.base_dir.clone())
        } else {
            self.base_dir.clone()
        };

        if !current_base_dir.exists() {
            std::fs::create_dir_all(&current_base_dir).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to create base dir: {}", e),
            })?;
        }

        // サンドボックスを現在のベースディレクトリ（workspace または vault）で初期化
        let sandbox =
            PathSandbox::new(&current_base_dir).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to initialize sandbox: {}", e),
            })?;

        let artifacts_base = current_base_dir.join("artifacts");
        if !artifacts_base.exists() {
            if let Err(e) = std::fs::create_dir_all(&artifacts_base) {
                tracing::error!(
                    "❌ [ArtifactStore] Failed to create artifacts directory {:?}: {}",
                    artifacts_base,
                    e
                );
            }
        }

        let relative_dir = Path::new("artifacts").join(&dir_name);
        let full_dir =
            sandbox
                .validate_path(&relative_dir)
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Security violation - invalid artifact path: {}", e),
                })?;

        std::fs::create_dir_all(&full_dir).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create artifact dir: {}", e),
        })?;

        let mut artifact_files = Vec::new();
        let mut manifest_hasher = Sha256::new();

        if req.files.len() > MAX_ARTIFACT_FILES {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Too many files in artifact: {} exceeds limit of {}",
                    req.files.len(),
                    MAX_ARTIFACT_FILES
                ),
            });
        }

        if req.tags.len() > 50 {
            return Err(AiomeError::Infrastructure {
                reason: format!("Too many tags: {} exceeds limit of 50", req.tags.len()),
            });
        }

        if req.karma_refs.len() > 100 {
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "Too many karma refs: {} exceeds limit of 100",
                    req.karma_refs.len()
                ),
            });
        }

        for (filename, content, mime_type) in req.files {
            if content.len() > MAX_ARTIFACT_FILE_SIZE {
                return Err(AiomeError::Infrastructure {
                    reason: format!(
                        "File size limit exceeded for {}: {} bytes exceeds limit of {} bytes",
                        filename,
                        content.len(),
                        MAX_ARTIFACT_FILE_SIZE
                    ),
                });
            }
            let hash = Self::calculate_hash(&content);
            let file_path = sandbox
                .validate_path(full_dir.join(&filename))
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Security violation - invalid file path: {}", e),
                })?;

            std::fs::write(&file_path, &content).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to write artifact file {}: {}", filename, e),
            })?;

            let file_meta = ArtifactFile {
                name: filename,
                mime_type,
                size_bytes: content.len() as u64,
                hash: hash.clone(),
            };

            manifest_hasher.update(hash.as_bytes());
            artifact_files.push(file_meta);
        }

        let file_manifest_json = serde_json::to_string(&artifact_files).unwrap_or_default();
        let tags_json = serde_json::to_string(&req.tags).unwrap_or_default();
        let karma_refs_json = serde_json::to_string(&req.karma_refs).unwrap_or_default();

        let mut embedding_blob: Option<Vec<u8>> = None;
        if let Some(ref provider) = self.embed_provider {
            let context = format!(
                "{} {:?} {} {}",
                req.title,
                req.category,
                req.tags.join(" "),
                req.text_content.as_deref().unwrap_or("")
            );
            if let Ok(vec) = provider.embed(&context, false).await {
                let encoder = PolarQuantEncoder::new(4, 32);
                let vec_f64: Vec<f64> = vec.into_iter().map(|f| f as f64).collect();
                embedding_blob = Some(encoder.encode(&vec_f64));
            }
        }

        let signature = format!("{:x}", manifest_hasher.finalize());
        let cat_str = serde_json::to_string(&req.category)
            .unwrap_or_default()
            .replace("\"", "");

        let q = format!(
            "INSERT INTO ai_artifacts (id, title, category, tags, created_by, dir_path, file_manifest, karma_refs, job_ref, signature, embedding, text_content) 
             VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, {10}, {11})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4),
            self.pool.ph(5), self.pool.ph(6), self.pool.ph(7), self.pool.ph(8), self.pool.ph(9),
            self.pool.ph(10), self.pool.ph(11)
        );

        sql_exec!(
            &self.pool,
            &q,
            &id,
            &req.title,
            &cat_str,
            &tags_json,
            &req.created_by,
            relative_dir.display().to_string(),
            &file_manifest_json,
            &karma_refs_json,
            req.job_ref,
            &signature,
            embedding_blob,
            req.text_content
        )?;

        info!("📦 Artifact saved: {}", id);

        // RT-6 Audit Logging
        if let Some(logger) = &self.audit_logger {
            let details = serde_json::json!({
                "artifact_id": id,
                "title": req.title,
                "category": cat_str,
                "is_protected": req.is_protected,
            });
            if let Err(e) = logger
                .log_event("ARTIFACT_CREATE", &req.created_by, &details)
                .await
            {
                tracing::error!(
                    "❌ [ArtifactStore] Audit log for ARTIFACT_CREATE failed: {}. Artifact ID: {}",
                    e,
                    id
                );
            }
        }

        // Phase 1 Step C: Enqueue CSAM async scan
        if matches!(req.category, ArtifactCategory::Image) {
            if let Some(jq) = &self.job_queue {
                if let Err(e) = jq
                    .enqueue("csam_scan", &id, "security", None, None, None, 0)
                    .await
                {
                    tracing::error!("🚨 [Security] CSAM scan enqueue failed for {}: {:?}", id, e);
                }
            } else {
                tracing::error!(
                    "🚨 [Security] csam_scan bypassed for {} because job_queue is NOT injected! This is a severe compliance risk.",
                    id
                );
            }
        }

        Ok(id)
    }

    async fn list_artifacts(
        &self,
        category: Option<ArtifactCategory>,
        limit: i64,
    ) -> Result<Vec<ArtifactMeta>, AiomeError> {
        let effective_limit = limit.min(100);
        let sql = if category.is_some() {
            format!(
                "SELECT * FROM ai_artifacts WHERE category = {} ORDER BY created_at DESC LIMIT {}",
                self.pool.ph(0),
                effective_limit
            )
        } else {
            format!(
                "SELECT * FROM ai_artifacts ORDER BY created_at DESC LIMIT {}",
                effective_limit
            )
        };

        match &self.pool {
            DatabasePool::Sqlite(p) => {
                let mut query = sqlx::query(&sql);
                if let Some(ref cat) = category {
                    query = query.bind(
                        serde_json::to_string(cat)
                            .unwrap_or_default()
                            .replace("\"", ""),
                    );
                }
                let rows = query.fetch_all(p).await.map_err(|e: sqlx::Error| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
                let mut results = Vec::new();
                for r in rows {
                    results.push(Self::map_sqlite_row(r)?);
                }
                Ok(results)
            }
            DatabasePool::Postgres(p) => {
                let mut query = sqlx::query(&sql);
                if let Some(ref cat) = category {
                    query = query.bind(
                        serde_json::to_string(cat)
                            .unwrap_or_default()
                            .replace("\"", ""),
                    );
                }
                let rows = query.fetch_all(p).await.map_err(|e: sqlx::Error| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
                let mut results = Vec::new();
                for r in rows {
                    results.push(Self::map_postgres_row(r)?);
                }
                Ok(results)
            }
        }
    }

    async fn fetch_artifact(&self, id: &str) -> Result<Option<ArtifactMeta>, AiomeError> {
        let q = format!("SELECT * FROM ai_artifacts WHERE id = {}", self.pool.ph(0));
        let meta = match &self.pool {
            DatabasePool::Sqlite(p) => {
                let row = sqlx::query(&q).bind(id).fetch_optional(p).await.map_err(
                    |e: sqlx::Error| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    },
                )?;
                if let Some(r) = row {
                    Some(Self::map_sqlite_row(r)?)
                } else {
                    None
                }
            }
            DatabasePool::Postgres(p) => {
                let row = sqlx::query(&q).bind(id).fetch_optional(p).await.map_err(
                    |e: sqlx::Error| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    },
                )?;
                if let Some(r) = row {
                    Some(Self::map_postgres_row(r)?)
                } else {
                    None
                }
            }
        };

        if let Some(mut m) = meta {
            m.edges = self.get_artifact_edges(id).await.unwrap_or_default();
            Ok(Some(m))
        } else {
            Ok(None)
        }
    }

    async fn read_artifact_file(
        &self,
        id: &str,
        filename: &str,
        jail: &bastion::fs_guard::Jail,
    ) -> Result<Vec<u8>, AiomeError> {
        let meta = self
            .fetch_artifact(id)
            .await?
            .ok_or(AiomeError::ArtifactNotFound {
                path: id.to_string(),
            })?;
        let root = jail.root().to_path_buf();
        let sandbox = PathSandbox::new(&root).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Sandbox error: {}", e),
        })?;
        let full_path = sandbox
            .validate_path(Path::new(&meta.dir_path).join(filename))
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Path error: {}", e),
            })?;

        // RT-6 Audit Logging (Critical for DRM)
        if let Some(logger) = &self.audit_logger {
            let details = serde_json::json!({
                "artifact_id": id,
                "filename": filename,
                "protected": meta.dir_path.contains("vault") || meta.dir_path.contains(".abyss_vault"),
            });
            if let Err(e) = logger
                .log_event("ARTIFACT_READ", &meta.created_by, &details)
                .await
            {
                tracing::error!(
                    "❌ [ArtifactStore] Audit log for ARTIFACT_READ failed: {}. Artifact ID: {}",
                    e,
                    id
                );
            }
        }

        std::fs::read(full_path).map_err(|e| AiomeError::Infrastructure {
            reason: format!("IO error: {}", e),
        })
    }

    async fn delete_artifact(
        &self,
        id: &str,
        jail: &bastion::fs_guard::Jail,
    ) -> Result<(), AiomeError> {
        let meta = self
            .fetch_artifact(id)
            .await?
            .ok_or(AiomeError::ArtifactNotFound {
                path: id.to_string(),
            })?;
        let root = jail.root().to_path_buf();
        let sandbox = PathSandbox::new(&root).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Sandbox error: {}", e),
        })?;
        if let Ok(full_dir) = sandbox.validate_path(&meta.dir_path) {
            for file in meta.files {
                let _ = std::fs::remove_file(full_dir.join(&file.name));
            }
            let _ = std::fs::remove_dir(full_dir);
        }
        sql_exec!(
            &self.pool,
            &format!("DELETE FROM ai_artifacts WHERE id = {}", self.pool.ph(0)),
            id
        )?;
        sql_exec!(
            &self.pool,
            &format!(
                "DELETE FROM artifact_edges WHERE source_id = {0} OR target_id = {0}",
                self.pool.ph(0)
            ),
            id
        )?;
        Ok(())
    }

    async fn get_artifact_edges(&self, id: &str) -> Result<Vec<ArtifactEdge>, AiomeError> {
        let q = format!(
            "SELECT * FROM artifact_edges WHERE source_id = {0} OR target_id = {0}",
            self.pool.ph(0)
        );
        match &self.pool {
            DatabasePool::Sqlite(p) => {
                let rows =
                    sqlx::query(&q)
                        .bind(id)
                        .fetch_all(p)
                        .await
                        .map_err(|e: sqlx::Error| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                let mut results = Vec::new();
                for r in rows {
                    results.push(ArtifactEdge {
                        id: r.get("id"),
                        source_id: r.get("source_id"),
                        target_id: r.get("target_id"),
                        source_type: r.get("source_type"),
                        relation: r.get("relation"),
                        metadata: serde_json::from_str(&r.get::<String, _>("metadata"))
                            .unwrap_or_default(),
                        created_at: r.get("created_at"),
                    });
                }
                Ok(results)
            }
            DatabasePool::Postgres(p) => {
                let rows =
                    sqlx::query(&q)
                        .bind(id)
                        .fetch_all(p)
                        .await
                        .map_err(|e: sqlx::Error| AiomeError::Infrastructure {
                            reason: e.to_string(),
                        })?;
                let mut results = Vec::new();
                for r in rows {
                    results.push(ArtifactEdge {
                        id: r.get("id"),
                        source_id: r.get("source_id"),
                        target_id: r.get("target_id"),
                        source_type: r.get("source_type"),
                        relation: r.get("relation"),
                        metadata: serde_json::from_str(&r.get::<String, _>("metadata"))
                            .unwrap_or_default(),
                        created_at: r.get("created_at"),
                    });
                }
                Ok(results)
            }
        }
    }

    async fn add_artifact_edge(&self, edge: ArtifactEdge) -> Result<(), AiomeError> {
        let q = format!("INSERT INTO artifact_edges (id, source_id, target_id, source_type, relation, metadata) VALUES ({0}, {1}, {2}, {3}, {4}, {5})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5));
        sql_exec!(
            &self.pool,
            &q,
            &edge.id,
            &edge.source_id,
            &edge.target_id,
            &edge.source_type,
            &edge.relation,
            serde_json::to_string(&edge.metadata).unwrap_or_else(|_| "{}".to_string())
        )?;
        Ok(())
    }

    async fn search_artifacts_semantic(
        &self,
        query: &str,
        category: Option<ArtifactCategory>,
        limit: i64,
    ) -> Result<Vec<ArtifactMeta>, AiomeError> {
        let provider = self
            .embed_provider
            .as_ref()
            .ok_or(AiomeError::Infrastructure {
                reason: "No embed provider".into(),
            })?;
        let query_vec = provider.embed(query, true).await?;
        let query_vec_f64: Vec<f64> = query_vec.iter().map(|&f| f as f64).collect();

        let mut sql =
            "SELECT id, embedding FROM ai_artifacts WHERE embedding IS NOT NULL".to_string();
        if category.is_some() {
            sql.push_str(&format!(" AND category = {}", self.pool.ph(0)));
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT 1000");

        let entries = match &self.pool {
            DatabasePool::Sqlite(p) => {
                let mut db_query = sqlx::query(&sql);
                if let Some(ref cat) = category {
                    db_query = db_query.bind(
                        serde_json::to_string(cat)
                            .unwrap_or_default()
                            .replace("\"", ""),
                    );
                }
                let rows = db_query.fetch_all(p).await.map_err(|e: sqlx::Error| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
                rows.into_iter()
                    .map(|r| (r.get::<String, _>("id"), r.get::<Vec<u8>, _>("embedding")))
                    .collect::<Vec<_>>()
            }
            DatabasePool::Postgres(p) => {
                let mut db_query = sqlx::query(&sql);
                if let Some(ref cat) = category {
                    db_query = db_query.bind(
                        serde_json::to_string(cat)
                            .unwrap_or_default()
                            .replace("\"", ""),
                    );
                }
                let rows = db_query.fetch_all(p).await.map_err(|e: sqlx::Error| {
                    AiomeError::Infrastructure {
                        reason: e.to_string(),
                    }
                })?;
                rows.into_iter()
                    .map(|r| (r.get::<String, _>("id"), r.get::<Vec<u8>, _>("embedding")))
                    .collect::<Vec<_>>()
            }
        };

        let embed_dim = provider.embedding_dim();
        let mut sim_results: Vec<(f64, String)> = Vec::new();
        for (id, emb_bytes) in entries {
            let score = StandardVectorOps::approximate_cosine_similarity(
                &query_vec_f64,
                &emb_bytes,
                embed_dim,
            );
            sim_results.push((score, id));
        }
        sim_results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut results = Vec::new();
        for (_, id_str) in sim_results.into_iter().take(limit as usize) {
            if let Some(meta) = self.fetch_artifact(&id_str).await? {
                results.push(meta);
            }
        }
        Ok(results)
    }
}
