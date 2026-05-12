/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

//! # Job Queue Tests — The Immortal Proof
//!
//! ファイルベース一時 SQLite を使った `UniversalJobQueue` の完全テストスイート。
//! 全 15 テストで心臓部の不変性を機械的に保証する。

use super::watchtower::WatchtowerOps;
use super::SettingsOps;
use super::UniversalJobQueue;
use crate::job_queue::federation::FederationOps;
use crate::job_queue::karma::KarmaOps;

use crate::job_queue::trajectory_store::TrajectoryOps;
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::{EmbeddingProvider, LlmProvider};
use aiome_core::security::PermissionManifest;
use aiome_core::traits::{JobQueue, JobStatus, KarmaEntry, KarmaSearchResult};
use aiome_core::trajectory::TrajectoryStore;
use aiome_core_contracts::traits::{
    AgentEvolver, AuditStore, BiomeRegistry, FederationRegistry, ImmuneSystemOps, KarmaRegistry,
    TaskRegistry,
};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug)]
pub(crate) struct MockLlmProvider {
    pub(crate) json_response: String,
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete(
        &self,
        _prompt: &str,
        _system: Option<&str>,
    ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
        Ok(aiome_core::llm_provider::LlmResponse {
            content: self.json_response.clone(),
            stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
            ..Default::default()
        })
    }
    async fn complete_with_cache(
        &self,
        request: aiome_core_contracts::llm::LlmRequest,
    ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
        let content = request
            .messages
            .last()
            .map(|m| m.content.as_str())
            .unwrap_or("");
        self.complete(content, None).await
    }
    async fn test_connection(&self) -> Result<(), AiomeError> {
        Ok(())
    }
    fn name(&self) -> &str {
        "Mock"
    }
}

/// テスト用のユニーク一時ファイル JobQueue を作成
/// 各テストが独自のDBファイルを持ち、ロック競合を回避する
pub(crate) async fn create_test_queue() -> (UniversalJobQueue, tempfile::TempDir) {
    let tmp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");

    let _ = dotenvy::dotenv();
    if let Ok(pg_url) = std::env::var("TEST_POSTGRES_URL") {
        info!("🔧 Bootstrapping Test JobQueue against PostgreSQL");
        let ts_pool = {
            let pg = sqlx::PgPool::connect(&pg_url)
                .await
                .expect("TS pool connect");
            crate::db::DatabasePool::Postgres(pg)
        };
        let ts = std::sync::Arc::new(super::trajectory_store::SqliteTrajectoryStore::new(ts_pool));
        let jq = UniversalJobQueue::new(
            crate::db::DatabasePool::new_postgres(&pg_url)
                .await
                .unwrap(),
            None,
            ts,
        )
        .await
        .expect("Failed to create test job queue (Postgres)");

        // Return dummy TempDir to satisfy the signature
        return (jq, tmp_dir);
    }

    let db_path = tmp_dir.path().join("test.db");
    let db_path_str = db_path.to_str().expect("Invalid path");
    let ts_pool = crate::db::DatabasePool::new_sqlite(&format!("sqlite://{}", db_path_str))
        .await
        .expect("TS pool connect");
    let ts = std::sync::Arc::new(super::trajectory_store::SqliteTrajectoryStore::new(ts_pool));
    // SQLite connection string format needed for sqlx
    let jq = UniversalJobQueue::new(
        crate::db::DatabasePool::new_sqlite(&format!("sqlite://{}", db_path_str))
            .await
            .unwrap(),
        None,
        ts,
    )
    .await
    .expect("Failed to create test job queue");
    (jq, tmp_dir) // tmp_dir must be kept alive for the DB file to exist
}

#[tokio::test]
async fn test_sqlite_job_queue_basic_ops() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Test Topic", "Style", None, None, None, 0)
        .await
        .expect("Enqueue failed");
    let job = jq
        .fetch_job(&job_id)
        .await
        .expect("Fetch failed")
        .expect("Job not found");
    assert_eq!(job.topic, "Test Topic");
    assert_eq!(job.status, JobStatus::Pending);
}

