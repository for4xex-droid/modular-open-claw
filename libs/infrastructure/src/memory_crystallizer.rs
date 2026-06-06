/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::belief_consistency_gate::{BeliefCheckResult, BeliefConsistencyGate};
use crate::cortex_synth::{SynthPair, SynthQualityJudge};
use crate::job_queue::DistillationOps;
use crate::slm_bridge::SlmBridge;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::LlmProvider;
use aiome_core_contracts::trajectory::{StepCategory, TrajectoryStep};
use aiome_core_contracts::AuditStore;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
/// LLM が生成するファクトのカテゴリ分類。
/// 現在はプロンプト出力のパース用に定義されており、
/// 将来的に `run_distillation_cycle` 内で構造化パースに使用予定。
pub enum FactCategory {
    Preference,
    Knowledge,
    Context,
    Behavior,
    Goal,
    General,
}

/// 短期記憶から長期Karmaへの結晶化エンジン
pub struct MemoryCrystallizer {
    provider: Arc<dyn LlmProvider + Send + Sync>,
    ops: Arc<dyn DistillationOps>,
    semaphore: Arc<Semaphore>,
    slm_bridge: Option<Arc<SlmBridge>>,
    belief_gate: Option<Arc<BeliefConsistencyGate>>,
    judge: Option<Arc<dyn SynthQualityJudge>>,
}

impl MemoryCrystallizer {
    /// 新しいインスタンスを生成する
    pub fn new(
        provider: Arc<dyn LlmProvider + Send + Sync>,
        ops: Arc<dyn DistillationOps>,
        semaphore: Arc<Semaphore>,
        slm_bridge: Option<Arc<SlmBridge>>,
        belief_gate: Option<Arc<BeliefConsistencyGate>>,
        judge: Option<Arc<dyn SynthQualityJudge>>,
    ) -> Self {
        Self {
            provider,
            ops,
            semaphore,
            slm_bridge,
            belief_gate,
            judge,
        }
    }

