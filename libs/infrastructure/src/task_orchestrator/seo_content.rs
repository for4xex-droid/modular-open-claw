/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Apache License, Version 2.0.
 */
use crate::task_orchestrator::{TaskConductor, TaskEvent};
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use aiome_core_contracts::traits::Job;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

pub struct SeoContentConductor {
    llm: Arc<dyn LlmProvider>,
}

impl SeoContentConductor {
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self { llm }
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

        if let Err(e) = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: "SEO Content Generation complete.".to_string(),
                percent: Some(100),
            })
            .await
        {
            tracing::warn!(
                "Failed to send completion event for SEO job {}: {}",
                job.id,
                e
            );
        }

        Ok((response.content, None))
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
            *self.last_user_prompt.lock().expect("lock poisoned") = Some(prompt.to_string()); // allow-anti-pattern
            *self.last_sys_prompt.lock().expect("lock poisoned") = // allow-anti-pattern
                Some(sys.unwrap_or("").to_string());
            Ok(LlmResponse {
                content: "SEO content...".to_string(),
                metadata: None,
                reasoning: None,
                stop_reason: StopReason::EndTurn,
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
        let conductor = SeoContentConductor::new(provider);
        assert_eq!(
            conductor.capable_categories(),
            vec!["seo_content".to_string()]
        );
        assert_eq!(conductor.conductor_name(), "SeoContentConductor");
    }

    #[tokio::test]
    async fn test_seo_content_conductor_conduct() {
        let provider = Arc::new(CapturingProvider::new());
        let conductor = SeoContentConductor::new(provider);

        let (tx, mut rx) = mpsc::channel(10);
        let mut job = Job::default();
        job.id = "test-job-1".to_string();
        job.topic = "Test Topic".to_string();
        job.category = "seo_content".to_string();

        let result = conductor.conduct(job, tx).await;
        assert!(result.is_ok());
        let (content, karma) = result.expect("conduct should succeed"); // allow-anti-pattern
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
        let conductor = SeoContentConductor::new(provider.clone());

        let (tx, _rx) = mpsc::channel(10);
        let mut job = Job::default();
        job.id = "test-prompt-1".to_string();
        job.topic = "Rust Web Development".to_string();

        conductor
            .conduct(job, tx)
            .await
            .expect("conduct should succeed before asserting prompt content"); // allow-anti-pattern

        let captured_sys = provider.last_sys_prompt.lock().expect("lock poisoned"); // allow-anti-pattern
        let sys = captured_sys
            .as_deref()
            .expect("system prompt must be passed"); // allow-anti-pattern
        assert!(
            sys.contains("SEO") && sys.contains("Content Strategist"),
            "System prompt must contain SEO domain expertise. Got: {}",
            sys
        );

        let captured_user = provider.last_user_prompt.lock().expect("lock poisoned"); // allow-anti-pattern
        let user = captured_user
            .as_deref()
            .expect("user prompt must be passed"); // allow-anti-pattern
        assert!(
            user.contains("Rust Web Development"),
            "User prompt must include the job topic. Got: {}",
            user
        );
    }

    #[tokio::test]
    async fn test_seo_content_conductor_empty_topic() {
        let provider = Arc::new(CapturingProvider::new());
        let conductor = SeoContentConductor::new(provider);

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
        let conductor = SeoContentConductor::new(provider);

        let (tx, _rx) = mpsc::channel(10);
        let mut job = Job::default();
        job.id = "test-fail-1".to_string();
        job.topic = "Valid Topic".to_string();

        let result = conductor.conduct(job, tx).await;
        assert!(result.is_err(), "LLM failure must propagate as error");
    }
}
