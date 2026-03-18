/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::JobQueue;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tracing::info;

pub struct SoulMutator {
    provider: Arc<dyn LlmProvider>,
    prosecutor_provider: Option<Arc<dyn LlmProvider>>,
    workspace_dir: PathBuf,
}

impl SoulMutator {
    /// 最小 Drift 閾値 (レベル1)
    const MIN_DRIFT_THRESHOLD: f64 = 0.30;
    /// 最大 Drift 閾値 (経験を積んだエージェント)
    const MAX_DRIFT_THRESHOLD: f64 = 0.55;
    pub fn new(provider: Arc<dyn LlmProvider>, workspace_dir: PathBuf) -> Self {
        Self {
            provider,
            prosecutor_provider: None,
            workspace_dir,
        }
    }

    pub fn with_prosecutor(mut self, prosecutor: Arc<dyn LlmProvider>) -> Self {
        self.prosecutor_provider = Some(prosecutor);
        self
    }

    /// 魂の変異（Transmigration）を試行する。
    pub async fn transmute(
        &self,
        job_queue: &dyn JobQueue,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "🧬 [SoulMutator] Starting Transmigration phase using {}...",
            self.provider.name()
        );

        let soul_filename = "SOUL.md";
        let evolving_soul_filename = "EVOLVING_SOUL.md";

        let soul_path = self.workspace_dir.join(soul_filename);
        let evolving_soul_path = self.workspace_dir.join(evolving_soul_filename);

        if !soul_path.exists() || !evolving_soul_path.exists() {
            return Err(format!(
                "{} or {} not found at {:?}. Transmutation impossible.",
                soul_filename, evolving_soul_filename, self.workspace_dir
            )
            .into());
        }

        // 1. Read Current Soul State
        let master_soul = fs::read_to_string(&soul_path).await?;
        let current_evolving_soul = fs::read_to_string(&evolving_soul_path).await?;

        // 2. Collect High-Karma Lessons
        let top_karmas = job_queue
            .fetch_all_karma(10)
            .await
            .map_err(|e| format!("Failed to fetch karma: {}", e))?;

        // Filter Technical/Creative high-weight karma manually as a proxy for fetch_relevant_karma
        let mut lessons = Vec::new();
        for k in top_karmas {
            if let Some(lesson) = k["lesson"].as_str() {
                lessons.push(format!("- {}", lesson));
            }
        }

        if lessons.is_empty() {
            info!("🧬 [SoulMutator] Not enough high-quality Karma accumulated yet. Skipping mutation.");
            return Ok(false);
        }

        // 3. Mutation Generation
        let preamble = format!(
            "AI の魂の進化プロセスの継続。EVOLVING_SOUL.md を更新せよ。\n\nユーザーソウル (核):\n{}\n\n蓄積された教訓:\n{}",
            master_soul,
            lessons.join("\n")
        );

        let prompt_text = format!("現在のあなたの進化状況を反映した、最新の EVOLVING_SOUL.md を生成せよ。現状を否定せず、教訓を取り入れて拡張すること。\n\n現在の内容:\n{}", current_evolving_soul);

        let resp = self
            .provider
            .complete(&prompt_text, Some(&preamble))
            .await
            .map_err(|e| format!("Mutation LLM failed: {}", e))?;

        let mut new_soul_content = resp.content;
        if new_soul_content.starts_with("```markdown") {
            new_soul_content = new_soul_content
                .trim_start_matches("```markdown")
                .trim()
                .to_string();
        } else if new_soul_content.starts_with("```") {
            new_soul_content = new_soul_content
                .trim_start_matches("```")
                .trim()
                .to_string();
        }
        if new_soul_content.ends_with("```") {
            new_soul_content = new_soul_content.trim_end_matches("```").trim().to_string();
        }

        let old_hash = self.compute_hash(&current_evolving_soul);
        let new_hash = self.compute_hash(&new_soul_content);

        if old_hash == new_hash {
            info!("🧬 [SoulMutator] Mutation resulted in no change. Staying in current state.");
            return Ok(false);
        }

