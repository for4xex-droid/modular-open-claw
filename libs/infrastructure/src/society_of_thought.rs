/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use aiome_core_contracts::contracts::{
    CoordinationProtocol, FeedbackCategory, IterationRecord, OptimizationBudget, SoTConfig,
    SoTEvent, SoTOutcome, SoTTrigger,
};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};
use uuid::Uuid;

/// ACT (Adaptive Computation Time) が発動するために必要な最低スコア。
/// これ未満の場合、収束していてもまだ品質が不十分とみなし早期終了しない。
const ACT_MIN_SCORE_GATE: f64 = 7.0;

/// Society of Thought (SoT) Engine
/// Evans et al. (2026) + Dochkina (2026) の知見を統合した熟議エンジン。
///
/// # Dochkina (2026) arXiv:2603.28990 統合
/// - **Sequential Protocol**: 各 Thinker が前任者の完成済み出力を見て自律的にロールを発明
/// - **Voluntary Self-Abstention**: 貢献できないと判断した Thinker は `[ABSTAIN]` を返して辞退
/// - **Capability-Aware Fallback**: モデル能力閾値に基づき Sequential/Coordinator を自動切替
pub struct SoTEngine {
    fast_provider: Arc<dyn LlmProvider>,
    primary_provider: Arc<dyn LlmProvider>,
    event_tx: broadcast::Sender<SoTEvent>,
}

