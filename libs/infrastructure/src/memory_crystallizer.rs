/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use crate::job_queue::UniversalJobQueue;
use crate::job_queue::WatchtowerOps;
use aiome_contracts::llm::LlmProvider;
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
    job_queue: Arc<UniversalJobQueue>,
    semaphore: Arc<Semaphore>,
}

impl MemoryCrystallizer {
    /// 新しいインスタンスを生成する
    pub fn new(
        provider: Arc<dyn LlmProvider + Send + Sync>,
        job_queue: Arc<UniversalJobQueue>,
        semaphore: Arc<Semaphore>,
    ) -> Self {
        Self {
            provider,
            job_queue,
            semaphore,
        }
    }

    /// `run_distillation_cycle` を実行する
    pub async fn run_distillation_cycle(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Skill-based Karma Distillation (Consolidating raw experiences)
        // Fetch skills that have 10+ raw karma entries
        let skills = self.job_queue.do_fetch_skills_for_distillation(10).await?;
        for skill in skills {
            if let Ok(_permit) = self.semaphore.try_acquire() {
                info!(
                    "💎 [MemoryCrystallizer] Crystallizing karma for skill: {}",
                    skill
                );
                let raw_karma = self.job_queue.do_fetch_raw_karma_for_skill(&skill).await?;

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
                            let ids: Vec<String> = raw_karma_chunk.iter().map(|(id, _)| id.clone()).collect();

                            self.job_queue
                                .do_apply_distilled_karma(
                                    &skill,
                                    &resp.content,
                                    &ids,
                                    "v1",
                                    None,
                                    None,
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

// Tests are temporarily disabled during infra consolidation.
