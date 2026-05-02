/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::AiomeError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

#[derive(Debug, Serialize, Deserialize, Clone)]
/// `SkillPerformance` 構造体
pub struct SkillPerformance {
    /// success_count
    pub success_count: u64,
    /// failure_count
    pub failure_count: u64,
    /// average_latency_ms
    pub average_latency_ms: u64,
    /// total_karma_weight
    pub total_karma_weight: f64,
}

/// スキルの並列実行と評価を行うアリーナ
pub struct SkillArena {
    performance_map: Arc<RwLock<HashMap<String, SkillPerformance>>>,
    pub culling_threshold: f64,
    pub protected_skills: std::collections::HashSet<String>,
    db_pool: Option<crate::db::DatabasePool>,
    incident_repo: Option<crate::aegis::incident_repo::IncidentRepository>,
}

impl SkillArena {
    /// 新しいインスタンスを生成する
    pub fn new() -> Self {
        let mut protected_skills = std::collections::HashSet::new();
        protected_skills.insert("essential_core".to_string());
        protected_skills.insert("immune_system".to_string());
        protected_skills.insert("commerce_engine".to_string());
        protected_skills.insert("skill_arena".to_string());

        Self {
            performance_map: Arc::new(RwLock::new(HashMap::new())),
            culling_threshold: 0.5,
            protected_skills,
            db_pool: None,
            incident_repo: None,
        }
    }

    pub fn with_db_pool(mut self, pool: crate::db::DatabasePool) -> Self {
        self.incident_repo = Some(crate::aegis::incident_repo::IncidentRepository::new(
            pool.clone(),
        ));
        self.db_pool = Some(pool);
        self
    }

