/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use aiome_core::error::AiomeError;
use aiome_core_contracts::traits::JobQueue;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct CognitiveThresholds {
    pub min_somatic_count: usize,
    pub variance_alert_threshold: f64,
    pub max_defense_ratio: f64,
    pub min_entropy_threshold: f64,
    pub min_entropy_sample_size: usize,
}

impl Default for CognitiveThresholds {
    fn default() -> Self {
        Self {
            min_somatic_count: 5,
            variance_alert_threshold: 0.1, // If variance is lower than this, it's flatlined (catatonic)
            max_defense_ratio: 0.3,        // If > 30% of recent jobs are Defense mechanisms
            min_entropy_threshold: 0.5,
            min_entropy_sample_size: 50,
        }
    }
}

pub struct CognitiveSentinel {
    thresholds: CognitiveThresholds,
}

impl CognitiveSentinel {
    pub fn new(thresholds: CognitiveThresholds) -> Self {
        Self { thresholds }
    }

    /// Evaluates the agent's mental health based on SomaticMarkers and other context.
    pub async fn diagnose(
        &self,
        job_queue: &dyn JobQueue,
        _agent_id: &str,
    ) -> Result<Option<String>, AiomeError> {
        // Fetch recent karmas
        let karmas = job_queue.fetch_all_karma(100).await?;

        let valences: Vec<f64> = karmas
            .iter()
            .filter_map(|k| k.get("somatic_valence").and_then(|v| v.as_f64()))
            .filter(|&v| v.is_finite()) // RED TEAM: Block NaN/Inf attacks
            .collect();

        if valences.len() < self.thresholds.min_somatic_count {
            return Ok(None);
        }

        let count = valences.len() as f64;
        let mean = valences.iter().copied().sum::<f64>() / count;
        let variance = valences
            .iter()
            .map(|&v| (v - mean) * (v - mean))
            .sum::<f64>()
            / count;

        if variance <= self.thresholds.variance_alert_threshold {
            return Ok(Some(format!(
                "⚠️ Agent is in a catatonic state. Somatic variance ({:.4}) is below threshold.",
                variance
            )));
        }

        // 2. Entropy Check (Shannon Entropy of somatic valences)
        if valences.len() >= self.thresholds.min_entropy_sample_size {
            let entropy = self.calculate_entropy(&valences);
            if entropy < self.thresholds.min_entropy_threshold {
                return Ok(Some(format!(
                    "⚠️ Agent cognitive entropy ({:.4}) is dangerously low. Stagnation detected.",
                    entropy
                )));
            }
        }

        // 3. Fetch recent jobs to measure failure rate
        let recent_jobs = job_queue.fetch_recent_jobs(50).await?;
        if recent_jobs.len() >= 10 {
            let failed_count = recent_jobs
                .iter()
                .filter(|j| j.status == aiome_core_contracts::traits::JobStatus::Failed)
                .count();
            let fail_rate = failed_count as f64 / recent_jobs.len() as f64;

            if fail_rate > 0.6 {
                return Ok(Some(format!(
                    "🚨 Agent Panic State Detected. Recent failure rate is {:.1}% ({} out of {}). Safety mechanisms engaged.",
                    fail_rate * 100.0,
                    failed_count,
                    recent_jobs.len()
                )));
            }
        }

        Ok(None)
    }

