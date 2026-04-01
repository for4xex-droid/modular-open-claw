/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::contracts::ArenaMatch;
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::JobQueue;
use chrono::Utc;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

/// スキルの並列実行と評価を行うアリーナ
pub struct SkillArena {
    provider: Arc<dyn LlmProvider>,
    /// 淘汰から保護されるスキル名のセット (G-25)
    pub protected_skills: HashSet<String>,
}

#[cfg(any(test, debug_assertions))]
impl Default for SkillArena {
    fn default() -> Self {
        Self {
            provider: Arc::new(aiome_core::llm_provider::MockLlmProvider::default()),
            protected_skills: Self::default_protected_skills(),
        }
    }
}

impl SkillArena {
    /// 新しいインスタンスを生成する
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            provider,
            protected_skills: Self::default_protected_skills(),
        }
    }

    /// デフォルトの保護スキルリストを取得 (G-25)
    fn default_protected_skills() -> HashSet<String> {
        let mut set = HashSet::new();
        set.insert("essential_core".to_string());
        set.insert("immune_system".to_string());
        set.insert("skill_arena".to_string());
        set.insert("commerce_engine".to_string());
        set
    }

    /// 二つの異なるスキル（WASM）の出力を比較し、勝利スキルを決定する
    pub async fn match_skill(
        &self,
        skill_a: &str,
        skill_b: &str,
        input: &str,
        jq: &impl JobQueue,
        sm: &crate::skills::WasmSkillManager,
    ) -> Result<Option<String>, AiomeError> {
        info!(
            "⚔️  Arena Match: {} vs {} (topic: {}) using {}",
            skill_a,
            skill_b,
            input,
            self.provider.name()
        );

        // 両方のスキルを実行
        let skill_a_v = crate::skills::VerifiedSkill::promote(skill_a.to_string());
        let skill_b_v = crate::skills::VerifiedSkill::promote(skill_b.to_string());
        let res_a = sm.call_skill(&skill_a_v, "call", input, None).await;
        let res_b = sm.call_skill(&skill_b_v, "call", input, None).await;

        let (out_a, out_b) = match (res_a, res_b) {
            (Ok(a), Ok(b)) => (a, b),
            (Err(e), _) => {
                warn!("❌ Skill A ({}) failed: {}", skill_a, e);
                return Ok(Some(skill_b.to_string())); // Aが落ちたのでBの勝利
            }
            (_, Err(e)) => {
                warn!("❌ Skill B ({}) failed: {}", skill_b, e);
                return Ok(Some(skill_a.to_string())); // Bが落ちたのでAの勝利
            }
        };

        // LLMに審判を依頼
        let judge_preamble = "あなたは『AI進化アリーナ』の公正な審判です。
二つのスキルの出力を比較し、どちらがよりユーザーの意図に忠実で、品質が高いかを判定してください。

【評価基準】
1. 内容の正確性と具体性
2. フォーマットの適切さ
3. エラーが含まれていないか

必ず以下のJSON形式で応答してください：
{
  \"winner\": \"スキル名A または スキル名B\",
  \"reasoning\": \"なぜそのスキルが勝ったのか（一言で）\"
}";

        let judge_prompt = format!(
            "input: {}\n\n--- OUTPUT A ({}): ---\n{}\n\n--- OUTPUT B ({}): ---\n{}",
            input, skill_a, out_a, skill_b, out_b
        );

        let judge_res = self
            .provider
            .complete(&judge_prompt, Some(judge_preamble))
            .await?;

        let json_str = crate::concept_manager::extract_json(&judge_res.content)?;
        let v: serde_json::Value =
            serde_json::from_str(json_str.as_str()).map_err(|e| AiomeError::Infrastructure {
                reason: format!("Judge JSON error: {}", e),
            })?;

        let winner_raw = v["winner"].as_str().unwrap_or("");
        let final_winner = if winner_raw.contains(skill_a) {
            Some(skill_a.to_string())
        } else if winner_raw.contains(skill_b) {
            Some(skill_b.to_string())
        } else {
            None
        };

        let match_record = ArenaMatch {
            id: Uuid::new_v4().to_string(),
            skill_a: skill_a.to_string(),
            skill_b: skill_b.to_string(),
            topic: input.to_string(),
            winner: final_winner.clone(),
            reasoning: v["reasoning"]
                .as_str()
                .unwrap_or("Decision made by autonomous judge.")
                .to_string(),
            created_at: Utc::now().to_rfc3339(),
        };

        if let Some(ref w) = final_winner {
            info!(
                "🏆 Match Winner: {} (Reason: {})",
                w, match_record.reasoning
            );
        } else {
            warn!("🤝 Match result: Draw");
        }

        jq.record_arena_match(&match_record).await?;

        Ok(final_winner)
    }

    /// アリーナの歴史から統計的に弱いスキルを特定し、淘汰（アンインストール）の準備をする
    pub async fn analyze_and_cull(
        &self,
        _jq: &impl JobQueue,
        sm: &crate::skills::WasmSkillManager,
    ) -> Result<Vec<String>, AiomeError> {
        info!("🧬 淘汰アルゴリズム（淘汰プロセス）を実行中...");

        let all_skills = sm.list_skills();
        if all_skills.len() < 5 {
            info!("🧬 インストールされているスキルが少ないため、淘汰をスキップします。");
            return Ok(Vec::new());
        }

        use rand::Rng;
        let mut rng = rand::thread_rng();

        // G-25: 保護されているスキルは淘汰候補から除外する
        if rng.gen_bool(0.1) {
            let candidates: Vec<&String> = all_skills
                .iter()
                .filter(|s| !self.protected_skills.contains(*s))
                .collect();

            if candidates.is_empty() {
                return Ok(Vec::new());
            }

            let victim = candidates[rng.gen_range(0..candidates.len())];
            warn!("⚠️ 競技の歴史を無視したランダム淘汰により、'{}' がアンインストールの候補に挙がりました。", victim);
            return Ok(vec![victim.clone()]);
        }

        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_arena_default_protection() {
        let arena = SkillArena::default();
        assert!(arena.protected_skills.contains("essential_core"));
        assert!(arena.protected_skills.contains("immune_system"));
        assert!(arena.protected_skills.contains("skill_arena"));
        assert!(arena.protected_skills.contains("commerce_engine"));
    }
}
