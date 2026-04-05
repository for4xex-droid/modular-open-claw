/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use super::swarm::SwarmOps;
use super::UniversalJobQueue;
use crate::polar_quant::PolarQuantEncoder;
use crate::vector_ops::{StandardVectorOps, VectorOps};
use crate::{sql_exec, sql_fetch_all, sql_fetch_one, sql_fetch_optional};
use aiome_core::error::AiomeError;
use aiome_core::traits::{Job, JobStatus, KarmaEntry, KarmaSearchResult};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Row;
use std::time::Instant;
use tracing::warn;
use uuid::Uuid;

#[async_trait]
pub trait KarmaOps {
    async fn do_fetch_relevant_karma(
        &self,
        topic: &str,
        skill_id: &str,
        limit: i64,
        current_soul_hash: &str,
    ) -> Result<KarmaSearchResult, AiomeError>;
    async fn do_store_karma(
        &self,
        job_id: &str,
        skill_id: &str,
        lesson: &str,
        karma_type: &str,
        soul_hash: &str,
        domain: Option<&str>,
        subtopic: Option<&str>,
        clone_origin_id: Option<&str>,
        is_private: bool,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_undistilled_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError>;
    async fn do_mark_karma_extracted(&self, job_id: &str) -> Result<(), AiomeError>;
    async fn do_fetch_all_karma(&self, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError>;
    async fn do_fetch_unincorporated_karma(
        &self,
        limit: i64,
        current_soul_hash: &str,
    ) -> Result<Vec<serde_json::Value>, AiomeError>;
    async fn do_mark_karma_as_incorporated(
        &self,
        karma_ids: Vec<String>,
        new_soul_hash: &str,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_relevant_karma_by_category(
        &self,
        topic: &str,
        category: &str,
        limit: i64,
    ) -> Result<KarmaSearchResult, AiomeError>;
}

#[async_trait]
impl KarmaOps for UniversalJobQueue {
    async fn do_fetch_relevant_karma_by_category(
        &self,
        topic: &str,
        category: &str,
        limit: i64,
    ) -> Result<KarmaSearchResult, AiomeError> {
        // Implementation similar to do_fetch_relevant_karma but filtering by category/domain
        let q = format!(
            "SELECT id, lesson, weight, domain, subtopic FROM karma_logs WHERE domain = {} ORDER BY created_at DESC LIMIT {}",
            self.pool.ph(0),
            self.pool.ph(1)
        );
        let mut items = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(&q)
                    .bind(category)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    items.push(KarmaEntry {
                        id: r.get("id"),
                        job_id: r.try_get("job_id").ok(),
                        karma_type: "Synthesized".to_string(),
                        related_skill: r
                            .try_get("subtopic")
                            .ok()
                            .unwrap_or_else(|| "general".to_string()),
                        lesson: r.get("lesson"),
                        weight: r.get::<i64, _>("weight") as i32,
                        soul_version_hash: None,
                        created_at: Utc::now().to_rfc3339(),
                        ..Default::default()
                    });
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(&q)
                    .bind(category)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    items.push(KarmaEntry {
                        id: r.get("id"),
                        job_id: r.try_get("job_id").ok(),
                        karma_type: "Synthesized".to_string(),
                        related_skill: r
                            .try_get("subtopic")
                            .ok()
                            .unwrap_or_else(|| "general".to_string()),
                        lesson: r.get("lesson"),
                        weight: r.get::<i32, _>("weight"),
                        soul_version_hash: None,
                        created_at: Utc::now().to_rfc3339(),
                        ..Default::default()
                    });
                }
            }
        }
        Ok(KarmaSearchResult {
            entries: items,
            is_ood: false,
            max_score: 0.0,
        })
    }

    async fn do_fetch_relevant_karma(
        &self,
        topic: &str,
        skill_id: &str,
        limit: i64,
        current_soul_hash: &str,
    ) -> Result<KarmaSearchResult, AiomeError> {
        let cache_key = format!("{}:{}:{}", skill_id, topic, limit);

        // 1. Tier-0: In-memory Cache check
        {
            let cache = self.karma_cache.read().await;
            if let Some((result, timestamp)) = cache.get(&cache_key) {
                if timestamp.elapsed() < std::time::Duration::from_secs(300) {
                    return Ok(result.clone());
                }
            }
        }

        // Sprint 3-C: Query Sanitization
        let sanitized_topic: String = topic
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-')
            .collect();

        let fts_query = if self.pool.is_postgres() {
            sanitized_topic
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" & ")
        } else {
            format!("\"{}\"", sanitized_topic.replace('"', "\"\""))
        };