    pub async fn init_db(&self) -> Result<(), AiomeError> {
        if let Some(crate::db::DatabasePool::Sqlite(pool)) = &self.db_pool {
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS skill_performance (
                    skill_name TEXT PRIMARY KEY,
                    success_count INTEGER NOT NULL,
                    failure_count INTEGER NOT NULL,
                    average_latency_ms INTEGER NOT NULL,
                    total_karma_weight REAL NOT NULL
                )",
            )
            .execute(pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to init skill_performance table: {}", e),
            })?;
        }
        Ok(())
    }

    pub async fn save_stats(&self) -> Result<(), AiomeError> {
        if let Some(crate::db::DatabasePool::Sqlite(pool)) = &self.db_pool {
            let map = self.performance_map.read().await;
            for (skill_name, perf) in map.iter() {
                sqlx::query(
                    "INSERT INTO skill_performance (skill_name, success_count, failure_count, average_latency_ms, total_karma_weight)
                     VALUES (?, ?, ?, ?, ?)
                     ON CONFLICT(skill_name) DO UPDATE SET
                        success_count = excluded.success_count,
                        failure_count = excluded.failure_count,
                        average_latency_ms = excluded.average_latency_ms,
                        total_karma_weight = excluded.total_karma_weight"
                )
                .bind(skill_name)
                .bind(perf.success_count as i64)
                .bind(perf.failure_count as i64)
                .bind(perf.average_latency_ms as i64)
                .bind(perf.total_karma_weight)
                .execute(pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to save skill performance: {}", e),
                })?;
            }
        }
        Ok(())
    }

    pub async fn load_stats(&self) -> Result<(), AiomeError> {
        if let Some(crate::db::DatabasePool::Sqlite(pool)) = &self.db_pool {
            let mut map = self.performance_map.write().await;

            let rows: Vec<(String, i64, i64, i64, f64)> = sqlx::query_as(
                "SELECT skill_name, success_count, failure_count, average_latency_ms, total_karma_weight FROM skill_performance"
            ).fetch_all(pool).await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to load skill performance: {}", e),
            })?;

            for (name, sc, fc, lat, weight) in rows {
                map.insert(
                    name,
                    SkillPerformance {
                        success_count: sc as u64,
                        failure_count: fc as u64,
                        average_latency_ms: lat as u64,
                        total_karma_weight: weight,
                    },
                );
            }
        }
        Ok(())
    }

    /// [A-3] Skill Culling
    /// Record the outcome of a skill execution to update its reputation.
    pub async fn record_outcome(
        &self,
        skill_name: &str,
        is_success: bool,
        latency_ms: u64,
        karma_delta: f64,
    ) {
        let need_incident = {
            let mut map = self.performance_map.write().await;
            let perf = map
                .entry(skill_name.to_string())
                .or_insert(SkillPerformance {
                    success_count: 0,
                    failure_count: 0,
                    average_latency_ms: 0,
                    total_karma_weight: 0.0,
                });

            if is_success {
                perf.success_count += 1;
            } else {
                perf.failure_count += 1;
            }

            perf.total_karma_weight += karma_delta;

            // Rolling average for latency
            let total_runs = perf.success_count + perf.failure_count;
            perf.average_latency_ms =
                (perf.average_latency_ms * (total_runs - 1) + latency_ms) / total_runs;

            // Check for culling
            if total_runs > 10 {
                let failure_rate = perf.failure_count as f64 / total_runs as f64;
                if failure_rate > self.culling_threshold {
                    warn!("🧹 [SkillArena] CULLING DETECTED: Skill '{}' has {}% failure rate. Marking for decommissioning.", skill_name, failure_rate * 100.0);
                }
            }

            !is_success
        }; // write lock dropped here

        // Aegis: Record failed skill outcome (outside lock scope)
        if need_incident {
            if let Some(repo) = &self.incident_repo {
                let trace = format!(
                    "SkillArena record_outcome: skill failed with latency {}ms",
                    latency_ms
                );
                if let Err(e) = repo
                    .insert_incident(skill_name, "N/A", "Arena Task", &trace)
                    .await
                {
                    warn!(
                        "⚠️ [Aegis] Failed to record arena incident for '{}': {}",
                        skill_name, e
                    );
                }
            }
        }
    }

    /// `get_stats` を実行する
    pub async fn get_stats(&self, skill_name: &str) -> Option<SkillPerformance> {
        let map = self.performance_map.read().await;
        map.get(skill_name).cloned()
    }

    /// アリーナの歴史から統計的に弱いスキルを特定し、淘汰（アンインストール）の準備をする
    pub async fn analyze_and_cull(&self) -> Result<Vec<String>, AiomeError> {
        let map = self.performance_map.read().await;
        let mut culled_skills = Vec::new();

        for (skill_name, perf) in map.iter() {
            if self.protected_skills.contains(skill_name) {
                continue;
            }

            let total_runs = perf.success_count + perf.failure_count;
            if total_runs > 10 {
                let failure_rate = perf.failure_count as f64 / total_runs as f64;
                if failure_rate > self.culling_threshold {
                    culled_skills.push(skill_name.clone());
                }
            }
        }

        Ok(culled_skills)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::collections::HashSet;

    #[tokio::test]
    async fn test_skill_arena_culling_logic() {
        // Arrange
        let mut arena = SkillArena::new();
        arena.culling_threshold = 0.5;
        // Mock protected skills
        arena.protected_skills.insert("essential_core".to_string());

        // Act
        // Make 'bad_skill' fail 8 out of 11 times (> 50%)
        for _ in 0..8 {
            arena.record_outcome("bad_skill", false, 1500, -1.0).await;
        }
        for _ in 0..3 {
            arena.record_outcome("bad_skill", true, 200, 0.5).await;
        }

        // Make 'essential_core' fail 8 out of 11 times (> 50%)
        for _ in 0..8 {
            arena
                .record_outcome("essential_core", false, 1500, -1.0)
                .await;
        }
        for _ in 0..3 {
            arena.record_outcome("essential_core", true, 200, 0.5).await;
        }

        // Make 'good_skill' fail 1 out of 11 times (< 50%)
        for _ in 0..1 {
            arena.record_outcome("good_skill", false, 1500, -1.0).await;
        }
        for _ in 0..10 {
            arena.record_outcome("good_skill", true, 200, 0.5).await;
        }

        // Call the new method we need to implement
        let culled = arena.analyze_and_cull().await.unwrap();

        // Assert
        assert!(culled.contains(&"bad_skill".to_string()));
        assert!(!culled.contains(&"good_skill".to_string()));
        assert!(
            !culled.contains(&"essential_core".to_string()),
            "Protected skill must not be culled"
        );
    }

    #[tokio::test]
    async fn test_record_outcome_incident_on_failure() {
        let pool = crate::db::DatabasePool::new_sqlite("sqlite::memory:")
            .await
            .unwrap();
        {
            let ts = std::sync::Arc::new(
                crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
            );
            let jq = crate::job_queue::UniversalJobQueue::new(pool.clone(), None, ts)
                .await
                .unwrap();
            crate::job_queue::migrations::DbInitializer::init_db(&jq)
                .await
                .unwrap();
        }

        let mut arena = SkillArena::new().with_db_pool(pool.clone());
        arena
            .record_outcome("failing_skill", false, 1500, -1.0)
            .await;

        if let crate::db::DatabasePool::Sqlite(sqlite_pool) = &pool {
            let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM aegis_incidents")
                .fetch_one(sqlite_pool)
                .await
                .unwrap();
            assert_eq!(row.0, 1, "Incident should be recorded on task failure");
        }
    }

    #[tokio::test]
    async fn test_skill_arena_sqlite_persistence() {
        // Arrange
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut arena = SkillArena::new();
        arena = arena.with_db_pool(crate::db::DatabasePool::Sqlite(pool.clone()));

        // Act
        // Initialize tables
        arena.init_db().await.unwrap();

        arena
            .record_outcome("persisted_skill", true, 120, 1.0)
            .await;
        arena
            .record_outcome("persisted_skill", false, 800, -2.0)
            .await;

        // Save
        arena.save_stats().await.unwrap();

        // Create a new arena instance and load
        let mut arena_loaded = SkillArena::new();
        arena_loaded = arena_loaded.with_db_pool(crate::db::DatabasePool::Sqlite(pool));
        arena_loaded.load_stats().await.unwrap();

        let stats = arena_loaded
            .get_stats("persisted_skill")
            .await
            .expect("Stats should be loaded");

        // Assert
        assert_eq!(stats.success_count, 1);
        assert_eq!(stats.failure_count, 1);
        assert_eq!(stats.total_karma_weight, -1.0);
    }
}