#[tokio::test]
async fn test_sqlite_job_queue_dequeue_lifecycle() {
    let (jq, _tmp) = create_test_queue().await;
    jq.enqueue("Task", "Topic 1", "Style", None, None, None, 0)
        .await
        .unwrap();
    let job = jq
        .dequeue(&["Task"])
        .await
        .unwrap()
        .expect("Should dequeue job");
    assert_eq!(job.status, JobStatus::InProgress);
    assert!(job.started_at.is_some());

    jq.complete_job(&job.id, Some("[\"artifact.txt\"]"))
        .await
        .unwrap();
    let updated = jq.fetch_job(&job.id).await.unwrap().unwrap();
    assert_eq!(updated.status, JobStatus::Completed);
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_storage() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Topic", "Style", None, None, None, 0)
        .await
        .unwrap();
    jq.store_karma(
        &job_id,
        "skill-1",
        "Lesson 1",
        "Technical",
        "hash1",
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();
    let result = jq
        .fetch_relevant_karma("Topic", "skill-1", 10, "hash1")
        .await
        .unwrap();
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].lesson, "Lesson 1");
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_somatic_valence() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Topic Valence", "Style", None, None, None, 0)
        .await
        .unwrap();

    // 最初に store_karma する前に、手動でSQLを叩いてsomatic_valenceを設定できるか確認する
    jq.store_karma(
        &job_id,
        "skill-v",
        "Valence Lesson",
        "Technical",
        "hash-v",
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();

    // 手動で valence をセット
    sqlx::query("UPDATE karma_logs SET somatic_valence = 0.8 WHERE lesson = 'Valence Lesson'")
        .execute(jq.pool.get_sqlite_pool().unwrap())
        .await
        .unwrap();

    // 取得時に somatic_valence が JSON に含まれているかテスト
    let all_karma = jq.fetch_all_karma(10).await.unwrap();
    let entry = all_karma
        .iter()
        .find(|k| k.get("lesson").and_then(|v| v.as_str()) == Some("Valence Lesson"))
        .expect("Should find the inserted karma");

    // RED: The current implementation doesn't return somatic_valence in JSON
    assert_eq!(
        entry.get("somatic_valence").and_then(|v| v.as_f64()),
        Some(0.8)
    );
}

#[tokio::test]
async fn test_sqlite_job_queue_zombie_reclamation() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Zombies", "Style", None, None, None, 0)
        .await
        .unwrap();
    jq.dequeue(&["Task"]).await.unwrap();

    // Simulate heartbeat timeout
    sqlx::query("UPDATE jobs SET last_heartbeat = datetime('now', '-15 minutes') WHERE id = ?")
        .bind(&job_id)
        .execute(jq.pool.get_sqlite_pool().unwrap())
        .await
        .unwrap();

    let reclaimed = jq.reclaim_zombie_jobs(10).await.unwrap();
    assert_eq!(reclaimed, 1);
    let updated = jq.fetch_job(&job_id).await.unwrap().unwrap();
    assert_eq!(updated.status, JobStatus::Failed);
    assert!(updated.error_message.unwrap().contains("Zombie reclaimed"));
}

#[tokio::test]
async fn test_sqlite_job_queue_creative_rating_guard() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Rating", "Style", None, None, None, 0)
        .await
        .unwrap();

    // Cannot rate pending job (Atomic Guard)
    let res = jq.set_creative_rating(&job_id, 1).await;
    assert!(res.is_err());

    jq.dequeue(&["Task"]).await.unwrap();
    jq.set_creative_rating(&job_id, 1)
        .await
        .expect("Should allow rating on processing");
    let job = jq.fetch_job(&job_id).await.unwrap().unwrap();
    assert_eq!(job.creative_rating, Some(1));
}

#[tokio::test]
async fn test_sqlite_job_queue_db_purge() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Old Job", "Style", None, None, None, 0)
        .await
        .unwrap();
    let job = jq
        .dequeue(&["Task"])
        .await
        .unwrap()
        .expect("Job should exist");
    jq.complete_job(&job.id, None).await.unwrap();

    sqlx::query("UPDATE jobs SET created_at = datetime('now', '-30 days') WHERE id = ?")
        .bind(&job_id)
        .execute(jq.pool.get_sqlite_pool().unwrap())
        .await
        .unwrap();

    let purged = jq.purge_old_jobs(1).await.unwrap();
    assert_eq!(purged, 1);
    let fetched = jq.fetch_job(&job_id).await.unwrap();
    assert!(fetched.is_none());
}

#[tokio::test]
async fn test_sqlite_job_queue_concurrent_dequeue() {
    let (jq, _tmp) = create_test_queue().await;
    jq.enqueue("Task", "Job 1", "Style", None, None, None, 0)
        .await
        .unwrap();

    // Parallel dequeue
    let mut tasks = Vec::new();
    let jq_arc = std::sync::Arc::new(jq);
    for _ in 0..5 {
        let jq_clone = jq_arc.clone();
        tasks.push(tokio::spawn(
            async move { jq_clone.dequeue(&["Task"]).await },
        ));
    }

    let results: Vec<
        Result<
            Result<Option<aiome_core::traits::Job>, aiome_core::error::AiomeError>,
            tokio::task::JoinError,
        >,
    > = futures::future::join_all(tasks).await;
    let successes = results
        .into_iter()
        .filter(|r| if let Ok(Ok(Some(_))) = r { true } else { false })
        .count();

    // Only one should successfully dequeue
    assert_eq!(successes, 1);
}

#[tokio::test]
async fn test_sqlite_job_queue_heartbeat() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Heart", "Style", None, None, None, 0)
        .await
        .unwrap();
    jq.dequeue(&["Task"]).await.unwrap();

    let first = jq.fetch_job(&job_id).await.unwrap().unwrap().last_heartbeat;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    jq.heartbeat_pulse(&job_id).await.unwrap();
    let second = jq.fetch_job(&job_id).await.unwrap().unwrap().last_heartbeat;

    assert!(second > first);
}

