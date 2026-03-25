use aiome_contracts::security::AgentHook;
use aiome_contracts::llm::{LlmRequest, LlmResponse};
use aiome_contracts::error::AiomeError;
use async_trait::async_trait;

#[derive(Debug)]
struct MockSecurityHook;

#[async_trait]
impl AgentHook for MockSecurityHook {
    async fn on_pre_execute(&self, _request: &LlmRequest) -> Result<(), AiomeError> {
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
async fn test_agent_hook_trait_exists() {
    let hook = MockSecurityHook;
    // This should fail to compile if AgentHook is not defined
}
