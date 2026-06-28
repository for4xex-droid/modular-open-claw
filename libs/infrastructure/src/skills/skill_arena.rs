/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::AiomeError;
use aiome_core_contracts::contracts::{FeedbackCategory, IterationRecord};
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
    /// 新設: 反復試行履歴の蓄積
    #[serde(default)]
    pub optimization_history: Vec<IterationRecord>,
    /// 新設: スコア履歴
    #[serde(default)]
    pub best_scores: Vec<f64>,
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

            // マイグレーション: カラムの追加 (非破壊的)
            sqlx::query("ALTER TABLE skill_performance ADD COLUMN optimization_history TEXT")
                .execute(pool)
                .await
                .ok();
            sqlx::query("ALTER TABLE skill_performance ADD COLUMN best_scores TEXT")
                .execute(pool)
                .await
                .ok();
        }
        Ok(())
    }

    pub async fn save_stats(&self) -> Result<(), AiomeError> {
        if let Some(crate::db::DatabasePool::Sqlite(pool)) = &self.db_pool {
            let map = self.performance_map.read().await;
            for (skill_name, perf) in map.iter() {
                let opt_history_json = serde_json::to_string(&perf.optimization_history)
                    .unwrap_or_else(|_| "[]".to_string());
                let best_scores_json =
                    serde_json::to_string(&perf.best_scores).unwrap_or_else(|_| "[]".to_string());

                sqlx::query(
                    "INSERT INTO skill_performance (skill_name, success_count, failure_count, average_latency_ms, total_karma_weight, optimization_history, best_scores)
                     VALUES (?, ?, ?, ?, ?, ?, ?)
                     ON CONFLICT(skill_name) DO UPDATE SET
                        success_count = excluded.success_count,
                        failure_count = excluded.failure_count,
                        average_latency_ms = excluded.average_latency_ms,
                        total_karma_weight = excluded.total_karma_weight,
                        optimization_history = excluded.optimization_history,
                        best_scores = excluded.best_scores"
                )
                .bind(skill_name)
                .bind(perf.success_count as i64)
                .bind(perf.failure_count as i64)
                .bind(perf.average_latency_ms as i64)
                .bind(perf.total_karma_weight)
                .bind(opt_history_json)
                .bind(best_scores_json)
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

            let rows: Vec<(String, i64, i64, i64, f64, Option<String>, Option<String>)> = sqlx::query_as(
                "SELECT skill_name, success_count, failure_count, average_latency_ms, total_karma_weight, optimization_history, best_scores FROM skill_performance"
            ).fetch_all(pool).await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to load skill performance: {}", e),
            })?;

            for (name, sc, fc, lat, weight, opt_hist, best_sc) in rows {
                let opt_history = opt_hist
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();
                let best_scores = best_sc
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                map.insert(
                    name,
                    SkillPerformance {
                        success_count: sc as u64,
                        failure_count: fc as u64,
                        average_latency_ms: lat as u64,
                        total_karma_weight: weight,
                        optimization_history: opt_history,
                        best_scores,
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
        self.record_outcome_with_feedback(skill_name, is_success, latency_ms, karma_delta, None)
            .await;
    }

    /// Record the outcome of a skill execution with structured feedback (ComPilot)
    pub async fn record_outcome_with_feedback(
        &self,
        skill_name: &str,
        is_success: bool,
        latency_ms: u64,
        karma_delta: f64,
        feedback: Option<FeedbackCategory>,
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
                    optimization_history: Vec::new(),
                    best_scores: Vec::new(),
                });

            if is_success {
                perf.success_count += 1;
            } else {
                perf.failure_count += 1;
            }

            perf.total_karma_weight += karma_delta;

            // フィードバックの蓄積 (ComPilot)
            if let Some(fb) = feedback {
                perf.optimization_history.push(IterationRecord {
                    round: (perf.success_count + perf.failure_count) as u32,
                    proposal_summary: format!("Skill run outcome: is_success={}", is_success),
                    feedback: fb,
                    timestamp: chrono::Utc::now(),
                });
            }

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
                    "SkillArena record_outcome_with_feedback: skill failed with latency {}ms",
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

    /// Boltzmann (softmax) 選択でスキルを確率的に選択する
    ///
    /// Autodata 論文由来。スキルの成功率に基づく確率分布でサンプリングし、
    /// 高成功率スキルを優遇しつつ低成功率スキルにも探索機会を保証する。
    ///
    /// - T = 0.1 (温度パラメータ: 低いほど搾取寄り)
    /// - exploration_floor = 0.05 (最低探索確率: 5%)
    /// - MIN_RUNS = 3 (最低試行回数: 未満はデフォルト 0.5)
    pub async fn select_skill_boltzmann(&self, candidates: &[String]) -> Option<String> {
        if candidates.is_empty() {
            return None;
        }

        const T: f64 = 0.1;
        const EXPLORATION_FLOOR: f64 = 0.05;
        const MIN_RUNS: u64 = 3;

        let map = self.performance_map.read().await;
        let mut scores = Vec::with_capacity(candidates.len());

        for candidate in candidates {
            let mut score = if let Some(perf) = map.get(candidate) {
                // 将来拡張: best_scores が空でなければその平均を優先
                if !perf.best_scores.is_empty() {
                    let sum: f64 = perf.best_scores.iter().sum();
                    sum / perf.best_scores.len() as f64
                } else {
                    let total_runs = perf.success_count + perf.failure_count;
                    if total_runs < MIN_RUNS {
                        0.5
                    } else {
                        perf.success_count as f64 / total_runs as f64
                    }
                }
            } else {
                0.5
            };

            // NaN / Infinity / 負の値に対するガード
            if !score.is_finite() || score < 0.0 {
                score = 0.5;
            }

            scores.push(score);
        }

        // 早期にロックを解放
        drop(map);

        // 1. exp(score / T) を計算
        let weights: Vec<f64> = scores.iter().map(|&s| (s / T).exp()).collect();

        // 2. 確率分布の計算
        let sum_weights: f64 = weights.iter().sum();
        let n = candidates.len() as f64;

        let mut probs: Vec<f64> = if sum_weights > 0.0 {
            weights.iter().map(|&w| w / sum_weights).collect()
        } else {
            vec![1.0 / n; candidates.len()]
        };

        // 3. exploration_floor を適用 (最低探索確率の保証)
        let floor_per_candidate = EXPLORATION_FLOOR / n;
        for p in &mut probs {
            if *p < floor_per_candidate {
                *p = floor_per_candidate;
            }
        }

        // 4. 再正規化
        let sum_probs: f64 = probs.iter().sum();
        if sum_probs > 0.0 {
            for p in &mut probs {
                *p /= sum_probs;
            }
        } else {
            probs = vec![1.0 / n; candidates.len()];
        }

        // 5. WeightedIndex によるサンプリング
        use rand::distributions::Distribution;
        let mut rng = rand::thread_rng();
        match rand::distributions::WeightedIndex::new(&probs) {
            Ok(dist) => {
                let idx = dist.sample(&mut rng);
                candidates.get(idx).cloned()
            }
            Err(_) => {
                // 万が一のフォールバック
                use rand::seq::SliceRandom;
                candidates.choose(&mut rng).cloned()
            }
        }
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

    #[tokio::test]
    async fn test_record_outcome_with_feedback_fallback() {
        let arena = SkillArena::new();

        // 1. 新メソッドで成功結果を記録
        let feedback = aiome_core_contracts::contracts::FeedbackCategory::Success {
            metrics: std::collections::HashMap::new(),
        };
        arena
            .record_outcome_with_feedback("feedback_skill", true, 100, 1.0, Some(feedback))
            .await;

        let stats = arena
            .get_stats("feedback_skill")
            .await
            .expect("Stats should exist");
        assert_eq!(stats.success_count, 1);

        // 2. 旧メソッド（ラッパー）での記録が機能することを確認
        arena
            .record_outcome("feedback_skill", false, 200, -0.5)
            .await;

        let stats2 = arena
            .get_stats("feedback_skill")
            .await
            .expect("Stats should exist");
        assert_eq!(stats2.success_count, 1);
        assert_eq!(stats2.failure_count, 1);
    }

    #[tokio::test]
    async fn test_boltzmann_favors_high_success_rate() {
        let arena = SkillArena::new();
        // Setup 3 skills with different success rates (all runs >= MIN_RUNS)
        // good_skill: 90% (9/10)
        for _ in 0..9 {
            arena.record_outcome("good_skill", true, 100, 1.0).await;
        }
        arena.record_outcome("good_skill", false, 100, -1.0).await;

        // mediocre_skill: 70% (7/10)
        for _ in 0..7 {
            arena.record_outcome("mediocre_skill", true, 100, 1.0).await;
        }
        for _ in 0..3 {
            arena
                .record_outcome("mediocre_skill", false, 100, -1.0)
                .await;
        }

        // bad_skill: 10% (1/10)
        arena.record_outcome("bad_skill", true, 100, 1.0).await;
        for _ in 0..9 {
            arena.record_outcome("bad_skill", false, 100, -1.0).await;
        }

        let candidates = vec![
            "good_skill".to_string(),
            "mediocre_skill".to_string(),
            "bad_skill".to_string(),
        ];

        let mut good_count = 0;
        let mut mediocre_count = 0;
        let mut bad_count = 0;

        for _ in 0..2000 {
            let selected = arena.select_skill_boltzmann(&candidates).await.unwrap();
            match selected.as_str() {
                "good_skill" => good_count += 1,
                "mediocre_skill" => mediocre_count += 1,
                "bad_skill" => bad_count += 1,
                _ => panic!("Unexpected skill selected"),
            }
        }

        println!(
            "Boltzmann sampling stats: good={}, mediocre={}, bad={}",
            good_count, mediocre_count, bad_count
        );

        // Good skill should be favored significantly
        assert!(
            good_count > mediocre_count,
            "Good skill must be selected more than mediocre skill"
        );
        assert!(
            mediocre_count > bad_count,
            "Mediocre skill must be selected more than bad skill"
        );
        assert!(
            good_count > 1200,
            "Good skill should be selected a majority of the time"
        );
        assert!(
            bad_count < 100,
            "Bad skill selection should be low but non-zero"
        );
    }

    #[tokio::test]
    async fn test_boltzmann_exploration_floor() {
        let arena = SkillArena::new();
        // terrible_skill: 0% (0/10)
        for _ in 0..10 {
            arena
                .record_outcome("terrible_skill", false, 100, -1.0)
                .await;
        }
        // perfect_skill: 100% (10/10)
        for _ in 0..10 {
            arena.record_outcome("perfect_skill", true, 100, 1.0).await;
        }

        let candidates = vec!["terrible_skill".to_string(), "perfect_skill".to_string()];

        let mut terrible_count = 0;
        let mut perfect_count = 0;

        for _ in 0..2000 {
            let selected = arena.select_skill_boltzmann(&candidates).await.unwrap();
            match selected.as_str() {
                "terrible_skill" => terrible_count += 1,
                "perfect_skill" => perfect_count += 1,
                _ => panic!("Unexpected skill selected"),
            }
        }

        // Even with 0% success rate, the exploration floor (5%) should guarantee
        // terrible_skill gets selected some of the time.
        // Expected value is ~49 for 2000 trials. Let's assert >= 20 to avoid variance flaking.
        assert!(
            terrible_count >= 20,
            "Terrible skill should be explored due to floor. Count: {}",
            terrible_count
        );
        assert!(perfect_count > terrible_count);
    }

    #[tokio::test]
    async fn test_boltzmann_min_runs_guard() {
        let arena = SkillArena::new();
        // newbie_skill: 100% (1/1) but runs < MIN_RUNS (3), so score is treated as 0.5
        arena.record_outcome("newbie_skill", true, 100, 1.0).await;

        // solid_skill: 70% (7/10) with runs >= MIN_RUNS (3), so score is 0.7
        for _ in 0..7 {
            arena.record_outcome("solid_skill", true, 100, 1.0).await;
        }
        for _ in 0..3 {
            arena.record_outcome("solid_skill", false, 100, -1.0).await;
        }

        let candidates = vec!["newbie_skill".to_string(), "solid_skill".to_string()];

        let mut newbie_count = 0;
        let mut solid_count = 0;

        for _ in 0..1000 {
            let selected = arena.select_skill_boltzmann(&candidates).await.unwrap();
            match selected.as_str() {
                "newbie_skill" => newbie_count += 1,
                "solid_skill" => solid_count += 1,
                _ => panic!("Unexpected skill selected"),
            }
        }

        // Since solid_skill has score 0.7 and newbie_skill has score 0.5,
        // solid_skill should be selected more often.
        assert!(
            solid_count > newbie_count,
            "Solid skill (0.7) should be selected more than newbie skill (0.5). Solid: {}, Newbie: {}",
            solid_count, newbie_count
        );
    }
}
