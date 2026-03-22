/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::job_queue::SqliteJobQueue;
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::{LlmResponse, StopReason};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::sync::Arc;
use tracing::{debug, info};

/// LLMセマンティックキャッシュ
/// 同一または類似のプロンプトに対する応答をSQLiteにキャッシュし、コストを削減する。
pub struct SemanticCache {
    jq: Arc<SqliteJobQueue>,
}

impl SemanticCache {
    /// SemanticCache の新規インスタンスを生成する
    pub fn new(jq: Arc<SqliteJobQueue>) -> Self {
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

        let row = sqlx::query(
            "SELECT response, provider_name, model_name FROM llm_response_cache 
             WHERE prompt_hash = ? AND created_at > datetime('now', '-' || ttl_seconds || ' seconds')"
        )
        .bind(&hash)
        .fetch_optional(&self.jq.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Cache lookup failed: {}", e),
        })?;

        if let Some(row) = row {
            debug!("🎯 [SemanticCache] Hit! Hash: {}", hash);
            return Ok(Some(LlmResponse {
                content: row.get(0),
                stop_reason: StopReason::EndTurn, // キャッシュからはEndTurn固定
            }));
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

        sqlx::query(
            "INSERT OR REPLACE INTO llm_response_cache 
             (prompt_hash, response, provider_name, model_name, ttl_seconds, created_at)
             VALUES (?, ?, ?, ?, ?, datetime('now'))",
        )
        .bind(&hash)
        .bind(&response.content)
        .bind(provider_name)
        .bind(model_name)
        .bind(ttl_seconds)
        .execute(&self.jq.pool)
        .await
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
        let jq = Arc::new(SqliteJobQueue::new(":memory:").await.unwrap());
        let cache = SemanticCache::new(jq);

        let prompt = "hello";
        let system = Some("you are a bot");
        let response = LlmResponse {
            content: "hi there".to_string(),
            stop_reason: StopReason::EndTurn,
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
