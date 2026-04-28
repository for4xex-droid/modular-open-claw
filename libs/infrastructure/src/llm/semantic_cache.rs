/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::job_queue::UniversalJobQueue;
use crate::polar_quant::PolarQuantEncoder;
use crate::vector_ops::{StandardVectorOps, VectorOps};
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{LlmResponse, StopReason};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::sync::Arc;
use tracing::{debug, info};

use async_trait::async_trait;
use crate::db::DatabasePool;
use aiome_core_contracts::llm::EmbeddingProvider;

#[async_trait]
pub trait SemanticCacheRepository: Send + Sync {
    async fn get_by_hash(&self, hash: &str) -> Result<Option<String>, AiomeError>;
    async fn get_all_embeddings(&self) -> Result<Vec<(String, Vec<u8>)>, AiomeError>;
    async fn set(
        &self,
        hash: &str,
        response: &str,
        provider_name: &str,
        model_name: &str,
        ttl_seconds: i64,
        embedding: Option<Vec<u8>>,
    ) -> Result<(), AiomeError>;
}

pub struct SqlSemanticCacheRepository {
    pool: DatabasePool,
}

impl SqlSemanticCacheRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SemanticCacheRepository for SqlSemanticCacheRepository {
    async fn get_by_hash(&self, hash: &str) -> Result<Option<String>, AiomeError> {
        let exact_q = format!(
            "SELECT response FROM llm_response_cache WHERE prompt_hash = {} AND created_at > {}",
            self.pool.ph(0),
            if self.pool.is_sqlite() {
                "datetime('now', '-' || ttl_seconds || ' seconds')"
            } else {
                "NOW() - (ttl_seconds || ' seconds')::interval"
            }
        );

        let hash_match: Option<String> = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar(&exact_q)
                .bind(hash)
                .fetch_optional(p)
                .await
                .ok()
                .flatten(),
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar(&exact_q)
                .bind(hash)
                .fetch_optional(p)
                .await
                .ok()
                .flatten(),
        };

        Ok(hash_match)
    }

    async fn get_all_embeddings(&self) -> Result<Vec<(String, Vec<u8>)>, AiomeError> {
        use sqlx::Row;
        let semantic_q = "SELECT response, prompt_embedding FROM llm_response_cache WHERE prompt_embedding IS NOT NULL ORDER BY created_at DESC LIMIT 100";
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(semantic_q)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to fetch embeddings: {}", e),
                    })?;
                Ok(rows
                    .into_iter()
                    .map(|row| {
                        let response: String = row.get("response");
                        let emb_bytes: Vec<u8> = row.get("prompt_embedding");
                        (response, emb_bytes)
                    })
                    .collect())
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(semantic_q)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("Failed to fetch embeddings: {}", e),
                    })?;
                Ok(rows
                    .into_iter()
                    .map(|row| {
                        let response: String = row.get("response");
                        let emb_bytes: Vec<u8> = row.get("prompt_embedding");
                        (response, emb_bytes)
                    })
                    .collect())
            }
        }
    }

    async fn set(
        &self,
        hash: &str,
        response: &str,
        provider_name: &str,
        model_name: &str,
        ttl_seconds: i64,
        embedding: Option<Vec<u8>>,
    ) -> Result<(), AiomeError> {
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                sqlx::query("INSERT OR REPLACE INTO llm_response_cache (prompt_hash, response, provider_name, model_name, ttl_seconds, prompt_embedding, created_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))")
                    .bind(hash)
                    .bind(response)
                    .bind(provider_name)
                    .bind(model_name)
                    .bind(ttl_seconds)
                    .bind(embedding)
                    .execute(p)
                    .await
                    .map(|_| ())
            }
            crate::db::DatabasePool::Postgres(p) => {
                sqlx::query("INSERT INTO llm_response_cache (prompt_hash, response, provider_name, model_name, ttl_seconds, prompt_embedding, created_at) VALUES ($1, $2, $3, $4, $5, $6, NOW()) ON CONFLICT (prompt_hash) DO UPDATE SET response = EXCLUDED.response, ttl_seconds = EXCLUDED.ttl_seconds, prompt_embedding = EXCLUDED.prompt_embedding, created_at = EXCLUDED.created_at")
                    .bind(hash)
                    .bind(response)
                    .bind(provider_name)
                    .bind(model_name)
                    .bind(ttl_seconds)
                    .bind(embedding)
                    .execute(p)
                    .await
                    .map(|_| ())
            }
        }
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Cache write failed: {}", e),
        })?;
        Ok(())
    }
}