        // --- Soul Drift Guard (Adaptive Intelligence v1.0) ---
        let stats = job_queue.get_agent_stats().await?;
        let drift = self.measure_drift(&master_soul, &new_soul_content);
        let threshold = self.get_adaptive_threshold(stats.level);

        if drift > threshold {
            use tracing::warn;
            warn!(
                "🛡️ [SoulDriftGuard] Mutation Drift {:.2} exceeds Level {} threshold {:.2}. Blocking transmute.",
                drift, stats.level, threshold
            );
            let _ = job_queue
                .record_evolution_event(
                    stats.level,
                    "DriftBlocked",
                    &format!(
                        "Transmute drift {:.2} > threshold {:.2}. Evolution protected.",
                        drift, threshold
                    ),
                    None,
                    None,
                )
                .await;
            return Ok(false);
        }

        // 4. Verification: Heterogeneous Dual-LLM Validator
        if let Some(prosecutor) = &self.prosecutor_provider {
            use aiome_core::traits::ConstitutionalValidator;
            let validator =
                crate::validator::DefaultConstitutionalValidator::new(prosecutor.clone());

            info!(
                "⚖️ [SoulMutator] Executing Constitutional Check via Prosecutor {}...",
                prosecutor.name()
            );
            validator
                .verify_constitutional(&new_soul_content, &master_soul)
                .await?;
        }

        info!(
            "🧬 [SoulMutator] Mutation detected and verified. New Hash: {}",
            new_hash
        );

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_path = evolving_soul_path.with_extension(format!("bak.{}", timestamp));
        let _ = fs::copy(&evolving_soul_path, &backup_path).await;

        // 5. Commit Mutation
        fs::write(&evolving_soul_path, &new_soul_content)
            .await
            .map_err(|e| format!("Failed to write EVOLVING_SOUL.md: {}", e))?;

        // 6. Record in JobQueue & Evolution Chronicle
        let stats = job_queue.get_agent_stats().await?;
        let _ = job_queue
            .record_soul_mutation(
                &old_hash,
                &new_hash,
                "Autonomous Evolution via Samsara Engine",
            )
            .await;
        let _ = job_queue
            .record_evolution_event(
                stats.level,
                "SoulMutation",
                &format!(
                    "Soul mutated from {} to {}. Reason: Autonomous Evolution.",
                    old_hash, new_hash
                ),
                None,
                None,
            )
            .await;

