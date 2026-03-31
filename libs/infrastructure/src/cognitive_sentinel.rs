use aiome_contracts::traits::JobQueue;
use aiome_core::error::AiomeError;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct CognitiveThresholds {
    pub min_somatic_count: usize,
    pub variance_alert_threshold: f64,
    pub max_defense_ratio: f64,
}

impl Default for CognitiveThresholds {
    fn default() -> Self {
        Self {
            min_somatic_count: 5,
            variance_alert_threshold: 0.1, // If variance is lower than this, it's flatlined (catatonic)
            max_defense_ratio: 0.3,        // If > 30% of recent jobs are Defense mechanisms
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

        // 2. Fetch recent jobs to measure failure rate
        let recent_jobs = job_queue.fetch_recent_jobs(50).await?;
        if recent_jobs.len() >= 10 {
            let failed_count = recent_jobs
                .iter()
                .filter(|j| j.status == aiome_contracts::traits::JobStatus::Failed)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::job_queue_mock::GlobalMockJobQueue;
    use serde_json::json;

    #[tokio::test]
    async fn test_diagnose_skips_when_insufficient_data() {
        let sentinel = CognitiveSentinel::new(CognitiveThresholds::default());
        let mock_queue = GlobalMockJobQueue::default();

        // Only 3 karmas (Threshold is 5)
        *mock_queue.karmas.lock().unwrap() = vec![
            json!({"somatic_valence": 0.5}),
            json!({"somatic_valence": -0.2}),
            json!({"somatic_valence": 0.1}),
        ];

        let result = sentinel.diagnose(&mock_queue, "agent_1").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_diagnose_detects_catatonic_state() {
        let sentinel = CognitiveSentinel::new(CognitiveThresholds::default());
        let mock_queue = GlobalMockJobQueue::default();

        // 5 karmas with exact same valence (variance = 0.0) -> flatline
        *mock_queue.karmas.lock().unwrap() = vec![
            json!({"somatic_valence": 0.5}),
            json!({"somatic_valence": 0.5}),
            json!({"somatic_valence": 0.5}),
            json!({"somatic_valence": 0.5}),
            json!({"somatic_valence": 0.5}),
        ];

        let result = sentinel.diagnose(&mock_queue, "agent_1").await.unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("catatonic"));
    }
}
