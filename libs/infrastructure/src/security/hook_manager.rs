/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * Licensed under the Business Source License 1.1.
 */

use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::{LlmRequest, LlmResponse};
use aiome_core_contracts::security::AgentHook;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug)]
pub struct HookManager {
    hooks: Vec<Arc<dyn AgentHook>>,
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HookManager {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn add_hook(&mut self, hook: Arc<dyn AgentHook>) {
        self.hooks.push(hook);
    }

    pub async fn trigger_pre_execute(&self, request: &LlmRequest) -> Result<(), AiomeError> {
        for hook in &self.hooks {
            hook.on_pre_execute(request).await?;
        }
        Ok(())
    }

    pub async fn trigger_post_execute(
        &self,
        request: &LlmRequest,
        response: &LlmResponse,
    ) -> Result<(), AiomeError> {
        for hook in &self.hooks {
            hook.on_post_execute(request, response).await?;
        }
        Ok(())
    }

    /// ジョブ完了時のフック通知（ベストエフォート）。
    /// `on_pre_execute` と異なり、1つのフックの失敗が残りのフックをブロックしない。
    pub async fn trigger_job_completed(
        &self,
        job_id: &str,
        status: &str,
    ) -> Result<(), AiomeError> {
        let mut last_error: Option<AiomeError> = None;
        for hook in &self.hooks {
            if let Err(e) = hook.on_job_completed(job_id, status).await {
                tracing::warn!(
                    "⚠️ [HookManager] Hook {:?} failed on job_completed({}): {}",
                    hook,
                    job_id,
                    e
                );
                last_error = Some(e);
            }
        }
        match last_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core_contracts::llm::LlmMessage;

    #[derive(Debug)]
    struct MockHook {
        pre_called: std::sync::atomic::AtomicBool,
        job_completed_called: std::sync::atomic::AtomicBool,
    }

    #[async_trait]
    impl AgentHook for MockHook {
        async fn on_pre_execute(&self, _request: &LlmRequest) -> Result<(), AiomeError> {
            self.pre_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        async fn on_post_execute(
            &self,
            _request: &LlmRequest,
            _response: &LlmResponse,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn on_job_completed(&self, _job_id: &str, _status: &str) -> Result<(), AiomeError> {
            self.job_completed_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_hook_manager_executes_hooks() {
        let mut manager = HookManager::new();
        let hook = Arc::new(MockHook {
            pre_called: std::sync::atomic::AtomicBool::new(false),
            job_completed_called: std::sync::atomic::AtomicBool::new(false),
        });
        manager.add_hook(hook.clone());

        let request = LlmRequest {
            messages: vec![LlmMessage {
                role: "user".to_string(),
                content: "test".to_string(),
                cache: false,
            }],
            temperature: None,
            max_tokens: None,
            stop_sequences: None,
            format: None,
            metadata: None,
        };

        manager
            .trigger_pre_execute(&request)
            .await
            .expect("Hook should pass"); // allow-anti-pattern
        assert!(hook.pre_called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_hook_manager_executes_job_completed_hooks() {
        let mut manager = HookManager::new();
        let hook = Arc::new(MockHook {
            pre_called: std::sync::atomic::AtomicBool::new(false),
            job_completed_called: std::sync::atomic::AtomicBool::new(false),
        });
        manager.add_hook(hook.clone());

        manager
            .trigger_job_completed("job-42", "completed")
            .await
            .expect("Hook should pass");
        assert!(hook
            .job_completed_called
            .load(std::sync::atomic::Ordering::SeqCst));
    }
}