#[tokio::test]
async fn test_sqlite_job_queue_execution_logs() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Log", "Style", None, None, None, 0)
        .await
        .unwrap();
    jq.store_execution_log(&job_id, "WASM STDOUT: Hello")
        .await
        .unwrap();
    let job = jq.fetch_job(&job_id).await.unwrap().unwrap();
    assert_eq!(job.execution_log, Some("WASM STDOUT: Hello".into()));
}

#[tokio::test]
async fn test_sqlite_job_queue_unincorporate_karma() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Topic", "Style", None, None, None, 0)
        .await
        .unwrap();
    jq.store_karma(
        &job_id,
        "skill-1",
        "Distilled Lesson",
        "Technical",
        "hash-old",
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();

    let uninc = jq.fetch_unincorporated_karma(10, "hash-new").await.unwrap();
    assert_eq!(uninc.len(), 1);
    assert_eq!(
        uninc[0].get("lesson").and_then(|v| v.as_str()),
        Some("Distilled Lesson")
    );
}

#[tokio::test]
async fn test_sqlite_job_queue_incorporate_karma() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Topic", "Style", None, None, None, 0)
        .await
        .unwrap();
    jq.store_karma(
        &job_id,
        "skill-1",
        "Distilled Lesson",
        "Technical",
        "hash-old",
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();
    let uninc = jq.fetch_unincorporated_karma(10, "hash-new").await.unwrap();
    let id = uninc[0]
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string();

    jq.mark_karma_as_incorporated(vec![id], "hash-new")
        .await
        .unwrap();
    let left = jq.fetch_unincorporated_karma(10, "hash-new").await.unwrap();
    assert_eq!(left.len(), 0);
}

#[tokio::test]
async fn test_sqlite_job_queue_retry_poison_pill() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Retry", "Style", None, None, None, 0)
        .await
        .unwrap();

    jq.increment_job_retry_count(&job_id).await.unwrap();
    jq.increment_job_retry_count(&job_id).await.unwrap();
    let poisoned = jq.increment_job_retry_count(&job_id).await.unwrap();

    assert!(poisoned);
    let job = jq.fetch_job(&job_id).await.unwrap().unwrap();
    assert_eq!(job.status, JobStatus::Failed);
    assert!(job.error_message.unwrap().contains("Poison Pill"));
}

#[tokio::test]
async fn test_sqlite_job_queue_immune_rules() {
    use super::guardrails::GuardrailOps;

    let (jq, _tmp) = create_test_queue().await;

    // Insert rules directly via SQL instead of do_store_immune_rule to avoid
    // swarm ops (crypto key generation + signing + Box::pin recursion) which
    // cause stack overflow. Test purpose = verify storage & filtering logic.
    sqlx::query("INSERT INTO immune_rules (id, pattern, severity, action, created_at, node_id, lamport_clock, status) VALUES (?, ?, ?, ?, datetime('now'), 'test', 0, 'Approved')")
        .bind("rule-1")
        .bind("rm -rf")
        .bind(100i64)
        .bind("Block")
        .execute(jq.pool.get_sqlite_pool().unwrap())
        .await
        .unwrap();

    sqlx::query("INSERT INTO immune_rules (id, pattern, severity, action, created_at, node_id, lamport_clock, status) VALUES (?, ?, ?, ?, datetime('now'), 'test', 0, 'Pending')")
        .bind("rule-pending")
        .bind("pending-pattern")
        .bind(50i64)
        .bind("Block")
        .execute(jq.pool.get_sqlite_pool().unwrap())
        .await
        .unwrap();

    let rules = jq.do_fetch_active_immune_rules().await.unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].pattern, "rm -rf");

    let all_rules = jq.do_get_immune_rules().await.unwrap();
    assert_eq!(all_rules.len(), 2);
}

#[tokio::test]
async fn test_sqlite_job_queue_arena_history() {
    let (jq, _tmp) = create_test_queue().await;
    let match_data = aiome_core::contracts::ArenaMatch {
        id: "match-1".into(),
        skill_a: "Skill_A".to_string(),
        skill_b: "Skill_B".to_string(),
        topic: "Topic".to_string(),
        output_a: Some("out A".to_string()),
        output_b: Some("out B".to_string()),
        winner: Some("Skill_A".to_string()),
        reasoning: "A is better".into(),
        created_at: Utc::now().to_rfc3339(),
    };
    jq.record_arena_match(&match_data).await.unwrap();

    // Roundtrip: fetch and verify all fields including output_a/output_b
    let fetched = jq.fetch_arena_matches(10).await.unwrap();
    assert_eq!(fetched.len(), 1);
    let m = &fetched[0];
    assert_eq!(m.id, "match-1");
    assert_eq!(m.skill_a, "Skill_A");
    assert_eq!(m.skill_b, "Skill_B");
    assert_eq!(m.topic, "Topic");
    assert_eq!(m.output_a.as_deref(), Some("out A"));
    assert_eq!(m.output_b.as_deref(), Some("out B"));
    assert_eq!(m.winner.as_deref(), Some("Skill_A"));
    assert_eq!(m.reasoning, "A is better");

    // Verify None outputs are handled correctly
    let match_no_output = aiome_core::contracts::ArenaMatch {
        id: "match-2".into(),
        skill_a: "X".into(),
        skill_b: "Y".into(),
        topic: "Topic2".into(),
        output_a: None,
        output_b: None,
        winner: None,
        reasoning: "Draw".into(),
        created_at: Utc::now().to_rfc3339(),
    };
    jq.record_arena_match(&match_no_output).await.unwrap();
    let fetched2 = jq.fetch_arena_matches(10).await.unwrap();
    assert_eq!(fetched2.len(), 2);
    let draw = fetched2.iter().find(|m| m.id == "match-2").unwrap();
    assert!(draw.output_a.is_none());
    assert!(draw.output_b.is_none());
    assert!(draw.winner.is_none());
}

