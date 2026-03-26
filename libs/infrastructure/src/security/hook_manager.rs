/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use aiome_contracts::error::AiomeError;
use aiome_contracts::llm::{LlmRequest, LlmResponse};
use aiome_contracts::security::AgentHook;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug)]
pub struct HookManager {
    hooks: Vec<Arc<dyn AgentHook>>,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiome_contracts::llm::LlmMessage;

    #[derive(Debug)]
    struct MockHook {
        pre_called: std::sync::atomic::AtomicBool,
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
    }

    #[tokio::test]
    async fn test_hook_manager_executes_hooks() {
        let mut manager = HookManager::new();
        let hook = Arc::new(MockHook {
            pre_called: std::sync::atomic::AtomicBool::new(false),
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
            .expect("Hook should pass");
        assert!(hook.pre_called.load(std::sync::atomic::Ordering::SeqCst));
    }
}
