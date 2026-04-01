/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_contracts::types::AgentStats;
use aiome_core::trajectory::FailureCategory;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Differentiates between repair approaches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RepairStrategy {
    RetryWithHint(String),
    SkipAndContinue,
    EscalateToHuman(String),
    AdjustParameters(HashMap<String, String>),
}

/// Dynamic calculator for maximum retry allowance based on Karma/TechExp.
pub struct RepairCalculator;

impl RepairCalculator {
    pub fn calculate_max_retries(stats: &AgentStats) -> u32 {
        match stats.level {
            ..=2 => 1,
            3..=5 => 3,
            _ => 5,
        }
    }
}

/// Suggests the best strategy based on Failure Category and past retries.
pub fn suggest_strategy(
    category: &FailureCategory,
    hint: &str,
    current_retries: u32,
    max_retries: u32,
) -> RepairStrategy {
    match category {
        FailureCategory::SystemFailure | FailureCategory::GuardrailsTriggered => {
            RepairStrategy::EscalateToHuman(format!("{:?} escalated immediately", category))
        }
        _ => {
            if current_retries >= max_retries {
                RepairStrategy::EscalateToHuman(format!("Max retries exceeded for {:?}", category))
            } else {
                RepairStrategy::RetryWithHint(hint.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::trajectory::FailureCategory;

    #[test]
    fn test_calculate_max_retries_levels() {
        let mut stats = AgentStats::default();
        stats.level = 1;
        stats.resonance = 0;
        assert_eq!(
            RepairCalculator::calculate_max_retries(&stats),
            1,
            "Level 1 should yield 1 retry"
        );

        stats.level = 3;
        assert_eq!(
            RepairCalculator::calculate_max_retries(&stats),
            3,
            "Level 3 should yield 3 retries"
        );

        stats.level = 10;
        assert_eq!(
            RepairCalculator::calculate_max_retries(&stats),
            5,
            "Level 10 should cap at 5 retries"
        );
    }

    #[test]
    fn test_suggest_strategy() {
        // Red test: Retry logic based on category and current retries
        let s1 = suggest_strategy(
            &FailureCategory::PlanAdherenceFailure,
            "Use different tool",
            0,
            3,
        );
        assert_eq!(
            s1,
            RepairStrategy::RetryWithHint("Use different tool".into())
        );

        // Beyond max retries
        let s2 = suggest_strategy(&FailureCategory::InvalidInvocation, "Fix JSON", 3, 3);
        assert_eq!(
            s2,
            RepairStrategy::EscalateToHuman("Max retries exceeded for InvalidInvocation".into())
        );

        // Unrecoverable errors escalate immediately
        let s3 = suggest_strategy(&FailureCategory::SystemFailure, "DB locked", 0, 3);
        assert_eq!(
            s3,
            RepairStrategy::EscalateToHuman("SystemFailure escalated immediately".into())
        );

        // Guardrails escalation immediately
        let s4 = suggest_strategy(
            &FailureCategory::GuardrailsTriggered,
            "Cannot write out of sandbox",
            0,
            3,
        );
        assert_eq!(
            s4,
            RepairStrategy::EscalateToHuman("GuardrailsTriggered escalated immediately".into())
        );
    }
}