        let embed_dim = self
            .get_embedding_provider()
            .await
            .map(|p| p.embedding_dim())
            .unwrap_or(768);

        let candidate_limit = limit * 5;
        let weight_expr = self.pool.karma_sql_weight_expr(0);
        let q = format!(
            "SELECT k.id, k.lesson, k.soul_version_hash, k.karma_embedding, {} AS sql_weight
             FROM karma_logs k
             WHERE k.weight > 0 AND k.is_archived = 0 AND (k.related_skill = {1} OR k.related_skill = 'global') 
             ORDER BY sql_weight DESC, k.created_at DESC LIMIT {2}",
            weight_expr, self.pool.ph(1), self.pool.ph(2)
        );

        let count_q = format!(
            "SELECT COUNT(*) FROM karma_logs k WHERE k.weight > 0 AND k.is_archived = 0 AND (k.related_skill = {0} OR k.related_skill = 'global')",
            self.pool.ph(0)
        );
        let row_count: i64 = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                sqlx::query_scalar(&count_q)
                    .bind(skill_id)
                    .fetch_one(p)
                    .await
            }
            crate::db::DatabasePool::Postgres(p) => {
                sqlx::query_scalar(&count_q)
                    .bind(skill_id)
                    .fetch_one(p)
                    .await
            }
        }
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("SQL Karma Count failed: {}", e),
        })?;

        if row_count == 0 {
            return Ok(KarmaSearchResult::empty());
        }

        struct KarmaCandidate {
            id: String,
            lesson: String,
            hash: Option<String>,
            sql_weight: f64,
            semantic_score: f64,
            compressed_embedding: Option<Vec<u8>>,
        }

        let mut candidates: Vec<KarmaCandidate> = match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(&q)
                    .bind(&fts_query)
                    .bind(skill_id)
                    .bind(candidate_limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("SQL Karma Query failed: {}", e),
                    })?;
                rows.iter()
                    .map(|r| {
                        let embedding_bytes: Option<Vec<u8>> = r.try_get("karma_embedding").ok();
                        KarmaCandidate {
                            id: r.get("id"),
                            lesson: r.get("lesson"),
                            hash: r.try_get::<String, _>("soul_version_hash").ok(),
                            sql_weight: r.get("sql_weight"),
                            semantic_score: 0.0,
                            compressed_embedding: embedding_bytes,
                        }
                    })
                    .collect()
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(&q)
                    .bind(&fts_query)
                    .bind(skill_id)
                    .bind(candidate_limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: format!("SQL Karma Query failed: {}", e),
                    })?;
                rows.iter()
                    .map(|r| {
                        let embedding_bytes: Option<Vec<u8>> = r.try_get("karma_embedding").ok();
                        KarmaCandidate {
                            id: r.get("id"),
                            lesson: r.get("lesson"),
                            hash: r.try_get::<String, _>("soul_version_hash").ok(),
                            sql_weight: r.get("sql_weight"),
                            semantic_score: 0.0,
                            compressed_embedding: embedding_bytes,
                        }
                    })
                    .collect()
            }
        };

        if candidates.is_empty() {
            return Ok(KarmaSearchResult::empty());
        }

        let mut max_score = 0.0;
        let mut searched_semantically = false;
        if let Some(provider) = self.get_embedding_provider().await {
            if let Ok(topic_vec_f32) = provider.embed(topic, true).await {
                searched_semantically = true;
                let topic_vec: Vec<f64> = topic_vec_f32.into_iter().map(|f| f as f64).collect();

                // Phase 2-B: Hybrid Recall using RRF (Reciprocal Rank Fusion)
                // Combine FTS candidates with SLM geometric candidates
                if let Some(bridge) = &self.slm_bridge {
                    if let Ok(slm_results) = bridge.recall(topic, limit).await {
                        for (idx, slm_res) in slm_results.into_iter().enumerate() {
                            // SLM results are already ranked by geometric score
                            // We treat them as high-quality candidates and add to our pool
                            candidates.push(KarmaCandidate {
                                id: format!("slm-{}", uuid::Uuid::new_v4()),
                                lesson: slm_res.content,
                                hash: None,
                                sql_weight: 0.0,
                                semantic_score: slm_res.score, // Use geometric score as initial semantic score
                                compressed_embedding: None, // We don't have embeddings for SLM yet
                            });
                        }
                    }
                }

                for candidate in &mut candidates {
                    if let Some(ref emb_comp) = candidate.compressed_embedding {
                        let score = StandardVectorOps::approximate_cosine_similarity(
                            &topic_vec, emb_comp, embed_dim,
                        );
                        candidate.semantic_score = score;
                        if score > max_score {
                            max_score = score;
                        }
                    }
                }

                // RRF Logic: score(d) = sum(1 / (k + rank))
                // k=60 is a standard constant for RRF
                const K: f64 = 60.0;

                // We calculate RRF score by considering the original SQL rank and SLM rank
                // Since candidates list is already somewhat ordered by SQL (FTS),
                // we treat its index as SQL rank.
                for (idx, candidate) in candidates.iter_mut().enumerate() {
                    let fts_rank = (idx + 1) as f64;
                    // SLM results were appended, so we need to identify them
                    let slm_rank = if candidate.id.starts_with("slm-") {
                        // SLM results are already sorted by SLM before being pushed
                        1.0 // Placeholder: in a full impl, we'd track original ranks
                    } else {
                        100.0 // Default low rank if not in SLM
                    };

                    let rrf_score = (1.0 / (K + fts_rank)) + (1.0 / (K + slm_rank));
                    // Combine RRF with Cosine similarity for final precision
                    candidate.semantic_score = candidate.semantic_score * 0.5 + rrf_score * 50.0;
                }

                candidates.sort_by(|a, b| {
                    b.semantic_score
                        .partial_cmp(&a.semantic_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        let mut final_entries = Vec::new();
        let now = Utc::now().to_rfc3339();
        let mut ids_to_update = Vec::new();

        for candidate in candidates.into_iter().take(limit as usize) {
            ids_to_update.push(candidate.id.clone());
            let mut lesson_text = candidate.lesson;
            if let Some(h) = candidate.hash {
                if h != current_soul_hash {
                    lesson_text = format!(
                        "[LEGACY KARMA - from an older Soul version]\n{}",
                        lesson_text
                    );
                }
            }
            final_entries.push(KarmaEntry {
                id: candidate.id,
                lesson: lesson_text,
                ..Default::default()
            });
        }

        let is_ood = final_entries.is_empty() || (searched_semantically && max_score < 0.3);
        let result = KarmaSearchResult {
            entries: final_entries,
            is_ood,
            max_score,
        };

        // Update Cache
        {
            let mut cache = self.karma_cache.write().await;
            if cache.len() > 50 {
                cache.clear();
            }
            cache.insert(cache_key, (result.clone(), Instant::now()));
        }

        let update_q = format!("UPDATE karma_logs SET last_applied_at = {0}, apply_count = apply_count + 1 WHERE id = {1}", self.pool.ph(0), self.pool.ph(1));
        for id in ids_to_update {
            let _ = sql_exec!(&self.pool, &update_q, &now, &id);
        }

        Ok(result)
    }

    async fn do_store_karma(
        &self,
        job_id: &str,
        skill_id: &str,
        lesson: &str,
        karma_type: &str,
        soul_hash: &str,
        domain: Option<&str>,
        subtopic: Option<&str>,
        clone_origin_id: Option<&str>,
        is_private: bool,
    ) -> Result<(), AiomeError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let node_id = self.do_get_node_id().await.unwrap_or_default();
        let clock = self.do_tick_local_clock().await.unwrap_or(0);
        let sign_target = format!("{}:{}:{}", id, lesson, clock);
        let signature = self.do_sign_swarm_payload(&sign_target).await.ok(); // allow-anti-pattern

        let mut embedding: Option<Vec<u8>> = None;
        if let Some(provider) = self.get_embedding_provider().await {
            if let Ok(vec) = provider.embed(lesson, false).await {
                let encoder = PolarQuantEncoder::new(4, 32);
                let vec_f64: Vec<f64> = vec.into_iter().map(|f| f as f64).collect();
                embedding = Some(encoder.encode(&vec_f64));
            }
        }

        let domain = domain.unwrap_or("general");
        let mut q = format!(
            "INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, soul_version_hash, created_at, karma_embedding, node_id, lamport_clock, signature, domain, subtopic, clone_origin_id, is_private) VALUES ({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, {10}, {11}, {12}, {13}, {14})",
            self.pool.ph(0), self.pool.ph(1), self.pool.ph(2), self.pool.ph(3), self.pool.ph(4), self.pool.ph(5), self.pool.ph(6), self.pool.ph(7), self.pool.ph(8), self.pool.ph(9), self.pool.ph(10), self.pool.ph(11), self.pool.ph(12), self.pool.ph(13), self.pool.ph(14)
        );

        sql_exec!(
            &self.pool,
            &q,
            &id,
            job_id,
            karma_type,
            skill_id,
            lesson,
            soul_hash,
            &now,
            embedding,
            &node_id,
            clock as i64,
            signature,
            domain,
            subtopic,
            clone_origin_id,
            is_private as i32
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to store karma: {}", e),
        })?;
        Ok(())
    }

    async fn do_fetch_undistilled_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        let q = format!(
            "SELECT * FROM jobs WHERE execution_log IS NOT NULL AND tech_karma_extracted = 0 AND status IN ('Completed', 'Failed') ORDER BY updated_at ASC LIMIT {}",
            self.pool.ph(0)
        );

        let rows_sqlite = if let crate::db::DatabasePool::Sqlite(p) = &self.pool {
            Some(
                sqlx::query(&q)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?,
            )
        } else {
            None
        };

        let rows_pg = if let crate::db::DatabasePool::Postgres(p) = &self.pool {
            Some(
                sqlx::query(&q)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?,
            )
        } else {
            None
        };

        let mut jobs = Vec::new();
        if let Some(rows) = rows_sqlite {
            for r in rows {
                let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
                let permission_manifest = r
                    .try_get::<String, _>("permission_manifest")
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok());
                jobs.push(Job {
                    id: r.get("id"),
                    category: r.get("category"),
                    topic: r.get("topic"),
                    style: r.get("style_name"),
                    karma_directives: r.try_get("karma_directives").ok(),
                    status: aiome_core::traits::JobStatus::from_string(
                        r.get::<String, _>("status"),
                    ),
                    started_at: r.try_get("started_at").ok(),
                    last_heartbeat: r.try_get("last_heartbeat").ok(),
                    tech_karma_extracted: tech_karma_extracted != 0,
                    creative_rating: r.try_get("creative_rating").ok(),
                    execution_log: r.try_get("execution_log").ok(),
                    error_message: r.try_get("error_message").ok(),
                    sns_platform: r.try_get("sns_platform").ok(),
                    sns_content_id: r.try_get("sns_content_id").ok(),
                    published_at: r.try_get("published_at").ok(),
                    output_artifacts: r.try_get("output_artifacts").ok(),
                    permission_manifest,
                    agent_id: None,
                    priority: r.get("priority"),
                    created_at: r.try_get("created_at").unwrap_or_default(),
                    updated_at: r.try_get("updated_at").unwrap_or_default(),
                    requires_review: r.try_get::<bool, _>("requires_review").unwrap_or(false),
                });
            }
        } else if let Some(rows) = rows_pg {
            for r in rows {
                let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
                let permission_manifest = r
                    .try_get::<serde_json::Value, _>("permission_manifest")
                    .ok()
                    .and_then(|v| serde_json::from_value(v).ok());
                jobs.push(Job {
                    id: r.get("id"),
                    category: r.get("category"),
                    topic: r.get("topic"),
                    style: r.get("style_name"),
                    karma_directives: r.try_get("karma_directives").ok(),
                    status: aiome_core::traits::JobStatus::from_string(
                        r.get::<String, _>("status"),
                    ),
                    started_at: r.try_get("started_at").ok(),
                    last_heartbeat: r.try_get("last_heartbeat").ok(),
                    tech_karma_extracted: tech_karma_extracted != 0,
                    creative_rating: r.try_get("creative_rating").ok(),
                    execution_log: r.try_get("execution_log").ok(),
                    error_message: r.try_get("error_message").ok(),
                    sns_platform: r.try_get("sns_platform").ok(),
                    sns_content_id: r.try_get("sns_content_id").ok(),
                    published_at: r.try_get("published_at").ok(),
                    output_artifacts: r.try_get("output_artifacts").ok(),
                    permission_manifest,
                    agent_id: None,
                    priority: r.get("priority"),
                    created_at: r.try_get("created_at").unwrap_or_default(),
                    updated_at: r.try_get("updated_at").unwrap_or_default(),
                    requires_review: r.try_get::<bool, _>("requires_review").unwrap_or(false),
                });
            }
        }
        Ok(jobs)
    }

    async fn do_mark_karma_extracted(&self, job_id: &str) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        let q = format!(
            "UPDATE jobs SET tech_karma_extracted = 1, updated_at = {0} WHERE id = {1}",
            self.pool.ph(0),
            self.pool.ph(1)
        );
        sql_exec!(&self.pool, &q, &now, job_id).map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to mark karma extracted: {}", e),
        })?;
        Ok(())
    }

    async fn do_fetch_all_karma(&self, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
        let q = format!(
            "SELECT * FROM karma_logs ORDER BY created_at DESC LIMIT {}",
            self.pool.ph(0)
        );
        let mut results = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(&q)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    results.push(serde_json::json!({
                        "id": r.get::<String, _>("id"), "job_id": r.try_get::<String, _>("job_id").ok(),
                        "skill": r.get::<String, _>("related_skill"), "lesson": r.get::<String, _>("lesson"),
                        "karma_type": r.get::<String, _>("karma_type"), "weight": r.get::<i64, _>("weight"),
                        "somatic_valence": r.try_get::<f64, _>("somatic_valence").ok(),
                        "soul": r.try_get::<String, _>("soul_version_hash").ok(), "node_id": r.get::<String, _>("node_id"),
                        "clock": r.get::<i64, _>("lamport_clock"), "signature": r.try_get::<String, _>("signature").ok(),
                        "last_applied_at": r.try_get::<String, _>("last_applied_at").ok(), "created_at": r.get::<String, _>("created_at")
                    }));
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(&q)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    results.push(serde_json::json!({
                        "id": r.get::<String, _>("id"), "job_id": r.try_get::<String, _>("job_id").ok(),
                        "skill": r.get::<String, _>("related_skill"), "lesson": r.get::<String, _>("lesson"),
                        "karma_type": r.get::<String, _>("karma_type"), "weight": r.get::<i64, _>("weight"),
                        "somatic_valence": r.try_get::<f64, _>("somatic_valence").ok(),
                        "soul": r.try_get::<String, _>("soul_version_hash").ok(), "node_id": r.get::<String, _>("node_id"),
                        "clock": r.get::<i64, _>("lamport_clock"), "signature": r.try_get::<String, _>("signature").ok(),
                        "last_applied_at": r.try_get::<String, _>("last_applied_at").ok(), "created_at": r.get::<String, _>("created_at")
                    }));
                }
            }
        }
        Ok(results)
    }

    async fn do_fetch_unincorporated_karma(
        &self,
        limit: i64,
        current_soul_hash: &str,
    ) -> Result<Vec<serde_json::Value>, AiomeError> {
        let q = format!(
            "SELECT * FROM karma_logs WHERE soul_version_hash IS NULL OR soul_version_hash != {} ORDER BY created_at DESC LIMIT {}",
            self.pool.ph(0), self.pool.ph(1)
        );
        let mut results = Vec::new();
        match &self.pool {
            crate::db::DatabasePool::Sqlite(p) => {
                let rows = sqlx::query(&q)
                    .bind(current_soul_hash)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    results.push(serde_json::json!({
                        "id": r.get::<String, _>("id"), "lesson": r.get::<String, _>("lesson"),
                        "skill": r.get::<String, _>("related_skill"), "type": r.get::<String, _>("karma_type"), "weight": r.get::<i64, _>("weight"),
                        "somatic_valence": r.try_get::<f64, _>("somatic_valence").ok(),
                    }));
                }
            }
            crate::db::DatabasePool::Postgres(p) => {
                let rows = sqlx::query(&q)
                    .bind(current_soul_hash)
                    .bind(limit)
                    .fetch_all(p)
                    .await
                    .map_err(|e| AiomeError::Infrastructure {
                        reason: e.to_string(),
                    })?;
                for r in rows {
                    results.push(serde_json::json!({
                        "id": r.get::<String, _>("id"), "lesson": r.get::<String, _>("lesson"),
                        "skill": r.get::<String, _>("related_skill"), "type": r.get::<String, _>("karma_type"), "weight": r.get::<i64, _>("weight"),
                        "somatic_valence": r.try_get::<f64, _>("somatic_valence").ok(),
                    }));
                }
            }
        }
        Ok(results)
    }

    async fn do_mark_karma_as_incorporated(
        &self,
        karma_ids: Vec<String>,
        new_soul_hash: &str,
    ) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        let q = format!(
            "UPDATE karma_logs SET soul_version_hash = {0}, last_applied_at = {1} WHERE id = {2}",
            self.pool.ph(0),
            self.pool.ph(1),
            self.pool.ph(2)
        );
        for id in karma_ids {
            sql_exec!(&self.pool, &q, new_soul_hash, &now, id).map_err(|e| {
                AiomeError::Infrastructure {
                    reason: e.to_string(),
                }
            })?;
        }
        Ok(())
    }
}