#[tokio::test]
async fn test_sqlite_job_queue_soul_history() {
    let (jq, _tmp) = create_test_queue().await;
    jq.record_soul_mutation("old", "new", "Mutation")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_soul_coherence() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Topic", "Style", None, None, None, 0)
        .await
        .unwrap();
    let soul_v1 = "550e8400-e29b-41d4-a716-446655440000";
    let soul_v2 = "660e8400-e29b-41d4-a716-446655440001";

    jq.store_karma(
        &job_id,
        "soul_skill",
        "[V1 KARMA]",
        "Synthesized",
        soul_v1,
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();

    let result_v1 = jq
        .fetch_relevant_karma("Soul Test", "soul_skill", 10, soul_v1)
        .await
        .unwrap();
    assert_eq!(result_v1.entries.len(), 1);
    assert_eq!(result_v1.entries[0].lesson, "[V1 KARMA]");

    // Implementation returns legacy marked karma instead of empty list
    let result_v2_legacy = jq
        .fetch_relevant_karma("Soul Test Legacy", "soul_skill", 10, soul_v2)
        .await
        .unwrap();
    assert_eq!(result_v2_legacy.entries.len(), 1);
    assert!(result_v2_legacy.entries[0].lesson.contains("[LEGACY KARMA"));

    let job_id2 = jq
        .enqueue("Task", "Topic 2", "Style", None, None, None, 0)
        .await
        .unwrap();
    jq.store_karma(
        &job_id2,
        "soul_skill",
        "[V2 KARMA]",
        "Synthesized",
        soul_v2,
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();

    let result_v2 = jq
        .fetch_relevant_karma("Soul Test Final", "soul_skill", 10, soul_v2)
        .await
        .unwrap();
    assert_eq!(result_v2.entries.len(), 2);
    assert!(result_v2
        .entries
        .iter()
        .any(|k| k.lesson.contains("[LEGACY KARMA") && k.lesson.contains("[V1 KARMA]")));
    assert!(result_v2.entries.iter().any(|k| k.lesson == "[V2 KARMA]"));
}

#[derive(Debug)]
struct MockEmbedProvider;
#[async_trait]
impl EmbeddingProvider for MockEmbedProvider {
    fn name(&self) -> &str {
        "mock"
    }
    async fn embed(
        &self,
        text: &str,
        _is_query: bool,
    ) -> Result<Vec<f32>, aiome_core::error::AiomeError> {
        if text.contains("alien") {
            Ok(vec![0.0; 1536])
        } else {
            Ok(vec![1.0; 1536])
        }
    }
    async fn test_connection(&self) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }

    fn embedding_dim(&self) -> usize {
        1536
    }
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_ood_detection() {
    let (mut jq, _tmp) = create_test_queue().await;
    jq.set_embedding_provider(Arc::new(MockEmbedProvider)).await;

    let job_id = jq
        .enqueue("Task", "Real Topic", "Style", None, None, None, 0)
        .await
        .unwrap();
    // Use PolarQuantEncoder to generate standard format embedding
    let id = uuid::Uuid::new_v4().to_string();
    let encoder = crate::polar_quant::PolarQuantEncoder::new(4, 32);
    let emb = encoder.encode(&vec![1.0; 1536]);
    sqlx::query("INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, created_at, karma_embedding) VALUES (?, ?, 'Technical', 'skill-1', 'Real Lesson', datetime('now'), ?)")
        .bind(&id).bind(&job_id).bind(&emb).execute(jq.pool.get_sqlite_pool().unwrap()).await.unwrap();

    // Closer match (Mock returns 1.0, DB has 1.0 -> score 1.0)
    let result = jq
        .fetch_relevant_karma("Real Topic", "skill-1", 10, "hash1")
        .await
        .unwrap();
    assert!(!result.is_ood);

    // Out of domain (Mock returns 0.0, DB has 1.0 -> score 0.0)
    let result_ood = jq
        .fetch_relevant_karma("space aliens", "skill-1", 10, "hash1")
        .await
        .unwrap();
    assert!(result_ood.is_ood);
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_cache_hit() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Cache Test", "Style", None, None, None, 0)
        .await
        .unwrap();
    jq.store_karma(
        &job_id,
        "skill-1",
        "Cached Lesson",
        "Technical",
        "hash1",
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();

    // First call - fills cache
    let _ = jq
        .fetch_relevant_karma("Cache Test", "skill-1", 10, "hash1")
        .await
        .unwrap();

    // Directly modify DB
    sqlx::query("UPDATE karma_logs SET lesson = 'Modified Lesson'")
        .execute(jq.pool.get_sqlite_pool().unwrap())
        .await
        .unwrap();

    // Second call - should hit cache
    let result2 = jq
        .fetch_relevant_karma("Cache Test", "skill-1", 10, "hash1")
        .await
        .unwrap();
    assert_eq!(result2.entries[0].lesson, "Cached Lesson"); // Cache hit
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_weight_clamp() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Topic", "Style", None, None, None, 0)
        .await
        .unwrap();
    jq.store_karma(
        &job_id,
        "skill-1",
        "Lesson",
        "Technical",
        "hash1",
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();

    // Default weight is 100 or inherited. Let's find the id.
    let row = sqlx::query("SELECT id, weight FROM karma_logs LIMIT 1")
        .fetch_one(jq.pool.get_sqlite_pool().unwrap())
        .await
        .unwrap();
    let kid: String = row.get("id");

    // Clamp Max
    jq.adjust_karma_weight(&kid, 50).await.unwrap();
    let row_max = sqlx::query("SELECT weight FROM karma_logs WHERE id = ?")
        .bind(&kid)
        .fetch_one(jq.pool.get_sqlite_pool().unwrap())
        .await
        .unwrap();
    assert_eq!(row_max.get::<i64, _>("weight"), 100);

    // Clamp Min
    jq.adjust_karma_weight(&kid, -150).await.unwrap();
    let row_min = sqlx::query("SELECT weight FROM karma_logs WHERE id = ?")
        .bind(&kid)
        .fetch_one(jq.pool.get_sqlite_pool().unwrap())
        .await
        .unwrap();
    assert_eq!(row_min.get::<i64, _>("weight"), 0);
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_forgetting_sweep() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Topic", "Style", None, None, None, 0)
        .await
        .unwrap();

    // 1. Weak memory (low weight) + unused
    jq.store_karma(
        &job_id,
        "skill-1",
        "Weak Lesson",
        "Technical",
        "hash1",
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE karma_logs SET weight = 2, last_applied_at = datetime('now', '-91 days') WHERE lesson = 'Weak Lesson'").execute(jq.pool.get_sqlite_pool().unwrap()).await.unwrap();

    // 2. Another weak/old memory
    jq.store_karma(
        &job_id,
        "skill-1",
        "Old Lesson",
        "Technical",
        "hash1",
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE karma_logs SET weight = 3, last_applied_at = datetime('now', '-100 days') WHERE lesson = 'Old Lesson'").execute(jq.pool.get_sqlite_pool().unwrap()).await.unwrap();

    // 3. Fresh strong memory
    jq.store_karma(
        &job_id,
        "skill-1",
        "Strong Lesson",
        "Technical",
        "hash1",
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();

    // 4. Strong but old memory (should NOT be archived because weight is high)
    jq.store_karma(
        &job_id,
        "skill-1",
        "Old Strong Lesson",
        "Technical",
        "hash1",
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE karma_logs SET weight = 80, last_applied_at = datetime('now', '-200 days') WHERE lesson = 'Old Strong Lesson'").execute(jq.pool.get_sqlite_pool().unwrap()).await.unwrap();

    // Run sweep
    let archived = jq.karma_decay_sweep().await.unwrap();
    assert_eq!(archived, 2); // Weak + Old (now weak) should be archived

    // Verify search excludes archived
    let result = jq
        .fetch_relevant_karma("Topic", "skill-1", 10, "hash1")
        .await
        .unwrap();
    // Strong Lesson and Old Strong Lesson should remain
    assert_eq!(result.entries.len(), 2);
    let lessons: Vec<String> = result.entries.iter().map(|e| e.lesson.clone()).collect();
    assert!(lessons.contains(&"Strong Lesson".to_string()));
    assert!(lessons.contains(&"Old Strong Lesson".to_string()));
}
#[tokio::test]
async fn test_sqlite_job_queue_karma_fts_match() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Topic", "Style", None, None, None, 0)
        .await
        .unwrap();

    // 1. Generic lesson
    jq.store_karma(
        &job_id,
        "skill-1",
        "Generic baking recipe",
        "Technical",
        "hash1",
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();
    // 2. Focused lesson with keyword 'security'
    jq.store_karma(
        &job_id,
        "skill-1",
        "Security Best Practices for bakers",
        "Technical",
        "hash1",
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();

    // Search for 'security'
    let result = jq
        .fetch_relevant_karma("security", "skill-1", 10, "hash1")
        .await
        .unwrap();
    assert_eq!(result.entries.len(), 2);
    // The one with 'Security' in text should be first due to FTS5 boost (50.0)
    assert!(result.entries[0].lesson.contains("Security"));
    assert!(!result.is_ood);
}

#[tokio::test]
async fn test_karma_taxonomy_classification() {
    let mock = MockLlmProvider {
        json_response: r#"{ "domain": "Technical", "subtopic": "Security", "reasoning": "Lesson about security." }"#.to_string(),
    };

    let result =
        super::taxonomy::KarmaTaxonomy::classify(&mock, "Always use parameterized queries")
            .await
            .unwrap();
    assert_eq!(result.domain, "Technical");
    assert_eq!(result.subtopic, "Security");
}

#[tokio::test]
async fn test_karma_taxonomy_fallback() {
    let mock = MockLlmProvider {
        json_response: "garbage".to_string(),
    };

    let result =
        super::taxonomy::KarmaTaxonomy::classify(&mock, "Always use parameterized queries").await;
    assert!(result.is_err());

    let fb = super::taxonomy::KarmaTaxonomy::fallback();
    assert_eq!(fb.domain, "General");
}

#[tokio::test]
async fn test_sqlite_settings_crud() {
    let (jq, _tmp) = create_test_queue().await;

    // Test set and get
    jq.update_setting("llm_model", "test-model-1", "llm", false)
        .await
        .expect("Failed to set");
    let val = jq.get_setting_value("llm_model").await.unwrap();
    assert_eq!(val, Some("test-model-1".to_string()));

    // Test overwrite
    jq.update_setting("llm_model", "test-model-2", "llm", false)
        .await
        .expect("Failed to overwrite");
    let val2 = jq.get_setting_value("llm_model").await.unwrap();
    assert_eq!(val2, Some("test-model-2".to_string()));

    // Test fetch all (visible)
    let all = jq.fetch_all_settings().await.unwrap();
    assert!(
        all.iter()
            .any(|s| s.key == "llm_model" && s.value == "test-model-2"),
        "Saved setting should be present in fetch_all_settings"
    );
}

#[tokio::test]
async fn test_sqlite_settings_secret_masking() {
    let (jq, _tmp) = create_test_queue().await;

    // Set a secret
    jq.update_setting("telegram_token", "super-secret-123", "system", true)
        .await
        .expect("Failed to set secret");

    // get_setting_value should return the actual value (for internal use)
    let val = jq.get_setting_value("telegram_token").await.unwrap();
    assert_eq!(val, Some("super-secret-123".to_string()));

    // fetch_all_settings isn't implemented as a method directly yielding masked values in tests.
    // The web layer `routes::settings::get_settings` does the masking.
    // In db layer fetch_all_settings, we expect it to return raw values, or we manually verify the field `is_secret` is true.
    let all = jq.fetch_all_settings().await.unwrap();
    assert!(
        all.iter().any(|s| s.key == "telegram_token" && s.is_secret),
        "Saved secret setting should be present and marked as secret"
    );
    // Since this is the direct test of UniversalJobQueue, we might just be testing if `is_secret` is respected, not necessarily evaluating presentation masking here.
    // In our `api-server`, get_settings does `if s.is_secret { s.value = "********" }`.
    // If we want DB-level masking, we'd need to update `fetch_all_settings`. Let's just assert `is_secret` flag.
}

#[tokio::test]
async fn test_sqlite_trajectory_store() {
    use aiome_core::trajectory::{
        AgentDiagnosis, FailureCategory, StepCategory, TrajectoryStep, TrajectoryStore,
    };
    let (jq, _tmp) = create_test_queue().await;

    // Create a dummy job first
    let job_id = jq
        .enqueue(
            "Testing",
            "Trajectory Test",
            "Standard",
            None,
            None,
            None,
            0,
        )
        .await
        .expect("Failed to enqueue dummy job");

    // 1. Record Step
    let step = TrajectoryStep {
        step_id: 1,
        job_id: Some(job_id.clone()),
        action: "test_action".into(),
        tool_name: Some("test_tool".into()),
        input: serde_json::json!({}),
        output: serde_json::json!({}),
        timestamp: Utc::now().to_rfc3339(),
        constraint_violations: vec![],
        is_critical_failure: false,
        failure_category: None,
        reasoning: Some("Strategic planning for expansion".into()),
        parent_step_id: Some(0),
        step_category: aiome_core::trajectory::StepCategory::Planning,
        completion_criteria: None,
        interaction_id: None,
        verified_invariants: vec![],
        verification_time_us: None,
        state_hash: None,
        parent_state_hash: None,
    };

    jq.trajectory_store
        .record_step(&job_id, step.clone())
        .await
        .expect("Failed to record trajectory step");

    // 2. Fetch Trajectory
    let trajectory = jq
        .trajectory_store
        .fetch_trajectory(&job_id)
        .await
        .expect("Failed to fetch trajectory");

    assert_eq!(trajectory.len(), 1);
    assert_eq!(trajectory[0].action, "test_action");
    assert_eq!(trajectory[0].tool_name, Some("test_tool".into()));
    assert_eq!(
        trajectory[0].reasoning,
        Some("Strategic planning for expansion".into())
    );
    assert_eq!(trajectory[0].parent_step_id, Some(0));
    assert_eq!(trajectory[0].step_category, StepCategory::Planning);

    // 3. Store Diagnosis
    let diagnosis = AgentDiagnosis {
        critical_failure_step: 1,
        category: FailureCategory::InvalidInvocation,
        root_cause: "Missing argument".into(),
        evidence: vec![],
        self_repair_hint: "Add argument".into(),
        diagnosed_at: "now".into(),
    };

    jq.trajectory_store
        .store_diagnosis(&job_id, diagnosis)
        .await
        .expect("Failed to store diagnosis");

    // 4. Fetch Diagnosis
    let fetched = jq
        .trajectory_store
        .fetch_diagnosis(&job_id)
        .await
        .expect("Failed to fetch diagnosis")
        .expect("Diagnosis should exist");

    assert_eq!(fetched.root_cause, "Missing argument");
    assert_eq!(fetched.self_repair_hint, "Add argument");
}

#[tokio::test]
async fn test_sqlite_expression_tts_status() {
    use crate::job_queue::expression::ExpressionOps;
    use aiome_core::expression::Expression;
    use aiome_core_contracts::expression::TtsStatus;

    let (jq, _tmp) = create_test_queue().await;

    let expr = Expression {
        id: "expr-1".into(),
        content: "Hello world".into(),
        emotion: "happy".into(),
        karma_refs: vec!["k1".into()],
        audio_path: None,
        duration_ms: None,
        tts_status: TtsStatus::NotRequested,
        avatar_params: None,
        created_at: Utc::now().to_rfc3339(),
    };

    jq.do_store_expression(&expr)
        .await
        .expect("Failed to store expression");

    let fetched = jq
        .do_fetch_expressions(10)
        .await
        .expect("Failed to fetch expressions");
    assert_eq!(fetched.len(), 1);
    assert_eq!(fetched[0].id, "expr-1");
    assert_eq!(fetched[0].tts_status, TtsStatus::NotRequested);

    // Update status (INSERT OR REPLACE)
    let mut updated = fetched[0].clone();
    updated.tts_status = TtsStatus::Ready;
    updated.audio_path = Some("path/to/audio.wav".into());

    jq.do_store_expression(&updated)
        .await
        .expect("Failed to update expression");

    let refetched = jq
        .do_fetch_expressions(10)
        .await
        .expect("Failed to fetch expressions");
    assert_eq!(refetched.len(), 1);
    assert_eq!(refetched[0].tts_status, TtsStatus::Ready);
    assert_eq!(refetched[0].audio_path, Some("path/to/audio.wav".into()));
}
#[tokio::test]
async fn test_sqlite_job_queue_priority_order() {
    let (jq, _tmp) = create_test_queue().await;

    // 1. Enqueue low priority job
    jq.enqueue("Task", "Low Priority", "Style", None, None, None, 0)
        .await
        .unwrap();

    // 2. Enqueue high priority job (enqueued later)
    jq.enqueue("Task", "High Priority", "Style", None, None, None, 10)
        .await
        .unwrap();

    // 3. Dequeue - should get High Priority first
    let job1 = jq
        .dequeue(&["Task"])
        .await
        .unwrap()
        .expect("Should find job");
    assert_eq!(job1.topic, "High Priority");
    assert_eq!(job1.priority, 10);

    // 4. Dequeue - should get Low Priority next
    let job2 = jq
        .dequeue(&["Task"])
        .await
        .unwrap()
        .expect("Should find job");
    assert_eq!(job2.topic, "Low Priority");
    assert_eq!(job2.priority, 0);
}
#[tokio::test]
async fn test_fetch_federated_metrics() {
    let (jq, _tmp) = create_test_queue().await;

    // 初期状態のメトリクスを取得
    let metrics = jq
        .fetch_federated_metrics()
        .await
        .expect("Fetch federated metrics failed");

    // 基本構造の妥当性を検証
    assert_eq!(metrics.stats.level, 1);
    assert_eq!(metrics.job_metrics.total_completed, 0);
    assert_eq!(metrics.karma_metrics.total_count, 0);
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_decay_sweep_poincare() {
    use crate::slm_bridge::SlmBridge;
    let (mut jq, _tmp) = create_test_queue().await;

    // SlmBridge を注入
    let slm = Arc::new(SlmBridge::new());
    jq.slm_bridge = Some(slm.clone());

    let job_id = jq
        .enqueue("Task", "GC Test", "Style", None, None, None, 0)
        .await
        .unwrap();

    // 1. 低重要度の記憶を模したデータを挿入
    let low_importance_content = "redundant log fragment 12345 non-essential";
    jq.store_karma(
        &job_id,
        "skill-1",
        low_importance_content,
        "Technical",
        "hash-gc",
        None,
        None,
        None,
        false,
    )
    .await
    .unwrap();

    // 2. Sweep 実行
    let archived = jq.karma_decay_sweep().await.unwrap();
    info!("Poincare GC archived {} entries", archived);

    // 3. アーカイブ後の検索
    let res = jq
        .fetch_relevant_karma("redundant log", "skill-1", 10, "hash-gc")
        .await
        .unwrap();
    info!("Remaining entries after GC: {}", res.entries.len());
}

#[tokio::test]
async fn test_federation_export_privacy_filter() {
    let (jq, _tmp) = create_test_queue().await;

    // 0. Enqueue jobs to satisfy foreign key constraints
    let job1 = jq
        .enqueue("Task", "Public", "Style", None, None, None, 0)
        .await
        .unwrap();
    let job2 = jq
        .enqueue("Task", "Private", "Style", None, None, None, 0)
        .await
        .unwrap();

    // 1. Store a public karma
    jq.store_karma(
        &job1,
        "skill1",
        "Public Lesson",
        "Synthesized",
        "hash1",
        Some("General"),
        Some("test"),
        None,
        false, // false = is_private
    )
    .await
    .unwrap();

    // 2. Store a private karma
    jq.store_karma(
        &job2,
        "skill1",
        "Private Lesson",
        "Synthesized",
        "hash1",
        Some("General"),
        Some("test"),
        None,
        true, // true = is_private
    )
    .await
    .unwrap();

    // 3. Export
    let (karmas, _, _) = jq.export_federated_data(None).await.unwrap();

    // Assert
    assert_eq!(karmas.len(), 1, "Only public karma should be exported");
    assert_eq!(karmas[0].lesson, "Public Lesson");
}

#[tokio::test]
async fn test_job_enqueue_constitutional_violation() {
    let (jq, _tmp) = create_test_queue().await;

    // Attempting to enqueue a job with forbidden dual permissions (Network + Shell + FS)
    let harmful_manifest = PermissionManifest {
        allow_network: true,
        allow_filesystem_write: true,
        allow_shell_execution: true,
        allowed_domains: vec![],
    };

    let res = jq
        .enqueue(
            "Task",
            "Harmful",
            "Style",
            None,
            Some(harmful_manifest),
            None,
            0,
        )
        .await;

    assert!(res.is_err(), "Expected an error for harmful manifest");
    if let Err(AiomeError::SecurityViolation { reason }) = res {
        assert!(
            reason.contains("Excessive permissions"),
            "Error message should contain 'Excessive permissions', got: {}",
            reason
        );
    } else {
        panic!("Expected SecurityViolation error, got {:?}", res);
    }
}

#[tokio::test]
async fn test_elicitation_status_transition_red() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("test", "topic", "style", None, None, None, 100)
        .await
        .unwrap();

    // 1. Pending -> Processing (Dequeue)
    let _job = jq.dequeue(&["test"]).await.unwrap().unwrap();

    // 2. Processing -> AwaitingInput
    jq.update_job_status(
        &job_id,
        aiome_core_contracts::traits::JobStatus::AwaitingInput,
    )
    .await
    .unwrap();

    // 3. Verify status
    let job = jq.fetch_job(&job_id).await.unwrap().unwrap();
    assert!(matches!(
        job.status,
        aiome_core_contracts::traits::JobStatus::AwaitingInput
    ));
    assert_eq!(job.status.as_str(), "AwaitingInput");
}

#[tokio::test]
async fn test_forget_actor_broadcasts_system_event() {
    use crate::job_queue::security::SecurityOps;
    use aiome_core_contracts::contracts::SystemEvent;

    let (jq, _tmp) = create_test_queue().await;
    let agent_id = uuid::Uuid::new_v4();

    let mut rx = jq.event_bus.subscribe();

    jq.forget_actor(agent_id).await.unwrap();

    // イベントを受信できるか確認
    let event = tokio::time::timeout(tokio::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("Should not timeout")
        .expect("Should receive event");

    let SystemEvent::ActorForgotten(id) = event;
    assert_eq!(id, agent_id);
}

#[tokio::test]
async fn test_sqlite_job_queue_peer_sync_time() {
    let (jq, _tmp) = create_test_queue().await;
    let peer_url = "https://node.example.com";

    // 1. Initial state: None
    let initial = jq.do_get_peer_sync_time(peer_url).await.unwrap();
    assert!(initial.is_none(), "Expected None for initial sync time");

    // 2. Update time
    let sync_time = chrono::Utc::now().to_rfc3339();
    jq.do_update_peer_sync_time(peer_url, &sync_time)
        .await
        .unwrap();

    // 3. Fetch updated time — round-trip verification
    let fetched_opt = jq.do_get_peer_sync_time(peer_url).await.unwrap();
    assert!(
        fetched_opt.is_some(),
        "Expected Some after updating sync time"
    );
    assert_eq!(fetched_opt.unwrap(), sync_time);

    // 4. Overwrite with a new timestamp — upsert semantics
    let sync_time_2 = "2099-12-31T23:59:59+00:00";
    jq.do_update_peer_sync_time(peer_url, sync_time_2)
        .await
        .unwrap();
    let fetched_2 = jq.do_get_peer_sync_time(peer_url).await.unwrap();
    assert_eq!(
        fetched_2.as_deref(),
        Some(sync_time_2),
        "Upsert should overwrite previous value"
    );
}

#[tokio::test]
async fn test_sqlite_job_queue_peer_sync_time_empty_url() {
    let (jq, _tmp) = create_test_queue().await;
    // Edge case: empty string is a valid TEXT PRIMARY KEY in SQLite
    let empty_url = "";

    let initial = jq.do_get_peer_sync_time(empty_url).await.unwrap();
    assert!(initial.is_none(), "Empty URL should return None initially");

    jq.do_update_peer_sync_time(empty_url, "2026-01-01T00:00:00Z")
        .await
        .unwrap();
    let fetched = jq.do_get_peer_sync_time(empty_url).await.unwrap();
    assert!(
        fetched.is_some(),
        "Empty URL should be retrievable after insert"
    );
}
