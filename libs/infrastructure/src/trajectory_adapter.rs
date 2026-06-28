/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use crate::AiomeError;
use aiome_core_contracts::traits::ConstitutionalValidator;
use aiome_core_contracts::trajectory::{TrajectoryStep, TrajectoryTriplet};
use std::sync::Arc;

/// Adapter to convert validated TrajectorySteps into TrajectoryTriplets
pub struct TrajectoryToTripletAdapter {
    validator: Arc<dyn ConstitutionalValidator>,
}

impl TrajectoryToTripletAdapter {
    pub fn new(validator: Arc<dyn ConstitutionalValidator>) -> Self {
        Self { validator }
    }

    /// Extacts a Triplet if it passes Constitutional validation
    pub async fn try_convert(
        &self,
        step: &TrajectoryStep,
    ) -> Result<Option<TrajectoryTriplet>, AiomeError> {
        let output_str = step.output.to_string();

        // Security filter: verify with ConstitutionalValidator
        // For trajectory conversion, we use an empty string for soul_md as we just need base safety checks
        match self.validator.verify_constitutional(&output_str, "").await {
            Ok(_) => {
                let triplet = TrajectoryTriplet {
                    prompt: step.input.to_string(),
                    response: output_str,
                    reward_signal: step.reward_signal.unwrap_or(0.0),
                    task_context: None,
                };
                Ok(Some(triplet))
            }
            Err(_) => {
                // Skip if validator rejects
                Ok(None)
            }
        }
    }