        Ok(true)
    }

    /// レベルアップに伴う戦術拡張（Behavioral Shift）を行う。
    pub async fn evolve_tactics(
        &self,
        job_queue: &dyn JobQueue,
        old_level: i32,
        new_level: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "🌟 [SoulMutator] Level Up detected ({} -> {}). Initiating Behavioral Shift...",
            old_level, new_level
        );

        let soul_filename = "SOUL.md";
        let evolving_soul_filename = "EVOLVING_SOUL.md";

        let soul_path = self.workspace_dir.join(soul_filename);
        let evolving_soul_path = self.workspace_dir.join(evolving_soul_filename);

        if !soul_path.exists() || !evolving_soul_path.exists() {
            return Err(format!(
                "SOUL.md or EVOLVING_SOUL.md not found at {:?}.",
                self.workspace_dir
            )
            .into());
        }

        let master_soul = fs::read_to_string(&soul_path).await?;

        // 1. Generate New Tactics
        let preamble = format!(
            "あなたはAiome OSの進化エンジンです。レベルアップに伴う行動変容(Behavioral Shift)を計画してください。\n\n現在のレベル: {}\n新しいレベル: {}\n\n憲法 (核):\n{}",
            old_level,
            new_level,
            master_soul
        );

        let prompt = format!(
            "レベルが {} に到達しました。現在の能力を最大限に活かし、より自律的、かつ協調的な行動をとるための「新しい行動方針」を 1つ提案してください。\n\
            出力フォーマット:\n### Level {} Shift: [方針名]\n[具体的な方針内容 (2-3文)]",
            new_level,
            new_level
        );

        let resp = self.provider.complete(&prompt, Some(&preamble)).await?;
        let proposal = resp.content;

        // --- Soul Drift Guard (Adaptive Intelligence v1.0) ---
        let current_evolving_soul = fs::read_to_string(&evolving_soul_path).await?;
        let candidate_soul = format!("{}\n\n{}", current_evolving_soul, proposal);
        let drift = self.measure_drift(&master_soul, &candidate_soul);
        let threshold = self.get_adaptive_threshold(new_level);

        if drift > threshold {
            use tracing::warn;
            warn!(
                "🛡️ [SoulDriftGuard] Tactical Drift {:.2} exceeds Level {} threshold {:.2}. Blocking evolution.",
                drift, new_level, threshold
            );
            let _ = job_queue
                .record_evolution_event(
                    new_level,
                    "DriftBlocked",
                    &format!(
                        "Tactical shift drift {:.2} > threshold {:.2}. Personality protected.",
                        drift, threshold
                    ),
                    None,
                    None,
                )
                .await;
            return Ok(());
        }

        // 2. Verification
        if let Some(prosecutor) = &self.prosecutor_provider {
            use aiome_core::traits::ConstitutionalValidator;
            let validator =
                crate::validator::DefaultConstitutionalValidator::new(prosecutor.clone());
            info!("⚖️ [SoulMutator] Verifying Level Up tactics...");
            validator
                .verify_constitutional(&proposal, &master_soul)
                .await?;
        }

        // 3. Append to EVOLVING_SOUL.md
        let mut content = fs::read_to_string(&evolving_soul_path).await?;
        content.push_str("\n\n");
        content.push_str(&proposal);
        content.push_str(&format!(
            "\n*(Reflected via Samsara Level Up at {})\n",
            chrono::Utc::now().to_rfc3339()
        ));

        fs::write(&evolving_soul_path, &content).await?;

        let _ = job_queue
            .record_soul_mutation(
                "LEVEL_UP",
                &format!("LV{}", new_level),
                "Level Up Behavioral Shift",
            )
            .await;
        let _ = job_queue
            .record_evolution_event(new_level, "TacticalShift", &proposal, None, None)
            .await;

        info!(
            "✅ [SoulMutator] Behavioral Shift completed for Level {}.",
            new_level
        );
        Ok(())
    }

    /// 現在の AI の中心的な人格定義を取得する
    pub async fn get_active_prompt(
        &self,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let paths = [
            self.workspace_dir.join("EVOLVING_SOUL.md"),
            self.workspace_dir.join("SOUL.md"),
        ];
        for p in paths {
            if p.exists() {
                return Ok(fs::read_to_string(p).await?);
            }
        }
        Ok("An autonomous AI system.".to_string())
    }

    fn compute_hash(&self, content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    /// Adaptive Threshold: レベルが上がるほど、人格の変位許容度（自律性）を拡大する。
    fn get_adaptive_threshold(&self, level: i32) -> f64 {
        let base = Self::MIN_DRIFT_THRESHOLD;
        let growth = (level as f64 * 0.025).min(Self::MAX_DRIFT_THRESHOLD - base);
        base + growth
    }

    /// Measure Drift via character-level N-gram Jaccard Distance
    /// Sprint 5-A: Japanese support — split_whitespace から文字レベル N-gram に変更
    /// 0.0 = 同一, 1.0 = 全く異なる
    fn measure_drift(&self, original: &str, mutated: &str) -> f64 {
        if original.is_empty() || mutated.is_empty() {
            return 1.0;
        }

        use std::collections::HashSet;

        // Character-level trigrams for language-agnostic comparison (supports Japanese, CJK, etc.)
        let n = 3;

        let get_ngrams = |text: &str, n: usize| -> HashSet<String> {
            let chars: Vec<char> = text.chars().collect();
            if chars.len() < n {
                // For very short text, use the text itself as a single n-gram
                let mut set = HashSet::new();
                set.insert(text.to_string());
                return set;
            }
            chars
                .windows(n)
                .map(|window| window.iter().collect())
                .collect()
        };

        let ngrams_a = get_ngrams(original, n);
        let ngrams_b = get_ngrams(mutated, n);

        if ngrams_a.is_empty() && ngrams_b.is_empty() {
            return 0.0;
        }
        if ngrams_a.is_empty() || ngrams_b.is_empty() {
            return 1.0;
        }

        let intersection_count = ngrams_a.intersection(&ngrams_b).count();
        let union_count = ngrams_a.union(&ngrams_b).count();

        if union_count == 0 {
            return 0.0;
        }

        1.0 - (intersection_count as f64 / union_count as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::biome::BiomeMessage;
    use aiome_core::contracts::SamsaraEvent;
    use aiome_core::error::AiomeError;
    use aiome_core::llm_provider::LlmProvider;
    use aiome_core::traits::{Job, JobQueue, KarmaSearchResult, SnsMetricsRecord};
    use async_trait::async_trait;
    use serde_json::json;
    use shared::watchtower::AgentStats;
    use uuid::Uuid;

    #[derive(Debug)]
    struct MockLlm {
        mutation_response: String,
    }

    #[async_trait]
    impl LlmProvider for MockLlm {
        fn name(&self) -> &str {
            "mock-llm"
        }
        async fn complete(
            &self,
            _p: &str,
            _pre: Option<&str>,
        ) -> Result<aiome_core::llm_provider::LlmResponse, AiomeError> {
            Ok(aiome_core::llm_provider::LlmResponse {
                content: self.mutation_response.clone(),
                stop_reason: aiome_core::llm_provider::StopReason::EndTurn,
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    struct MockJobQueue {
        karma_lessons: Vec<String>,
        stats: AgentStats,
    }

    #[async_trait]
    impl JobQueue for MockJobQueue {
        async fn enqueue(
            &self,
            _category: &str,
            _topic: &str,
            _style: &str,
            _karma_directives: Option<&str>,
            _permission_manifest: Option<aiome_core::security::PermissionManifest>,
            _agent_id: Option<uuid::Uuid>,
        ) -> Result<String, AiomeError> {
            Ok(Uuid::new_v4().to_string())
        }
        async fn dequeue(&self, _categories: &[&str]) -> Result<Option<Job>, AiomeError> {
            Ok(None)
        }
        async fn fetch_all_karma(&self, _limit: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
            Ok(self
                .karma_lessons
                .iter()
                .map(|l| json!({"lesson": l}))
                .collect())
        }
        async fn get_agent_stats(&self) -> Result<AgentStats, AiomeError> {
            Ok(self.stats.clone())
        }
        async fn record_evolution_event(
            &self,
            _l: i32,
            _t: &str,
            _d: &str,
            _k: Option<&str>,
            _r: Option<&str>,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn record_soul_mutation(
            &self,
            _o: &str,
            _n: &str,
            _r: &str,
        ) -> Result<(), AiomeError> {
            Ok(())
        }

        async fn fetch_job(&self, _: &str) -> Result<Option<Job>, AiomeError> {
            Ok(None)
        }
        async fn complete_job(&self, _: &str, _: Option<&str>) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fail_job(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_relevant_karma(
            &self,
            _: &str,
            _: &str,
            _: i64,
            _: &str,
        ) -> Result<KarmaSearchResult, AiomeError> {
            Ok(KarmaSearchResult {
                entries: vec![],
                is_ood: false,
                max_score: 0.0,
            })
        }
        async fn store_karma(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn adjust_karma_weight(&self, _: &str, _: i32) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn karma_decay_sweep(&self) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn reclaim_zombie_jobs(&self, _: i64) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn set_creative_rating(&self, _: &str, _: i32) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn heartbeat_pulse(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn store_execution_log(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_undistilled_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
        }
        async fn mark_karma_extracted(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }

        async fn fetch_job_retry_count(&self, _: &str) -> Result<i64, AiomeError> {
            Ok(0)
        }
        async fn increment_job_retry_count(&self, _: &str) -> Result<bool, AiomeError> {
            Ok(true)
        }
        async fn reset_job_retry_count(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn store_immune_rule(
            &self,
            _: &aiome_core::contracts::ImmuneRule,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn delete_immune_rule(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_active_immune_rules(
            &self,
        ) -> Result<Vec<aiome_core::contracts::ImmuneRule>, AiomeError> {
            Ok(vec![])
        }
        async fn record_arena_match(
            &self,
            _: &aiome_core::contracts::ArenaMatch,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn export_federated_data(
            &self,
            _: Option<&str>,
        ) -> Result<
            (
                Vec<aiome_core::contracts::FederatedKarma>,
                Vec<aiome_core::contracts::ImmuneRule>,
                Vec<aiome_core::contracts::ArenaMatch>,
            ),
            AiomeError,
        > {
            Ok((vec![], vec![], vec![]))
        }
        async fn import_federated_data(
            &self,
            _: Vec<aiome_core::contracts::FederatedKarma>,
            _: Vec<aiome_core::contracts::ImmuneRule>,
            _: Vec<aiome_core::contracts::ArenaMatch>,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn get_peer_sync_time(&self, _: &str) -> Result<Option<String>, AiomeError> {
            Ok(None)
        }
        async fn update_peer_sync_time(&self, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn get_immune_rules(
            &self,
        ) -> Result<Vec<aiome_core::contracts::ImmuneRule>, AiomeError> {
            Ok(vec![])
        }
        async fn get_node_id(&self) -> Result<String, AiomeError> {
            Ok("mock-node".into())
        }
        async fn sign_swarm_payload(&self, _: &str) -> Result<String, AiomeError> {
            Ok("mock-sig".into())
        }
        async fn tick_local_clock(&self) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn sync_local_clock(&self, _: u64) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn storage_gc(&self, _: f64) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn store_chat_message(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_chat_history(
            &self,
            _: &str,
            _: i64,
        ) -> Result<Vec<serde_json::Value>, AiomeError> {
            Ok(vec![])
        }
        async fn store_expression(
            &self,
            _: &aiome_core::expression::Expression,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_expressions(
            &self,
            _: i64,
        ) -> Result<Vec<aiome_core::expression::Expression>, AiomeError> {
            Ok(vec![])
        }
        async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
            Ok(false)
        }
        async fn set_auto_expression_enabled(&self, _: bool) -> Result<(), AiomeError> {
            Ok(())
        }

        async fn purge_old_jobs(&self, _: i64) -> Result<u64, AiomeError> {
            Ok(0)
        }
        async fn link_sns_data(&self, _: &str, _: &str, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_jobs_for_evaluation(&self, _: i64, _: i64) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
        }
        async fn record_sns_metrics(
            &self,
            _: &str,
            _: i64,
            _: i64,
            _: i64,
            _: i64,
            _: Option<&str>,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_pending_evaluations(
            &self,
            _: i64,
        ) -> Result<Vec<SnsMetricsRecord>, AiomeError> {
            Ok(vec![])
        }
        async fn apply_final_verdict(
            &self,
            _: i64,
            _: aiome_core::contracts::OracleVerdict,
            _: &str,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_recent_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
        }
        async fn add_resonance(&self, _: i32) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn add_tech_exp(&self, _: i32) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn add_creativity(&self, _: i32) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn sync_samsara_level(&self) -> Result<Option<SamsaraEvent>, AiomeError> {
            Ok(None)
        }
        async fn get_biome_topic_status(
            &self,
            _: &str,
        ) -> Result<Option<(i32, Option<String>)>, AiomeError> {
            Ok(None)
        }
        async fn advance_biome_turn(&self, _: &str, _: i64) -> Result<i32, AiomeError> {
            Ok(0)
        }
        async fn fetch_biome_messages(
            &self,
            _: &str,
            _: i64,
        ) -> Result<Vec<serde_json::Value>, AiomeError> {
            Ok(vec![])
        }
        async fn store_biome_message(&self, _: &BiomeMessage) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn update_biome_reputation(&self, _: &str, _: f64) -> Result<f64, AiomeError> {
            Ok(0.0)
        }
        async fn archive_biome_topic(&self, _: &str) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn fetch_evolution_history(
            &self,
            _: i64,
        ) -> Result<Vec<serde_json::Value>, AiomeError> {
            Ok(vec![])
        }
        async fn get_pending_job_count(&self) -> Result<i64, AiomeError> {
            Ok(0)
        }
        async fn get_job_count_since(
            &self,
            _: chrono::DateTime<chrono::Utc>,
        ) -> Result<i64, AiomeError> {
            Ok(0)
        }
        async fn fetch_top_performing_jobs(&self, _: i64) -> Result<Vec<Job>, AiomeError> {
            Ok(vec![])
        }
        async fn fetch_unincorporated_karma(
            &self,
            _: i64,
            _: &str,
        ) -> Result<Vec<serde_json::Value>, AiomeError> {
            Ok(vec![])
        }
        async fn mark_karma_as_incorporated(
            &self,
            _: Vec<String>,
            _: &str,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn get_system_agent_id(&self) -> Result<uuid::Uuid, AiomeError> {
            Ok(uuid::Uuid::new_v4())
        }
    }

    #[tokio::test]
    async fn test_soul_transmute_success() {
        let temp_dir = PathBuf::from("/tmp/soul_test_success");
        let _ = fs::remove_dir_all(&temp_dir).await;
        let _ = fs::create_dir_all(&temp_dir).await;

        let base = "one two three four five six seven eight nine ten";
        let evolving = "one two three four five six seven eight nine ten";
        fs::write(temp_dir.join("SOUL.md"), base).await.unwrap();
        fs::write(temp_dir.join("EVOLVING_SOUL.md"), evolving)
            .await
            .unwrap();

        let llm = Arc::new(MockLlm {
            mutation_response: "one two three four five six seven eight nine ten eleven"
                .to_string(),
        });
        let mutator = SoulMutator::new(llm, temp_dir.clone());
        let jq = MockJobQueue {
            karma_lessons: vec!["Learned that empathy matters.".into()],
            stats: AgentStats {
                level: 1,
                exp: 100,
                resonance: 5,
                creativity: 5,
                fatigue: 0,
            },
        };

        let res = mutator.transmute(&jq).await;
        assert!(res.is_ok(), "Transmute failed: {:?}", res.err());
        assert!(res.unwrap(), "Should have mutated (drift check failed?)");

        let new_evolving = fs::read_to_string(temp_dir.join("EVOLVING_SOUL.md"))
            .await
            .unwrap();
        assert!(new_evolving.contains("eleven"));
    }

    #[tokio::test]
    async fn test_soul_drift_protection() {
        let temp_dir = PathBuf::from("/tmp/soul_test_drift");
        let _ = fs::remove_dir_all(&temp_dir).await;
        let _ = fs::create_dir_all(&temp_dir).await;
        fs::write(temp_dir.join("SOUL.md"), "A B C D E")
            .await
            .unwrap();
        fs::write(temp_dir.join("EVOLVING_SOUL.md"), "A B C")
            .await
            .unwrap();

        // Mutate to something totally different to trigger drift
        let llm = Arc::new(MockLlm {
            mutation_response: "X Y Z W Q R S T".to_string(),
        });
        let mutator = SoulMutator::new(llm, temp_dir.clone());
        let jq = MockJobQueue {
            karma_lessons: vec!["Chaos is good.".into()],
            stats: AgentStats {
                level: 1,
                exp: 100,
                resonance: 5,
                creativity: 5,
                fatigue: 0,
            },
        };

        let res = mutator.transmute(&jq).await;
        assert!(res.is_ok());
        assert!(!res.unwrap()); // Drift should block transmute
    }
}
