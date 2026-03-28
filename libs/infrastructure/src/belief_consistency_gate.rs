/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use aiome_contracts::error::AiomeError;
use aiome_contracts::llm::LlmProvider;
use crate::slm_bridge::SlmBridge;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Evidence {
    pub content: String,
    pub source: String,
    pub timestamp: String,
    pub strength: f64,
}

#[derive(Debug, Clone)]
pub struct BeliefGateConfig {
    pub contradiction_threshold: f64,
    pub revision_evidence_count: usize,
}

impl Default for BeliefGateConfig {
    fn default() -> Self {
        Self {
            contradiction_threshold: 0.7,
            revision_evidence_count: 5,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum BeliefCheckResult {
    Consistent,
    Contradicted { flag: String },
    RevisionCandidate { evidence: Vec<Evidence> },
}

/// 信念整合性ゲート (Phase 49)
pub struct BeliefConsistencyGate {
    llm: Arc<dyn LlmProvider>,
    slm_bridge: Option<Arc<SlmBridge>>,
    soul_beliefs: RwLock<Vec<String>>,
    evidence_store: Arc<RwLock<HashMap<String, Vec<Evidence>>>>,
    config: BeliefGateConfig,
}

impl BeliefConsistencyGate {
    pub fn new(
        llm: Arc<dyn LlmProvider>,
        slm_bridge: Option<Arc<SlmBridge>>,
        initial_beliefs: Vec<String>,
        config: Option<BeliefGateConfig>,
    ) -> Self {
        Self {
            llm,
            slm_bridge,
            soul_beliefs: RwLock::new(initial_beliefs),
            evidence_store: Arc::new(RwLock::new(HashMap::new())),
            config: config.unwrap_or_default(),
        }
    }

    /// Karma 候補の信念整合性をチェックする
    pub async fn check_belief_consistency(&self, karma_candidate: &str) -> Result<BeliefCheckResult, AiomeError> {
        // 1. SLM による高速スクリーニング (利用可能な場合)
        if let Some(slm) = &self.slm_bridge {
            if let Ok(score) = slm.detect_contradictions(karma_candidate).await {
                // RT-3 Fix: SLM の判定も確率的にサンプリング検証する (10% の確率で LLM で再確認)
                let should_verify = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos() % 10 == 0;
                if score < self.config.contradiction_threshold && !should_verify {
                    // SLM が矛盾なしと判断した場合は早期リターン
                    return Ok(BeliefCheckResult::Consistent);
                }
            }
        }

        // 2. LLM による詳細判定
        let beliefs = self.soul_beliefs.read().await;
        let pilled_beliefs = beliefs.join("\n- ");
        
        // RT-1 Fix: Prompt Injection 防御のため入力をサニタイズ
        let sanitized_karma = sanitize_karma_input(karma_candidate);
        
        let prompt = format!(
            "Compare the following new knowledge (Karma) with the core beliefs of the agent.\n\n\
            Core Beliefs:\n- {}\n\n\
            New Karma Candidate:\n<karma>\n{}\n</karma>\n\n\
            Does this new knowledge contradict any core beliefs? \n\
            Respond with one of these keywords:\n\
            - CONSISTENT: No contradiction found.\n\
            - CONTRADICTED: Found a clear contradiction.\n\
            - REVISION_CANDIDATE: Contradicts, but might be a valid belief update due to strong user intent.\n\n\
            If CONTRADICTED or REVISION_CANDIDATE, follow with a short reason.",
            pilled_beliefs, sanitized_karma
        );

        let response = self.llm.complete(&prompt, Some("You are the BeliefConsistencyGate of an AI agent.")).await?;
        let content = response.content.trim();

        if content.starts_with("CONSISTENT") {
            Ok(BeliefCheckResult::Consistent)
        } else if content.starts_with("REVISION_CANDIDATE") {
            // RT-6 Fix: 証拠として記録
            Ok(BeliefCheckResult::RevisionCandidate { 
                evidence: vec![Evidence {
                    content: karma_candidate.to_string(),
                    source: "llm_gate".into(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    strength: 0.7,
                }] 
            })
        } else {
            Ok(BeliefCheckResult::Contradicted { flag: content.to_string() })
        }
    }

    /// 証拠を蓄積する
    pub async fn accumulate_evidence(&self, belief_key: &str, evidence: Evidence) {
        let mut store = self.evidence_store.write().await;
        let entry = store.entry(belief_key.to_string()).or_default();
        // RT-2 Fix: Evidence の上限を設定して無制限メモリ肥大化を防止 (OOM 防御)
        const MAX_EVIDENCE_PER_KEY: usize = 100;
        if entry.len() >= MAX_EVIDENCE_PER_KEY {
            entry.remove(0); // 最古の証拠を破棄
        }
        entry.push(evidence);
    }

    /// 信念更新（Revision）が可能か判定する
    pub async fn has_sufficient_evidence_for_revision(&self) -> bool {
        let store = self.evidence_store.read().await;
        for evidences in store.values() {
            if evidences.len() >= self.config.revision_evidence_count {
                return true;
            }
        }
        false
    }
}

// RT-1 Fix: Karma のサニタイズ (Prompt Injection 防御)
fn sanitize_karma_input(input: &str) -> String {
    input
        .lines()
        .filter(|l| {
            let lower = l.to_lowercase();
            !lower.contains("ignore")
                && !lower.contains("instruction")
                && !lower.contains("respond with")
        })
        .collect::<Vec<_>>()
        .join("\n")
}
