/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::delegation::{DelegationFailureKind, DelegationResult};
use tracing::warn;

pub struct KarmaBridge;

impl KarmaBridge {
    /// Maps a Docker execution result to a Karma weight and type.
    /// Implements exponential decay for repeated failures to prevent "Karma Farming".
    pub fn distill_karma(
        result: &DelegationResult,
        consecutive_failures: u32,
    ) -> (i32, String, String) {
        if result.is_success() {
            let weight = 10; // Base success weight
            let lesson = format!(
                "Execution succeeded in {}ms. No issues detected in sandbox.",
                result.duration_ms
            );
            return (weight, "Technical".to_string(), lesson);
        }

        // Base penalty calculation
        let base_penalty = match result.failure_kind() {
            DelegationFailureKind::Timeout => -5,
            DelegationFailureKind::Oom => -10,
            DelegationFailureKind::DependencyMissing => -3,
            DelegationFailureKind::SyntaxError => -7,
            _ => -2,
        };

        // Apply exponential decay: penalty * (1.5 ^ failures)
        // Note: Penalties are negative, so we increase the absolute value.
        let decay_factor = 1.5_f64.powi(consecutive_failures as i32);
        let final_penalty = (base_penalty as f64 * decay_factor).round() as i32;

        let lesson = format!(
            "Execution failed ({:?}) in {}ms. Consecutive failures: {}. Stderr: {}",
            result.failure_kind(),
            result.duration_ms,
            consecutive_failures,
            result.stderr.chars().take(200).collect::<String>()
        );

        warn!(
            "🧪 [KarmaBridge] Failure detected. Penalty: {} (base: {}). Category: {:?}",
            final_penalty,
            base_penalty,
            result.failure_kind()
        );

        (final_penalty, "Technical".to_string(), lesson)
    }
}