    fn calculate_entropy(&self, valences: &[f64]) -> f64 {
        if valences.is_empty() {
            return 0.0;
        }

        let num_bins = 10;
        let mut counts = vec![0usize; num_bins];
        for &v in valences {
            let clamped = v.clamp(-1.0, 1.0);
            let normalized = (clamped + 1.0) / 2.0;
            // Clamp bin index to [0, num_bins-1] to prevent OOB on edge-case floats
            let bin = ((normalized * num_bins as f64).floor() as usize).min(num_bins - 1);
            counts[bin] += 1;
        }

        let mut entropy = 0.0;
        let total = valences.len() as f64;

        for count in counts {
            if count > 0 {
                let p = (count as f64) / total;
                entropy -= p * p.log2();
            }
        }

        entropy
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::test_utils::job_queue_mock::GlobalMockJobQueue;
    use serde_json::json;

    #[tokio::test]
    async fn test_diagnose_skips_when_insufficient_data() {
        let sentinel = CognitiveSentinel::new(CognitiveThresholds::default());
        let mock_queue = GlobalMockJobQueue::default();

        // Only 3 karmas (Threshold is 5)
        *mock_queue.karmas.lock().unwrap() /* allow-anti-pattern */ = vec![
            // allow-anti-pattern
            json!({"somatic_valence": 0.5}),
            json!({"somatic_valence": -0.2}),
            json!({"somatic_valence": 0.1}),
        ];

        let result = sentinel.diagnose(&mock_queue, "agent_1").await.unwrap(); // allow-anti-pattern
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_diagnose_detects_catatonic_state() {
        let sentinel = CognitiveSentinel::new(CognitiveThresholds::default());
        let mock_queue = GlobalMockJobQueue::default();

        // 5 karmas with exact same valence (variance = 0.0) -> flatline
        *mock_queue.karmas.lock().unwrap() /* allow-anti-pattern */ = vec![
            // allow-anti-pattern
            json!({"somatic_valence": 0.5}),
            json!({"somatic_valence": 0.5}),
            json!({"somatic_valence": 0.5}),
            json!({"somatic_valence": 0.5}),
            json!({"somatic_valence": 0.5}),
        ];

        let result = sentinel.diagnose(&mock_queue, "agent_1").await.unwrap(); // allow-anti-pattern
        assert!(result.is_some());
        assert!(result.unwrap().contains("catatonic")); // allow-anti-pattern
    }

    #[tokio::test]
    async fn test_diagnose_detects_low_entropy() {
        let mut thresholds = CognitiveThresholds::default();
        thresholds.min_entropy_sample_size = 5; // drop it for testing
                                                // ensure variance passes so we can reach entropy check
        thresholds.variance_alert_threshold = -1.0;

        let sentinel = CognitiveSentinel::new(thresholds);
        let mock_queue = GlobalMockJobQueue::default();

        // 5 karmas with low variety, but variance > -1.0
        *mock_queue.karmas.lock().unwrap() /* allow-anti-pattern */ = vec![
            json!({"somatic_valence": 0.5}),
            json!({"somatic_valence": 0.5}),
            json!({"somatic_valence": 0.5}),
            json!({"somatic_valence": 0.5}),
            json!({"somatic_valence": 0.5}),
        ];

        let result = sentinel.diagnose(&mock_queue, "agent_1").await.unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("entropy"));
    }

    #[test]
    fn test_calculate_entropy_uniform_distribution() {
        let sentinel = CognitiveSentinel::new(CognitiveThresholds::default());
        // 10 values spread uniformly across bins → max entropy = log2(10) ≈ 3.32
        let valences: Vec<f64> = (0..10).map(|i| -1.0 + (i as f64 * 0.2)).collect();
        let entropy = sentinel.calculate_entropy(&valences);
        assert!(
            entropy > 3.0,
            "Uniform distribution should have high entropy, got {}",
            entropy
        );
    }

    #[test]
    fn test_calculate_entropy_single_bin() {
        let sentinel = CognitiveSentinel::new(CognitiveThresholds::default());
        // All same value → entropy = 0
        let valences = vec![0.5; 100];
        let entropy = sentinel.calculate_entropy(&valences);
        assert!(
            (entropy - 0.0).abs() < 1e-10,
            "Single-bin entropy should be 0.0, got {}",
            entropy
        );
    }

    #[test]
    fn test_calculate_entropy_boundary_values() {
        let sentinel = CognitiveSentinel::new(CognitiveThresholds::default());
        // Edge values at exact -1.0 and 1.0 must not panic
        let valences = vec![-1.0, 1.0, -1.0, 1.0];
        let entropy = sentinel.calculate_entropy(&valences);
        assert!(
            entropy >= 0.0,
            "Entropy must be non-negative, got {}",
            entropy
        );
    }

    #[test]
    fn test_calculate_entropy_empty() {
        let sentinel = CognitiveSentinel::new(CognitiveThresholds::default());
        let entropy = sentinel.calculate_entropy(&[]);
        assert!((entropy - 0.0).abs() < 1e-10);
    }
}
