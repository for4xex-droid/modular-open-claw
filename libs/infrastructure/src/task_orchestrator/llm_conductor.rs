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

pub struct GenericLlmConductor {
    llm: Arc<dyn LlmProvider>,
    categories: Vec<String>,
}

impl GenericLlmConductor {
    pub fn new(llm: Arc<dyn LlmProvider>, categories: Vec<&str>) -> Self {
        Self {
            llm,
            categories: categories.into_iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[async_trait]
impl TaskConductor for GenericLlmConductor {
    fn capable_categories(&self) -> Vec<String> {
        self.categories.clone()
    }

    fn conductor_name(&self) -> &str {
        "GenericLlmConductor"
    }

    async fn conduct(
        &self,
        job: Job,
        progress_tx: mpsc::Sender<TaskEvent>,
    ) -> Result<(String, Option<String>), AiomeError> {
        info!("🧠 [GenericLlm] Executing job: {}", job.topic);

        if let Err(e) = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: format!("Starting generic LLM task for topic: {}", job.topic),
                percent: Some(10),
            })
            .await
        {
            tracing::warn!("Failed to send progress event: {}", e);
        }

        let prompt = format!(
            "Please perform the following {} task:\nTopic: {}",
            job.category, job.topic
        );

        let response = self.llm.complete(&prompt, None).await?;

        if let Err(e) = progress_tx
            .send(TaskEvent::Progress {
                job_id: job.id.clone(),
                conductor_id: self.conductor_name().to_string(),
                message: "Task complete.".to_string(),
                percent: Some(100),
            })
            .await
        {
            tracing::warn!("Failed to send completion event: {}", e);
        }

        // Return (Content, ExtractedKarma/Directives)
        Ok((response.content, None))
    }
}
