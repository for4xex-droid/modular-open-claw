/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
use crate::task_orchestrator::{TaskConductor, TaskEvent};
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use aiome_core_contracts::traits::Job;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

use crate::publisher::PublishPipeline;

pub struct SeoContentConductor {
    llm: Arc<dyn LlmProvider>,
    publish_pipeline: Arc<PublishPipeline>,
    geo_enabled: bool,
    geo_url: String,
    geo_threshold: u32,
}

impl SeoContentConductor {
    pub fn new(
        llm: Arc<dyn LlmProvider>,
        publish_pipeline: Arc<PublishPipeline>,
        geo_enabled: bool,
        geo_url: String,
        geo_threshold: u32,
    ) -> Self {
        Self {
            llm,
            publish_pipeline,
            geo_enabled,
            geo_url,
            geo_threshold,
        }
    }
}

#[async_trait]
impl TaskConductor for SeoContentConductor {
    fn capable_categories(&self) -> Vec<String> {
        vec!["seo_content".to_string()]
    }

    fn conductor_name(&self) -> &str {
        "SeoContentConductor"
    }

    async fn conduct(
        &self,
        job: Job,
        progress_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<(String, Option<String>), AiomeError> {
        info!("📈 [SEO] Executing content generation job: {}", job.topic);

        if job.topic.trim().is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "SEO topic cannot be empty".to_string(),
            });
        }

        if let Err(e) = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: format!(
                    "Analyzing SEO intent and crafting content for topic: {}",
                    job.topic
                ),
                percent: Some(20),
            })
            .await
        {
            tracing::warn!(
                "Failed to send progress event for SEO job {}: {}",
                job.id,
                e
            );
        }

        // Domain-specific specialized prompt for SEO logic
        let system_prompt = "You are an expert SEO Content Strategist. Your goal is to produce highly optimized, user-centric, and search-engine friendly content. Follow SEO best practices (H1, meta descriptions, semantic HTML/Markdown, keyword density).";

        let prompt = format!(
            "Generate an SEO optimized article for the following topic: {}\nEnsure the output is ready for direct publishing.",
            job.topic
        );

        let response = self.llm.complete(&prompt, Some(system_prompt)).await?;

        let mut final_content = response.content;
        let mut geo_passed = true;

        if self.geo_enabled {
            if progress_tx
                .send(TaskEvent::Progress {
                    job_id: job.id.clone(),
                    conductor_id: self.conductor_name().to_string(),
                    message: "Running Generative Engine Optimization (GEO) audit...".to_string(),
                    percent: Some(80),
                })
                .await
                .is_err()
            {
                tracing::debug!("[SEO] Progress receiver dropped during GEO audit");
            }

            let payload = serde_json::json!({
                "content": final_content,
                "topic": job.topic
            });

            // SEC: Use global SSRF-protected HTTP client with per-request timeout
            let client = aiome_core::http::get_http_client();
            match client
                .post(&self.geo_url)
                .timeout(std::time::Duration::from_secs(30))
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    match resp.json::<serde_json::Value>().await {
                        Ok(geo_result) => {
                            let score = (geo_result["score"].as_u64().unwrap_or(0)).min(100) as u32;
                            geo_passed = score >= self.geo_threshold;

                            if let Some(optimized) = geo_result["optimized_content"].as_str() {
                                final_content = optimized.to_string();
                            }

                            if progress_tx
                                .send(TaskEvent::QualityGate {
                                    job_id: job.id.clone(),
                                    score,
                                    passed: geo_passed,
                                    conductor: self.conductor_name().to_string(),
                                    review_decision: None,
                                    feedback: None,
                                })
                                .await
                                .is_err()
                            {
                                tracing::debug!(
                                    "[SEO] Progress receiver dropped during QualityGate"
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "GEO Optimizer returned invalid JSON (graceful degradation): {}",
                                e
                            );
                            if progress_tx
                                .send(TaskEvent::QualityGate {
                                    job_id: job.id.clone(),
                                    score: 0,
                                    passed: true, // graceful degradation: don't block on parse error
                                    conductor: self.conductor_name().to_string(),
                                    review_decision: None,
                                    feedback: None,
                                })
                                .await
                                .is_err()
                            {
                                tracing::debug!("[SEO] Progress receiver dropped during QualityGate (JSON fallback)");
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("GEO Optimizer audit failed (graceful degradation): {}", e);
                    // Emit QualityGate with score 0 + passed=true to signal unavailability
                    if progress_tx
                        .send(TaskEvent::QualityGate {
                            job_id: job.id.clone(),
                            score: 0,
                            passed: true, // graceful degradation: publish despite GEO unavailability
                            conductor: self.conductor_name().to_string(),
                            review_decision: None,
                            feedback: None,
                        })
                        .await
                        .is_err()
                    {
                        tracing::debug!(
                            "[SEO] Progress receiver dropped during QualityGate (GEO unavailable)"
                        );
                    }
                }
                Ok(resp) => {
                    tracing::warn!(
                        "GEO Optimizer returned error status (graceful degradation): {}",
                        resp.status()
                    );
                    if progress_tx
                        .send(TaskEvent::QualityGate {
                            job_id: job.id.clone(),
                            score: 0,
                            passed: true, // graceful degradation: publish despite GEO error
                            conductor: self.conductor_name().to_string(),
                            review_decision: None,
                            feedback: None,
                        })
                        .await
                        .is_err()
                    {
                        tracing::debug!(
                            "[SEO] Progress receiver dropped during QualityGate (GEO error)"
                        );
                    }
                }
            }
        }

        if !geo_passed {
            if progress_tx.send(TaskEvent::AwaitingInput {
                job_id: job.id.clone(),
                reason: "GEO Score below threshold. Content requires correction to improve citability.".to_string(),
            }).await.is_err() {
                tracing::debug!("[SEO] Progress receiver dropped during AwaitingInput");
            }
            return Err(AiomeError::Infrastructure {
                reason: format!(
                    "GEO optimization failed to meet the threshold of {}. \
                     Watchtower will now trigger an autonomous re-generation cycle.",
                    self.geo_threshold
                ),
            });
        }

        if self.geo_enabled {
            // [R5] PublishPipeline's first caller: Autonomous SEO publishing if GEO passed
            let publish_meta = serde_json::json!({ "topic": job.topic });
            if let Err(e) = self
                .publish_pipeline
                .run_job("wordpress", &final_content, &[], &publish_meta)
                .await
            {
                tracing::warn!("Failed to publish SEO content: {}", e);
            } else {
                tracing::info!("SEO content published successfully to wordpress.");
            }
        }

        if progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: "SEO Content Generation complete.".to_string(),
                percent: Some(100),
            })
            .await
            .is_err()
        {
            tracing::debug!("[SEO] Progress receiver dropped during completion");
        }

        Ok((final_content, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core::llm_provider::LlmProvider;
    use aiome_core_contracts::llm::{LlmResponse, StopReason};
    use tokio::sync::mpsc;

    use std::sync::Mutex;

    #[derive(Debug)]
    struct CapturingProvider {
        last_sys_prompt: Mutex<Option<String>>,
        last_user_prompt: Mutex<Option<String>>,
    }

    impl CapturingProvider {
        fn new() -> Self {
            Self {
                last_sys_prompt: Mutex::new(None),
                last_user_prompt: Mutex::new(None),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmProvider for CapturingProvider {
        async fn complete(
            &self,
            prompt: &str,
            sys: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            *self.last_user_prompt.lock().expect("lock poisoned") = Some(prompt.to_string());
            *self.last_sys_prompt.lock().expect("lock poisoned") =
                Some(sys.unwrap_or("").to_string());
            Ok(LlmResponse {
                content: "SEO content...".to_string(),

                stop_reason: StopReason::EndTurn,
                ..Default::default()
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Ok(())
        }
        fn name(&self) -> &str {
            "CapturingProvider"
        }
    }

    #[tokio::test]
    async fn test_seo_content_conductor_categories() {
        let provider = Arc::new(CapturingProvider::new());
        let publish_pipeline = Arc::new(crate::publisher::PublishPipeline::new(vec![]));
        let conductor = SeoContentConductor::new(
            provider,
            publish_pipeline,
            false,
            "http://localhost:8080".to_string(),
            60,
        );
        assert_eq!(
            conductor.capable_categories(),
            vec!["seo_content".to_string()]
        );
        assert_eq!(conductor.conductor_name(), "SeoContentConductor");
    }

    #[tokio::test]
    async fn test_seo_content_conductor_conduct() {
        let provider = Arc::new(CapturingProvider::new());
        let publish_pipeline = Arc::new(crate::publisher::PublishPipeline::new(vec![]));
        let conductor = SeoContentConductor::new(
            provider,
            publish_pipeline,
            false,
            "http://localhost:8080".to_string(),
            60,
        );

        let (tx, mut rx) = mpsc::channel(10);
        let mut job = Job::default();
        job.id = "test-job-1".to_string();
        job.topic = "Test Topic".to_string();
        job.category = "seo_content".to_string();

        let result = conductor.conduct(job, tx).await;
        assert!(result.is_ok());
        let (content, karma) = result.expect("conduct should succeed");
        assert_eq!(content, "SEO content...");
        assert!(karma.is_none());

        // Verify progress events
        let mut events = Vec::new();
        while let Ok(evt) = rx.try_recv() {
            events.push(evt);
        }
        assert_eq!(
            events.len(),
            2,
            "Should have sending and completion progress events"
        );
    }

    /// Core architectural contract: SeoContentConductor MUST pass a domain-specific
    /// system prompt to the LLM. Without this, it's just GenericLlmConductor with
    /// extra steps. This test is the reason SeoContentConductor exists as a separate type.
    #[tokio::test]
    async fn test_seo_content_conductor_injects_system_prompt() {
        let provider = Arc::new(CapturingProvider::new());
        let publish_pipeline = Arc::new(crate::publisher::PublishPipeline::new(vec![]));
        let conductor = SeoContentConductor::new(
            provider.clone(),
            publish_pipeline,
            false,
            "http://localhost:8080".to_string(),
            60,
        );

        let (tx, _rx) = mpsc::channel(10);
        let mut job = Job::default();
        job.id = "test-prompt-1".to_string();
        job.topic = "Rust Web Development".to_string();

        conductor
            .conduct(job, tx)
            .await
            .expect("conduct should succeed before asserting prompt content");

        let captured_sys = provider.last_sys_prompt.lock().expect("lock poisoned");
        let sys = captured_sys
            .as_deref()
            .expect("system prompt must be passed");
        assert!(
            sys.contains("SEO") && sys.contains("Content Strategist"),
            "System prompt must contain SEO domain expertise. Got: {}",
            sys
        );

        let captured_user = provider.last_user_prompt.lock().expect("lock poisoned");
        let user = captured_user
            .as_deref()
            .expect("user prompt must be passed");
        assert!(
            user.contains("Rust Web Development"),
            "User prompt must include the job topic. Got: {}",
            user
        );
    }

    #[tokio::test]
    async fn test_seo_content_conductor_empty_topic() {
        let provider = Arc::new(CapturingProvider::new());
        let publish_pipeline = Arc::new(crate::publisher::PublishPipeline::new(vec![]));
        let conductor = SeoContentConductor::new(
            provider,
            publish_pipeline,
            false,
            "http://localhost:8080".to_string(),
            60,
        );

        let (tx, _rx) = mpsc::channel(10);

        // Empty string
        let mut job = Job::default();
        job.id = "test-empty-1".to_string();
        job.topic = "".to_string();
        let result = conductor.conduct(job, tx.clone()).await;
        assert!(result.is_err(), "Empty topic must be rejected");

        // Whitespace-only string
        let mut job2 = Job::default();
        job2.id = "test-empty-2".to_string();
        job2.topic = "   \t\n  ".to_string();
        let result2 = conductor.conduct(job2, tx).await;
        assert!(result2.is_err(), "Whitespace-only topic must be rejected");
    }

    #[derive(Debug)]
    struct FailingProvider;
    #[async_trait::async_trait]
    impl LlmProvider for FailingProvider {
        async fn complete(
            &self,
            _prompt: &str,
            _sys: Option<&str>,
        ) -> Result<LlmResponse, AiomeError> {
            Err(AiomeError::Infrastructure {
                reason: "LLM unavailable".to_string(),
            })
        }
        async fn test_connection(&self) -> Result<(), AiomeError> {
            Err(AiomeError::Infrastructure {
                reason: "down".to_string(),
            })
        }
        fn name(&self) -> &str {
            "FailingProvider"
        }
    }

    #[tokio::test]
    async fn test_seo_content_conductor_llm_failure() {
        let provider = Arc::new(FailingProvider);
        let publish_pipeline = Arc::new(crate::publisher::PublishPipeline::new(vec![]));
        let conductor = SeoContentConductor::new(
            provider,
            publish_pipeline,
            false,
            "http://localhost:8080".to_string(),
            60,
        );

        let (tx, _rx) = mpsc::channel(10);
        let mut job = Job::default();
        job.id = "test-fail-1".to_string();
        job.topic = "Valid Topic".to_string();

        let result = conductor.conduct(job, tx).await;
        assert!(result.is_err(), "LLM failure must propagate as error");
    }
}
