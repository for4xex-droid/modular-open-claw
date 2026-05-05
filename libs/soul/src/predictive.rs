/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveModel {
    pub domains: HashMap<String, DomainModel>,
    pub global_surprise_sensitivity: f64,
    pub ema_learning_rate: f64,
}

impl Default for PredictiveModel {
    fn default() -> Self {
        Self {
            domains: HashMap::new(),
            global_surprise_sensitivity: 0.5,
            ema_learning_rate: 0.05,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainModel {
    pub prediction_accuracy: f64, // 0.0-1.0 (high = accurate)
    pub experience_count: u64,
    pub local_plasticity: f64, // High acc -> low plastic (stubborn), Low acc -> high plastic (flexible)
    pub last_surprise: f64,    // Latest error
}

impl Default for DomainModel {
    fn default() -> Self {
        Self {
            prediction_accuracy: 0.5,
            experience_count: 0,
            local_plasticity: 0.5,
            last_surprise: 0.0,
        }
    }
}

impl PredictiveModel {
    pub fn update_plasticity(&mut self, domain: &str, actual_outcome: f64, predicted: f64) {
        let surprise = (actual_outcome - predicted).abs();
        let dm = self.domains.entry(domain.to_string()).or_default();

        // Accurate prediction means less plasticity (more stubborn)
        // Inaccurate prediction means more plasticity (more flexible)
        let alpha = self.ema_learning_rate;
        dm.prediction_accuracy =
            dm.prediction_accuracy * (1.0 - alpha) + (1.0 - surprise.clamp(0.0, 1.0)) * alpha;
        dm.local_plasticity = 1.0 - dm.prediction_accuracy;
        dm.last_surprise = surprise;
        dm.experience_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predictive_model_update_plasticity() {
        let mut p = PredictiveModel::default();
        p.ema_learning_rate = 0.1;

        // Accurate prediction
        p.update_plasticity("test", 1.0, 1.0); // surprise = 0
        let dm_acc = p.domains.get("test").unwrap().prediction_accuracy;
        assert!(dm_acc > 0.5); // should increase from default 0.5

        // Inaccurate prediction
        p.update_plasticity("test", 0.0, 1.0); // surprise = 1
        let dm2 = p.domains.get("test").unwrap();
        assert!(dm2.prediction_accuracy < dm_acc);
    }
}
