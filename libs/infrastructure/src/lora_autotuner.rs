/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */

use std::collections::HashMap;

/// Result of hyperparameter tuning
#[derive(Debug, Clone, PartialEq)]
pub struct TunedHyperparams {
    pub learning_rate: f64,
    pub epochs: u32,
    pub lora_rank: u32,
    pub batch_size: u32,
}

impl Default for TunedHyperparams {
    fn default() -> Self {
        Self {
            learning_rate: 1e-4,
            epochs: 3,
            lora_rank: 8,
            batch_size: 4,
        }
    }
}

pub struct TrainingMetrics {
    pub loss_history: Vec<f64>,
    pub previous_params: TunedHyperparams,
}

pub struct LoraAutotuner;

impl LoraAutotuner {
    /// Suggests optimal hyperparameters based on historical loss.
    pub fn suggest_hyperparams(metrics: &TrainingMetrics) -> TunedHyperparams {
        let mut new_params = metrics.previous_params.clone();

        if metrics.loss_history.len() < 3 {
            return new_params;
        }

        let n = metrics.loss_history.len();
        let l_last = metrics.loss_history[n - 1];

        // 1. Check for overfitting (loss approaches 0)
        if l_last < 0.05 {
            if new_params.lora_rank > 4 {
                new_params.lora_rank /= 2;
            }
            if new_params.epochs > 1 {
                new_params.epochs -= 1;
            }
            return new_params;
        }

        // 2. Check for stagnation
        let recent_variance: f64 = metrics
            .loss_history
            .iter()
            .skip(n - 3)
            .map(|&x| (x - l_last).abs())
            .sum();
        if recent_variance < 0.05 {
            new_params.learning_rate *= 2.0;
            return new_params;
        }

        // 3. Check for oscillation
        let mut reversals = 0;
        for i in 1..n - 1 {
            let diff1 = metrics.loss_history[i] - metrics.loss_history[i - 1];
            let diff2 = metrics.loss_history[i + 1] - metrics.loss_history[i];
            if diff1 * diff2 < 0.0 {
                reversals += 1;
            }
        }

        if reversals >= 2 {
            new_params.learning_rate *= 0.5;
        }

        new_params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_hyperparams_stagnation() {
        // FLAT LOSS -> Should increase LR
        let metrics = TrainingMetrics {
            loss_history: vec![2.0, 1.99, 1.98, 1.98, 1.98],
            previous_params: TunedHyperparams {
                learning_rate: 1e-4,
                epochs: 3,
                lora_rank: 8,
                batch_size: 4,
            },
        };

        let new_params = LoraAutotuner::suggest_hyperparams(&metrics);
        assert!(
            new_params.learning_rate > 1e-4,
            "LR should increase for stagnation"
        );
    }

    #[test]
    fn test_suggest_hyperparams_oscillation() {
        // OSCILLATING LOSS -> Should decrease LR
        let metrics = TrainingMetrics {
            loss_history: vec![1.5, 2.5, 1.2, 2.7, 1.1],
            previous_params: TunedHyperparams {
                learning_rate: 5e-4,
                epochs: 3,
                lora_rank: 8,
                batch_size: 4,
            },
        };

        let new_params = LoraAutotuner::suggest_hyperparams(&metrics);
        assert!(
            new_params.learning_rate < 5e-4,
            "LR should decrease for oscillation"
        );
    }

    #[test]
    fn test_suggest_hyperparams_overfitting() {
        // SHARP PLUNGE TO ~0 -> Reduce rank to prevent overfitting
        let metrics = TrainingMetrics {
            loss_history: vec![2.0, 1.0, 0.1, 0.01, 0.001],
            previous_params: TunedHyperparams {
                learning_rate: 1e-4,
                epochs: 5,
                lora_rank: 16,
                batch_size: 4,
            },
        };

        let new_params = LoraAutotuner::suggest_hyperparams(&metrics);
        assert!(
            new_params.lora_rank < 16,
            "Rank should decrease to prevent overfitting"
        );
        assert!(
            new_params.epochs < 5,
            "Epochs should decrease to prevent overfitting"
        );
    }
}
