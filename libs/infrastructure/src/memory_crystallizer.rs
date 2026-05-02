/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use crate::belief_consistency_gate::{BeliefCheckResult, BeliefConsistencyGate};
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
}

impl MemoryCrystallizer {
    /// 新しいインスタンスを生成する
    pub fn new(
        provider: Arc<dyn LlmProvider + Send + Sync>,
        ops: Arc<dyn DistillationOps>,
        semaphore: Arc<Semaphore>,
        slm_bridge: Option<Arc<SlmBridge>>,
        belief_gate: Option<Arc<BeliefConsistencyGate>>,
    ) -> Self {
        Self {
            provider,
            ops,
            semaphore,
            slm_bridge,
            belief_gate,
        }
    }

    /// `run_distillation_cycle` を実行する
    pub async fn run_distillation_cycle(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Skill-based Karma Distillation (Consolidating raw experiences)
        // Fetch skills that have 10+ raw karma entries
        let skills = self.ops.fetch_skills_for_distillation(10).await?;
        for skill in skills {
            if let Ok(_permit) = self.semaphore.try_acquire() {
                info!(
                    "💎 [MemoryCrystallizer] Crystallizing karma for skill: {}",
                    skill
                );
                let raw_karma = self.ops.fetch_raw_karma_for_skill(&skill).await?;

                // VULN-63: OOM Prevention - process in batches of 50 to avoid massive string allocation
                for raw_karma_chunk in raw_karma.chunks(50) {
                    let lessons = raw_karma_chunk
                        .iter()
                        .map(|(_, lesson)| format!("- {}", lesson))
                        .collect::<Vec<_>>()
                        .join("\n");

                    let prompt = format!(
                        "以下の技能「{}」に関する生の教訓を抽象化し、本質的な知恵（Fact）に結晶化してください。\n\
                        また、各Factに対して以下のカテゴリのいずれかを割り当ててください：\n\
                        - Preference (ユーザーの好み)\n\
                        - Knowledge (技術的・一般的な知識)\n\
                        - Context (現在の状況・背景)\n\
                        - Behavior (エージェントの振る舞い方)\n\
                        - Goal (達成すべき目標)\n\n\
                        教訓リスト:\n{}\n\n\
                        出力形式: [Category] 内容 の形式で短い箇条書き。日本語で出力せよ。",
                        skill, lessons
                    );

                    match self.provider.complete(&prompt, None).await {
                        Ok(resp) => {
                            let _soul_hash = "v2_fact_categorized";
                            let ids: Vec<String> =
                                raw_karma_chunk.iter().map(|(id, _)| id.clone()).collect();

                            let mut domain = None;

                            // Phase 49: Belief Consistency Gate
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
                                        let step = TrajectoryStep {
                                            job_id: Some(format!("crystallize-{}", skill)),
                                            action: "RequestBeliefRevision".into(),
                                            step_category: StepCategory::Decision,
                                            output: serde_json::to_value(evidence)
                                                .unwrap_or_default(),
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

                            self.ops
                                .apply_distilled_karma(
                                    &skill,
                                    &resp.content,
                                    &ids,
                                    "v1",
                                    None,
                                    domain.as_deref(),
                                    None,
                                )
                                .await?;
                            info!(
                                "✅ [MemoryCrystallizer] Karma crystallized with facts for {} (Batch of {})",
                                skill,
                                ids.len()
                            );
                        }
                        Err(e) => {
                            warn!(
                                "⚠️ [MemoryCrystallizer] Failed to crystallize karma chunk for {}: {:?}",
                                skill, e
                            );
                        }
                    }
                }
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
                reasoning: None,
                metadata: None,
            })
        }
        async fn test_connection(&self) -> Result<(), aiome_core_contracts::error::AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "mock"
        }
    }

    #[tokio::test]
    async fn test_memory_crystallizer_initialization() {
        let provider = Arc::new(MockLlm);
        // UniversalJobQueue の実体化（インメモリ）を試みる
        let pool = crate::db::DatabasePool::Sqlite(
            sqlx::sqlite::SqlitePoolOptions::new()
                .connect("sqlite::memory:")
                .await
                .unwrap(), // allow-anti-pattern
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
            );

            assert!(crystallizer.slm_bridge.is_some());
        }
    }
}