/// LLMセマンティックキャッシュ
/// 同一または類似のプロンプトに対する応答をSQLiteにキャッシュし、コストを削減する。
pub struct SemanticCache {
    repo: Arc<dyn SemanticCacheRepository>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
}

impl SemanticCache {
    /// SemanticCache の新規インスタンスを生成する
    pub fn new(
        repo: Arc<dyn SemanticCacheRepository>,
        embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    ) -> Self {
        Self { repo, embedding_provider }
    }

    /// プロンプトのハッシュ計算
    fn compute_hash(prompt: &str, system: Option<&str>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        if let Some(sys) = system {
            hasher.update(sys.as_bytes());
        }
        hex::encode(hasher.finalize())
    }

    /// キャッシュから取得
    pub async fn get(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<Option<LlmResponse>, AiomeError> {
        let hash = Self::compute_hash(prompt, system);

        // 1. 完全一致 (Hash) を優先
        if let Some(content) = self.repo.get_by_hash(&hash).await? {
            debug!("🎯 [SemanticCache] Exact Hit! Hash: {}", hash);
            return Ok(Some(LlmResponse {
                content,
                stop_reason: StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            }));
        }

        // 2. セマンティック一致 (Vector)
        if let Some(provider) = &self.embedding_provider {
            let embed_dim = provider.embedding_dim();
            if let Ok(query_vec_f32) = provider.embed(prompt, true).await {
                let query_vec: Vec<f64> = query_vec_f32.iter().map(|&f| f as f64).collect();

                if let Ok(rows) = self.repo.get_all_embeddings().await {
                    let mut best: Option<String> = None;
                    for (response, emb_bytes) in rows {
                        let score = StandardVectorOps::approximate_cosine_similarity(
                            &query_vec, &emb_bytes, embed_dim,
                        );
                        if score > 0.95 {
                            best = Some(response);
                            info!(
                                "🧠 [SemanticCache] Semantic Hit! Score: {:.4}",
                                score
                            );
                            break;
                        }
                    }

                    if let Some(content) = best {
                        return Ok(Some(LlmResponse {
                            content,
                            stop_reason: StopReason::EndTurn,
                            reasoning: None,
                            metadata: None,
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    /// キャッシュに保存
    pub async fn set(
        &self,
        prompt: &str,
        system: Option<&str>,
        response: &LlmResponse,
        provider_name: &str,
        model_name: &str,
        ttl_seconds: i64,
    ) -> Result<(), AiomeError> {
        let hash = Self::compute_hash(prompt, system);
        let mut embedding: Option<Vec<u8>> = None;

        if let Some(provider) = &self.embedding_provider {
            if let Ok(vec) = provider.embed(prompt, false).await {
                let encoder = PolarQuantEncoder::new(4, 32);
                let vec_f64: Vec<f64> = vec.into_iter().map(|f| f as f64).collect();
                embedding = Some(encoder.encode(&vec_f64));
            }
        }

        self.repo
            .set(
                &hash,
                &response.content,
                provider_name,
                model_name,
                ttl_seconds,
                embedding,
            )
            .await?;

        debug!("💾 [SemanticCache] Stored hash: {}", hash);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_semantic_cache_roundtrip() {
        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap(); // allow-anti-pattern
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        let jq = Arc::new(crate::job_queue::UniversalJobQueue::new(pool.clone(), None, ts).await.unwrap()); // allow-anti-pattern
        crate::job_queue::migrations::DbInitializer::init_db(&*jq).await.unwrap();

        let repo = Arc::new(SqlSemanticCacheRepository::new(pool.clone()));
        let cache = SemanticCache::new(repo, None);

        let prompt = "hello";
        let system = Some("you are a bot");
        let response = LlmResponse {
            content: "hi there".to_string(),
            stop_reason: StopReason::EndTurn,
            reasoning: None,
            metadata: None,
        };

        // Initially empty
        let cached = cache.get(prompt, system).await.unwrap(); // allow-anti-pattern
        assert!(cached.is_none());

        // Cache it
        cache
            .set(
                prompt,
                system,
                &response,
                "mock-provider",
                "mock-model",
                3600,
            )
            .await
            .unwrap(); // allow-anti-pattern

        // Retrieve it
        let cached = cache.get(prompt, system).await.unwrap(); // allow-anti-pattern
        assert!(cached.is_some());
        let cached = cached.unwrap(); // allow-anti-pattern
        assert_eq!(cached.content, "hi there");
    }
}