    /// Converts steps to triplets and stores high-reward ones in KarmaRegistry
    pub async fn extract_and_store_triplets(
        &self,
        steps: Vec<TrajectoryStep>,
        job_id: &str,
        skill_id: &str,
        soul_hash: &str,
        min_reward_threshold: f64,
        karma_registry: Arc<dyn aiome_core_contracts::traits::KarmaRegistry>,
    ) -> Result<usize, AiomeError> {
        let mut count = 0;

        for step in steps {
            if let Some(reward) = step.reward_signal {
                if reward >= min_reward_threshold {
                    if let Some(triplet) = self.try_convert(&step).await? {
                        let lesson_json = serde_json::to_string(&triplet).map_err(|e| {
                            AiomeError::Infrastructure {
                                reason: format!("Failed to serialize triplet: {}", e),
                            }
                        })?;

                        karma_registry
                            .store_karma(
                                job_id,
                                skill_id,
                                &lesson_json,
                                "trajectory_triplet",
                                soul_hash,
                                None,
                                None,
                                None,
                                true, // is_private (internal RL loop data)
                            )
                            .await?;

                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockConstitutionalValidator {
        should_pass: bool,
    }

    #[async_trait]
    impl ConstitutionalValidator for MockConstitutionalValidator {
        async fn verify_constitutional(
            &self,
            _output: &str,
            _soul_md: &str,
        ) -> Result<(), AiomeError> {
            if self.should_pass {
                Ok(())
            } else {
                Err(AiomeError::Infrastructure {
                    reason: "Constitutional Violation".into(),
                })
            }
        }
    }

    #[tokio::test]
    async fn test_adapter_rejects_unvalidated_step() {
        let validator = Arc::new(MockConstitutionalValidator { should_pass: false });
        let adapter = TrajectoryToTripletAdapter::new(validator);

        let step = TrajectoryStep {
            reward_signal: Some(0.9),
            input: serde_json::json!("test prompt"),
            output: serde_json::json!("test response"),
            ..Default::default()
        };

        let result = adapter.try_convert(&step).await.unwrap();
        assert!(result.is_none(), "Should reject if validator fails");
    }

    #[tokio::test]
    async fn test_adapter_accepts_validated_step() {
        let validator = Arc::new(MockConstitutionalValidator { should_pass: true });
        let adapter = TrajectoryToTripletAdapter::new(validator);

        let step = TrajectoryStep {
            reward_signal: Some(0.9),
            input: serde_json::json!("test prompt"),
            output: serde_json::json!("test response"),
            ..Default::default()
        };

        let result = adapter.try_convert(&step).await.unwrap();
        assert!(result.is_some(), "Should accept if validator passes");

        let triplet = result.unwrap();
        assert_eq!(triplet.reward_signal, 0.9);
        assert_eq!(triplet.prompt, "\"test prompt\"");
        assert_eq!(triplet.response, "\"test response\"");
    }

    #[tokio::test]
    async fn test_extract_and_store_triplets_green() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let validator = Arc::new(MockConstitutionalValidator { should_pass: true });
        let adapter = TrajectoryToTripletAdapter::new(validator);

        let steps = vec![
            TrajectoryStep {
                reward_signal: Some(0.9), // Above threshold
                input: serde_json::json!("prompt 1"),
                output: serde_json::json!("response 1"),
                ..Default::default()
            },
            TrajectoryStep {
                reward_signal: Some(0.3), // Below threshold
                input: serde_json::json!("prompt 2"),
                output: serde_json::json!("response 2"),
                ..Default::default()
            },
            TrajectoryStep {
                reward_signal: None, // No reward
                input: serde_json::json!("prompt 3"),
                output: serde_json::json!("response 3"),
                ..Default::default()
            },
        ];

        #[derive(Debug)]
        struct MockKR {
            pub stored_count: Arc<AtomicUsize>,
        }
        #[async_trait]
        impl aiome_core_contracts::traits::KarmaRegistry for MockKR {
            async fn fetch_relevant_karma(
                &self,
                _: &str,
                _: &str,
                _: i64,
                _: &str,
            ) -> Result<aiome_core_contracts::traits::KarmaSearchResult, AiomeError> {
                Ok(aiome_core_contracts::traits::KarmaSearchResult::empty())
            }
            async fn store_karma(
                &self,
                _: &str,
                _: &str,
                _: &str,
                _: &str,
                _: &str,
                _: Option<&str>,
                _: Option<&str>,
                _: Option<&str>,
                _: bool,
            ) -> Result<(), AiomeError> {
                self.stored_count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            async fn adjust_karma_weight(&self, _: &str, _: i32) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn karma_decay_sweep(&self) -> Result<u64, AiomeError> {
                Ok(0)
            }
            async fn fetch_undistilled_jobs(
                &self,
                _: i64,
            ) -> Result<Vec<aiome_core_contracts::traits::Job>, AiomeError> {
                Ok(vec![])
            }
            async fn mark_karma_extracted(&self, _: &str) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn fetch_all_karma(&self, _: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
                Ok(vec![])
            }
            async fn fetch_unincorporated_karma(
                &self,
                _: i64,
                _: &str,
            ) -> Result<Vec<serde_json::Value>, AiomeError> {
                Ok(vec![])
            }
            async fn mark_karma_as_incorporated(
                &self,
                _: Vec<String>,
                _: &str,
            ) -> Result<(), AiomeError> {
                Ok(())
            }
            async fn fetch_relevant_karma_by_category(
                &self,
                _: &str,
                _: &str,
                _: i64,
            ) -> Result<aiome_core_contracts::traits::KarmaSearchResult, AiomeError> {
                Ok(aiome_core_contracts::traits::KarmaSearchResult::empty())
            }
            async fn recall_from_slm(
                &self,
                _: &str,
                _: i64,
            ) -> Result<aiome_core_contracts::traits::KarmaSearchResult, AiomeError> {
                Ok(aiome_core_contracts::traits::KarmaSearchResult::empty())
            }
        }

        let stored_count = Arc::new(AtomicUsize::new(0));
        let kr = Arc::new(MockKR {
            stored_count: stored_count.clone(),
        });

        let result = adapter
            .extract_and_store_triplets(steps, "job-1", "skill-1", "hash-1", 0.5, kr)
            .await;

        assert!(result.is_ok(), "Method should return Ok(count)");
        let count = result.unwrap();
        assert_eq!(count, 1, "Only 1 step is above the threshold of 0.5");
        assert_eq!(
            stored_count.load(Ordering::SeqCst),
            1,
            "store_karma should have been called 1 time"
        );
    }
}