    /// 短期記憶（Raw Karma）を蒸留し、長期的な知恵（Distilled Fact）として結晶化する。
    ///
    /// 処理フロー:
    /// 1. 閾値以上の Raw Karma を持つスキルを取得
    /// 2. LLM でカテゴリ付きファクトに抽象化
    /// 3. CortexSynth Quality Gate で品質検証
    /// 4. Belief Consistency Gate で信念矛盾チェック
    /// 5. 合格した結果を Distilled Karma として永続化
    ///
    /// # スキル処理上限
    /// 1サイクルあたり最大 100 スキルまで処理し、過剰なリソース消費を防止する。
    pub async fn run_distillation_cycle(&self) -> Result<(), AiomeError> {
        // 1. Skill-based Karma Distillation (Consolidating raw experiences)
        // Fetch skills that have 10+ raw karma entries
        let skills = self.ops.fetch_skills_for_distillation(10).await?;
        // OOM / CPU 防御: 1サイクルあたりの処理スキル数を制限
        const MAX_SKILLS_PER_CYCLE: usize = 100;
        for skill in skills.iter().take(MAX_SKILLS_PER_CYCLE) {
            if let Ok(_permit) = self.semaphore.try_acquire() {
                info!(
                    "💎 [MemoryCrystallizer] Crystallizing karma for skill: {}",
                    skill
                );
                let raw_karma = match self.ops.fetch_raw_karma_for_skill(skill).await {
                    Ok(k) => k,
                    Err(e) => {
                        warn!(
                            "⚠️ [MemoryCrystallizer] Failed to fetch raw karma for {}: {:?}",
                            skill, e
                        );
                        continue;
                    }
                };

                // VULN-63: OOM Prevention - process in batches of 50 to avoid massive string allocation
                for raw_karma_chunk in raw_karma.chunks(50) {
                    // 個別 lesson の長さを制限し、極端に大きな入力による OOM を防止
                    const MAX_LESSON_CHARS: usize = 2000;
                    let lessons = raw_karma_chunk
                        .iter()
                        .map(|(_, lesson)| {
                            let truncated: String = lesson.chars().take(MAX_LESSON_CHARS).collect();
                            format!("- {}", truncated)
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    // プロンプトインジェクション対策: XML デリミタでユーザーデータを分離
                    let prompt = format!(
                        "以下の技能に関する生の教訓を抽象化し、本質的な知恵（Fact）に結晶化してください。\n\
                        また、各Factに対して以下のカテゴリのいずれかを割り当ててください：\n\
                        - Preference (ユーザーの好み)\n\
                        - Knowledge (技術的・一般的な知識)\n\
                        - Context (現在の状況・背景)\n\
                        - Behavior (エージェントの振る舞い方)\n\
                        - Goal (達成すべき目標)\n\n\
                        <SKILL>{}</SKILL>\n\
                        <LESSONS>\n{}\n</LESSONS>\n\n\
                        出力形式: [Category] 内容 の形式で短い箇条書き。日本語で出力せよ。",
                        skill, lessons
                    );

                    match self.provider.complete(&prompt, None).await {
                        Ok(resp) => {
                            let ids: Vec<String> =
                                raw_karma_chunk.iter().map(|(id, _)| id.clone()).collect();

                            let mut domain = None;

                            // Phase 1.5: CortexSynth Quality Gate
                            if let Some(judge) = &self.judge {
                                let eval_pair = SynthPair {
                                    instruction: format!("Abstract facts for skill: {}", skill),
                                    response: resp.content.clone(),
                                    source_article_id: "karma_distillation".to_string(),
                                    // SynthPair requires quality_score but karma distillation
                                    // has no self-reported score. Use 1.0 as a neutral placeholder
                                    // so the Judge evaluates purely on content quality.
                                    quality_score: 1.0,
                                };
                                match judge.evaluate(&eval_pair).await {
                                    Ok(v) if !v.accept => {
                                        warn!(
                                            "⚠️ [MemoryCrystallizer] Judge rejected crystallization (score={:.2}): {}",
                                            v.score, v.reasoning
                                        );
                                        // Skip persistence for rejected content to maintain data purity.
                                        // Raw karma is NOT deleted — it remains available for re-processing
                                        // in the next cycle, so no data loss occurs.
                                        continue;
                                    }
                                    Err(e) => {
                                        // Graceful degradation: Judge failure should not block the pipeline.
                                        warn!("⚠️ [MemoryCrystallizer] Judge error (fallback: accept): {}", e);
                                    }
                                    _ => {}
                                }
                            }

                            // Belief Consistency Gate: 結晶化内容が魂の信念と矛盾しないか検証
                            if let Some(gate) = &self.belief_gate {
                                match gate.check_belief_consistency(&resp.content).await {
                                    Ok(BeliefCheckResult::Consistent) => {
                                        // No action needed
                                    }
                                    Ok(BeliefCheckResult::Contradicted { flag }) => {
                                        warn!(
                                            "🛡️ [MemoryCrystallizer] Belief contradiction detected for {}: {}",
                                            skill, flag
                                        );
                                        domain = Some("belief_contradicted".to_string());
                                    }
                                    Ok(BeliefCheckResult::RevisionCandidate { evidence }) => {
                                        info!(
                                            "🧠 [MemoryCrystallizer] Belief revision candidate detected for {}. Recording to DAG.",
                                            skill
                                        );
                                        // Karma 保存はスキップし、DAG に証拠を記録
                                        let evidence_value = match serde_json::to_value(&evidence) {
                                            Ok(v) => v,
                                            Err(e) => {
                                                warn!("⚠️ [MemoryCrystallizer] Failed to serialize belief evidence: {:?}", e);
                                                serde_json::Value::Null
                                            }
                                        };
                                        let step = TrajectoryStep {
                                            job_id: Some(format!("crystallize-{}", skill)),
                                            action: "RequestBeliefRevision".into(),
                                            step_category: StepCategory::Decision,
                                            output: evidence_value,
                                            reasoning: Some(format!(
                                                "Evidence for skill: {}",
                                                skill
                                            )),
                                            ..Default::default()
                                        };
                                        if let Err(e) = self.ops.store_trajectory_step(step).await {
                                            warn!("⚠️ [MemoryCrystallizer] Failed to record belief revision step for {}: {:?}", skill, e);
                                        }
                                        continue;
                                    }
                                    Err(e) => {
                                        warn!("⚠️ [MemoryCrystallizer] Belief gate error: {:?}", e);
                                    }
                                }
                            }

                            if let Err(e) = self
                                .ops
                                .apply_distilled_karma(
                                    skill,
                                    &resp.content,
                                    &ids,
                                    "v2_fact_categorized",
                                    None,
                                    domain.as_deref(),
                                    None,
                                )
                                .await
                            {
                                warn!(
                                    "⚠️ [MemoryCrystallizer] Failed to persist distilled karma for {}: {:?}",
                                    skill, e
                                );
                            }
                            info!(
                                "✅ [MemoryCrystallizer] Karma crystallized with facts for {} (Batch of {})",
                                skill,
                                ids.len()
                            );
                        }
                        Err(e) => {
                            warn!(
                                "⚠️ [MemoryCrystallizer] Failed to crystallize karma chunk for {} (chunk_size={}): {:?}",
                                skill, raw_karma_chunk.len(), e
                            );
                            // LLM 呼び出し失敗はチャンク単位でスキップ。
                            // raw_karma は未消費のまま残り、次サイクルで再処理される。
                        }
                    }
                }
            } else {
                info!(
                    "⏸️ [MemoryCrystallizer] Semaphore exhausted, skipping skill: {}",
                    skill
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_queue::UniversalJobQueue;
    use crate::slm_bridge::SlmBridge;
    use aiome_core_contracts::llm::{LlmProvider, LlmResponse, StopReason};
    use aiome_core_contracts::trajectory::TrajectoryStep;
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    #[derive(Debug)]
    struct MockLlm;
    #[async_trait]
    impl LlmProvider for MockLlm {
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<LlmResponse, aiome_core_contracts::error::AiomeError> {
            Ok(LlmResponse {
                content: "[Knowledge] TDD is essential for quality.".into(),
                stop_reason: StopReason::EndTurn,
                ..Default::default()
            })
        }
        async fn test_connection(&self) -> Result<(), aiome_core_contracts::error::AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "mock"
        }
    }

    /// LLM が常にエラーを返す Mock
    #[derive(Debug)]
    struct FailingLlm;
    #[async_trait]
    impl LlmProvider for FailingLlm {
        async fn complete(
            &self,
            _prompt: &str,
            _system: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            Err(AiomeError::Infrastructure {
                reason: "LLM unavailable".to_string(),
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "failing_mock"
        }
    }

    /// run_distillation_cycle テスト用の Mock DistillationOps
    #[derive(Debug, Default)]
    struct MockDistillationOps {
        skills: Vec<String>,
        raw_karma: Vec<(String, String)>,
        applied: std::sync::Mutex<Vec<(String, String)>>,
    }

    #[async_trait]
    impl DistillationOps for MockDistillationOps {
        async fn fetch_skills_for_distillation(
            &self,
            _threshold: i64,
        ) -> Result<Vec<String>, AiomeError> {
            Ok(self.skills.clone())
        }
        async fn fetch_raw_karma_for_skill(
            &self,
            _skill: &str,
        ) -> Result<Vec<(String, String)>, AiomeError> {
            Ok(self.raw_karma.clone())
        }
        async fn apply_distilled_karma(
            &self,
            skill: &str,
            distilled_lesson: &str,
            _old_karma_ids: &[String],
            _soul_hash: &str,
            _domain: Option<&str>,
            _subtopic: Option<&str>,
            _clone_origin_id: Option<&str>,
        ) -> Result<(), AiomeError> {
            self.applied
                .lock()
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Mutex poisoned: {}", e),
                })?
                .push((skill.to_string(), distilled_lesson.to_string()));
            Ok(())
        }
        async fn store_trajectory_step(&self, _step: TrajectoryStep) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_memory_crystallizer_initialization() -> Result<(), Box<dyn std::error::Error>> {
        let provider = Arc::new(MockLlm);
        // UniversalJobQueue の実体化（インメモリ）を試みる
        let pool = crate::db::DatabasePool::Sqlite(
            sqlx::sqlite::SqlitePoolOptions::new()
                .connect("sqlite::memory:")
                .await?,
        );
        let ts = std::sync::Arc::new(
            crate::job_queue::trajectory_store::SqliteTrajectoryStore::new(pool.clone()),
        );
        if let Ok(jq) = UniversalJobQueue::new(pool.clone(), None, ts).await {
            let semaphore = Arc::new(Semaphore::new(1));
            let slm = Some(Arc::new(SlmBridge::new()));

            let crystallizer = MemoryCrystallizer::new(
                provider,
                Arc::new(jq) as Arc<dyn DistillationOps>,
                semaphore,
                slm,
                None,
                None,
            );

            assert!(crystallizer.slm_bridge.is_some());
        }

        Ok(())
    }

    /// 正常系: スキルが存在し、LLM が成功する場合に結晶化が完了すること
    #[tokio::test]
    async fn test_distillation_cycle_success() {
        let mock_ops = Arc::new(MockDistillationOps {
            skills: vec!["rust_basics".to_string()],
            raw_karma: vec![
                ("k1".to_string(), "Error handling is crucial".to_string()),
                ("k2".to_string(), "Use Result instead of panic".to_string()),
            ],
            ..Default::default()
        });

        let crystallizer = MemoryCrystallizer::new(
            Arc::new(MockLlm),
            mock_ops.clone() as Arc<dyn DistillationOps>,
            Arc::new(Semaphore::new(1)),
            None,
            None,
            None,
        );

        let result = crystallizer.run_distillation_cycle().await;
        assert!(result.is_ok(), "Distillation cycle should succeed");

        let applied = mock_ops.applied.lock().expect("lock");
        assert_eq!(applied.len(), 1, "Should have applied 1 distilled karma");
        assert_eq!(applied[0].0, "rust_basics");
    }

    /// 異常系: LLM エラー時にパニックせずチャンクをスキップすること
    #[tokio::test]
    async fn test_distillation_cycle_llm_failure_skips_chunk() {
        let mock_ops = Arc::new(MockDistillationOps {
            skills: vec!["failing_skill".to_string()],
            raw_karma: vec![("k1".to_string(), "Some lesson".to_string())],
            ..Default::default()
        });

        let crystallizer = MemoryCrystallizer::new(
            Arc::new(FailingLlm),
            mock_ops.clone() as Arc<dyn DistillationOps>,
            Arc::new(Semaphore::new(1)),
            None,
            None,
            None,
        );

        let result = crystallizer.run_distillation_cycle().await;
        assert!(result.is_ok(), "LLM failure should not propagate as error");

        let applied = mock_ops.applied.lock().expect("lock");
        assert!(
            applied.is_empty(),
            "No karma should be applied on LLM failure"
        );
    }

    /// エッジケース: スキルリストが空の場合、何も処理されないこと
    #[tokio::test]
    async fn test_distillation_cycle_no_skills() {
        let mock_ops = Arc::new(MockDistillationOps::default());

        let crystallizer = MemoryCrystallizer::new(
            Arc::new(MockLlm),
            mock_ops as Arc<dyn DistillationOps>,
            Arc::new(Semaphore::new(1)),
            None,
            None,
            None,
        );

        let result = crystallizer.run_distillation_cycle().await;
        assert!(result.is_ok());
    }

    /// エッジケース: セマフォが枯渇している場合、スキルがスキップされること
    #[tokio::test]
    async fn test_distillation_cycle_semaphore_exhausted() {
        let mock_ops = Arc::new(MockDistillationOps {
            skills: vec!["blocked_skill".to_string()],
            raw_karma: vec![("k1".to_string(), "lesson".to_string())],
            ..Default::default()
        });

        // セマフォ容量 0 → try_acquire は常に失敗
        let crystallizer = MemoryCrystallizer::new(
            Arc::new(MockLlm),
            mock_ops.clone() as Arc<dyn DistillationOps>,
            Arc::new(Semaphore::new(0)),
            None,
            None,
            None,
        );

        let result = crystallizer.run_distillation_cycle().await;
        assert!(
            result.is_ok(),
            "Semaphore exhaustion should not cause error"
        );

        let applied = mock_ops.applied.lock().expect("lock");
        assert!(
            applied.is_empty(),
            "No karma should be applied when semaphore is exhausted"
        );
    }
}
