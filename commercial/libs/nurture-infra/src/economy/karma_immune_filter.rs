/*
 * Project NURTURE - Autonomous AI Agent Economy
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 */

use aiome_core_contracts::contracts::FederatedKarma;
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::traits::JobQueue;
use std::sync::Arc;

pub struct FilterResult {
    pub accepted: Vec<FederatedKarma>,
    pub rejected: Vec<(FederatedKarma, String)>,
}

pub struct KarmaImmuneFilter {
    job_queue: Arc<dyn JobQueue>,
}

impl KarmaImmuneFilter {
    pub fn new(job_queue: Arc<dyn JobQueue>) -> Self {
        Self { job_queue }
    }

    /// 免疫ルールに基づいて Karma をフィルタリングする (SC-3)
    pub async fn filter(
        &self,
        karmas: Vec<FederatedKarma>,
        _quality_threshold: i32,
    ) -> Result<FilterResult, AiomeError> {
        let rules = self.job_queue.fetch_active_immune_rules().await?;

        let mut accepted = Vec::new();
        let mut rejected = Vec::new();

        for karma in karmas {
            let mut is_rejected = false;

            // 1. パーソナリティ保護ルールマッチング (正規表現等)
            for rule in &rules {
                if karma.lesson.contains(rule.pattern.as_str()) {
                    // 単純な文字含みチェックから開始
                    rejected.push((karma.clone(), format!("Matched immune rule: {}", rule.id)));
                    is_rejected = true;
                    break;
                }
            }

            if !is_rejected {
                accepted.push(karma);
            }
        }

        Ok(FilterResult { accepted, rejected })
    }
}
