/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::job_queue::UniversalJobQueue;
use crate::polar_quant::PolarQuantEncoder;
use crate::vector_ops::{StandardVectorOps, VectorOps};
use aiome_contracts::error::AiomeError;
use aiome_contracts::llm::{LlmResponse, StopReason};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::sync::Arc;
use tracing::{debug, info};

/// LLMセマンティックキャッシュ
/// 同一または類似のプロンプトに対する応答をSQLiteにキャッシュし、コストを削減する。
pub struct SemanticCache {
    jq: Arc<UniversalJobQueue>,
}

impl SemanticCache {
    /// SemanticCache の新規インスタンスを生成する
    pub fn new(jq: Arc<UniversalJobQueue>) -> Self {
        Self { jq }
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
        let pool = self.jq.get_pool();
        let exact_q = format!(
            "SELECT response FROM llm_response_cache WHERE prompt_hash = {} AND created_at > {}",
            pool.ph(0),
            if pool.is_sqlite() {
                "datetime('now', '-' || ttl_seconds || ' seconds')"
            } else {
                "NOW() - (ttl_seconds || ' seconds')::interval"
            }
        );

        let hash_match: Option<String> = match &pool {
            crate::db::DatabasePool::Sqlite(p) => sqlx::query_scalar(&exact_q)
                .bind(&hash)
                .fetch_optional(p)
                .await
                .ok()
                .flatten(),
            crate::db::DatabasePool::Postgres(p) => sqlx::query_scalar(&exact_q)
                .bind(&hash)
                .fetch_optional(p)
                .await
                .ok()
                .flatten(),
        };

        if let Some(content) = hash_match {
            debug!("🎯 [SemanticCache] Exact Hit! Hash: {}", hash);
            return Ok(Some(LlmResponse {
                content,
                stop_reason: StopReason::EndTurn,
                reasoning: None,
                metadata: None,
            }));
        }

        // 2. セマンティック一致 (Vector)
        if let Some(provider) = self.jq.get_embedding_provider().await {
            let embed_dim = provider.embedding_dim();
            if let Ok(query_vec_f32) = provider.embed(prompt, true).await {
                let query_vec: Vec<f64> = query_vec_f32.iter().map(|&f| f as f64).collect();
                let semantic_q = "SELECT response, prompt_embedding FROM llm_response_cache WHERE prompt_embedding IS NOT NULL ORDER BY created_at DESC LIMIT 100";

                let hit = match &pool {
                    crate::db::DatabasePool::Sqlite(p) => {
                        if let Ok(rows) = sqlx::query(semantic_q).fetch_all(p).await {
                            let mut best: Option<String> = None;
                            for row in rows {
                                let emb_bytes: Vec<u8> = row.get("prompt_embedding");
                                let score = StandardVectorOps::approximate_cosine_similarity(
                                    &query_vec, &emb_bytes, embed_dim,
                                );
                                if score > 0.95 {
                                    best = Some(row.get("response"));
                                    info!(
                                        "🧠 [SemanticCache] Semantic Hit (Sqlite)! Score: {:.4}",
                                        score
                                    );
                                    break;
                                }
                            }
                            best
                        } else {
                            None
                        }
                    }
                    crate::db::DatabasePool::Postgres(p) => {
                        if let Ok(rows) = sqlx::query(semantic_q).fetch_all(p).await {
                            let mut best: Option<String> = None;
                            for row in rows {
                                let emb_bytes: Vec<u8> = row.get("prompt_embedding");
                                let score = StandardVectorOps::approximate_cosine_similarity(
                                    &query_vec, &emb_bytes, embed_dim,
                                );
                                if score > 0.95 {
                                    best = Some(row.get("response"));
                                    info!(
                                        "🧠 [SemanticCache] Semantic Hit (Postgres)! Score: {:.4}",
                                        score
                                    );
                                    break;
                                }
                            }
                            best
                        } else {
                            None
                        }
                    }
                };

                if let Some(content) = hit {
                    return Ok(Some(LlmResponse {
                        content,
                        stop_reason: StopReason::EndTurn,
                        reasoning: None,
                        metadata: None,
                    }));
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

        if let Some(provider) = self.jq.get_embedding_provider().await {
            if let Ok(vec) = provider.embed(prompt, false).await {
                let encoder = PolarQuantEncoder::new(4, 32);
                let vec_f64: Vec<f64> = vec.into_iter().map(|f| f as f64).collect();
                embedding = Some(encoder.encode(&vec_f64));
            }
        }

        let pool = self.jq.get_pool();
        match &pool {
            crate::db::DatabasePool::Sqlite(p) => {
                sqlx::query("INSERT OR REPLACE INTO llm_response_cache (prompt_hash, response, provider_name, model_name, ttl_seconds, prompt_embedding, created_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))")
                    .bind(&hash).bind(&response.content).bind(provider_name).bind(model_name).bind(ttl_seconds).bind(embedding).execute(p).await.map(|_| ())
            }
            crate::db::DatabasePool::Postgres(p) => {
                sqlx::query("INSERT INTO llm_response_cache (prompt_hash, response, provider_name, model_name, ttl_seconds, prompt_embedding, created_at) VALUES ($1, $2, $3, $4, $5, $6, NOW()) ON CONFLICT (prompt_hash) DO UPDATE SET response = EXCLUDED.response, ttl_seconds = EXCLUDED.ttl_seconds, prompt_embedding = EXCLUDED.prompt_embedding, created_at = EXCLUDED.created_at")
                    .bind(&hash).bind(&response.content).bind(provider_name).bind(model_name).bind(ttl_seconds).bind(embedding).execute(p).await.map(|_| ())
            }
        }
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Cache write failed: {}", e),
        })?;

        debug!("💾 [SemanticCache] Stored hash: {}", hash);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_semantic_cache_roundtrip() {
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(
                crate::db::DatabasePool::Sqlite(
                    sqlx::sqlite::SqlitePoolOptions::new()
                        .connect("sqlite::memory:")
                        .await
                        .unwrap(),
                ),
            ),
        );
        let jq = Arc::new(UniversalJobQueue::new(":memory:", None, ts).await.unwrap());
        let cache = SemanticCache::new(jq);

        let prompt = "hello";
        let system = Some("you are a bot");
        let response = LlmResponse {
            content: "hi there".to_string(),
            stop_reason: StopReason::EndTurn,
            reasoning: None,
            metadata: None,
        };

        // Initially empty
        let cached = cache.get(prompt, system).await.unwrap();
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
            .unwrap();

        // Retrieve it
        let cached = cache.get(prompt, system).await.unwrap();
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.content, "hi there");
    }
}
