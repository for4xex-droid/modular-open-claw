/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::dream_state::DreamState;
use aiome_core::error::AiomeError;
use std::sync::atomic::{AtomicU32, Ordering};
use tracing::warn;

const MAX_GUARDIAN_LLM_CALLS: u32 = 3;

impl DreamState {
    pub async fn biome_crisis_guardian(
        &self,
        epoch_llm_call_count: &AtomicU32,
        crisis_type: &str,
        vulnerability_report: &str,
    ) -> Result<Option<String>, AiomeError> {
        // Layer 2: 固有カウンタチェック
        let current_calls = epoch_llm_call_count.load(Ordering::Relaxed);
        if current_calls >= MAX_GUARDIAN_LLM_CALLS {
            warn!("⚠️ [CrisisGuardian] LLM call limit reached for this epoch. Using template response.");
            return Ok(Some(self.crisis_template_response(crisis_type)));
        }

        // LLM プロンプト
        let prompt = format!(
            "Analyze the following Biome crisis and cellular automaton vulnerability report, and suggest mitigating action targets for the agent:\n\
            Crisis Type: {}\n\
            Vulnerability Report: {}\n\
            Please generate a concise recommendations block.",
            crisis_type, vulnerability_report
        );
        let system_prompt = "You are the Crisis Guardian AI, a protective sub-agent. Provide direct advice to the player/agent. Do not use verbose introductions.";

        // LLM 呼び出し
        let resp = self.llm.complete(&prompt, Some(system_prompt)).await;

        match resp {
            Ok(r) => {
                epoch_llm_call_count.fetch_add(1, Ordering::Relaxed);
                Ok(Some(r.content))
            }
            Err(e) => {
                // Layer 3: フォールバック
                warn!("⚠️ [CrisisGuardian] LLM failed: {}. Using template.", e);
                Ok(Some(self.crisis_template_response(crisis_type)))
            }
        }
    }

    fn crisis_template_response(&self, crisis_type: &str) -> String {
        match crisis_type {
            "meteor" => "⚠️ 元素カタリストが接近中。Fe/Si の注入をお勧めします。".to_string(),
            "ice_age" => {
                "⚠️ 休眠圧力が上昇中。活性度の低いセルが凍結される可能性があります。".to_string()
            }
            _ => "⚠️ 環境変化を検出しました。".to_string(),
        }
    }
}