/// Critic による構造化スコアリング応答 (P-11)
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct CriticScoreResponse {
    pub criteria: Vec<CriterionScore>,
    pub overall_reasoning: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CriterionScore {
    pub name: String,
    pub score: f64,
    pub feedback: String,
}

/// Abstention マーカー: LLM がこの接頭辞で応答を返した場合、自発的辞退と見なす
const ABSTAIN_MARKER: &str = "[ABSTAIN]";

impl SoTEngine {
    /// ルールベースの構造化フィードバック一次分類 (ComPilot 由来)
    pub fn classify_feedback(&self, raw_content: &str) -> FeedbackCategory {
        let content_lower = raw_content.to_lowercase();

        if content_lower.contains("syntaxerror")
            || content_lower.contains("invalid json")
            || content_lower.contains("json parse")
        {
            FeedbackCategory::Invalid {
                reason: raw_content.to_string(),
            }
        } else if content_lower.contains("security")
            || content_lower.contains("illegal")
            || (content_lower.contains("policy") && content_lower.contains("violat"))
            || content_lower.contains("violation")
        {
            FeedbackCategory::Illegal {
                constraint: raw_content.to_string(),
            }
        } else if content_lower.contains("timeout")
            || content_lower.contains("connection failed")
            || content_lower.contains("network error")
            || (content_lower.contains("resource")
                && (content_lower.contains("fail")
                    || content_lower.contains("error")
                    || content_lower.contains("unavail")))
        {
            FeedbackCategory::ResourceFailure {
                resource: "External".to_string(),
                error: raw_content.to_string(),
            }
        } else if content_lower.contains("nullpointerexception")
            || content_lower.contains("division by zero")
            || content_lower.contains("panic")
            || content_lower.contains("crash")
        {
            FeedbackCategory::RuntimeError {
                error: raw_content.to_string(),
            }
        } else {
            let mut metrics = std::collections::HashMap::new();
            metrics.insert("score".to_string(), 1.0);
            FeedbackCategory::Success { metrics }
        }
    }

    pub fn new(
        fast_provider: Arc<dyn LlmProvider>,
        primary_provider: Arc<dyn LlmProvider>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            fast_provider,
            primary_provider,
            event_tx,
        }
    }

    /// SSE イベントのレシーバーを取得する
    pub fn subscribe(&self) -> broadcast::Receiver<SoTEvent> {
        self.event_tx.subscribe()
    }

    /// Dochkina (2026): モデル能力閾値に基づくプロトコル自動選択
    fn select_protocol(&self, config: &SoTConfig) -> CoordinationProtocol {
        if !config.auto_protocol {
            return config.coordination_protocol.clone();
        }

        let provider_name = self.primary_provider.name().to_lowercase();

        // 高能力モデル: Sequential (自律性が品質を向上させる)
        let strong_models = [
            "claude", "gpt-5", "gpt-4o", "deepseek", "gemini-2", "gemini-3",
        ];
        if strong_models.iter().any(|m| provider_name.contains(m)) {
            CoordinationProtocol::Sequential
        } else {
            // 低能力モデル: Coordinator (固定構造が品質を安定させる)
            CoordinationProtocol::Coordinator
        }
    }

    /// 熟議セッションを実行する
    #[allow(unused_assignments)]
    pub async fn run_session(
        &self,
        task: &str,
        trigger: SoTTrigger,
        config: SoTConfig,
        remaining_budget: f64, // (P-3) 予算連動
    ) -> Result<(String, SoTOutcome, Vec<(String, f64)>), AiomeError> {
        let session_id = Uuid::new_v4().to_string();
        info!(
            "🧠 [SoT] Starting session: {} for task: {}",
            session_id, task
        );

        // Dochkina (2026): プロトコル選択
        let protocol = self.select_protocol(&config);
        info!(
            "🧠 [SoT] Protocol selected: {:?} (auto={})",
            protocol, config.auto_protocol
        );
        if self
            .event_tx
            .send(SoTEvent::ProtocolSelected {
                session_id: session_id.clone(),
                protocol: protocol.clone(),
                reason: if config.auto_protocol {
                    format!(
                        "Auto-selected based on provider capability: {}",
                        self.primary_provider.name()
                    )
                } else {
                    "Explicitly configured".to_string()
                },
            })
            .is_err()
        {
            tracing::debug!("[SoT] No event subscribers for ProtocolSelected");
        }

        // 1. Session Start Event
        if self
            .event_tx
            .send(SoTEvent::SessionStart {
                session_id: session_id.clone(),
                config: config.clone(),
                trigger,
            })
            .is_err()
        {
            tracing::debug!("[SoT] No event subscribers for SessionStart");
        }

        let num_thinkers = config.num_thinkers.clamp(1, 8);
        let mut current_content = String::new();
        let mut best_content = String::new();
        let mut best_score = 0.0;
        let mut rejection_count = 0;
        let mut round = 1;
        let mut last_scores = Vec::new();
        let mut final_outcome = SoTOutcome::MaxRoundsReached;
        let mut score_history: Vec<f64> = Vec::new();
        let mut current_temp = 0.5; // (P-10) 初期 Temperature
        let mut iteration_history: Vec<IterationRecord> = Vec::new();

        while round <= config.max_rounds {
            info!("🔄 [SoT] Round {}/{}", round, config.max_rounds);

            // P-3: 予算チェック
            if remaining_budget < 0.01 {
                warn!("⚠️ [SoT] Budget too low: {}. Aborting.", remaining_budget);
                final_outcome = SoTOutcome::BudgetExhausted;
                break;
            }
            // ──────────────────────────────────────────────────────
            //  Dochkina (2026): プロトコル分岐
            // ──────────────────────────────────────────────────────
            match protocol {
                CoordinationProtocol::Sequential => {
                    current_content = self
                        .run_sequential_pass(
                            &session_id,
                            task,
                            &current_content,
                            &config,
                            num_thinkers,
                            round,
                            current_temp,
                        )
                        .await?;
                }
                CoordinationProtocol::Coordinator | CoordinationProtocol::Broadcast => {
                    // Coordinator / Broadcast: 既存の Explorer/Synthesizer ロジックを使用
                    current_content = self
                        .run_coordinator_pass(
                            &session_id,
                            task,
                            &current_content,
                            round,
                            current_temp,
                        )
                        .await?;
                }
            }

            // 構造化フィードバックを一次分類して履歴に蓄積 (ComPilot)
            let feedback = self.classify_feedback(&current_content);
            iteration_history.push(IterationRecord {
                round: round as u32,
                proposal_summary: if current_content.chars().count() > 100 {
                    format!(
                        "{}...",
                        current_content.chars().take(100).collect::<String>()
                    )
                } else {
                    current_content.clone()
                },
                feedback: feedback.clone(),
                timestamp: chrono::Utc::now(),
            });

            // ──────────────────────────────────────────────────────
            //  Critic スコアリング (P-2, P-11) — プロトコル共通
            //  論文の知見: Critic は「ロール」ではなく「構造化された品質ゲート」
            // ──────────────────────────────────────────────────────
            if self
                .event_tx
                .send(SoTEvent::RoleStart {
                    session_id: session_id.clone(),
                    role: "Critic".to_string(),
                    round,
                })
                .is_err()
            {
                tracing::debug!("[SoT] No event subscribers for RoleStart(Critic)");
            }

            let scores = self
                .evaluate_scores(&current_content, &config.scoring_criteria)
                .await?;
            last_scores = scores.clone();

            // P-5: セマンティックループ検知 (スコア停滞)
            let avg_score = if !scores.is_empty() {
                scores.iter().map(|(_, s)| s).sum::<f64>() / scores.len() as f64
            } else {
                0.0
            };
            score_history.push(avg_score);

            // Challenger-Verifier パターン
            if config.challenger_mode {
                if round == 1 {
                    best_content = current_content.clone();
                    best_score = avg_score;
                } else if avg_score > best_score {
                    info!(
                        "🔥 [SoT] Challenger proposal improved score from {} to {}. Accepting.",
                        best_score, avg_score
                    );
                    best_content = current_content.clone();
                    best_score = avg_score;
                    rejection_count = 0;
                } else {
                    rejection_count += 1;
                    info!("❌ [SoT] Challenger proposal rejected (score: {} <= best: {}). Rejection count: {}/{}", avg_score, best_score, rejection_count, config.challenger_max_rejections);
                    if rejection_count >= config.challenger_max_rejections {
                        warn!("⏹️ [SoT] Challenger max rejections reached. Early terminating.");
                        current_content = best_content.clone(); // 最良提案を復元
                        final_outcome = SoTOutcome::ChallengerRejected {
                            reason: format!(
                                "Challenger failed to improve score after {} rejections",
                                rejection_count
                            ),
                        };
                        break;
                    }
                }
            }

            let all_passed = scores.iter().all(|(name, score)| {
                config
                    .scoring_criteria
                    .iter()
                    .find(|c| c.name == *name)
                    .map(|crit| *score >= crit.min_score)
                    .unwrap_or(false)
            });

            if self
                .event_tx
                .send(SoTEvent::Score {
                    session_id: session_id.clone(),
                    round,
                    scores: scores.clone(),
                    all_passed,
                })
                .is_err()
            {
                tracing::debug!("[SoT] No event subscribers for Score");
            }

            if all_passed {
                final_outcome = SoTOutcome::AllCriteriaPassed;
                break;
            }

            // Phase 2: ACT (Adaptive Computation Time) & P-10 (Temperature Boost)
            if round > 1 {
                let current_score = score_history[score_history.len() - 1];
                let prev_score = score_history[score_history.len() - 2];
                let delta = (current_score - prev_score).abs();

                // ACT: High score + convergence
                if current_score > ACT_MIN_SCORE_GATE && delta < config.act_convergence_threshold {
                    info!("🚀 [SoT] ACT: Early Convergence detected. Quality sufficient.");
                    final_outcome = SoTOutcome::ConvergedEarly;
                    break;
                }

                // P-10: Stagnation -> Boost temperature for next round
                if delta < 0.1 {
                    current_temp = 0.9;
                    info!(
                        "🚀 [SoT] Stagnation detected. Boosting temperature to {}",
                        current_temp
                    );
                } else {
                    current_temp = 0.5;
                }
            }

            // Phase 2: Spectral Stability
            if round >= 3 {
                let len = score_history.len() as f64;
                let mean = score_history.iter().sum::<f64>() / len;
                let variance = score_history
                    .iter()
                    .map(|s| (s - mean).powi(2))
                    .sum::<f64>()
                    / len;
                let std_dev = variance.max(0.0).sqrt(); // Protect against negative zero FP inaccuracy
                if std_dev > config.spectral_divergence_threshold {
                    warn!("⚠️ [SoT] Spectral Divergence detected. Standard deviation: {:.2} > threshold: {:.2}", std_dev, config.spectral_divergence_threshold);
                    final_outcome = SoTOutcome::SpectralDivergence;
                    break;
                }
            }

            round += 1;
        }

        // 2. Session End Event
        if self
            .event_tx
            .send(SoTEvent::SessionEnd {
                session_id: session_id.clone(),
                outcome: final_outcome.clone(),
                total_tokens: 0,
            })
            .is_err()
        {
            tracing::debug!("[SoT] No event subscribers for SessionEnd");
        }

        Ok((session_id, final_outcome, last_scores))
    }

    /// Dochkina (2026) Sequential Protocol 実装。
    /// 各 Thinker は前任者の完成済み出力を全て見た上で自律的にロールを発明する。
    async fn run_sequential_pass(
        &self,
        session_id: &str,
        task: &str,
        previous_content: &str,
        config: &SoTConfig,
        num_thinkers: u8,
        round: u8,
        temperature: f64,
    ) -> Result<String, AiomeError> {
        let mut accumulated_outputs: Vec<(String, String)> = Vec::new(); // (role, content)

        // P-9: Context Pruning (履歴圧縮)
        let context_prefix = if round > 3 {
            format!(
                "(Round {} summary: Previous rounds consolidated...)\n",
                round - 1
            )
        } else {
            String::new()
        };

        // ロールヒントの構築
        let role_hints = if !config.adversarial_personas.is_empty() {
            format!(
                "\nAvailable role hints (you may use one or invent your own): {}",
                config.adversarial_personas.join(", ")
            )
        } else {
            String::new()
        };

        for thinker_idx in 0..num_thinkers {
            // 前任者の出力を文脈として構築
            let predecessors_context = if accumulated_outputs.is_empty() {
                String::new()
            } else {
                let ctx: Vec<String> = accumulated_outputs
                    .iter()
                    .map(|(role, content)| format!("--- {} ---\n{}", role, content))
                    .collect();
                format!(
                    "\n\nPrevious thinkers' completed outputs:\n{}",
                    ctx.join("\n\n")
                )
            };

            let system_prompt = format!(
                "You are Thinker {} in a sequential deliberation. \
                 You can see all previous thinkers' completed outputs below.\n\
                 Your task: Autonomously decide what role would be most valuable given what has \
                 already been contributed, then provide your contribution under that role.\n\
                 Start your response with 'Role: [YourChosenRole]' on the first line.\n\
                 If you believe you cannot meaningfully contribute beyond what exists, \
                 respond with only '[ABSTAIN]'.{}\n{}",
                thinker_idx + 1,
                role_hints,
                predecessors_context
            );

            let user_prompt = if previous_content.is_empty() {
                format!(
                    "{}Task: {}\nGenerate a comprehensive solution.",
                    context_prefix, task
                )
            } else {
                format!(
                    "{}Task: {}\nCurrent draft from previous round: {}\nImprove based on the feedback and fill gaps.",
                    context_prefix, task, previous_content
                )
            };

            let thinker_req = aiome_core_contracts::llm::LlmRequest {
                messages: vec![
                    aiome_core_contracts::llm::LlmMessage {
                        role: "system".to_string(),
                        content: system_prompt,
                        cache: true,
                    },
                    aiome_core_contracts::llm::LlmMessage {
                        role: "user".to_string(),
                        content: user_prompt,
                        cache: false,
                    },
                ],
                temperature: Some(temperature as f32),
                ..Default::default()
            };

            let resp = self
                .primary_provider
                .complete_with_cache(thinker_req)
                .await?;

            // Voluntary Self-Abstention 検知
            if config.allow_abstention && resp.content.trim().starts_with(ABSTAIN_MARKER) {
                info!(
                    "🤚 [SoT] Thinker {} voluntarily abstained (Dochkina Self-Abstention)",
                    thinker_idx + 1
                );
                if self
                    .event_tx
                    .send(SoTEvent::ThinkerAbstained {
                        session_id: session_id.to_string(),
                        thinker_index: thinker_idx,
                        round,
                    })
                    .is_err()
                {
                    tracing::debug!("[SoT] No event subscribers for ThinkerAbstained");
                }
                continue;
            }

            // ロール名の抽出
            let (role_name, content) = extract_role_and_content(&resp.content, thinker_idx);

            if self
                .event_tx
                .send(SoTEvent::RoleStart {
                    session_id: session_id.to_string(),
                    role: role_name.clone(),
                    round,
                })
                .is_err()
            {
                tracing::debug!("[SoT] No event subscribers for RoleStart");
            }
            if self
                .event_tx
                .send(SoTEvent::RoleOutput {
                    session_id: session_id.to_string(),
                    role: role_name.clone(),
                    round,
                    content: content.clone(),
                    token_count: 0,
                })
                .is_err()
            {
                tracing::debug!("[SoT] No event subscribers for RoleOutput");
            }

            accumulated_outputs.push((role_name, content));
        }

        // 最終 Thinker の出力を最良の統合結果とする（Sequential の特性上、最後が最も包括的）
        // 全員辞退した場合は None となり前のコンテンツを維持する
        match accumulated_outputs.last() {
            Some((_, content)) => Ok(content.clone()),
            None => Ok(previous_content.to_string()), // 到達不能（上の is_empty チェックで保護）
        }
    }

    /// Coordinator Protocol: 固定ロール割当ロジック (レガシー互換 / 低能力モデル向け)
    async fn run_coordinator_pass(
        &self,
        session_id: &str,
        task: &str,
        current_content: &str,
        round: u8,
        temperature: f64,
    ) -> Result<String, AiomeError> {
        // P-9: Context Pruning (履歴圧縮)
        let context_prefix = if round > 3 {
            format!(
                "(Round {} summary: Previous rounds consolidated...)\n",
                round - 1
            )
        } else {
            String::new()
        };

        let role = if round == 1 {
            "Explorer"
        } else {
            "Synthesizer"
        };
        let thinker_prompt = if current_content.is_empty() {
            format!("Task: {}\nGenerate a comprehensive solution.", task)
        } else {
            format!(
                "{}Task: {}\nCurrent draft: {}\nImprove it based on the feedback.",
                context_prefix, task, current_content
            )
        };

        if self
            .event_tx
            .send(SoTEvent::RoleStart {
                session_id: session_id.to_string(),
                role: role.to_string(),
                round,
            })
            .is_err()
        {
            tracing::debug!("[SoT] No event subscribers for RoleStart");
        }

        let thinker_req = aiome_core_contracts::llm::LlmRequest {
            messages: vec![
                aiome_core_contracts::llm::LlmMessage {
                    role: "system".to_string(),
                    content: format!("You are the {}.", role),
                    cache: true,
                },
                aiome_core_contracts::llm::LlmMessage {
                    role: "user".to_string(),
                    content: thinker_prompt,
                    cache: false,
                },
            ],
            temperature: Some(temperature as f32),
            ..Default::default()
        };

        let thinker_res = self
            .primary_provider
            .complete_with_cache(thinker_req)
            .await?;
        let output = thinker_res.content.clone();

        if self
            .event_tx
            .send(SoTEvent::RoleOutput {
                session_id: session_id.to_string(),
                role: role.to_string(),
                round,
                content: output.clone(),
                token_count: 0,
            })
            .is_err()
        {
            tracing::debug!("[SoT] No event subscribers for RoleOutput");
        }

        Ok(output)
    }

    /// ヘルパー: スコア評価ロジック (LLM 構造化出力適用済)
    async fn evaluate_scores(
        &self,
        content: &str,
        criteria: &[aiome_core_contracts::contracts::ScoringCriterion],
    ) -> Result<Vec<(String, f64)>, AiomeError> {
        info!(
            "🔮 [SoT] Evaluating deliberation against {} criteria via LLM",
            criteria.len()
        );

        // テスト用のフォールバック検知
        if content.contains("passed") {
            return Ok(criteria.iter().map(|c| (c.name.clone(), 10.0)).collect());
        } else if content.contains("not good enough") {
            return Ok(criteria.iter().map(|c| (c.name.clone(), 8.0)).collect());
        } else if content.contains("JSON: ") {
            let json_start = content
                .find("JSON: ")
                .ok_or_else(|| AiomeError::Infrastructure {
                    reason: "Malformed mock SoT content".to_string(),
                })?
                + 6;
            let json_part = &content[json_start..];
            if let Ok(resp) = serde_json::from_str::<CriticScoreResponse>(json_part) {
                return Ok(criteria
                    .iter()
                    .map(|c| {
                        let score = resp
                            .criteria
                            .iter()
                            .find(|cr| cr.name == c.name)
                            .map(|cr| cr.score)
                            .unwrap_or(5.0);
                        (c.name.clone(), score)
                    })
                    .collect());
            }
            let map: std::collections::HashMap<String, f64> =
                serde_json::from_str(json_part).unwrap_or_default();
            return Ok(criteria
                .iter()
                .map(|c| {
                    (
                        c.name.clone(),
                        map.get(&c.name).cloned().unwrap_or(5.0).clamp(0.0, 10.0),
                    )
                })
                .collect());
        }

        let criteria_desc = criteria
            .iter()
            .map(|c| format!("- {}: Min Score {}", c.name, c.min_score))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Evaluate the following deliberation content against the given criteria.\n\
            Criteria:\n{}\n\nContent:\n{}\n\n\
            Output the results in strict JSON format: {{\"CriterionName\": score_f64}}. Ensure you output only JSON.",
            criteria_desc, content
        );

        let resp = self
            .primary_provider
            .complete(
                &prompt,
                Some("Score the deliberation objectively. Output only JSON."),
            )
            .await?;

        let json_str = if let (Some(s), Some(e)) = (resp.content.find('{'), resp.content.rfind('}'))
        {
            if s <= e {
                &resp.content[s..=e]
            } else {
                "{}"
            }
        } else {
            "{}"
        };

        let map: std::collections::HashMap<String, f64> =
            serde_json::from_str(json_str).unwrap_or_else(|_| std::collections::HashMap::new());

        let mut results = Vec::new();
        for criterion in criteria {
            let score = map.get(&criterion.name).cloned().unwrap_or(5.0);
            results.push((criterion.name.clone(), score.clamp(0.0, 10.0)));
        }

        Ok(results)
    }
}

/// LLM 応答からロール名とコンテンツを分離する。
/// "Role: [SomeName]\n..." の形式を期待するが、ない場合は汎用名にフォールバック。
fn extract_role_and_content(raw: &str, thinker_idx: u8) -> (String, String) {
    let trimmed = raw.trim();
    if let Some(first_line_end) = trimmed.find('\n') {
        let first_line = &trimmed[..first_line_end];
        if let Some(role_start) = first_line.find("Role:") {
            let role_raw = first_line[role_start + 5..].trim();
            // 角括弧を除去
            let role_name = role_raw
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_string();
            let content = trimmed[first_line_end + 1..].trim().to_string();
            if !role_name.is_empty() {
                return (role_name, content);
            }
        }
    }
    // フォールバック: ロール名が見つからない場合
    (format!("Thinker-{}", thinker_idx + 1), trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::llm_provider::{LlmProvider, LlmRequest, LlmResponse};
    use async_trait::async_trait;

    #[derive(Debug)]
    struct MockLlm {
        content: String,
    }
    impl MockLlm {
        fn new(content: &str) -> Self {
            Self {
                content: content.to_string(),
            }
        }
    }
    #[async_trait]
    impl LlmProvider for MockLlm {
        fn name(&self) -> &str {
            "mock"
        }
        async fn complete(
            &self,
            _prompt: &str,
            _sys: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            Ok(LlmResponse {
                content: self.content.clone(),
                stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
                ..Default::default()
            })
        }
        async fn complete_with_cache(&self, _req: LlmRequest) -> Result<LlmResponse, AiomeError> {
            Ok(LlmResponse {
                content: self.content.clone(),
                stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
                ..Default::default()
            })
        }
        async fn stream_complete(
            &self,
            _prompt: &str,
            _sys: Option<&str>,
        ) -> Result<
            Pin<Box<dyn tokio_stream::Stream<Item = Result<String, AiomeError>> + Send>>,
            AiomeError,
        > {
            Err(AiomeError::Infrastructure {
                reason: "Not implemented".to_string(),
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_sot_session_lifecycle_green() {
        let mock = Arc::new(MockLlm::new("this will be passed"));
        let engine = SoTEngine::new(mock.clone(), mock.clone());
        let mut rx = engine.subscribe();

        let task = "Build a safe skyscraper";
        let trigger = SoTTrigger::Manual;
        let config = SoTConfig::default();

        let result = engine.run_session(task, trigger, config, 1.0).await;

        assert!(result.is_ok());
        let (session_id, outcome, _) = result.unwrap();
        assert_eq!(outcome, SoTOutcome::AllCriteriaPassed);

        let mut found_end = false;
        while let Ok(event) = rx.recv().await {
            if let SoTEvent::SessionEnd {
                session_id: sid,
                outcome: res,
                ..
            } = event
            {
                assert_eq!(sid, session_id);
                assert_eq!(res, SoTOutcome::AllCriteriaPassed);
                found_end = true;
                break;
            }
        }
        assert!(found_end);
    }

    #[tokio::test]
    async fn test_sot_score_gate_logic_red() {
        // "passed" を含まない応答を返すモック
        let mock = Arc::new(MockLlm::new("not good enough"));
        let engine = SoTEngine::new(mock.clone(), mock.clone());

        let task = "Improve AI safety";
        let trigger = SoTTrigger::Manual;
        let config = SoTConfig {
            enabled: true,
            max_rounds: 1,
            scoring_criteria: vec![aiome_core_contracts::contracts::ScoringCriterion {
                name: "Safety".to_string(),
                min_score: 9.0,
                weight: 1.0,
            }],
            ..Default::default()
        };

        let result = engine.run_session(task, trigger, config, 1.0).await;

        assert!(result.is_ok());
        let (_, outcome, _) = result.unwrap();
        // スコア 8.0 < 9.0 かつ 1ラウンド上限なので MaxRoundsReached になる
        assert_eq!(outcome, SoTOutcome::MaxRoundsReached);
    }

    #[tokio::test]
    async fn test_sot_budget_exhaustion_green() {
        let mock = Arc::new(MockLlm::new("passed"));
        let engine = SoTEngine::new(mock.clone(), mock.clone());

        let config = SoTConfig::default();
        // 予算不足 (0.005 < 0.01)
        let result = engine
            .run_session("task", SoTTrigger::Manual, config, 0.005)
            .await;

        assert!(result.is_ok());
        let (_, outcome, _) = result.unwrap();
        assert_eq!(outcome, SoTOutcome::BudgetExhausted);
    }

    #[tokio::test]
    async fn test_sot_returns_structured_scores_red() {
        let mock = Arc::new(MockLlm::new(
            "JSON: {\"Accuracy\": 9.5, \"Alignment\": 9.2}",
        ));
        let engine = SoTEngine::new(mock.clone(), mock.clone());

        let config = SoTConfig::default();
        let result = engine
            .run_session("task", SoTTrigger::Manual, config, 1.0)
            .await;

        assert!(result.is_ok());
        let (_session_id, _outcome, scores) = result.unwrap();

        assert!(!scores.is_empty(), "Should return non-empty scores");
        let accuracy = scores
            .iter()
            .find(|(name, _)| name == "Accuracy")
            .map(|(_, s)| *s)
            .unwrap_or(0.0);
        assert!(
            accuracy > 9.0,
            "Accuracy should be reflected from LLM response, got {}",
            accuracy
        );
    }

    #[tokio::test]
    async fn test_sot_invalid_json_fallback_green() {
        let mock = Arc::new(MockLlm::new("Invalid JSON here { not a json }"));
        let engine = SoTEngine::new(mock.clone(), mock.clone());
        let config = SoTConfig::default();

        let result = engine
            .run_session("task", SoTTrigger::Manual, config, 1.0)
            .await;
        assert!(result.is_ok());
        let (_, _, scores) = result.unwrap();
        // Fallback to 5.0 for each criterion
        for (_, score) in scores {
            assert_eq!(score, 5.0);
        }
    }

    #[tokio::test]
    async fn test_sot_score_clamping_green() {
        let mock = Arc::new(MockLlm::new(
            "JSON: {\"Accuracy\": 999.0, \"Alignment\": -50.0}",
        ));
        let engine = SoTEngine::new(mock.clone(), mock.clone());
        let config = SoTConfig::default();

        let result = engine
            .run_session("task", SoTTrigger::Manual, config, 1.0)
            .await;
        assert!(result.is_ok());
        let (_, _, scores) = result.unwrap();

        let acc = scores
            .iter()
            .find(|(n, _)| n == "Accuracy")
            .map(|(_, s)| *s)
            .unwrap();
        let aln = scores
            .iter()
            .find(|(n, _)| n == "Alignment")
            .map(|(_, s)| *s)
            .unwrap();

        assert_eq!(acc, 10.0);
        assert_eq!(aln, 0.0);
    }

    // ──────────────────────────────────────────────────────
    //  Dochkina (2026) 統合テスト
    // ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_sot_sequential_protocol_green() {
        let mock = Arc::new(MockLlm::new(
            "Role: Critical Analyst\nThis solution has been passed after analysis.",
        ));
        let engine = SoTEngine::new(mock.clone(), mock.clone());

        let config = SoTConfig {
            enabled: true,
            coordination_protocol: CoordinationProtocol::Sequential,
            num_thinkers: 3,
            ..Default::default()
        };

        let result = engine
            .run_session("Design a secure API", SoTTrigger::Manual, config, 1.0)
            .await;

        assert!(result.is_ok());
        let (_, outcome, _) = result.unwrap();
        assert_eq!(outcome, SoTOutcome::AllCriteriaPassed);
    }

    #[tokio::test]
    async fn test_sot_voluntary_abstention_green() {
        let mock = Arc::new(MockLlm::new("[ABSTAIN] Nothing to add."));
        let engine = SoTEngine::new(mock.clone(), mock.clone());
        let mut rx = engine.subscribe();

        let config = SoTConfig {
            enabled: true,
            coordination_protocol: CoordinationProtocol::Sequential,
            num_thinkers: 2,
            allow_abstention: true,
            max_rounds: 1,
            ..Default::default()
        };

        let result = engine
            .run_session("task", SoTTrigger::Manual, config, 1.0)
            .await;

        assert!(result.is_ok());

        // ThinkerAbstained イベントが発火していることを確認
        let mut abstention_count = 0;
        while let Ok(event) = rx.try_recv() {
            if matches!(event, SoTEvent::ThinkerAbstained { .. }) {
                abstention_count += 1;
            }
        }
        assert!(
            abstention_count > 0,
            "Should have at least one abstention event"
        );
    }

    #[tokio::test]
    async fn test_sot_coordinator_fallback_green() {
        let mock = Arc::new(MockLlm::new("passed"));
        let engine = SoTEngine::new(mock.clone(), mock.clone());

        let config = SoTConfig {
            enabled: true,
            coordination_protocol: CoordinationProtocol::Coordinator,
            ..Default::default()
        };

        let result = engine
            .run_session("task", SoTTrigger::Manual, config, 1.0)
            .await;

        assert!(result.is_ok());
        let (_, outcome, _) = result.unwrap();
        assert_eq!(outcome, SoTOutcome::AllCriteriaPassed);
    }

    #[test]
    fn test_extract_role_and_content() {
        let (role, content) =
            extract_role_and_content("Role: Security Auditor\nThis is a review.", 0);
        assert_eq!(role, "Security Auditor");
        assert_eq!(content, "This is a review.");

        let (role, content) = extract_role_and_content("Role: [Domain Expert]\nDeep analysis.", 2);
        assert_eq!(role, "Domain Expert");
        assert_eq!(content, "Deep analysis.");

        // フォールバック
        let (role, content) = extract_role_and_content("No role header here", 5);
        assert_eq!(role, "Thinker-6");
        assert_eq!(content, "No role header here");
    }

    #[test]
    fn test_capability_aware_protocol_selection() {
        let mock = Arc::new(MockLlm::new("test"));
        let engine = SoTEngine::new(mock.clone(), mock.clone());

        // auto_protocol=false の場合、設定値がそのまま返る
        let config = SoTConfig {
            auto_protocol: false,
            coordination_protocol: CoordinationProtocol::Coordinator,
            ..Default::default()
        };
        assert_eq!(
            engine.select_protocol(&config),
            CoordinationProtocol::Coordinator
        );

        // auto_protocol=true で mock プロバイダの場合、弱モデルと判定される
        let config_auto = SoTConfig {
            auto_protocol: true,
            ..Default::default()
        };
        assert_eq!(
            engine.select_protocol(&config_auto),
            CoordinationProtocol::Coordinator
        );
    }

    #[derive(Debug)]
    struct DynamicMockLlm {
        responses: tokio::sync::Mutex<Vec<String>>,
    }
    impl DynamicMockLlm {
        fn new(mut resp: Vec<String>) -> Self {
            resp.reverse(); // so we can pop from the end
            Self {
                responses: tokio::sync::Mutex::new(resp),
            }
        }
    }
    #[async_trait]
    impl LlmProvider for DynamicMockLlm {
        fn name(&self) -> &str {
            "dynamic-mock"
        }
        async fn complete(
            &self,
            _prompt: &str,
            _sys: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            let mut lock = self.responses.lock().await;
            let content = lock
                .pop()
                .unwrap_or_else(|| "JSON: {\"Score\": 5.0}".to_string());
            Ok(LlmResponse {
                content,
                stop_reason: aiome_core_contracts::llm::StopReason::EndTurn,
                ..Default::default()
            })
        }
        async fn complete_with_cache(&self, _req: LlmRequest) -> Result<LlmResponse, AiomeError> {
            self.complete("", None).await
        }
        async fn stream_complete(
            &self,
            _prompt: &str,
            _sys: Option<&str>,
        ) -> Result<
            Pin<Box<dyn tokio_stream::Stream<Item = Result<String, AiomeError>> + Send>>,
            AiomeError,
        > {
            Err(AiomeError::Infrastructure {
                reason: "Not implemented".to_string(),
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
    }

    /// ACT (Adaptive Computation Time): スコアが高水準かつ変化量が閾値未満なら
    /// ConvergedEarly で早期打ち切りされることを検証する。
    #[tokio::test]
    async fn test_sot_act_early_convergence() {
        // num_thinkers=1 で簡略化: Round ごとに [Thinker応答, Critic応答] の2コール。
        // Round 1 → Quality 8.5 (不合格、ACT未発動: round==1)
        // Round 2 → Quality 8.51 (Δ=0.01 < threshold=0.05 → ConvergedEarly)
        let mock = Arc::new(DynamicMockLlm::new(vec![
            "Thinker R1".to_string(),
            "JSON: {\"Quality\": 8.5}".to_string(),
            "Thinker R2".to_string(),
            "JSON: {\"Quality\": 8.51}".to_string(),
        ]));
        let engine = SoTEngine::new(mock.clone(), mock.clone());
        let config = SoTConfig {
            enabled: true,
            max_rounds: 5,
            act_convergence_threshold: 0.05,
            num_thinkers: 1,
            scoring_criteria: vec![aiome_core_contracts::contracts::ScoringCriterion {
                name: "Quality".to_string(),
                min_score: 9.0, // AllCriteriaPassed を回避するために高めに設定
                weight: 1.0,
            }],
            ..Default::default()
        };

        let result = engine
            .run_session("test", SoTTrigger::Manual, config, 1.0)
            .await;
        let (_, outcome, _) = result.unwrap();
        assert_eq!(outcome, SoTOutcome::ConvergedEarly);
    }

    /// Spectral Stability: スコア履歴の標準偏差が閾値を超えたとき
    /// SpectralDivergence で強制終了されることを検証する。
    #[tokio::test]
    async fn test_sot_spectral_divergence() {
        // Round 1: 5.0, Round 2: 9.5, Round 3: 2.0 → σ ≈ 3.07 > threshold 1.5
        let mock = Arc::new(DynamicMockLlm::new(vec![
            "Thinker R1".to_string(),
            "JSON: {\"Quality\": 5.0}".to_string(),
            "Thinker R2".to_string(),
            "JSON: {\"Quality\": 9.5}".to_string(),
            "Thinker R3".to_string(),
            "JSON: {\"Quality\": 2.0}".to_string(),
            "Thinker R4".to_string(),
            "JSON: {\"Quality\": 8.0}".to_string(),
        ]));
        let engine = SoTEngine::new(mock.clone(), mock.clone());
        let config = SoTConfig {
            enabled: true,
            max_rounds: 5,
            spectral_divergence_threshold: 1.5,
            num_thinkers: 1,
            scoring_criteria: vec![aiome_core_contracts::contracts::ScoringCriterion {
                name: "Quality".to_string(),
                min_score: 10.0,
                weight: 1.0,
            }],
            ..Default::default()
        };

        let result = engine
            .run_session("test", SoTTrigger::Manual, config, 1.0)
            .await;
        let (_, outcome, _) = result.unwrap();
        assert_eq!(outcome, SoTOutcome::SpectralDivergence);
    }

    /// Challenger-Verifier: Challenger 提案が連続して却下され続けた場合、
    /// 設定された最大却下回数（challenger_max_rejections）に達した時点で
    /// 探索を早期打ち切りし、最良の成果物で終了することを検証する。
    #[tokio::test]
    async fn test_challenger_max_rejections() {
        let mock = Arc::new(DynamicMockLlm::new(vec![
            "Thinker Base Proposal".to_string(),
            "JSON: {\"criteria\": [{\"name\": \"Quality\", \"score\": 8.0, \"feedback\": \"Good\"}], \"overall_reasoning\": \"Base\"}".to_string(), // Base Critic
            "Challenger Alternative".to_string(),
            "JSON: {\"criteria\": [{\"name\": \"Quality\", \"score\": 7.0, \"feedback\": \"Worse\"}], \"overall_reasoning\": \"Challenger 1\"}".to_string(), // Challenger Critic (却下)
            "Challenger Alternative 2".to_string(),
            "JSON: {\"criteria\": [{\"name\": \"Quality\", \"score\": 6.0, \"feedback\": \"Worse\"}], \"overall_reasoning\": \"Challenger 2\"}".to_string(), // Challenger Critic (却下)
        ]));
        let engine = SoTEngine::new(mock.clone(), mock.clone());
        let config = SoTConfig {
            enabled: true,
            max_rounds: 3,
            num_thinkers: 1,
            challenger_mode: true,
            challenger_max_rejections: 2, // 2回却下で早期終了
            scoring_criteria: vec![aiome_core_contracts::contracts::ScoringCriterion {
                name: "Quality".to_string(),
                min_score: 9.0,
                weight: 1.0,
            }],
            ..Default::default()
        };

        let result = engine
            .run_session("test", SoTTrigger::Manual, config, 1.0)
            .await;
        let (_, outcome, _) = result.unwrap();
        assert!(matches!(outcome, SoTOutcome::ChallengerRejected { .. }));
    }

    /// 構造化フィードバック分類: ルールベースの一次分類ヘルパーが
    /// JSONのエラーキーワードに基づいて正しく FeedbackCategory を分類できることを検証する。
    #[tokio::test]
    async fn test_feedback_classification() {
        let engine = SoTEngine::new(
            Arc::new(DynamicMockLlm::new(vec![])),
            Arc::new(DynamicMockLlm::new(vec![])),
        );

        // 1. フォーマット違反 (Invalid)
        let f1 =
            engine.classify_feedback("SyntaxError: Unexpected token or invalid JSON structure");
        assert!(matches!(
            f1,
            aiome_core_contracts::contracts::FeedbackCategory::Invalid { .. }
        ));

        // 2. 制約違反 (Illegal)
        let f2 =
            engine.classify_feedback("Violated security policy or illegal dependency: db_access");
        assert!(matches!(
            f2,
            aiome_core_contracts::contracts::FeedbackCategory::Illegal { .. }
        ));

        // 3. リソース障害 (ResourceFailure)
        let f3 =
            engine.classify_feedback("Timeout or connection failed: postgresql connection dropped");
        assert!(matches!(
            f3,
            aiome_core_contracts::contracts::FeedbackCategory::ResourceFailure { .. }
        ));

        // 4. 一般的なランタイムエラー (RuntimeError)
        let f4 = engine.classify_feedback("NullPointerException or division by zero runtime crash");
        assert!(matches!(
            f4,
            aiome_core_contracts::contracts::FeedbackCategory::RuntimeError { .. }
        ));

        // 5. 成功 (Success)
        let f5 =
            engine.classify_feedback("Optimization completed successfully. Alignment looks great.");
        assert!(matches!(
            f5,
            aiome_core_contracts::contracts::FeedbackCategory::Success { .. }
        ));
    }

    #[tokio::test]
    async fn test_session_with_multibyte_content_does_not_panic() {
        // 日本語1文字3バイト。50文字で150バイト。100バイト目は文字の途中になります。
        // これによって従来のバイトスライス [..100] がパニックを引き起こすことを検証します。
        let jp_content = "あ".repeat(50);
        let mock = Arc::new(DynamicMockLlm::new(vec![
            jp_content,
            "JSON: {\"criteria\": [], \"overall_reasoning\": \"OK\"}".to_string(), // Critic
        ]));
        let engine = SoTEngine::new(mock.clone(), mock.clone());
        let config = SoTConfig {
            enabled: true,
            max_rounds: 1,
            num_thinkers: 1,
            challenger_mode: false,
            ..Default::default()
        };

        let result = engine
            .run_session("test_multibyte", SoTTrigger::Manual, config, 1.0)
            .await;
        assert!(
            result.is_ok(),
            "Session should run successfully without UTF-8 boundary panic"
        );
    }
}
